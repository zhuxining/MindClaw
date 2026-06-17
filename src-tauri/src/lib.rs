mod commands;
mod config;
mod error;
mod secret_store;
mod services;
mod storage;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = std::sync::Arc::new(commands::AppState::new());
    let state_for_setup = app_state.clone();

    tauri::Builder::default()
        .setup(move |app| {
            // ── 托盘图标与菜单 ──────────────────────────────────
            use tauri::{
                image::Image,
                menu::{MenuBuilder, MenuItemBuilder},
                tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
                Manager,
            };

            let show = MenuItemBuilder::with_id("show", "显示 MindClaw").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;
            let menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;

            let png_bytes = include_bytes!("../icons/32x32.png");
            let decoded = image::load_from_memory(png_bytes)
                .expect("托盘图标解码失败")
                .into_rgba8();
            let (width, height) = decoded.dimensions();
            let icon = Image::new_owned(decoded.into_raw(), width, height);

            let _tray = TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .tooltip("MindClaw")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // ── GatewaySupervisor 启动 ─────────────────────────
            tauri::async_runtime::spawn(async move {
                if let Err(error) = state_for_setup.start().await {
                    eprintln!("GatewaySupervisor start failed: {error}");
                }
            });

            Ok(())
        })
        .manage(app_state)
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // 窗口关闭 → 隐藏到托盘，不退出应用
                api.prevent_close();
                let _ = window.hide();
            }
        })
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
            // 渠道凭证
            commands::set_channel_credentials,
            commands::test_channel_connection,
            commands::get_channel_connection_status,
            commands::clear_channel_credentials,
            // 渠道描述符与状态
            commands::list_channel_descriptors,
            commands::list_channels,
            commands::get_channels_status,
            // 消息
            commands::get_messages,
            commands::clear_messages,
            // ACP Server
            commands::list_acp_servers,
            commands::save_acp_server,
            commands::get_acp_server_status,
            commands::fetch_acp_registry,
            commands::install_acp_agent,
            // Agent / Skill / SlashCommand
            commands::list_agents,
            commands::save_agent,
            commands::set_default_agent,
            commands::list_skills,
            commands::save_skill,
            commands::bind_skill,
            commands::list_slash_commands,
            commands::save_slash_command,
            commands::get_conversation_execution_state,
            // 配置
            commands::get_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
