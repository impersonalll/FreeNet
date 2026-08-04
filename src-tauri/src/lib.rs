use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock, atomic::{AtomicBool, AtomicU32, Ordering}};
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
pub struct HotkeyConfig {
    pub play_pause: Option<String>,
    pub next_track: Option<String>,
    pub prev_track: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub zapret_bat_file: Option<String>,
    pub zapret_release: Option<String>,
    pub hosts_bypass: Option<bool>,
    pub download_dir: Option<String>,
    pub hotkeys: Option<HotkeyConfig>,
    pub media_keys_enabled: Option<bool>,
    pub clipboard_enabled: Option<bool>,
    pub hosts_providers: Option<Vec<String>>,
    pub custom_hosts_url: Option<String>,
    pub auto_accept_enabled: Option<bool>,
    pub auto_accept_games: Option<Vec<String>>,
    pub selected_bypass: Option<String>,
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

pub fn is_process_alive(pid: u32) -> bool {
    // ProcessIdToSessionId only checks that the PID exists and does NOT
    // open a process handle, so it works across integrity levels (e.g. the
    // medium-integrity helper checking the elevated main process), unlike
    // OpenProcess which UIPI blocks for higher-integrity targets.
    extern "system" {
        fn ProcessIdToSessionId(dw_process_id: u32, p_session_id: *mut u32) -> i32;
    }
    let mut session_id: u32 = 0;
    unsafe { ProcessIdToSessionId(pid, &mut session_id) != 0 }
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
                // Match both "winws" and "winws.exe" — the toolhelp snapshot
                // always includes the ".exe" suffix, but callers often pass the
                // bare name. Also tolerate build-specific names like
                // "winws64.exe" by matching on the prefix.
                fn strip_exe(s: &[u16]) -> &[u16] {
                    if s.ends_with(&[b'.' as u16, b'e' as u16, b'x' as u16, b'e' as u16]) {
                        &s[..s.len() - 4]
                    } else {
                        s
                    }
                }
                let exe_name = strip_exe(&exe_upper);
                let target_name = strip_exe(&target_upper);
                let bare_match = exe_name == target_name
                    || (target_name.len() >= 4 && exe_name.starts_with(target_name));
                if bare_match {
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
    let child = Command::new(&winws_exe)
        .args(&args)
        .current_dir(winws_exe.parent().unwrap())
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start winws.exe: {}", e))?;

    // Track the spawn PID directly (more reliable than matching by name,
    // which can miss the process if it starts slowly or under a variant name).
    let spawned_pid = child.id();
    drop(child); // Release the handle, track by PID

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let track_pid = if is_process_alive(spawned_pid) {
        spawned_pid
    } else {
        find_pid_by_name("winws").ok_or("winws.exe not found after launch")?
    };

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
            // Also try to kill TgWsProxy_windows.exe directly (PID may be stale)
            if let Some(pid) = find_pid_by_name("TgWsProxy_windows.exe") {
                kill_process_tree(pid);
            }
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

// Stops everything the same way as on app close: kill tracked PIDs plus all
// matching winws/tg-ws-proxy processes by name.
#[tauri::command]
fn stop_all_services(app: tauri::AppHandle) -> Result<String, String> {
    cleanup_all_processes(&app);
    if let Some(state) = app.try_state::<AppStateWrapper>() {
        let mut state_lock = state.state.lock().unwrap();
        state_lock.tg_proxy.running = false;
        state_lock.zapret.running = false;
        state.pids.lock().unwrap().clear();
    }
    Ok("All services stopped".into())
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
    } else {
        // winws is up (or tracked PID alive) — reflect that in the status even
        // if the app just restarted and the flag was never set to true.
        state_lock.zapret.running = true;
        if let Some(pid) = winws_pid {
            // Update tracked PID to winws if wrapper exited
            pids.insert("zapret".into(), pid);
        }
    }

    state_lock.clone()
}

// --- Bypass services (zapret, GoodbyeDPI, ByeDPI) + tg-ws-proxy ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BypassServiceInfo {
    pub key: String,
    pub name: String,
    pub description: String,
    pub installed: bool,
    pub running: bool,
    pub exclusive: bool,
    pub installed_version: Option<String>,
    pub latest_version: Option<String>,
}

// Keys that are mutually exclusive: only one DPI bypass can run at a time.
const EXCLUSIVE_BYPASSES: &[&str] = &["zapret", "goodbyedpi", "byedpi"];

// Directories for each tool.
fn get_bypass_dir(app: &tauri::AppHandle, key: &str) -> PathBuf {
    let dir = match key {
        "zapret" => get_zapret_dir(app),
        "tg-ws-proxy" => get_tg_proxy_dir(app),
        _ => get_download_dir(app).join("bypass").join(key),
    };
    fs::create_dir_all(&dir).ok();
    dir
}

fn bypass_metadata(key: &str) -> Option<(&'static str, &'static str, bool)> {
    match key {
        "zapret" => Some((
            "zapret (winws)",
            "DPI bypass with customizable strategies (general.bat)",
            true,
        )),
        "goodbyedpi" => Some((
            "GoodbyeDPI",
            "DPI bypass via WinDivert (ValdikSS)",
            true,
        )),
        "byedpi" => Some((
            "ByeDPI",
            "DPI bypass, actively maintained fork of GoodbyeDPI",
            true,
        )),
        "tg-ws-proxy" => Some((
            "tg-ws-proxy",
            "Telegram WebSocket proxy (runs alongside a bypass)",
            false,
        )),
        _ => None,
    }
}

fn bypass_running_image(key: &str) -> Option<&'static str> {
    match key {
        "zapret" => Some("winws"),
        "goodbyedpi" => Some("goodbyedpi"),
        "byedpi" => Some("ciadpi"),
        "tg-ws-proxy" => Some("TgWsProxy_windows"),
        _ => None,
    }
}

fn bypass_exe_name(key: &str) -> Option<&'static str> {
    match key {
        "zapret" => Some("winws.exe"),
        "goodbyedpi" => Some("goodbyedpi.exe"),
        "byedpi" => Some("ciadpi.exe"),
        "tg-ws-proxy" => Some("TgWsProxy_windows.exe"),
        _ => None,
    }
}

// Recursively finds the exe inside the extracted release folder.
fn find_bypass_exe(dir: &PathBuf, exe_name: &str) -> Option<PathBuf> {
    let mut result: Option<PathBuf> = None;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if path
                    .file_name()
                    .map(|n| n.eq_ignore_ascii_case(exe_name))
                    .unwrap_or(false)
                {
                    result = Some(path);
                }
            } else if path.is_dir() {
                if let Some(p) = find_bypass_exe(&path, exe_name) {
                    result = Some(p);
                }
            }
        }
    }
    result
}

fn is_bypass_installed(app: &tauri::AppHandle, key: &str) -> bool {
    let dir = get_bypass_dir(app, key);
    match key {
        "zapret" => walk_dir_for_bat(&dir),
        "tg-ws-proxy" => dir.join("TgWsProxy_windows.exe").exists(),
        _ => find_bypass_exe(&dir, bypass_exe_name(key).unwrap_or("")).is_some(),
    }
}

fn is_bypass_running(key: &str) -> bool {
    if let Some(image) = bypass_running_image(key) {
        find_pid_by_name(image).is_some()
    } else {
        false
    }
}

// Returns the currently running mutually-exclusive bypass key (or None).
#[tauri::command]
fn get_active_bypass() -> Option<String> {
    EXCLUSIVE_BYPASSES
        .iter()
        .find(|key| is_bypass_running(key))
        .map(|k| k.to_string())
}

#[tauri::command]
fn get_bypass_services(app: tauri::AppHandle) -> Vec<BypassServiceInfo> {
    let mut out = Vec::new();
    for key in EXCLUSIVE_BYPASSES.iter().chain(std::iter::once(&"tg-ws-proxy")) {
        let (name, description, exclusive) = match bypass_metadata(key) {
            Some(m) => m,
            None => continue,
        };
        let dir = get_bypass_dir(&app, key);
        let version = get_running_version(&dir);
        out.push(BypassServiceInfo {
            key: key.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            installed: is_bypass_installed(&app, key),
            running: is_bypass_running(key),
            exclusive,
            installed_version: version.clone(),
            latest_version: version,
        });
    }
    out
}

async fn download_bypass_inner(
    app: &tauri::AppHandle,
    key: &str,
) -> Result<String, String> {
    match key {
        "zapret" => {
            let state = app
                .state::<AppStateWrapper>();
            download_zapret(app, &state).await
        }
        "tg-ws-proxy" => {
            let state = app.state::<AppStateWrapper>();
            download_tg_proxy(app, &state).await
        }
        _ => {
            let (repo, name, _) = bypass_metadata(key).ok_or("Unknown bypass service")?;
            let release = fetch_github_latest(repo).await?;
            let version = extract_version_from_tag(&release.tag_name);

            let asset = match key {
                "goodbyedpi" => release
                    .assets
                    .iter()
                    .find(|a| a.name.ends_with(".zip") && !a.name.to_lowercase().contains("x86"))
                    .or_else(|| release.assets.iter().find(|a| a.name.ends_with(".zip"))),
                "byedpi" => release
                    .assets
                    .iter()
                    .find(|a| a.name.contains("x86_64") && a.name.ends_with(".zip"))
                    .or_else(|| release.assets.iter().find(|a| a.name.ends_with(".zip"))),
                _ => None,
            }
            .ok_or("No ZIP asset found in release")?;

            let dir = get_bypass_dir(app, key);
            let zip_path = dir.join(format!("{}.zip", key));
            download_file(&asset.browser_download_url, &zip_path).await?;
            extract_zip(&zip_path, &dir)?;
            fs::remove_file(&zip_path).ok();
            save_running_version(&dir, &version);

            Ok(format!("Downloaded {} v{}", name, version))
        }
    }
}

#[tauri::command]
async fn download_bypass(app: tauri::AppHandle, key: String) -> Result<String, String> {
    download_bypass_inner(&app, &key).await
}

