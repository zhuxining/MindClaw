mod commands;
mod config;
mod error;
mod services;
mod storage;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = commands::AppState::new();

    tauri::Builder::default()
        .manage(app_state)
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(
            tauri_plugin_stronghold::Builder::new(|pass| {
                use std::hash::{Hash, Hasher};
                let mut h = std::collections::hash_map::DefaultHasher::new();
                pass.hash(&mut h);
                h.finish().to_le_bytes().to_vec()
            })
            .build(),
        )
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
        .invoke_handler(tauri::generate_handler![
            // 泛化渠道命令（新）
            commands::set_channel_credentials,
            commands::test_channel_connection,
            commands::get_channel_connection_status,
            commands::list_channels,
            commands::poll_channel_messages,
            // 兼容旧版飞书命令（deprecated）
            commands::set_feishu_credentials,
            commands::test_feishu_connection,
            commands::get_feishu_connection_status,
            commands::poll_feishu_messages,
            // 消息管理
            commands::get_messages,
            commands::clear_messages,
            commands::process_message,
            // 路由规则
            commands::get_route_rules,
            commands::add_route_rule,
            commands::remove_route_rule,
            // 配置
            commands::get_config,
            commands::update_feishu_poll_interval,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
