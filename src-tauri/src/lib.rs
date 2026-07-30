use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::os::windows::process::CommandExt;
use tauri::Manager;

// --- Types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub name: String,
    pub installed: bool,
    pub installed_version: Option<String>,
    pub latest_version: Option<String>,
    pub running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub tg_proxy: ServiceStatus,
    pub zapret: ServiceStatus,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            tg_proxy: ServiceStatus {
                name: "tg-ws-proxy".into(),
                installed: false,
                installed_version: None,
                latest_version: None,
                running: false,
            },
            zapret: ServiceStatus {
                name: "zapret-discord-youtube".into(),
                installed: false,
                installed_version: None,
                latest_version: None,
                running: false,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub zapret_bat_file: Option<String>,
    pub zapret_release: Option<String>,
    pub hosts_bypass: Option<bool>,
    pub download_dir: Option<String>,
}

pub struct AppStateWrapper {
    pub state: Mutex<AppState>,
    pub config: Mutex<AppConfig>,
    pub pids: Mutex<HashMap<String, u32>>,
}

// --- GitHub API types ---

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

// --- Helpers ---

const DETACHED_PROCESS: u32 = 0x00000008;
const CREATE_NO_WINDOW: u32 = 0x08000000;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;

fn get_data_dir(app: &tauri::AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_data_dir()
        .expect("failed to get app data dir");
    fs::create_dir_all(&dir).ok();
    dir
}

fn get_download_dir(app: &tauri::AppHandle) -> PathBuf {
    let config = load_config(app);
    if let Some(ref custom) = config.download_dir {
        let p = PathBuf::from(custom);
        if p.exists() {
            return p;
        }
    }
    get_data_dir(app)
}

fn get_zapret_dir(app: &tauri::AppHandle) -> PathBuf {
    let dir = get_download_dir(app).join("zapret");
    fs::create_dir_all(&dir).ok();
    dir
}

fn get_tg_proxy_dir(app: &tauri::AppHandle) -> PathBuf {
    let dir = get_download_dir(app).join("tg-ws-proxy");
    fs::create_dir_all(&dir).ok();
    dir
}

fn get_config_path(app: &tauri::AppHandle) -> PathBuf {
    get_data_dir(app).join("config.json")
}

fn load_config(app: &tauri::AppHandle) -> AppConfig {
    let path = get_config_path(app);
    if path.exists() {
        let data = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        AppConfig::default()
    }
}

fn save_config_to_disk(app: &tauri::AppHandle, config: &AppConfig) {
    let path = get_config_path(app);
    let data = serde_json::to_string_pretty(config).unwrap();
    fs::write(&path, data).ok();
}

async fn fetch_github_latest(repo: &str) -> Result<GitHubRelease, String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .get(&url)
        .header("User-Agent", "FREENET")
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;
    resp.json::<GitHubRelease>()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))
}

async fn download_file(url: &str, dest: &PathBuf) -> Result<(), String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .header("User-Agent", "FREENET")
        .send()
        .await
        .map_err(|e| format!("Download error: {}", e))?;

    let mut file = fs::File::create(dest).map_err(|e| format!("File create error: {}", e))?;
    let mut stream = resp.bytes_stream();
    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Write error: {}", e))?;
    }
    Ok(())
}

fn extract_zip(zip_path: &PathBuf, dest: &PathBuf) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|e| format!("Open zip error: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Read zip error: {}", e))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| format!("Entry error: {}", e))?;
        let out_path = dest.join(entry.mangled_name());

        if entry.is_dir() {
            fs::create_dir_all(&out_path).ok();
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).ok();
            }
            let mut out_file =
                fs::File::create(&out_path).map_err(|e| format!("Create file error: {}", e))?;
            std::io::copy(&mut entry, &mut out_file)
                .map_err(|e| format!("Copy error: {}", e))?;
        }
    }
    Ok(())
}

fn extract_version_from_tag(tag: &str) -> String {
    tag.trim_start_matches('v').to_string()
}

fn get_running_version(data_dir: &PathBuf) -> Option<String> {
    let version_file = data_dir.join("version.txt");
    if version_file.exists() {
        Some(fs::read_to_string(version_file).unwrap_or_default().trim().to_string())
    } else {
        None
    }
}