async fn start_bypass_inner(app: &tauri::AppHandle, key: &str) -> Result<String, String> {
    // Mutually exclusive: starting a DPI bypass stops all the others first.
    if EXCLUSIVE_BYPASSES.contains(&key) {
        for other in EXCLUSIVE_BYPASSES {
            if *other != key && is_bypass_running(other) {
                let _ = stop_bypass_inner(app, other);
            }
        }
    }

    match key {
        "zapret" => {
            // Reuse the existing zapret start logic through the service API.
            let state = app.state::<AppStateWrapper>();
            start_zapret(app, &state).await
        }
        "tg-ws-proxy" => {
            let state = app.state::<AppStateWrapper>();
            start_tg_proxy(app, &state).await
        }
        _ => {
            let dir = get_bypass_dir(app, key);
            let (exe_name, args): (&str, Vec<String>) = match key {
                "goodbyedpi" => (
                    "goodbyedpi.exe",
                    vec![
                        "-1".into(),
                        "--dpi-desync=fake,fakedsplit".into(),
                        "--dpi-desync-autottl=2".into(),
                        "--dpi-desync-fooling=md5sig".into(),
                        "--dpi-desync-fake-from-http=1".into(),
                        "--dpi-desync-ttl=3".into(),
                    ],
                ),
                "byedpi" => (
                    "ciadpi.exe",
                    vec![
                        "--split".into(),
                        "1".into(),
                        "--disorder".into(),
                        "3+s".into(),
                        "--mod-http=h,d".into(),
                        "--auto=torst".into(),
                        "--tlsrec".into(),
                        "1+s".into(),
                    ],
                ),
                _ => return Err(format!("Unknown bypass service: {}", key)),
            };

            let exe_path = find_bypass_exe(&dir, exe_name).ok_or("Bypass tool not installed")?;

            use std::process::Command;
            let child = Command::new(&exe_path)
                .args(&args)
                .current_dir(exe_path.parent().unwrap())
                .creation_flags(CREATE_NO_WINDOW)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .stdin(std::process::Stdio::null())
                .spawn()
                .map_err(|e| format!("Failed to start {}: {}", key, e))?;

            let pid = child.id();
            drop(child); // Release the handle, track by PID

            // Keep a copy of the pid in the shared map so it is cleaned up on close.
            if let Some(state) = app.try_state::<AppStateWrapper>() {
                state.pids.lock().unwrap().insert(format!("bypass_{}", key), pid);
            }

            Ok(format!("{} started (PID: {})", key, pid))
        }
    }
}

#[tauri::command]
async fn start_bypass(app: tauri::AppHandle, key: String) -> Result<String, String> {
    if is_bypass_running(&key) {
        return Err(format!("{} is already running", key));
    }
    start_bypass_inner(&app, &key).await
}

fn stop_bypass_inner(app: &tauri::AppHandle, key: &str) -> Result<String, String> {
    // Kill by name (robust even if PID tracking was lost).
    let image = match key {
        "zapret" => "winws.exe",
        "goodbyedpi" => "goodbyedpi.exe",
        "byedpi" => "ciadpi.exe",
        "tg-ws-proxy" => "TgWsProxy_windows.exe",
        _ => return Err(format!("Unknown bypass service: {}", key)),
    };
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/IM", image])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    // Also kill any tracked PID from the shared map.
    if let Some(state) = app.try_state::<AppStateWrapper>() {
        let mut pids = state.pids.lock().unwrap();
        if let Some(pid) = pids.remove(&format!("bypass_{}", key)) {
            kill_process_tree(pid);
        }
        // Reset the in-memory service status flags.
        let mut state_lock = state.state.lock().unwrap();
        match key {
            "zapret" => state_lock.zapret.running = false,
            "tg-ws-proxy" => state_lock.tg_proxy.running = false,
            _ => {}
        }
    }

    Ok(format!("{} stopped", key))
}

#[tauri::command]
fn stop_bypass(app: tauri::AppHandle, key: String) -> Result<String, String> {
    stop_bypass_inner(&app, &key)
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

// --- Zapret custom domains (list-general-user.txt) ---

fn find_zapret_user_list(app: &tauri::AppHandle) -> Option<PathBuf> {
    find_file_recursive(&get_zapret_dir(app), "list-general-user.txt")
}

fn find_file_recursive(dir: &PathBuf, target: &str) -> Option<PathBuf> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if path.file_name().map(|f| f.to_string_lossy().as_ref() == target).unwrap_or(false) {
                    return Some(path);
                }
            } else if path.is_dir() {
                if let Some(found) = find_file_recursive(&path, target) {
                    return Some(found);
                }
            }
        }
    }
    None
}

#[tauri::command]
fn get_zapret_user_domains(app: tauri::AppHandle) -> Vec<String> {
    let Some(path) = find_zapret_user_list(&app) else {
        return Vec::new();
    };
    let Ok(content) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect()
}

#[tauri::command]
fn add_zapret_user_domain(app: tauri::AppHandle, domain: String) -> Result<Vec<String>, String> {
    let domain = domain.trim().to_lowercase();
    if domain.is_empty() {
        return Err("Domain cannot be empty".into());
    }
    if domain.contains(char::is_whitespace) || domain.contains('#') {
        return Err("Invalid domain".into());
    }
    let mut domains = get_zapret_user_domains(app.clone());
    if domains.iter().any(|d| d == &domain) {
        return Ok(domains);
    }
    domains.push(domain);
    write_zapret_user_domains(&app, &domains)?;
    Ok(domains)
}

#[tauri::command]
fn remove_zapret_user_domain(app: tauri::AppHandle, domain: String) -> Result<Vec<String>, String> {
    let domain = domain.trim().to_lowercase();
    let mut domains = get_zapret_user_domains(app.clone());
    domains.retain(|d| d != &domain);
    write_zapret_user_domains(&app, &domains)?;
    Ok(domains)
}

