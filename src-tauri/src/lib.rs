pub mod agent;
pub mod bus;
pub mod channels;
pub mod cli;
pub mod commands;
pub mod cron;
pub mod error;
pub mod gateway;
pub mod heartbeat;
pub mod models;
pub mod providers;
pub mod runtime;
pub mod services;
pub mod storage;

use runtime::AppRuntime;
use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_stronghold::Builder::new(|_pass| todo!()).build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match AppRuntime::builder().build().await {
                    Ok(rt) => {
                        let rt = Arc::new(rt);
                        if let Err(e) = rt.start().await {
                            tracing::error!(error = %e, "failed_to_start_runtime");
                            return;
                        }
                        handle.manage(rt);
                        tracing::info!("app_runtime_injected");
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "failed_to_build_runtime");
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::conversation::send_message,
            commands::conversation::get_session_history,
            commands::daily::get_daily,
            commands::daily::save_daily,
            commands::tasks::list_tasks,
            commands::tasks::create_task,
            commands::tasks::update_task_status,
            commands::tasks::rebuild_tasks_index,
            commands::knowledge::search_knowledge,
            commands::knowledge::get_knowledge,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::set_api_key,
            commands::system::get_system_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