fn save_running_version(data_dir: &PathBuf, version: &str) {
    let version_file = data_dir.join("version.txt");
    fs::write(version_file, version).ok();
}

fn is_process_alive(pid: u32) -> bool {
    use std::ffi::c_void;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

    extern "system" {
        fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> *mut c_void;
        fn GetExitCodeProcess(handle: *mut c_void, exit_code: *mut u32) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }

    const STILL_ACTIVE: u32 = 259;

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }
        let mut exit_code: u32 = 0;
        let success = GetExitCodeProcess(handle, &mut exit_code);
        CloseHandle(handle);
        success != 0 && exit_code == STILL_ACTIVE
    }
}

fn kill_process_tree(pid: u32) {
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
}

fn find_pid_by_name(name: &str) -> Option<u32> {
    use std::ffi::c_void;

    #[repr(C)]
    #[allow(non_snake_case)]
    struct PROCESSENTRY32W {
        dw_size: u32,
        cnt_usage: u32,
        th32_process_id: u32,
        th32_default_heap_id: usize,
        th32_module_id: u32,
        cnt_threads: u32,
        th32_parent_process_id: u32,
        pcPri_class_base: i32,
        dw_flags: u32,
        sz_exe_file: [u16; 260],
    }

    extern "system" {
        fn CreateToolhelp32Snapshot(dw_flags: u32, th32_process_id: u32) -> *mut c_void;
        fn Process32FirstW(h_snapshot: *mut c_void, lppe: *mut PROCESSENTRY32W) -> i32;
        fn Process32NextW(h_snapshot: *mut c_void, lppe: *mut PROCESSENTRY32W) -> i32;
        fn CloseHandle(hObject: *mut c_void) -> i32;
    }

    const TH32CS_SNAPPROCESS: u32 = 0x00000002;

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot.is_null() || snapshot as isize == -1 {
            return None;
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dw_size = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        let target_upper: Vec<u16> = name.to_uppercase().encode_utf16().collect();

        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let exe_len = entry.sz_exe_file.iter().position(|&c| c == 0).unwrap_or(260);
                let exe_upper: Vec<u16> = entry.sz_exe_file[..exe_len].iter().map(|c| {
                    if *c >= b'a' as u16 && *c <= b'z' as u16 { *c - 32 } else { *c }
                }).collect();
                if exe_upper == target_upper {
                    CloseHandle(snapshot);
                    return Some(entry.th32_process_id);
                }
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
    }
    None
}

fn find_bat_recursive(dir: &PathBuf, target: &str) -> Result<PathBuf, String> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if path.file_name().map(|f| f.to_string_lossy().as_ref() == target).unwrap_or(false) {
                    return Ok(path);
                }
            } else if path.is_dir() {
                if let Ok(found) = find_bat_recursive(&path, target) {
                    return Ok(found);
                }
            }
        }
    }
    Err(format!("Bat file '{}' not found in zapret directory", target))
}

// --- Commands ---

#[tauri::command]
fn get_data_dir_path(app: tauri::AppHandle) -> String {
    get_data_dir(&app).to_string_lossy().to_string()
}

#[tauri::command]
fn get_download_dir_path(app: tauri::AppHandle) -> String {
    get_download_dir(&app).to_string_lossy().to_string()
}

#[tauri::command]
fn select_download_dir(app: tauri::AppHandle) -> Option<String> {
    let handle = rfd::FileDialog::new()
        .set_title("Выберите папку для загрузок")
        .pick_folder()?;
    let path_str = handle.to_string_lossy().to_string();
    let mut config = load_config(&app);
    config.download_dir = Some(path_str.clone());
    let config_path = get_config_path(&app);
    let data = serde_json::to_string_pretty(&config).ok()?;
    fs::write(&config_path, data).ok();
    Some(path_str)
}