fn write_zapret_user_domains(app: &tauri::AppHandle, domains: &[String]) -> Result<(), String> {
    let Some(path) = find_zapret_user_list(app) else {
        return Err("Zapret user list not found. Install zapret first.".into());
    };
    let mut content = String::from("# Never leave this file empty\n");
    for d in domains {
        content.push_str(d);
        content.push('\n');
    }
    fs::write(&path, content).map_err(|e| format!("Failed to write user list: {}", e))
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostsProvider {
    pub key: String,
    pub name: String,
    pub description: String,
    pub url: Option<String>,
    pub custom: bool,
}

fn get_hosts_path() -> PathBuf {
    PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts")
}

// Built-in hosts providers. `custom` is a special entry backed by a user URL.
fn builtin_hosts_providers() -> Vec<HostsProvider> {
    vec![
        HostsProvider {
            key: "geohidedns".into(),
            name: "GeoHideDNS".into(),
            description: "Ad and tracker blocking hosts list".into(),
            url: Some("https://raw.githubusercontent.com/incognico/ad-host-list/master/hosts.txt".into()),
            custom: false,
        },
        HostsProvider {
            key: "stevenblack".into(),
            name: "StevenBlack Unified".into(),
            description: "Blocks ads, trackers and malware (unified list)".into(),
            url: Some("https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts".into()),
            custom: false,
        },
        HostsProvider {
            key: "someonewhocares".into(),
            name: "someonewhocares (Dan Pollock)".into(),
            description: "Blocks advertising, trackers and malware hosts".into(),
            url: Some("https://someonewhocares.org/hosts/hosts/".into()),
            custom: false,
        },
        HostsProvider {
            key: "custom".into(),
            name: "Custom URL".into(),
            description: "Use your own hosts file from a custom URL".into(),
            url: None,
            custom: true,
        },
    ]
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
fn get_hosts_providers() -> Vec<HostsProvider> {
    builtin_hosts_providers()
}

// Fetches a remote hosts file and parses out all "IP domain" entries.
async fn fetch_hosts_entries(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .get(url)
        .header("User-Agent", "FREENET")
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP status: {}", resp.status()));
    }
    let text = resp.text().await.map_err(|e| format!("Read error: {}", e))?;

    let mut out = String::new();
    let mut seen: HashSet<String> = HashSet::new();
    for line in text.lines() {
        // Strip inline comments, trim whitespace
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let ip = parts.next().unwrap_or("").to_string();
        if ip.is_empty() || ip.starts_with("::") {
            continue;
        }
        for domain in parts {
            let domain = domain.to_lowercase();
            if domain.is_empty() || !domain.contains('.') || domain.starts_with('*') {
                continue;
            }
            let entry = format!("{} {}", ip, domain);
            if seen.insert(domain.clone()) {
                out.push_str(&format!("{}\n", entry));
            }
        }
    }
    if out.is_empty() {
        return Err("No hosts entries found in the file".into());
    }
    Ok(out)
}

// Fetches entries for one provider key. "geohidedns" uses the built-in static
// domain list; "custom" uses the user URL; the rest are fetched from the web.
async fn fetch_provider_entries(key: &str, config: &AppConfig) -> Result<String, String> {
    if key == "custom" {
        let url = config.custom_hosts_url.clone().ok_or("Custom hosts URL is not set")?;
        return fetch_hosts_entries(&url).await;
    }
    if key == "geohidedns" {
        // Built-in static domain list (kept for the classic unblocking list)
        let mut block = String::new();
        for (domain, ip) in get_blocked_domains() {
            block.push_str(&format!("{} {}\n", ip, domain));
        }
        return Ok(block);
    }
    let provider = builtin_hosts_providers()
        .into_iter()
        .find(|p| p.key == key)
        .ok_or("Unknown hosts provider")?;
    let url = provider.url.ok_or("Provider has no URL")?;
    fetch_hosts_entries(&url).await
}

#[tauri::command]
async fn set_hosts_bypass(app: tauri::AppHandle, enabled: bool) -> Result<String, String> {
    let config = load_config(&app);
    let providers = config.hosts_providers.clone().unwrap_or_default();

    let hosts_path = get_hosts_path();
    let mut content = fs::read_to_string(&hosts_path)
        .map_err(|e| format!("Cannot read hosts file: {}. Run as admin.", e))?;
    let start_idx = content.find(HOSTS_MARKER_START);
    let end_idx = content.find(HOSTS_MARKER_END).map(|i| i + HOSTS_MARKER_END.len());
    if let (Some(s), Some(e)) = (start_idx, end_idx) {
        content.drain(s..e);
    }
    if enabled {
        if providers.is_empty() {
            return Err("No hosts providers selected".into());
        }
        let mut all_entries = String::new();
        for key in &providers {
            let entries = fetch_provider_entries(key, &config).await?;
            all_entries.push_str(&entries);
            all_entries.push('\n');
        }
        let mut block = format!(
            "\n# FREENET hosts providers: {}\n{}\n",
            providers.join(", "),
            HOSTS_MARKER_START
        );
        block.push_str(&all_entries);
        block.push_str(&format!("{}\n", HOSTS_MARKER_END));
        content.push_str(&block);
    }
    fs::write(&hosts_path, &content)
        .map_err(|e| format!("Cannot write hosts file: {}. Run as admin.", e))?;
    if enabled {
        Ok(format!("Hosts bypass enabled (providers: {})", providers.join(", ")))
    } else {
        Ok("Hosts bypass disabled".into())
    }
}

// Returns the currently selected provider keys (defaults to geohidedns).
#[tauri::command]
fn get_selected_hosts_providers(app: tauri::AppHandle) -> Vec<String> {
    let config = load_config(&app);
    config.hosts_providers.unwrap_or_default()
}

// Persists the selected provider keys.
#[tauri::command]
fn set_selected_hosts_providers(app: tauri::AppHandle, providers: Vec<String>) -> Result<(), String> {
    let mut config = load_config(&app);
    config.hosts_providers = Some(providers);
    save_config_to_disk(&app, &config);
    Ok(())
}

#[tauri::command]
fn save_config_value(
    app: tauri::AppHandle,
    key: String,
    value: String,
    _state: tauri::State<'_, AppStateWrapper>,
) {
    // Load from disk (not the in-memory snapshot) so unrelated persisted fields
    // like hotkeys are never overwritten with defaults.
    let mut config = load_config(&app);
    match key.as_str() {
        "zapret_bat_file" => config.zapret_bat_file = Some(value),
        "zapret_release" => config.zapret_release = Some(value),
        "hosts_bypass" => config.hosts_bypass = Some(value == "true"),
        "download_dir" => config.download_dir = Some(value),
        "media_keys_enabled" => config.media_keys_enabled = Some(value == "true"),
        "clipboard_enabled" => config.clipboard_enabled = Some(value == "true"),
        "custom_hosts_url" => config.custom_hosts_url = Some(value),
        "auto_accept_enabled" => config.auto_accept_enabled = Some(value == "true"),
        "selected_bypass" => config.selected_bypass = Some(value),
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
        "media_keys_enabled" => config.media_keys_enabled.map(|v| v.to_string()),
        "clipboard_enabled" => config.clipboard_enabled.map(|v| v.to_string()),
        "custom_hosts_url" => config.custom_hosts_url,
        "auto_accept_enabled" => config.auto_accept_enabled.map(|v| v.to_string()),
        "selected_bypass" => config.selected_bypass,
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
    // tg-ws-proxy may not be tracked by PID (get_all_status prunes it),
    // so kill by name as well.
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/IM", "TgWsProxy_windows.exe"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    // Belt and suspenders: kill again after a moment
    std::thread::sleep(std::time::Duration::from_millis(200));
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/IM", "winws.exe"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/IM", "TgWsProxy_windows.exe"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    // Bypass tools (GoodbyeDPI, ByeDPI)
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/IM", "goodbyedpi.exe"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/IM", "ciadpi.exe"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
}

// --- App update ---

#[derive(Serialize, Deserialize, Clone)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub needs_update: bool,
    pub download_url: String,
}

#[tauri::command]
async fn check_app_update() -> Result<UpdateInfo, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Client error: {}", e))?;
    let resp = client
        .get("https://api.github.com/repos/impersonalll/FreeNet/releases/latest")
        .header("User-Agent", "FREENET")
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("GitHub API error: {}", resp.status()));
    }
    let release: GitHubRelease = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))?;
    let latest_version = extract_version_from_tag(&release.tag_name);
    let needs_update = latest_version != current_version;
    let download_url = if needs_update {
        release
            .assets
            .iter()
            .find(|a| a.name.ends_with(".exe") && !a.name.contains("setup"))
            .map(|a| a.browser_download_url.clone())
            .unwrap_or_default()
    } else {
        String::new()
    };
    Ok(UpdateInfo {
        current_version,
        latest_version,
        needs_update,
        download_url,
    })
}

// --- Media keys & hotkeys ---

// Diagnostic logging for the media-key chain. Writes to
// %TEMP%\freenet_media_debug.log so we can see exactly where the
// hook -> pipe -> helper -> SendInput chain breaks.
fn media_log(msg: &str) {
    use std::io::Write;
    let path = std::env::temp_dir().join("freenet_media_debug.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "[{}] {}", std::process::id(), msg);
    }
}


fn simulate_media_key(action: &str) -> Result<(), String> {
    // Send WM_APPCOMMAND to a SINGLE target window. Broadcasting to every
    // top-level window would double-toggle when multiple media windows exist
    // (e.g. two Chrome windows): play arrives twice -> immediate pause. So we
    // target the foreground window; if it is FREENET itself, walk the Z-order
    // to the next visible window that does not belong to our process.
    const WM_APPCOMMAND: u32 = 0x0319;
    const APPCOMMAND_MEDIA_PLAY_PAUSE: u32 = 14;
    const APPCOMMAND_MEDIA_NEXTTRACK: u32 = 11;
    const APPCOMMAND_MEDIA_PREVIOUSTRACK: u32 = 12;

    let app_cmd = match action {
        "play_pause" => APPCOMMAND_MEDIA_PLAY_PAUSE,
        "next" => APPCOMMAND_MEDIA_NEXTTRACK,
        "prev" => APPCOMMAND_MEDIA_PREVIOUSTRACK,
        _ => return Err(format!("Unknown action: {}", action)),
    };

    unsafe {
        extern "system" {
            fn GetWindowThreadProcessId(h_wnd: *mut std::ffi::c_void, pid: *mut u32) -> u32;
            fn SendMessageW(
                h_wnd: *mut std::ffi::c_void,
                msg: u32,
                w_param: usize,
                l_param: isize,
            ) -> isize;
        }
        let target = find_media_target();
        if !target.is_null() {
            let lparam = (app_cmd as isize) << 16;
            SendMessageW(target, WM_APPCOMMAND, 0, lparam);
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(target, &mut pid);
            media_log(&format!("WM_APPCOMMAND cmd={} sent to hwnd={:?} pid={}", app_cmd, target, pid));
        } else {
            media_log("WM_APPCOMMAND skipped: no target window");
        }
    }
    media_log(&format!("simulate_media_key done: {}", action));
    Ok(())
}

// Parses "X,Y" into screen coordinates for a simulated click.
fn parse_click_coords(coords: &str) -> Option<(i32, i32)> {
    let (x, y) = coords.trim().split_once(',')?;
    let x: i32 = x.trim().parse().ok()?;
    let y: i32 = y.trim().parse().ok()?;
    Some((x, y))
}

// Simulates a left mouse click at screen coordinates. Runs inside the
// non-elevated helper so the click reaches games running at the user's
// integrity level (UIPI would block injection from the elevated main app).
fn simulate_mouse_click(x: i32, y: i32) {
    unsafe {
        extern "system" {
            fn SetCursorPos(x: i32, y: i32) -> i32;
            fn mouse_event(
                dw_flags: u32,
                dx: u32,
                dy: u32,
                dw_data: u32,
                dw_extra_info: *mut std::ffi::c_void,
            );
        }
        const MOUSEEVENTF_LEFTDOWN: u32 = 0x0002;
        const MOUSEEVENTF_LEFTUP: u32 = 0x0004;

        media_log(&format!("simulate_mouse_click ({}, {})", x, y));
        SetCursorPos(x, y);
        std::thread::sleep(std::time::Duration::from_millis(40));
        mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, std::ptr::null_mut());
        std::thread::sleep(std::time::Duration::from_millis(60));
        mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, std::ptr::null_mut());
    }
}

// Simulates a left mouse click on a target window WITHOUT touching the real
// cursor: converts the screen coords to client coords and posts WM_LBUTTONDOWN
// / WM_LBUTTONUP straight to the window. Runs inside the non-elevated helper so
// the messages reach games at the user's integrity level (UIPI would block
// them from the elevated main app). Returns false if the window was not found
// or the message could not be posted.
fn simulate_window_click(hwnd: *mut std::ffi::c_void, sx: i32, sy: i32) -> bool {
    unsafe {
        const WM_LBUTTONDOWN: u32 = 0x0201;
        const WM_LBUTTONUP: u32 = 0x0202;
        const MK_LBUTTON: usize = 0x0001;

        let mut pt = POINT { x: sx, y: sy };
        if hwnd.is_null() || ScreenToClient(hwnd, &mut pt) == 0 {
            media_log("simulate_window_click: no window or ScreenToClient failed");
            return false;
        }
        let lparam: isize = ((pt.y as isize) << 16) | (pt.x as isize & 0xFFFF);
        media_log(&format!(
            "simulate_window_click hwnd={:p} client=({}, {})",
            hwnd, pt.x, pt.y
        ));
        PostMessageW(hwnd, WM_LBUTTONDOWN, MK_LBUTTON, lparam);
        std::thread::sleep(std::time::Duration::from_millis(50));
        PostMessageW(hwnd, WM_LBUTTONUP, 0, lparam);
        true
    }
}

