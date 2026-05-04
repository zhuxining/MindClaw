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
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::new().build())
        // TODO: Stronghold key derivation — placeholder hash until API key management is implemented
        // TODO: Stronghold key derivation — placeholder hash until API key management is implemented
        .plugin(tauri_plugin_stronghold::Builder::new(|pass| {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            pass.hash(&mut h);
            h.finish().to_le_bytes().to_vec()
        }).build())
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
                        // 桥接出站消息总线 → Tauri 事件（驱动前端流式输出）
                        let bus = rt.bus().clone();
                        let emit_handle = handle.clone();
                        tauri::async_runtime::spawn(async move {
                            match bus.take_outbound_rx().await {
                                Ok(mut rx) => {
                                    while let Some(msg) = rx.recv().await {
                                        if let Err(e) = emit_handle.emit("mindclaw://agent-event", &msg) {
                                            tracing::warn!(error = %e, "failed_to_emit_agent_event");
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(error = %e, "outbound_rx_unavailable");
                                }
                            }
                        });
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
            commands::conversation::list_sessions,
            commands::conversation::delete_session,
            commands::daily::get_daily,
            commands::daily::save_daily,
            commands::daily::read_note,
            commands::daily::save_note,
            commands::tasks::list_tasks,
            commands::tasks::create_task,
            commands::tasks::get_task,
            commands::tasks::update_task,
            commands::tasks::delete_task,
            commands::tasks::update_task_status,
            commands::tasks::rebuild_tasks_index,
            commands::settings::get_settings,
            commands::settings::get_workspace_prefs,
            commands::settings::set_vault,
            commands::settings::save_settings,
            commands::settings::save_workspace_prefs,
            commands::settings::set_api_key,
            commands::system::get_system_status,
            commands::vault::list_vault_dir,
            commands::vault::list_vault_files_recursive,
            commands::vault::resolve_source_item,
            commands::vault::search_vault,
            commands::vault::get_relevant_notes,
            commands::vault::get_vault_note,
            commands::vault::list_dir,
            commands::vault::list_all_tags,
            commands::vault::list_notes_by_filter,
            commands::vault::list_memories,
            commands::skills::list_skills,
            commands::skills::search_skills,
            commands::skills::activate_skill,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