#[tauri::command]
fn is_installed(app: tauri::AppHandle, service: String) -> bool {
    let data_dir = match service.as_str() {
        "tg-ws-proxy" => get_tg_proxy_dir(&app),
        "zapret-discord-youtube" => get_zapret_dir(&app),
        _ => return false,
    };
    if !data_dir.exists() {
        return false;
    }
    match service.as_str() {
        "tg-ws-proxy" => data_dir.join("TgWsProxy_windows.exe").exists(),
        "zapret-discord-youtube" => walk_dir_for_bat(&data_dir),
        _ => false,
    }
}

fn walk_dir_for_bat(dir: &PathBuf) -> bool {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if path.extension().map(|e| e == "bat").unwrap_or(false) {
                    return true;
                }
            } else if path.is_dir() {
                if walk_dir_for_bat(&path) {
                    return true;
                }
            }
        }
    }
    false
}

#[tauri::command]
fn get_installed_version(app: tauri::AppHandle, service: String) -> Option<String> {
    let data_dir = match service.as_str() {
        "tg-ws-proxy" => get_tg_proxy_dir(&app),
        "zapret-discord-youtube" => get_zapret_dir(&app),
        _ => return None,
    };
    get_running_version(&data_dir)
}

#[tauri::command]
async fn check_version(service: String) -> Result<String, String> {
    let repo = match service.as_str() {
        "tg-ws-proxy" => "Flowseal/tg-ws-proxy",
        "zapret-discord-youtube" => "Flowseal/zapret-discord-youtube",
        _ => return Err(format!("Unknown service: {}", service)),
    };
    let release = fetch_github_latest(repo).await?;
    Ok(extract_version_from_tag(&release.tag_name))
}

#[tauri::command]
async fn download_service(
    app: tauri::AppHandle,
    service: String,
    state: tauri::State<'_, AppStateWrapper>,
) -> Result<String, String> {
    match service.as_str() {
        "tg-ws-proxy" => download_tg_proxy(&app, &state).await,
        "zapret-discord-youtube" => download_zapret(&app, &state).await,
        _ => Err(format!("Unknown service: {}", service)),
    }
}

async fn download_tg_proxy(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppStateWrapper>,
) -> Result<String, String> {
    let release = fetch_github_latest("Flowseal/tg-ws-proxy").await?;
    let version = extract_version_from_tag(&release.tag_name);
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == "TgWsProxy_windows.exe")
        .ok_or("Windows exe not found in release assets")?;
    let dir = get_tg_proxy_dir(app);
    let exe_path = dir.join("TgWsProxy_windows.exe");
    download_file(&asset.browser_download_url, &exe_path).await?;
    save_running_version(&dir, &version);
    let mut state_lock = state.state.lock().unwrap();
    state_lock.tg_proxy.installed = true;
    state_lock.tg_proxy.installed_version = Some(version.clone());
    state_lock.tg_proxy.latest_version = Some(version.clone());
    Ok(format!("Downloaded tg-ws-proxy v{}", version))
}

async fn download_zapret(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppStateWrapper>,
) -> Result<String, String> {
    let release = fetch_github_latest("Flowseal/zapret-discord-youtube").await?;
    let version = extract_version_from_tag(&release.tag_name);
    let asset = release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".zip"))
        .ok_or("ZIP not found in release assets")?;
    let dir = get_zapret_dir(app);
    let zip_path = dir.join(format!("zapret-{}.zip", version));
    download_file(&asset.browser_download_url, &zip_path).await?;
    extract_zip(&zip_path, &dir)?;
    fs::remove_file(&zip_path).ok();
    save_running_version(&dir, &version);
    let mut state_lock = state.state.lock().unwrap();
    state_lock.zapret.installed = true;
    state_lock.zapret.installed_version = Some(version.clone());
    state_lock.zapret.latest_version = Some(version.clone());
    Ok(format!("Downloaded zapret v{}", version))
}

#[tauri::command]
async fn start_service(
    app: tauri::AppHandle,
    service: String,
    state: tauri::State<'_, AppStateWrapper>,
) -> Result<String, String> {
    match service.as_str() {
        "tg_proxy" => start_tg_proxy(&app, &state).await,
        "zapret" => start_zapret(&app, &state).await,
        _ => Err(format!("Unknown service: {}", service)),
    }
}