// Picks the single window that should receive the media command: the
// foreground window, or the next visible top-level window in Z-order that
// does not belong to this process (used when FREENET itself is focused).
unsafe fn find_media_target() -> *mut std::ffi::c_void {
    extern "system" {
        fn GetForegroundWindow() -> *mut std::ffi::c_void;
        fn GetCurrentProcessId() -> u32;
        fn GetWindowThreadProcessId(h_wnd: *mut std::ffi::c_void, pid: *mut u32) -> u32;
        fn GetWindow(h_wnd: *mut std::ffi::c_void, u_cmd: u32) -> *mut std::ffi::c_void;
        fn IsWindowVisible(h_wnd: *mut std::ffi::c_void) -> i32;
    }
    const GW_HWNDNEXT: u32 = 2;
    let my_pid = GetCurrentProcessId();

    let fg = GetForegroundWindow();
    if !fg.is_null() {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(fg, &mut pid);
        if pid != my_pid {
            return fg;
        }
    }

    // Foreground is FREENET (or none) — walk the Z-order from the foreground
    // window and pick the first visible window owned by another process.
    let mut hwnd = fg;
    let mut guard = 0;
    while !hwnd.is_null() && guard < 100 {
        hwnd = GetWindow(hwnd, GW_HWNDNEXT);
        if hwnd.is_null() {
            break;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid != my_pid && pid != 0 && IsWindowVisible(hwnd) != 0 {
            return hwnd;
        }
        guard += 1;
    }
    std::ptr::null_mut()
}

// --- Media key injection via non-elevated helper ---
//
// The main app runs elevated (needed for zapret/hosts). Windows UIPI blocks
// input injection from an elevated process into non-elevated apps such as a
// browser with Spotify. So the actual SendInput is done by a helper process
// spawned at medium integrity (same as the browser) that we reach over a
// named pipe: \\.\pipe\FREENET_MEDIA_<main_pid>.

const INVALID_HANDLE_VALUE: *mut std::ffi::c_void = -1isize as *mut std::ffi::c_void;

extern "system" {
    fn CreateNamedPipeW(
        lp_name: *const u16,
        dw_open_mode: u32,
        dw_pipe_mode: u32,
        n_max_instances: u32,
        n_out_buffer_size: u32,
        n_in_buffer_size: u32,
        n_default_time_out: u32,
        lp_security_attributes: *mut std::ffi::c_void,
    ) -> *mut std::ffi::c_void;
    fn ConnectNamedPipe(h_named_pipe: *mut std::ffi::c_void, lp_overlapped: *mut std::ffi::c_void) -> i32;
    fn ReadFile(
        h_file: *mut std::ffi::c_void,
        lp_buffer: *mut u8,
        n_number_of_bytes_to_read: u32,
        lp_number_of_bytes_read: *mut u32,
        lp_overlapped: *mut std::ffi::c_void,
    ) -> i32;
    fn WriteFile(
        h_file: *mut std::ffi::c_void,
        lp_buffer: *const u8,
        n_number_of_bytes_to_write: u32,
        lp_number_of_bytes_written: *mut u32,
        lp_overlapped: *mut std::ffi::c_void,
    ) -> i32;
    fn FlushFileBuffers(h_file: *mut std::ffi::c_void) -> i32;
    fn CloseHandle(h_object: *mut std::ffi::c_void) -> i32;
    fn GetLastError() -> u32;
    fn CreateFileW(
        lp_file_name: *const u16,
        dw_desired_access: u32,
        dw_share_mode: u32,
        lp_security_attributes: *mut std::ffi::c_void,
        dw_creation_disposition: u32,
        dw_flags_and_attributes: u32,
        h_template_file: *mut std::ffi::c_void,
    ) -> *mut std::ffi::c_void;
}

const PIPE_ACCESS_DUPLEX: u32 = 0x00000003;
const PIPE_TYPE_MESSAGE: u32 = 0x00000004;
const PIPE_READMODE_MESSAGE: u32 = 0x00000002;
const PIPE_WAIT: u32 = 0x00000000;
const PIPE_UNLIMITED_INSTANCES: u32 = 0x000000FF;
const GENERIC_WRITE: u32 = 0x40000000;
const OPEN_EXISTING: u32 = 3;
const ERROR_PIPE_CONNECTED: u32 = 535;
const ERROR_PIPE_BUSY: u32 = 231;
const ERROR_FILE_NOT_FOUND: u32 = 2;

#[repr(C)]
struct POINT {
    x: i32,
    y: i32,
}

extern "system" {
    fn EnumWindows(
        lp_enum_func: Option<unsafe extern "system" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> i32>,
        l_param: *mut std::ffi::c_void,
    ) -> i32;
    fn GetWindowThreadProcessId(h_wnd: *mut std::ffi::c_void, pid: *mut u32) -> u32;
    fn IsWindowVisible(h_wnd: *mut std::ffi::c_void) -> i32;
    fn PostMessageW(
        h_wnd: *mut std::ffi::c_void,
        msg: u32,
        w_param: usize,
        l_param: isize,
    ) -> i32;
    fn ScreenToClient(h_wnd: *mut std::ffi::c_void, lp_point: *mut POINT) -> i32;
}

fn media_pipe_name() -> String {
    format!(r"\\.\pipe\FREENET_MEDIA_{}", std::process::id())
}

// Sends a media command to the non-elevated helper. Returns false when the
// helper is not reachable (caller may then fall back to direct injection).
fn send_media_command(cmd: &str) -> bool {
    unsafe {
        let name16: Vec<u16> = media_pipe_name()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let mut last_err = 0u32;
        for attempt in 0..3 {
            let handle = CreateFileW(name16.as_ptr(), GENERIC_WRITE, 0, std::ptr::null_mut(), OPEN_EXISTING, 0, std::ptr::null_mut());
            if handle != INVALID_HANDLE_VALUE {
                let bytes = cmd.as_bytes();
                let mut written: u32 = 0;
                WriteFile(handle, bytes.as_ptr(), bytes.len() as u32, &mut written, std::ptr::null_mut());
                FlushFileBuffers(handle);
                CloseHandle(handle);
                return true;
            }
            last_err = GetLastError();
            if last_err != ERROR_PIPE_BUSY && last_err != ERROR_FILE_NOT_FOUND {
                return false;
            }
            if attempt < 2 {
                std::thread::sleep(std::time::Duration::from_millis(30));
            }
        }
        let _ = last_err;
        false
    }
}

// Entry point for the non-elevated helper process. Listens on the named
// pipe and injects media keys. Exits when told to quit or when the main
// process dies.
pub fn run_media_helper(main_pid: Option<u32>) {
    media_log(&format!("run_media_helper started, main_pid={:?}", main_pid));
    if let Some(pid) = main_pid {
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            if !is_process_alive(pid) {
                media_log("main process died, helper exiting");
                std::process::exit(0);
            }
        });
    }

    let Some(pid) = main_pid else { return };
    let pipe_name = format!(r"\\.\pipe\FREENET_MEDIA_{}", pid);
    media_log(&format!("helper pipe: {}", pipe_name));
    let name16: Vec<u16> = pipe_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    loop {
        unsafe {
            let handle = CreateNamedPipeW(
                name16.as_ptr(),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                1024,
                1024,
                0,
                std::ptr::null_mut(),
            );
            if handle == INVALID_HANDLE_VALUE {
                std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }
            if ConnectNamedPipe(handle, std::ptr::null_mut()) == 0 {
                if GetLastError() != ERROR_PIPE_CONNECTED {
                    CloseHandle(handle);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }
            }
            let mut buf = [0u8; 256];
            let mut read: u32 = 0;
            if ReadFile(handle, buf.as_mut_ptr(), buf.len() as u32, &mut read, std::ptr::null_mut()) != 0 && read > 0 {
                let cmd = String::from_utf8_lossy(&buf[..read as usize]).into_owned();
                match cmd.as_str() {
                    "play_pause" | "next" | "prev" => {
                        media_log(&format!("helper received cmd: {}", cmd));
                        let _ = simulate_media_key(&cmd);
                    }
                    "quit" => {
                        media_log("helper received quit");
                        CloseHandle(handle);
                        std::process::exit(0);
                    }
                    _ if cmd.starts_with("click ") => {
                        let coords = &cmd[6..];
                        if let Some((x, y)) = parse_click_coords(coords) {
                            media_log(&format!("helper received click ({}, {})", x, y));
                            simulate_mouse_click(x, y);
                        }
                    }
                    _ if cmd.starts_with("clickwin ") => {
                        let args: Vec<&str> = cmd[9..].split(',').collect();
                        if args.len() == 3 {
                            let hwnd = args[0].trim().parse::<usize>().ok();
                            let x = args[1].trim().parse::<i32>().ok();
                            let y = args[2].trim().parse::<i32>().ok();
                            if let (Some(hwnd), Some(x), Some(y)) = (hwnd, x, y) {
                                media_log(&format!("helper received clickwin ({}, {})", x, y));
                                simulate_window_click(hwnd as *mut std::ffi::c_void, x, y);
                            }
                        }
                    }
                    _ => {}
                }
            }
            CloseHandle(handle);
        }
    }
}

// Spawns the non-elevated helper process (medium integrity) so injected
// input reaches non-elevated windows. Primary path uses the interactive
// user token; if that fails, falls back to asking explorer.exe to launch
// the helper (which also runs at the user's integrity level).
fn spawn_media_helper() {
    media_log("spawn_media_helper: trying CreateProcessAsUserW");
    if !spawn_helper_with_user_token() {
        media_log("  CreateProcessAsUserW failed, falling back to explorer");
        spawn_helper_via_explorer();
    } else {
        media_log("  CreateProcessAsUserW OK");
    }
}

