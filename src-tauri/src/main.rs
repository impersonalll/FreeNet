#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let helper_pid: Option<u32> = if args.iter().any(|a| a == "--media-helper") {
        args.iter()
            .position(|a| a == "--media-helper")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse().ok())
    } else {
        read_helper_marker()
    };

    if let Some(pid) = helper_pid {
        freenet_lib::run_media_helper(Some(pid));
    } else {
        freenet_lib::run();
    }
}

// Fallback launch path: when the elevated app cannot spawn the helper via
// CreateProcessAsUser, it asks explorer.exe to start this exe (which then
// runs at the user's medium integrity level). explorer.exe does not forward
// command-line arguments reliably, so helper mode is signalled through a
// marker file that holds the main app's PID. The marker is only honoured
// while that process is actually alive — otherwise it is stale and the app
// starts normally.
fn read_helper_marker() -> Option<u32> {
    let marker = std::env::temp_dir().join("freenet_helper_launch.flag");
    let content = std::fs::read_to_string(&marker).ok()?;
    let _ = std::fs::remove_file(&marker);
    let pid: u32 = content.trim().parse().ok()?;
    if freenet_lib::is_process_alive(pid) {
        Some(pid)
    } else {
        None
    }
}