async fn start_tg_proxy(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppStateWrapper>,
) -> Result<String, String> {
    let dir = get_tg_proxy_dir(app);
    let exe_path = dir.join("TgWsProxy_windows.exe");
    if !exe_path.exists() {
        return Err("tg-ws-proxy not installed".into());
    }

    use std::process::Command;
    let child = Command::new(&exe_path)
        .current_dir(&dir)
        .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start tg-ws-proxy: {}", e))?;

    let pid = child.id();
    drop(child); // Release the handle, track by PID

    let mut pids = state.pids.lock().unwrap();
    pids.insert("tg_proxy".into(), pid);

    let mut state_lock = state.state.lock().unwrap();
    state_lock.tg_proxy.running = true;

    Ok(format!("tg-ws-proxy started (PID: {})", pid))
}

fn parse_winws_command(bat_path: &PathBuf) -> Result<(PathBuf, Vec<String>), String> {
    let content = fs::read_to_string(bat_path).map_err(|e| format!("Cannot read bat: {}", e))?;
    let bat_dir = bat_path.parent().unwrap().to_path_buf();
    let bin_dir = bat_dir.join("bin");
    let lists_dir = bat_dir.join("lists");

    let mut winws_line = String::new();
    let mut in_winws = false;
    for line in content.lines() {
        let trimmed = line.trim().trim_end_matches('^').trim();
        if trimmed.contains("winws.exe") {
            in_winws = true;
            winws_line = trimmed.to_string();
        } else if in_winws && (trimmed.starts_with("--") || trimmed.is_empty()) {
            if !trimmed.is_empty() {
                winws_line.push(' ');
                winws_line.push_str(trimmed);
            }
        } else if in_winws {
            break;
        }
    }
    if winws_line.is_empty() {
        return Err("winws.exe not found in bat".into());
    }

    let winws_line = winws_line
        .replace("%~dp0", &format!("{}\\", bat_dir.to_string_lossy()))
        .replace("%BIN%", &format!("{}\\", bin_dir.to_string_lossy()))
        .replace("%LISTS%", &format!("{}\\", lists_dir.to_string_lossy()))
        .replace("%GameFilterTCP%", "12")
        .replace("%GameFilterUDP%", "12")
        .replace("%GameFilter%", "12");

    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for c in winws_line.chars() {
        match c {
            '"' => in_quotes = !in_quotes,
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        args.push(current);
    }

    let mut start_idx = 0;
    for (i, arg) in args.iter().enumerate() {
        if arg.ends_with("winws.exe") {
            start_idx = i + 1;
            break;
        }
    }
    args = args[start_idx..].to_vec();

    Ok((bin_dir.join("winws.exe"), args))
}

async fn start_zapret(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppStateWrapper>,
) -> Result<String, String> {
    let dir = get_zapret_dir(app);
    let config = load_config(app);
    let bat_name = config.zapret_bat_file.unwrap_or_else(|| "general.bat".into());
    let bat_path = find_bat_recursive(&dir, &bat_name)?;
    let (winws_exe, args) = parse_winws_command(&bat_path)?;

    use std::process::Command;
    Command::new(&winws_exe)
        .args(&args)
        .current_dir(winws_exe.parent().unwrap())
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start winws.exe: {}", e))?;

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let winws_pid = find_pid_by_name("winws");
    let track_pid = winws_pid.ok_or("winws.exe not found after launch")?;

    let mut pids = state.pids.lock().unwrap();
    pids.insert("zapret".into(), track_pid);

    let mut state_lock = state.state.lock().unwrap();
    state_lock.zapret.running = true;

    Ok(format!("zapret started (winws PID: {})", track_pid))
}

#[tauri::command]
async fn stop_service(
    service: String,
    state: tauri::State<'_, AppStateWrapper>,
) -> Result<String, String> {
    let pid = {
        let mut pids = state.pids.lock().unwrap();
        pids.remove(&service)
    };

    if let Some(pid) = pid {
        kill_process_tree(pid);
    }

    let mut state_lock = state.state.lock().unwrap();
    match service.as_str() {
        "tg_proxy" => {
            state_lock.tg_proxy.running = false;
            Ok("tg-ws-proxy stopped".into())
        }
        "zapret" => {
            state_lock.zapret.running = false;
            // Also try to kill winws directly
            if let Some(pid) = find_pid_by_name("winws") {
                kill_process_tree(pid);
            }
            Ok("zapret stopped".into())
        }
        _ => Err(format!("Unknown service: {}", service)),
    }
}

#[tauri::command]
fn get_all_status(
    state: tauri::State<'_, AppStateWrapper>,
) -> AppState {
    let mut state_lock = state.state.lock().unwrap();
    let mut pids = state.pids.lock().unwrap();

    // Check tg_proxy
    let tg_pid = pids.get("tg_proxy").copied();
    let tg_alive = tg_pid.map(|pid| is_process_alive(pid)).unwrap_or(false);
    if !tg_alive {
        pids.remove("tg_proxy");
        state_lock.tg_proxy.running = false;
    }

    // Check zapret - also check winws
    let zapret_pid = pids.get("zapret").copied();
    let winws_pid = find_pid_by_name("winws");
    let zapret_alive = if let Some(pid) = zapret_pid {
        is_process_alive(pid) || winws_pid.is_some()
    } else {
        winws_pid.is_some()
    };

    if !zapret_alive {
        pids.remove("zapret");
        state_lock.zapret.running = false;
    } else if state_lock.zapret.running && winws_pid.is_some() {
        // Update tracked PID to winws if wrapper exited
        pids.insert("zapret".into(), winws_pid.unwrap());
    }

    state_lock.clone()
}

#[tauri::command]
fn list_bat_files(app: tauri::AppHandle) -> Vec<String> {
    let dir = get_zapret_dir(&app);
    let mut result = Vec::new();
    collect_bat_files(&dir, &mut result);
    result
}

fn collect_bat_files(dir: &PathBuf, result: &mut Vec<String>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if path.extension().map(|e| e == "bat").unwrap_or(false) {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    if name.to_lowercase().contains("general") {
                        result.push(name);
                    }
                }
            } else if path.is_dir() {
                collect_bat_files(&path, result);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct ReleaseInfo {
    tag: String,
    version: String,
    date: String,
}

#[tauri::command]
async fn list_releases(service: String) -> Result<Vec<ReleaseInfo>, String> {
    let repo = match service.as_str() {
        "zapret-discord-youtube" => "Flowseal/zapret-discord-youtube",
        "tg-ws-proxy" => "Flowseal/tg-ws-proxy",
        _ => return Err(format!("Unknown service: {}", service)),
    };
    let url = format!("https://api.github.com/repos/{}/releases?per_page=20", repo);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .get(&url)
        .header("User-Agent", "FREENET")
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("GitHub API error: {}", resp.status()));
    }
    let releases: Vec<GitHubRelease> = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))?;
    Ok(releases
        .into_iter()
        .take(20)
        .map(|r| ReleaseInfo {
            tag: r.tag_name.clone(),
            version: extract_version_from_tag(&r.tag_name),
            date: String::new(),
        })
        .collect())
}