fn spawn_helper_with_user_token() -> bool {
    use std::ffi::c_void;

    unsafe {
        extern "system" {
            fn WTSGetActiveConsoleSessionId() -> u32;
            fn WTSQueryUserToken(session_id: u32, ph_token: *mut *mut c_void) -> i32;
            fn CreateProcessAsUserW(
                h_token: *mut c_void,
                lp_application_name: *const u16,
                lp_command_line: *mut u16,
                lp_process_attributes: *mut c_void,
                lp_thread_attributes: *mut c_void,
                b_inherit_handles: i32,
                dw_creation_flags: u32,
                lp_environment: *mut c_void,
                lp_current_directory: *const u16,
                lp_startup_info: *mut c_void,
                lp_process_information: *mut c_void,
            ) -> i32;
        }

        #[repr(C)]
        struct StartupInfoW {
            cb: u32,
            lp_reserved: *mut u16,
            lp_desktop: *mut u16,
            lp_title: *mut u16,
            dw_x: u32,
            dw_y: u32,
            dw_x_size: u32,
            dw_y_size: u32,
            dw_x_count_chars: u32,
            dw_y_count_chars: u32,
            dw_fill_attribute: u32,
            dw_flags: u32,
            w_show_window: u16,
            cb_reserved2: u16,
            lp_reserved2: *mut u8,
            h_std_input: *mut c_void,
            h_std_output: *mut c_void,
            h_std_error: *mut c_void,
        }

        #[repr(C)]
        struct ProcessInformation {
            h_process: *mut c_void,
            h_thread: *mut c_void,
            dw_process_id: u32,
            dw_thread_id: u32,
        }

        enable_privileges();

        let session_id = WTSGetActiveConsoleSessionId();
        let mut user_token: *mut c_void = std::ptr::null_mut();
        if WTSQueryUserToken(session_id, &mut user_token) == 0 {
            return false;
        }

        let exe = match std::env::current_exe() {
            Ok(e) => e,
            Err(_) => {
                CloseHandle(user_token);
                return false;
            }
        };
        let cmd = format!("\"{}\" --media-helper {}", exe.to_string_lossy(), std::process::id());
        let mut cmd16: Vec<u16> = cmd.encode_utf16().collect();
        cmd16.push(0);

        let mut si: StartupInfoW = std::mem::zeroed();
        si.cb = std::mem::size_of::<StartupInfoW>() as u32;
        let mut pi: ProcessInformation = std::mem::zeroed();

        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let ok = CreateProcessAsUserW(
            user_token,
            std::ptr::null_mut(),
            cmd16.as_mut_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            CREATE_NO_WINDOW,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut si as *mut _ as *mut c_void,
            &mut pi as *mut _ as *mut c_void,
        );
        CloseHandle(user_token);
        if ok != 0 {
            CloseHandle(pi.h_process);
            CloseHandle(pi.h_thread);
            true
        } else {
            false
        }
    }
}

fn spawn_helper_via_explorer() {
    let Some(exe) = std::env::current_exe().ok() else { return };
    // explorer.exe launches the exe with the interactive user's token
    // (medium integrity). The helper mode is signalled via a marker file
    // because explorer does not forward command-line arguments reliably.
    let marker = std::env::temp_dir().join("freenet_helper_launch.flag");
    let _ = fs::write(&marker, std::process::id().to_string());
    let _ = std::process::Command::new("explorer.exe").arg(&exe).spawn();
}

fn enable_privileges() {
    use std::ffi::c_void;

    #[repr(C)]
    struct LuidAndAttributes {
        luid: i64,
        attributes: u32,
    }

    #[repr(C)]
    struct TokenPrivileges {
        privilege_count: u32,
        privileges: [LuidAndAttributes; 1],
    }

    unsafe {
        extern "system" {
            fn OpenProcessToken(h_process: *mut c_void, desired_access: u32, token_handle: *mut *mut c_void) -> i32;
            fn LookupPrivilegeValueW(system_name: *const u16, name: *const u16, luid: *mut i64) -> i32;
            fn AdjustTokenPrivileges(
                token_handle: *mut c_void,
                disable_all_privileges: i32,
                new_state: *mut c_void,
                buffer_length: u32,
                previous_state: *mut c_void,
                return_length: *mut u32,
            ) -> i32;
        }
        const TOKEN_ADJUST_PRIVILEGES: u32 = 0x0020;
        const TOKEN_QUERY: u32 = 0x0008;
        const SE_PRIVILEGE_ENABLED: u32 = 0x0002;

        let mut token: *mut c_void = std::ptr::null_mut();
        if OpenProcessToken(-1isize as *mut c_void, TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token) == 0 {
            return;
        }
        for name in ["SeAssignPrimaryTokenPrivilege", "SeIncreaseQuotaPrivilege"] {
            let name16: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
            let mut luid: i64 = 0;
            if LookupPrivilegeValueW(std::ptr::null(), name16.as_ptr(), &mut luid) != 0 {
                let mut tp: TokenPrivileges = std::mem::zeroed();
                tp.privilege_count = 1;
                tp.privileges[0] = LuidAndAttributes { luid, attributes: SE_PRIVILEGE_ENABLED };
                AdjustTokenPrivileges(
                    token,
                    0,
                    &mut tp as *mut _ as *mut c_void,
                    std::mem::size_of::<TokenPrivileges>() as u32,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
            }
        }
        CloseHandle(token);
    }
}

#[tauri::command]
fn send_media_key(action: String) -> Result<(), String> {
    media_log(&format!("send_media_key command: {}", action));
    if !send_media_command(&action) {
        // Helper not reachable — fall back to direct injection (works for
        // same-integrity windows, e.g. desktop media players).
        let _ = simulate_media_key(&action);
    }
    Ok(())
}

#[tauri::command]
fn save_hotkeys(app: tauri::AppHandle, hotkeys: HotkeyConfig) -> Result<(), String> {
    let mut config = load_config(&app);
    config.hotkeys = Some(hotkeys);
    save_config_to_disk(&app, &config);
    Ok(())
}

#[tauri::command]
fn load_hotkeys(app: tauri::AppHandle) -> Result<Option<HotkeyConfig>, String> {
    let config = load_config(&app);
    Ok(config.hotkeys)
}

// --- Global hotkey registration (low-level keyboard hook) ---
//
// A WH_KEYBOARD_LL hook intercepts keys system-wide, regardless of which
// window/app has focus, so user-captured combos like Ctrl+F5 always fire.

#[repr(C)]
#[derive(Copy, Clone)]
struct Kbdllhookstruct {
    vk_code: u32,
    scan_code: u32,
    flags: u32,
    time: u32,
    dw_extra_info: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct WMsg {
    hwnd: usize,
    message: u32,
    w_param: usize,
    l_param: isize,
    time: u32,
    pt_x: i32,
    pt_y: i32,
}

#[derive(Copy, Clone)]
struct HookAction {
    mods: u32,
    vk: u16,
    media_vk: u16,
}

const MOD_ALT: u32 = 0x0001;
const MOD_CONTROL: u32 = 0x0002;
const MOD_SHIFT: u32 = 0x0004;
const MOD_WIN: u32 = 0x0008;

static HOTKEY_RUNNING: AtomicBool = AtomicBool::new(false);
static HOTKEY_THREAD: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);
static HOTKEY_THREAD_ID: AtomicU32 = AtomicU32::new(0);
static HOOK_ACTIONS: Mutex<Vec<HookAction>> = Mutex::new(Vec::new());
static ARMED_KEYS: OnceLock<Mutex<HashSet<u16>>> = OnceLock::new();

fn armed_keys() -> std::sync::MutexGuard<'static, HashSet<u16>> {
    ARMED_KEYS
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap()
}

fn modifiers_held_now() -> u32 {
    extern "system" {
        fn GetAsyncKeyState(v_key: i32) -> i16;
    }
    let mut m = 0u32;
    unsafe {
        let down = |vk: i32| (GetAsyncKeyState(vk) as i32 & 0x8000) != 0;
        if down(0x11) { m |= MOD_CONTROL; } // VK_CONTROL
        if down(0x10) { m |= MOD_SHIFT; }  // VK_SHIFT
        if down(0x12) { m |= MOD_ALT; }    // VK_MENU
        if down(0x5B) || down(0x5C) { m |= MOD_WIN; } // VK_LWIN / VK_RWIN
    }
    m
}

unsafe extern "system" fn keyboard_hook_proc(n_code: i32, w_param: usize, l_param: isize) -> isize {
    extern "system" {
        fn CallNextHookEx(hhk: *mut std::ffi::c_void, n_code: i32, w_param: usize, l_param: isize) -> isize;
    }
    const HC_ACTION: i32 = 0;
    const WM_KEYDOWN: u32 = 0x0100;
    const WM_KEYUP: u32 = 0x0101;
    const WM_SYSKEYDOWN: u32 = 0x0104;
    const WM_SYSKEYUP: u32 = 0x0105;

    if n_code != HC_ACTION {
        return CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param);
    }

    let msg = w_param as u32;
    let kbd = &*(l_param as *const Kbdllhookstruct);
    let vk = kbd.vk_code as u16;

    if msg == WM_KEYUP || msg == WM_SYSKEYUP {
        armed_keys().remove(&vk);
        return CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param);
    }

    if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
        let mods_now = modifiers_held_now();
        let matched = HOOK_ACTIONS
            .lock()
            .unwrap()
            .iter()
            .find(|a| a.vk == vk && a.mods == mods_now)
            .copied();
        if let Some(a) = matched {
            media_log(&format!("HOTKEY fired vk=0x{:X} mods=0x{:X} -> media_vk=0x{:X}", vk, mods_now, a.media_vk));
            let mut armed = armed_keys();
            if !armed.insert(vk) {
                // Key still held — swallow the auto-repeat, don't re-trigger.
                return 1;
            }
            drop(armed);
            let action = match a.media_vk {
                0xB3 => "play_pause",
                0xB0 => "next",
                _ => "prev",
            };
            // Inject via the non-elevated helper (never block the hook thread).
            let _ = std::thread::spawn(move || {
                let ok = send_media_command(action);
                media_log(&format!("send_media_command -> {}", ok));
                if !ok {
                    media_log("  falling back to direct simulate_media_key");
                    let r = simulate_media_key(action);
                    media_log(&format!("  direct simulate result: {:?}", r));
                }
            });
            return 1; // consume the combo so apps never receive it
        }
    }

    CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param)
}

