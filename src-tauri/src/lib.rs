mod bing;
mod cache_manager;
mod commands;
mod downloader;
mod errors;
mod gemini;
mod openai_compatible;
mod process_utils;
mod progress;
mod subtitle_commands;
mod subtitle_export;
mod subtitle_store;
mod subtitle_types;
mod tool_installer;
mod tool_paths;
mod whisper;
mod whisper_manager;
mod ytdlp;

use downloader::DownloadState;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(DownloadState::default())
        .manage(subtitle_commands::SubtitleTaskState::default())
        .setup(|app| {
            subtitle_store::recover_interrupted_projects(&app.handle());
            subtitle_store::clean_all_temp(&app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::check_dependencies,
            commands::check_ytdlp_update,
            commands::parse_video,
            commands::start_download,
            commands::cancel_download,
            commands::open_path,
            commands::open_parent_folder,
            cache_manager::get_cache_status,
            cache_manager::clear_app_cache,
            tool_installer::install_missing_tools,
            tool_installer::update_ytdlp,
            tool_installer::get_tools_directory,
            subtitle_commands::get_gemini_settings,
            subtitle_commands::save_gemini_settings,
            subtitle_commands::test_gemini_connection,
            subtitle_commands::test_openai_compatible_connection,
            subtitle_commands::estimate_transcription_cost,
            subtitle_commands::analyze_subtitle_source,
            subtitle_commands::create_subtitle_project,
            subtitle_commands::import_subtitle_track,
            subtitle_commands::list_subtitle_projects,
            subtitle_commands::delete_subtitle_project,
            subtitle_commands::clear_subtitle_projects,
            subtitle_commands::start_gemini_transcription,
            subtitle_commands::start_whisper_transcription,
            subtitle_commands::start_gemini_translation,
            subtitle_commands::start_bing_translation,
            subtitle_commands::start_openai_compatible_translation,
            subtitle_commands::start_gemini_polish,
            subtitle_commands::retry_subtitle_chunk,
            subtitle_commands::cancel_subtitle_task,
            subtitle_commands::save_subtitle_segments,
            subtitle_commands::export_subtitles,
            subtitle_commands::burn_subtitles,
            whisper_manager::list_whisper_models,
            whisper_manager::list_whisper_runtimes,
            whisper_manager::download_whisper_model,
            whisper_manager::download_whisper_runtime,
            whisper_manager::delete_whisper_model,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run YDLite");
}