#[tauri::command]
async fn download_release(
    app: tauri::AppHandle,
    service: String,
    version: String,
    state: tauri::State<'_, AppStateWrapper>,
) -> Result<String, String> {
    let repo = match service.as_str() {
        "zapret-discord-youtube" => "Flowseal/zapret-discord-youtube",
        "tg-ws-proxy" => "Flowseal/tg-ws-proxy",
        _ => return Err(format!("Unknown service: {}", service)),
    };
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let url = format!("https://api.github.com/repos/{}/releases/tags/{}", repo, version);
    let resp = client
        .get(&url)
        .header("User-Agent", "FREENET")
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;
    let release: GitHubRelease = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))?;
    let ver = extract_version_from_tag(&release.tag_name);
    match service.as_str() {
        "zapret-discord-youtube" => {
            let asset = release
                .assets
                .iter()
                .find(|a| a.name.ends_with(".zip"))
                .ok_or("ZIP not found")?;
            let dir = get_zapret_dir(&app);
            let zip_path = dir.join(format!("zapret-{}.zip", ver));
            download_file(&asset.browser_download_url, &zip_path).await?;
            extract_zip(&zip_path, &dir)?;
            fs::remove_file(&zip_path).ok();
            save_running_version(&dir, &ver);
            let mut state_lock = state.state.lock().unwrap();
            state_lock.zapret.installed = true;
            state_lock.zapret.installed_version = Some(ver.clone());
            Ok(format!("Downloaded zapret v{}", ver))
        }
        "tg-ws-proxy" => {
            let asset = release
                .assets
                .iter()
                .find(|a| a.name == "TgWsProxy_windows.exe")
                .ok_or("Windows exe not found")?;
            let dir = get_tg_proxy_dir(&app);
            let exe_path = dir.join("TgWsProxy_windows.exe");
            download_file(&asset.browser_download_url, &exe_path).await?;
            save_running_version(&dir, &ver);
            let mut state_lock = state.state.lock().unwrap();
            state_lock.tg_proxy.installed = true;
            state_lock.tg_proxy.installed_version = Some(ver.clone());
            Ok(format!("Downloaded tg-ws-proxy v{}", ver))
        }
        _ => Err("Unknown service".into()),
    }
}