fn stop_hotkey_thread() {
    extern "system" {
        fn PostThreadMessageW(id_thread: u32, msg: u32, w_param: usize, l_param: isize) -> i32;
    }
    const WM_QUIT: u32 = 0x0012;

    let mut guard = HOTKEY_THREAD.lock().unwrap();
    if let Some(handle) = guard.take() {
        HOTKEY_RUNNING.store(false, Ordering::SeqCst);
        let tid = HOTKEY_THREAD_ID.load(Ordering::SeqCst);
        if tid != 0 {
            unsafe { PostThreadMessageW(tid, WM_QUIT, 0, 0); }
        }
        let _ = handle.join();
    }
    HOTKEY_THREAD_ID.store(0, Ordering::SeqCst);
    armed_keys().clear();
}

fn install_hotkey_hook(actions: Vec<HookAction>) {
    stop_hotkey_thread();
    let empty = actions.is_empty();
    *HOOK_ACTIONS.lock().unwrap() = actions;
    if empty {
        return;
    }

    HOTKEY_RUNNING.store(true, Ordering::SeqCst);
    let handle = std::thread::spawn(move || {
        unsafe {
            extern "system" {
                fn SetWindowsHookExW(id_hook: i32, lpfn: unsafe extern "system" fn(i32, usize, isize) -> isize, h_mod: *mut std::ffi::c_void, dw_thread_id: u32) -> *mut std::ffi::c_void;
                fn UnhookWindowsHookEx(hhk: *mut std::ffi::c_void) -> i32;
                fn GetCurrentThreadId() -> u32;
                fn PeekMessageW(lp_msg: *mut WMsg, h_wnd: usize, w_msg_filter_min: u32, w_msg_filter_max: u32, w_remove_msg: u32) -> i32;
                fn GetMessageW(lp_msg: *mut WMsg, h_wnd: usize, w_msg_filter_min: u32, w_msg_filter_max: u32) -> i32;
                fn TranslateMessage(lp_msg: *const WMsg) -> i32;
                fn DispatchMessageW(lp_msg: *const WMsg) -> isize;
                fn GetLastError() -> u32;
            }
            const WH_KEYBOARD_LL: i32 = 13;

            HOTKEY_THREAD_ID.store(GetCurrentThreadId(), Ordering::SeqCst);

            // The message queue MUST exist before installing the hook,
            // otherwise the low-level hook callback is never invoked.
            let mut msg: WMsg = std::mem::zeroed();
            PeekMessageW(&mut msg, 0, 0, 0, 0x0001); // PM_REMOVE

            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, keyboard_hook_proc, std::ptr::null_mut(), 0);
            if hook.is_null() {
                let err = GetLastError();
                media_log(&format!("SetWindowsHookExW FAILED err={}", err));
                HOTKEY_RUNNING.store(false, Ordering::SeqCst);
                HOTKEY_THREAD_ID.store(0, Ordering::SeqCst);
                return;
            }
            media_log("SetWindowsHookExW OK, entering message loop");

            while HOTKEY_RUNNING.load(Ordering::SeqCst) {
                let ret = GetMessageW(&mut msg, 0, 0, 0);
                if ret <= 0 {
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            UnhookWindowsHookEx(hook);
            HOTKEY_THREAD_ID.store(0, Ordering::SeqCst);
        }
    });
    *HOTKEY_THREAD.lock().unwrap() = Some(handle);
}

// Parses a combo string like "Ctrl+F5" / "Shift+Alt+G" / "F9" / "Space"
// into (modifiers, virtual_key_code). Returns None for media keys
// (they work natively, no registration needed) or unparseable combos.
fn parse_hotkey(key: &str) -> Option<(u32, u16)> {
    let mut mods = 0u32;
    let mut main_key: Option<&str> = None;
    for part in key.split('+') {
        match part.trim().to_lowercase().as_str() {
            "ctrl" | "control" => mods |= MOD_CONTROL,
            "shift" => mods |= MOD_SHIFT,
            "alt" => mods |= MOD_ALT,
            "win" | "meta" => mods |= MOD_WIN,
            _ => main_key = Some(part.trim()),
        }
    }
    let main_key = main_key?;
    if main_key.starts_with("Media") {
        return None;
    }
    let upper = main_key.to_uppercase();
    let vk: u16 = match upper.as_str() {
        "SPACE" => 0x20,
        "BACKSPACE" => 0x08,
        "TAB" => 0x09,
        "ENTER" => 0x0D,
        "ESC" | "ESCAPE" => 0x1B,
        "HOME" => 0x24,
        "END" => 0x23,
        "PAGEUP" => 0x21,
        "PAGEDOWN" => 0x22,
        "INSERT" => 0x2D,
        "DELETE" => 0x2E,
        "UP" => 0x26,
        "DOWN" => 0x28,
        "LEFT" => 0x25,
        "RIGHT" => 0x27,
        _ => {
            if let Some(n) = upper.strip_prefix('F').and_then(|s| s.parse::<u32>().ok()) {
                if (1..=24).contains(&n) {
                    0x6F + n as u16
                } else {
                    0
                }
            } else {
                let bytes = upper.as_bytes();
                if bytes.len() == 1 && bytes[0].is_ascii_digit() {
                    0x30 + (bytes[0] - b'0') as u16
                } else if bytes.len() == 1 && bytes[0].is_ascii_uppercase() {
                    bytes[0] as u16
                } else {
                    0
                }
            }
        }
    };
    if vk == 0 {
        return None;
    }
    // Letters/digits without a modifier would intercept normal typing — skip them.
    if mods == 0 && (0x30..=0x5A).contains(&vk) {
        return None;
    }
    Some((mods, vk))
}

#[tauri::command]
fn register_music_hotkeys(_app: tauri::AppHandle, hotkeys: HotkeyConfig) -> Result<(), String> {
    let mut actions: Vec<HookAction> = Vec::new();
    for (bind, media_vk) in [
        (hotkeys.play_pause.as_deref(), 0xB3u16),
        (hotkeys.next_track.as_deref(), 0xB0u16),
        (hotkeys.prev_track.as_deref(), 0xB1u16),
    ] {
        if let Some(bind) = bind {
            media_log(&format!("bind: {} -> 0x{:X}", bind, media_vk));
            if let Some((mods, vk)) = parse_hotkey(bind) {
                actions.push(HookAction { mods, vk, media_vk });
                media_log(&format!("  parsed: mods=0x{:X} vk=0x{:X}", mods, vk));
            } else {
                media_log("  NOT parsed (media key or unsupported)");
            }
        }
    }
    media_log(&format!("installing hook with {} actions", actions.len()));
    install_hotkey_hook(actions);
    Ok(())
}

// Enables/disables the Global Media Keys plugin. When disabling, the hook is
// torn down so binds no longer fire; when enabling, saved binds are re-registered.
#[tauri::command]
fn set_media_keys_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let mut config = load_config(&app);
    config.media_keys_enabled = Some(enabled);
    save_config_to_disk(&app, &config);

    if enabled {
        if let Some(ref hotkeys) = config.hotkeys {
            register_music_hotkeys(app, hotkeys.clone())?;
        }
    } else {
        stop_hotkey_thread();
        media_log("media keys plugin disabled, hook stopped");
    }
    Ok(())
}

// --- Clipboard Manager Plugin ---
// Watches the Windows clipboard while enabled, keeping a rolling history of
// copied text (dedup + move-to-front). The frontend can show the buffer and
// copy any item back with a click.

const CLIPBOARD_HISTORY_LIMIT: usize = 50;

static CLIPBOARD_RUNNING: AtomicBool = AtomicBool::new(false);
static CLIPBOARD_THREAD: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);
static CLIPBOARD_HISTORY: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn clipboard_history_path(app: &tauri::AppHandle) -> PathBuf {
    get_data_dir(app).join("clipboard_history.json")
}

fn clipboard_sequence() -> u32 {
    extern "system" {
        fn GetClipboardSequenceNumber() -> u32;
    }
    unsafe { GetClipboardSequenceNumber() }
}

fn read_clipboard_text() -> Option<String> {
    unsafe {
        extern "system" {
            fn OpenClipboard(h_wnd: *mut std::ffi::c_void) -> i32;
            fn CloseClipboard() -> i32;
            fn GetClipboardData(u_format: u32) -> *mut std::ffi::c_void;
            fn GlobalLock(h_mem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
            fn GlobalUnlock(h_mem: *mut std::ffi::c_void) -> i32;
        }
        const CF_UNICODETEXT: u32 = 13;
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return None;
        }
        let data = GetClipboardData(CF_UNICODETEXT);
        if data.is_null() {
            CloseClipboard();
            return None;
        }
        let ptr = GlobalLock(data);
        if ptr.is_null() {
            CloseClipboard();
            return None;
        }
        let mut chars: Vec<u16> = Vec::new();
        let mut i = 0usize;
        loop {
            let ch = *((ptr as *const u16).add(i));
            if ch == 0 {
                break;
            }
            chars.push(ch);
            i += 1;
        }
        GlobalUnlock(data);
        CloseClipboard();
        Some(String::from_utf16_lossy(&chars))
    }
}

