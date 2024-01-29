#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Local;
use log::{info, Level, LevelFilter, Log, Metadata, Record};
use std::sync::{Arc, Mutex};
use tauri::command;
use tauri::Manager;
use tauri::SystemTray;
use tauri::{CustomMenuItem, SystemTrayMenu, SystemTrayMenuItem};

mod endpoints;
mod http_server;
mod wcferry;
use http_server::HttpServer;

struct FrontendLogger {
    app_handle: tauri::AppHandle,
}

impl Log for FrontendLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let msg = format!(
                "{} [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            );
            self.app_handle.emit_all("log-message", msg).unwrap();
        }
    }

    fn flush(&self) {}
}

struct AppState {
    http_server: HttpServer,
}

#[command]
async fn start_server(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    host: String,
    port: u16,
) -> Result<(), String> {
    let host_bytes = host
        .split('.')
        .map(|part| part.parse::<u8>().unwrap_or(0))
        .collect::<Vec<u8>>()
        .try_into()
        .map_err(|_| "Invalid host address".to_string())?;

    {
        let mut app_state = state.inner().lock().unwrap();
        app_state.http_server.start(host_bytes, port)?;
    }

    info!("Server started on http://{}:{}", host, port);
    Ok(())
}

#[command]
async fn stop_server(state: tauri::State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    {
        let mut app_state = state.inner().lock().unwrap();
        app_state.http_server.stop()?;
    }

    info!("Server stopped");
    Ok(())
}

fn handle_system_tray_event(app_handle: &tauri::AppHandle, event: tauri::SystemTrayEvent) {
    match event {
        tauri::SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "quit" => {
                std::process::exit(0);
            }
            "hide" => {
                if let Some(window) = app_handle.get_window("main") {
                    window.hide().unwrap();
                    let tray_handle = app_handle.tray_handle();
                    tray_handle.get_item("hide").set_enabled(false).unwrap();
                    tray_handle.get_item("show").set_enabled(true).unwrap();
                }
            }
            "show" => {
                if let Some(window) = app_handle.get_window("main") {
                    window.show().unwrap();
                    let tray_handle = app_handle.tray_handle();
                    tray_handle.get_item("show").set_enabled(false).unwrap();
                    tray_handle.get_item("hide").set_enabled(true).unwrap();
                }
            }
            _ => {}
        },
        _ => {}
    }
}

fn main() {
    let show = CustomMenuItem::new("show".to_string(), "显示").disabled();
    let hide = CustomMenuItem::new("hide".to_string(), "隐藏");
    let quit = CustomMenuItem::new("quit".to_string(), "退出");

    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_item(hide)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    let tray = SystemTray::new().with_menu(tray_menu);

    let app = tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.app_handle();

            // 初始化日志记录器
            log::set_boxed_logger(Box::new(FrontendLogger { app_handle }))
                .map(|()| log::set_max_level(LevelFilter::Info))
                .expect("Failed to initialize logger");

            Ok(())
        })
        .system_tray(tray)
        .on_system_tray_event(handle_system_tray_event)
        .manage(Arc::new(Mutex::new(AppState {
            http_server: HttpServer::new(),
        })))
        .invoke_handler(tauri::generate_handler![start_server, stop_server]);

    app.run(tauri::generate_context!())
        .expect("error while running tauri application");
}