// --- Hosts file bypass ---

const HOSTS_MARKER_START: &str = "# === FREENET BLOCK BYPASS START ===";
const HOSTS_MARKER_END: &str = "# === FREENET BLOCK BYPASS END ===";

fn get_hosts_path() -> PathBuf {
    PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts")
}

fn get_blocked_domains() -> Vec<(&'static str, &'static str)> {
    vec![
        ("soundcloud.com", "149.154.167.99"),
        ("gemini.google.com", "142.250.74.78"),
        ("chatgpt.com", "104.18.32.7"),
        ("openai.com", "104.18.32.7"),
        ("sentry.io", "185.199.108.133"),
        ("chat.openai.com", "104.18.32.7"),
        ("cdn.oaistatic.com", "104.18.32.7"),
        ("androidapp.io", "142.250.74.78"),
        ("t.me", "149.154.167.99"),
        ("telegram.org", "149.154.167.99"),
    ]
}

#[tauri::command]
fn get_hosts_status() -> Result<bool, String> {
    let hosts_path = get_hosts_path();
    let content = fs::read_to_string(&hosts_path)
        .map_err(|e| format!("Cannot read hosts file: {}. Run as admin.", e))?;
    Ok(content.contains(HOSTS_MARKER_START))
}

#[tauri::command]
fn set_hosts_bypass(enabled: bool) -> Result<String, String> {
    let hosts_path = get_hosts_path();
    let mut content = fs::read_to_string(&hosts_path)
        .map_err(|e| format!("Cannot read hosts file: {}. Run as admin.", e))?;
    let start_idx = content.find(HOSTS_MARKER_START);
    let end_idx = content.find(HOSTS_MARKER_END).map(|i| i + HOSTS_MARKER_END.len());
    if let (Some(s), Some(e)) = (start_idx, end_idx) {
        content.drain(s..e);
    }
    if enabled {
        let domains = get_blocked_domains();
        let mut block = format!("\n{}\n", HOSTS_MARKER_START);
        for (domain, ip) in &domains {
            block.push_str(&format!("{} {}\n", ip, domain));
        }
        block.push_str(&format!("{}\n", HOSTS_MARKER_END));
        content.push_str(&block);
    }
    fs::write(&hosts_path, &content)
        .map_err(|e| format!("Cannot write hosts file: {}. Run as admin.", e))?;
    if enabled {
        Ok("Hosts bypass enabled".into())
    } else {
        Ok("Hosts bypass disabled".into())
    }
}

#[tauri::command]
fn save_config_value(
    app: tauri::AppHandle,
    key: String,
    value: String,
    state: tauri::State<'_, AppStateWrapper>,
) {
    let mut config = state.config.lock().unwrap();
    match key.as_str() {
        "zapret_bat_file" => config.zapret_bat_file = Some(value),
        "zapret_release" => config.zapret_release = Some(value),
        "hosts_bypass" => config.hosts_bypass = Some(value == "true"),
        "download_dir" => config.download_dir = Some(value),
        _ => {}
    }
    save_config_to_disk(&app, &config);
}