fn write_clipboard_text(text: &str) -> Result<(), String> {
    unsafe {
        extern "system" {
            fn OpenClipboard(h_wnd: *mut std::ffi::c_void) -> i32;
            fn CloseClipboard() -> i32;
            fn EmptyClipboard() -> i32;
            fn SetClipboardData(u_format: u32, h_mem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
            fn GlobalAlloc(u_flags: u32, dw_bytes: usize) -> *mut std::ffi::c_void;
            fn GlobalLock(h_mem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
            fn GlobalUnlock(h_mem: *mut std::ffi::c_void) -> i32;
        }
        const CF_UNICODETEXT: u32 = 13;
        const GMEM_MOVEABLE: u32 = 0x0002;
        let mut wide: Vec<u16> = text.encode_utf16().collect();
        wide.push(0);
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return Err("Cannot open clipboard".into());
        }
        EmptyClipboard();
        let mem = GlobalAlloc(GMEM_MOVEABLE, wide.len() * 2);
        if mem.is_null() {
            CloseClipboard();
            return Err("GlobalAlloc failed".into());
        }
        let dest = GlobalLock(mem) as *mut u16;
        if dest.is_null() {
            CloseClipboard();
            return Err("GlobalLock failed".into());
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), dest, wide.len());
        GlobalUnlock(mem);
        SetClipboardData(CF_UNICODETEXT, mem);
        CloseClipboard();
    }
    Ok(())
}

fn clipboard_watcher_loop(app: tauri::AppHandle) {
    let history_path = clipboard_history_path(&app);
    if let Ok(data) = fs::read_to_string(&history_path) {
        if let Ok(hist) = serde_json::from_str::<Vec<String>>(&data) {
            *CLIPBOARD_HISTORY.lock().unwrap() = hist;
        }
    }
    let mut last_seq = clipboard_sequence();
    while CLIPBOARD_RUNNING.load(Ordering::SeqCst) {
        let seq = clipboard_sequence();
        if seq != last_seq {
            last_seq = seq;
            if let Some(text) = read_clipboard_text() {
                let text = text.trim().to_string();
                if !text.is_empty() {
                    let mut hist = CLIPBOARD_HISTORY.lock().unwrap();
                    if let Some(pos) = hist.iter().position(|h| *h == text) {
                        let t = hist.remove(pos);
                        hist.insert(0, t);
                    } else {
                        hist.insert(0, text);
                    }
                    hist.truncate(CLIPBOARD_HISTORY_LIMIT);
                    let snapshot = hist.clone();
                    drop(hist);
                    if let Ok(data) = serde_json::to_string(&snapshot) {
                        fs::write(&history_path, data).ok();
                    }
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(400));
    }
}

fn start_clipboard_watcher(app: tauri::AppHandle) {
    stop_clipboard_thread();
    CLIPBOARD_RUNNING.store(true, Ordering::SeqCst);
    let handle = std::thread::spawn(move || clipboard_watcher_loop(app));
    *CLIPBOARD_THREAD.lock().unwrap() = Some(handle);
}

fn stop_clipboard_thread() {
    CLIPBOARD_RUNNING.store(false, Ordering::SeqCst);
    if let Some(handle) = CLIPBOARD_THREAD.lock().unwrap().take() {
        let _ = handle.join();
    }
}

#[tauri::command]
fn set_clipboard_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let mut config = load_config(&app);
    config.clipboard_enabled = Some(enabled);
    save_config_to_disk(&app, &config);
    if enabled {
        start_clipboard_watcher(app);
    } else {
        stop_clipboard_thread();
    }
    Ok(())
}

#[tauri::command]
fn get_clipboard_history() -> Vec<String> {
    CLIPBOARD_HISTORY.lock().unwrap().clone()
}

#[tauri::command]
fn copy_clipboard_item(text: String) -> Result<(), String> {
    write_clipboard_text(&text)
}

#[tauri::command]
fn clear_clipboard_history(app: tauri::AppHandle) -> Result<(), String> {
    *CLIPBOARD_HISTORY.lock().unwrap() = Vec::new();
    fs::remove_file(clipboard_history_path(&app)).ok();
    Ok(())
}

// --- Auto-Accept Match Plugin ---
// Watches for the green "ACCEPT" button that appears during matchmaking in
// Dota 2 / CS2 / CS:GO / Valorant / LoL / Apex and clicks it. The loop is
// lazy: while no known game process is running it just sleeps and checks the
// process list (cheap). Only when a game is up does it capture the central
// screen region and look for the button.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoAcceptGame {
    pub key: String,
    pub name: String,
    pub processes: Vec<String>,
}

pub fn auto_accept_games() -> Vec<AutoAcceptGame> {
    vec![
        AutoAcceptGame {
            key: "dota2".into(),
            name: "Dota 2".into(),
            processes: vec!["dota2".into()],
        },
        AutoAcceptGame {
            key: "cs2".into(),
            name: "CS2 / CS:GO".into(),
            processes: vec!["cs2".into(), "csgo".into()],
        },
        AutoAcceptGame {
            key: "valorant".into(),
            name: "Valorant".into(),
            processes: vec!["valorant".into()],
        },
        AutoAcceptGame {
            key: "lol".into(),
            name: "LoL / TFT".into(),
            processes: vec!["league of legends".into()],
        },
        AutoAcceptGame {
            key: "apex".into(),
            name: "Apex Legends".into(),
            processes: vec!["r5apex".into()],
        },
    ]
}

fn selected_auto_accept_processes(app: &tauri::AppHandle) -> Vec<String> {
    let config = load_config(app);
    let selected = config.auto_accept_games.unwrap_or_default();
    if selected.is_empty() {
        return Vec::new();
    }
    auto_accept_games()
        .into_iter()
        .filter(|g| selected.contains(&g.key))
        .flat_map(|g| g.processes)
        .collect()
}

static AUTO_ACCEPT_RUNNING: AtomicBool = AtomicBool::new(false);
static AUTO_ACCEPT_THREAD: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);

fn any_game_running(app: &tauri::AppHandle) -> bool {
    selected_auto_accept_processes(app)
        .iter()
        .any(|name| find_pid_by_name(name).is_some())
}

#[repr(C)]
struct EnumWindowState {
    pids: std::collections::HashSet<u32>,
    found: *mut std::ffi::c_void,
}

unsafe extern "system" fn enum_windows_proc(
    hwnd: *mut std::ffi::c_void,
    l_param: *mut std::ffi::c_void,
) -> i32 {
    let state = &mut *(l_param as *mut EnumWindowState);
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if state.pids.contains(&pid) && IsWindowVisible(hwnd) != 0 {
        state.found = hwnd;
        return 0; // stop enumeration
    }
    1 // continue
}

// Finds the first visible top-level window belonging to any selected game
// process. The click is delivered straight to this window via PostMessage, so
// the real cursor is never moved.
fn find_game_window(app: &tauri::AppHandle) -> *mut std::ffi::c_void {
    let pids: std::collections::HashSet<u32> = selected_auto_accept_processes(app)
        .iter()
        .filter_map(|name| find_pid_by_name(name))
        .collect();
    if pids.is_empty() {
        return std::ptr::null_mut();
    }
    let mut state = EnumWindowState {
        pids,
        found: std::ptr::null_mut(),
    };
    unsafe {
        EnumWindows(
            Some(enum_windows_proc),
            &mut state as *mut EnumWindowState as *mut std::ffi::c_void,
        );
        state.found
    }
}

