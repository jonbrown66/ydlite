mod commands;
mod downloader;
mod errors;
mod progress;
mod tool_installer;
mod tool_paths;
mod ytdlp;

use downloader::DownloadState;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(DownloadState::default())
        .invoke_handler(tauri::generate_handler![
            commands::check_dependencies,
            commands::check_ytdlp_update,
            commands::parse_video,
            commands::start_download,
            commands::cancel_download,
            commands::open_path,
            commands::open_parent_folder,
            tool_installer::install_missing_tools,
            tool_installer::update_ytdlp,
            tool_installer::get_tools_directory,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run YDLite");
}