#[tauri::command]
fn load_config_value(
    app: tauri::AppHandle,
    key: String,
) -> Option<String> {
    let config = load_config(&app);
    match key.as_str() {
        "zapret_bat_file" => config.zapret_bat_file,
        "zapret_release" => config.zapret_release,
        "hosts_bypass" => config.hosts_bypass.map(|v| v.to_string()),
        _ => None,
    }
}

#[tauri::command]
fn minimize_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.minimize();
    }
}

#[tauri::command]
fn hide_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

// --- Entry point ---

fn is_admin_check() -> bool {
    use std::ffi::c_void;
    extern "system" {
        fn OpenProcessToken(h: *mut c_void, a: u32, t: *mut *mut c_void) -> i32;
        fn GetTokenInformation(t: *mut c_void, c: u32, i: *mut c_void, l: u32, r: *mut u32) -> i32;
        fn CloseHandle(h: *mut c_void) -> i32;
    }
    unsafe {
        let mut token: *mut c_void = std::ptr::null_mut();
        if OpenProcessToken(-1isize as *mut c_void, 0x0008, &mut token) == 0 { return false; }
        let mut et: u32 = 0;
        let mut rl: u32 = 0;
        let ok = GetTokenInformation(token, 18, &mut et as *mut _ as *mut c_void, 4, &mut rl);
        CloseHandle(token);
        ok != 0 && et == 2
    }
}

fn shell_execute_runas(exe: &str, args: &str, dir: &str) -> Result<(), String> {
    use std::ffi::c_void;
    extern "system" {
        fn ShellExecuteW(h: *mut c_void, op: *const u16, file: *const u16, params: *const u16, dir: *const u16, show: i32) -> *mut c_void;
    }
    unsafe {
        let op: Vec<u16> = "runas\0".encode_utf16().collect();
        let file: Vec<u16> = exe.encode_utf16().chain(std::iter::once(0)).collect();
        let params: Vec<u16> = args.encode_utf16().chain(std::iter::once(0)).collect();
        let d: Vec<u16> = dir.encode_utf16().chain(std::iter::once(0)).collect();
        let r = ShellExecuteW(std::ptr::null_mut(), op.as_ptr(), file.as_ptr(), params.as_ptr(), d.as_ptr(), 0);
        if r as isize <= 32 { Err(format!("ShellExecuteW error {}", r as isize)) } else { Ok(()) }
    }
}

fn cleanup_all_processes(app: &tauri::AppHandle) {
    // Kill tracked PIDs
    if let Some(state) = app.try_state::<AppStateWrapper>() {
        let mut pids = state.pids.lock().unwrap();
        for (_name, pid) in pids.drain() {
            kill_process_tree(pid);
        }
    }
    // Kill ALL winws.exe processes by name (multiple instances possible)
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/IM", "winws.exe"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    // Belt and suspenders: kill again after a moment
    std::thread::sleep(std::time::Duration::from_millis(200));
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/IM", "winws.exe"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppStateWrapper {
            state: Mutex::new(AppState::default()),
            config: Mutex::new(AppConfig::default()),
            pids: Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![
            minimize_window,
            hide_window,
            get_data_dir_path,
            get_download_dir_path,
            select_download_dir,
            is_installed,
            get_installed_version,
            check_version,
            get_all_status,
            download_service,
            download_release,
            list_releases,
            start_service,
            stop_service,
            list_bat_files,
            save_config_value,
            load_config_value,
            get_hosts_status,
            set_hosts_bypass,
        ])
        .setup(|app| {
            if !is_admin_check() {
                let exe = std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                let dir = std::path::PathBuf::from(&exe).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                if shell_execute_runas(&exe, "", &dir).is_ok() {
                    std::process::exit(0);
                }
            }

            let show = tauri::menu::MenuItem::with_id(app, "show", "Show FREENET", true, None::<&str>)?;
            let quit = tauri::menu::MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = tauri::menu::MenuBuilder::new(app).items(&[&show, &quit]).build()?;

            let _tray = tauri::tray::TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("FREENET")
                .on_menu_event(move |app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            cleanup_all_processes(app);
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed => {
                    let app = window.app_handle();
                    cleanup_all_processes(app);
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