// Captures the central region of the primary display into a top-down BGRA
// buffer. Returns (screen_x, screen_y, width, height, pixels).
fn capture_center_region() -> Option<(i32, i32, i32, i32, Vec<u8>)> {
    unsafe {
        extern "system" {
            fn GetSystemMetrics(n_index: i32) -> i32;
            fn GetDC(h_wnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
            fn ReleaseDC(h_wnd: *mut std::ffi::c_void, h_dc: *mut std::ffi::c_void) -> i32;
            fn CreateCompatibleDC(h_dc: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
            fn DeleteDC(h_dc: *mut std::ffi::c_void) -> i32;
            fn CreateCompatibleBitmap(
                h_dc: *mut std::ffi::c_void,
                cx: i32,
                cy: i32,
            ) -> *mut std::ffi::c_void;
            fn SelectObject(
                h_dc: *mut std::ffi::c_void,
                h_obj: *mut std::ffi::c_void,
            ) -> *mut std::ffi::c_void;
            fn DeleteObject(h_obj: *mut std::ffi::c_void) -> i32;
            fn BitBlt(
                h_dc_dest: *mut std::ffi::c_void,
                x_dest: i32,
                y_dest: i32,
                width: i32,
                height: i32,
                h_dc_src: *mut std::ffi::c_void,
                x_src: i32,
                y_src: i32,
                rop: u32,
            ) -> i32;
            fn GetDIBits(
                h_dc: *mut std::ffi::c_void,
                h_bmp: *mut std::ffi::c_void,
                start: u32,
                lines: u32,
                lp_bits: *mut std::ffi::c_void,
                lp_bmi: *mut std::ffi::c_void,
                usage: u32,
            ) -> i32;
        }

        #[repr(C)]
        struct BITMAPINFOHEADER {
            bi_size: u32,
            bi_width: i32,
            bi_height: i32,
            bi_planes: u16,
            bi_bit_count: u16,
            bi_compression: u32,
            bi_size_image: u32,
            bi_x_pels_per_meter: i32,
            bi_y_pels_per_meter: i32,
            bi_clr_used: u32,
            bi_clr_important: u32,
        }
        #[repr(C)]
        struct BITMAPINFO {
            bmi_header: BITMAPINFOHEADER,
            bmi_colors: [u32; 0],
        }

        const SM_CXSCREEN: i32 = 0;
        const SM_CYSCREEN: i32 = 1;
        const SRCCOPY: u32 = 0x00CC0020;
        const BI_RGB: u32 = 0;
        const DIB_RGB_COLORS: u32 = 0;

        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        if screen_w <= 0 || screen_h <= 0 {
            return None;
        }

        // Central region ~62% of width, ~55% of height. The ACCEPT button
        // always appears centered on screen in these games.
        let w = (screen_w as f64 * 0.62) as i32;
        let h = (screen_h as f64 * 0.55) as i32;
        let sx = (screen_w - w) / 2;
        let sy = (screen_h - h) / 2;

        let screen_dc = GetDC(std::ptr::null_mut());
        if screen_dc.is_null() {
            return None;
        }
        let mem_dc = CreateCompatibleDC(screen_dc);
        let bmp = CreateCompatibleBitmap(screen_dc, w, h);
        if mem_dc.is_null() || bmp.is_null() {
            if !mem_dc.is_null() {
                DeleteDC(mem_dc);
            }
            if !bmp.is_null() {
                DeleteObject(bmp);
            }
            ReleaseDC(std::ptr::null_mut(), screen_dc);
            return None;
        }

        let old = SelectObject(mem_dc, bmp);
        BitBlt(mem_dc, 0, 0, w, h, screen_dc, sx, sy, SRCCOPY);

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmi_header.bi_size = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmi_header.bi_width = w;
        bmi.bmi_header.bi_height = -h; // top-down
        bmi.bmi_header.bi_planes = 1;
        bmi.bmi_header.bi_bit_count = 32;
        bmi.bmi_header.bi_compression = BI_RGB;

        let mut pixels = vec![0u8; (w * h * 4) as usize];
        let ok = GetDIBits(
            mem_dc,
            bmp,
            0,
            h as u32,
            pixels.as_mut_ptr() as *mut std::ffi::c_void,
            &mut bmi as *mut BITMAPINFO as *mut std::ffi::c_void,
            DIB_RGB_COLORS,
        );

        SelectObject(mem_dc, old);
        DeleteObject(bmp);
        DeleteDC(mem_dc);
        ReleaseDC(std::ptr::null_mut(), screen_dc);

        if ok == 0 {
            return None;
        }
        Some((sx, sy, w, h, pixels))
    }
}

// Scans the captured BGRA pixels for a large green cluster (the ACCEPT
// button). Returns the on-screen click point of its center.
fn find_accept_button(sx: i32, sy: i32, w: i32, h: i32, pixels: &[u8]) -> Option<(i32, i32)> {
    // Minimum button size relative to the capture region (covers 720p..4K).
    let min_w = (w as f64 * 0.08) as i32;
    let min_h = (h as f64 * 0.05) as i32;

    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;
    let mut count: u64 = 0;

    for y in 0..h {
        let row = (y as usize) * (w as usize) * 4;
        for x in 0..w {
            let i = row + (x as usize) * 4;
            let b = pixels[i] as i32;
            let g = pixels[i + 1] as i32;
            let r = pixels[i + 2] as i32;
            // Saturated green: green clearly dominates red and blue.
            if g > 120 && g > r + 40 && g > b + 40 {
                if x < min_x {
                    min_x = x;
                }
                if x > max_x {
                    max_x = x;
                }
                if y < min_y {
                    min_y = y;
                }
                if y > max_y {
                    max_y = y;
                }
                count += 1;
            }
        }
    }

    if count < 200 {
        return None;
    }
    let bw = max_x - min_x;
    let bh = max_y - min_y;
    // The accept button is a wide, short rectangle; reject scattered noise.
    if bw < min_w || bh < min_h || bh > bw * 3 {
        return None;
    }
    Some((sx + (min_x + max_x) / 2, sy + (min_y + max_y) / 2))
}

fn auto_accept_loop(app: tauri::AppHandle) {
    let mut last_click = std::time::Instant::now()
        .checked_sub(std::time::Duration::from_secs(60))
        .unwrap_or(std::time::Instant::now());

    while AUTO_ACCEPT_RUNNING.load(Ordering::SeqCst) {
        // Lazy: no selected game process -> nothing to do, just check again.
        if !any_game_running(&app) {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            continue;
        }

        // Resolve the target game window. If we cannot find one we must NOT
        // click at all: injecting via PostMessage needs a window handle, and
        // the fallback would move the real cursor (which we never do now).
        let hwnd = find_game_window(&app);
        if hwnd.is_null() {
            std::thread::sleep(std::time::Duration::from_millis(700));
            continue;
        }

        // Debounce: don't click more than once every 4 seconds so we never
        // double-click a button or fight the user's own cursor.
        if last_click.elapsed() < std::time::Duration::from_secs(4) {
            std::thread::sleep(std::time::Duration::from_millis(700));
            continue;
        }

        if let Some((sx, sy, w, h, pixels)) = capture_center_region() {
            if let Some((cx, cy)) = find_accept_button(sx, sy, w, h, &pixels) {
                media_log(&format!("auto-accept: clickwin ({}, {})", cx, cy));
                send_media_command(&format!("clickwin {},{}", hwnd as usize, format!("{},{}", cx, cy)));
                last_click = std::time::Instant::now();
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(700));
    }
}

fn start_auto_accept(app: tauri::AppHandle) {
    stop_auto_accept_thread();
    AUTO_ACCEPT_RUNNING.store(true, Ordering::SeqCst);
    let handle = std::thread::spawn(move || auto_accept_loop(app));
    *AUTO_ACCEPT_THREAD.lock().unwrap() = Some(handle);
}

fn stop_auto_accept_thread() {
    AUTO_ACCEPT_RUNNING.store(false, Ordering::SeqCst);
    if let Some(handle) = AUTO_ACCEPT_THREAD.lock().unwrap().take() {
        let _ = handle.join();
    }
}

#[tauri::command]
fn set_auto_accept_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let mut config = load_config(&app);
    config.auto_accept_enabled = Some(enabled);
    save_config_to_disk(&app, &config);

    if enabled {
        start_auto_accept(app);
        media_log("auto-accept plugin enabled");
    } else {
        stop_auto_accept_thread();
        media_log("auto-accept plugin disabled");
    }
    Ok(())
}

#[tauri::command]
fn get_auto_accept_games() -> Vec<AutoAcceptGame> {
    auto_accept_games()
}

#[tauri::command]
fn get_selected_auto_accept_games(app: tauri::AppHandle) -> Vec<String> {
    load_config(&app).auto_accept_games.unwrap_or_default()
}

#[tauri::command]
fn set_selected_auto_accept_games(app: tauri::AppHandle, games: Vec<String>) -> Result<(), String> {
    let mut config = load_config(&app);
    config.auto_accept_games = Some(games);
    save_config_to_disk(&app, &config);
    Ok(())
}

#[tauri::command]
async fn apply_app_update(download_url: String) -> Result<String, String> {
    let exe_path = std::env::current_exe().map_err(|e| format!("Cannot get exe path: {}", e))?;
    let exe_dir = exe_path.parent().ok_or("Cannot get exe dir")?;
    let temp_exe = exe_dir.join("freenet-update.exe");
    let bat_path = exe_dir.join("freenet-update.bat");

    // Download new exe
    download_file(&download_url, &temp_exe).await?;

    // Create batch script that replaces exe and restarts
    let bat_content = format!(
        r#"@echo off
timeout /t 2 /nobreak > nul
del /f /q "{}"
move /y "{}" "{}"
start "" "{}"
del /f /q "%~f0""#,
        exe_path.to_string_lossy(),
        temp_exe.to_string_lossy(),
        exe_path.to_string_lossy(),
        exe_path.to_string_lossy(),
    );
    fs::write(&bat_path, bat_content).map_err(|e| format!("Cannot write update bat: {}", e))?;

    // Launch the bat file detached
    use std::process::Command;
    Command::new("cmd.exe")
        .args(["/c", bat_path.to_string_lossy().as_ref()])
        .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Cannot start update: {}", e))?;

    // Exit current process
    std::process::exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Single instance check via named mutex
    {
        use std::ffi::c_void;
        extern "system" {
            fn CreateMutexW(lpMutexAttributes: *mut c_void, bInitialOwner: i32, lpName: *const u16) -> *mut c_void;
            fn GetLastError() -> u32;
        }
        const ERROR_ALREADY_EXISTS: u32 = 183;
        let name: Vec<u16> = "Global\\FREENET_SINGLE_INSTANCE\0".encode_utf16().collect();
        unsafe {
            let _ = CreateMutexW(std::ptr::null_mut(), 1, name.as_ptr());
            if GetLastError() == ERROR_ALREADY_EXISTS {
                return;
            }
        }
    }

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
            stop_all_services,
            list_bat_files,
            get_zapret_user_domains,
            add_zapret_user_domain,
            remove_zapret_user_domain,
            save_config_value,
            load_config_value,
            get_hosts_status,
            set_hosts_bypass,
            get_hosts_providers,
            get_selected_hosts_providers,
            set_selected_hosts_providers,
            check_app_update,
            apply_app_update,
            send_media_key,
            save_hotkeys,
            load_hotkeys,
            register_music_hotkeys,
            set_media_keys_enabled,
            set_clipboard_enabled,
            get_clipboard_history,
            copy_clipboard_item,
            clear_clipboard_history,
            set_auto_accept_enabled,
            get_auto_accept_games,
            get_selected_auto_accept_games,
            set_selected_auto_accept_games,
            get_bypass_services,
            get_active_bypass,
            download_bypass,
            start_bypass,
            stop_bypass,
        ])
        .setup(|app| {
            if !is_admin_check() {
                let exe = std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                let dir = std::path::PathBuf::from(&exe).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                if shell_execute_runas(&exe, "", &dir).is_ok() {
                    std::process::exit(0);
                }
            }

            // Non-elevated media helper (UIPI workaround for input injection).
            spawn_media_helper();

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
                            send_media_command("quit");
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

            // Register music hotkeys on startup (only if the plugin is enabled)
            {
                let config = load_config(&app.handle());
                let enabled = config.media_keys_enabled.unwrap_or(true);
                if enabled {
                    if let Some(ref hotkeys) = config.hotkeys {
                        let _ = register_music_hotkeys(app.handle().clone(), hotkeys.clone());
                    }
                }
                if config.clipboard_enabled.unwrap_or(false) {
                    start_clipboard_watcher(app.handle().clone());
                }
                // NOTE: Auto Accept Match is "IN DEV" and cannot be enabled
                // yet, so it is never auto-started.
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed => {
                    let app = window.app_handle();
                    stop_clipboard_thread();
                    stop_auto_accept_thread();
                    cleanup_all_processes(app);
                    send_media_command("quit");
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
