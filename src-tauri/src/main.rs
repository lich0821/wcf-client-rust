#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Local;
use log::{info, Level, LevelFilter, Log, Metadata, Record};
use std::sync::{Arc, Mutex};
use tauri::{
    command, AppHandle, CustomMenuItem, Manager, SystemTray, SystemTrayMenu, SystemTrayMenuItem,
    WindowEvent,
};

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
    cburl: String,
) -> Result<(), String> {
    let host_bytes = host
        .split('.')
        .map(|part| part.parse::<u8>().unwrap_or(0))
        .collect::<Vec<u8>>()
        .try_into()
        .map_err(|_| "Invalid host address".to_string())?;

    {
        let mut app_state = state.inner().lock().unwrap();
        app_state.http_server.start(host_bytes, port, cburl)?;
    }

    info!("服务启动，监听 http://{}:{}", host, port);
    info!("浏览器访问 http://localhost:{}/swagger/ 查看文档", port);
    Ok(())
}

#[command]
async fn stop_server(state: tauri::State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    {
        let mut app_state = state.inner().lock().unwrap();
        app_state.http_server.stop()?;
    }

    info!("服务停止");
    Ok(())
}

fn cleanup(state: tauri::State<'_, Arc<Mutex<AppState>>>) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    rt.block_on(async {
        if let Err(e) = stop_server(state).await {
            eprintln!("Failed to stop server: {}", e);
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    });
}

fn handle_system_tray_event(app_handle: &tauri::AppHandle, event: tauri::SystemTrayEvent) {
    match event {
        tauri::SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "quit" => {
                cleanup(app_handle.state());
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

fn init_window(window: tauri::Window) {
    window.hide().unwrap();
    if let Ok(Some(monitor)) = window.primary_monitor() {
        let monitor_size = monitor.size();
        if let Ok(window_size) = window.outer_size() {
            let x = (monitor_size.width as i32 - window_size.width as i32) / 2;
            let y = (monitor_size.height as i32 - window_size.height as i32) / 2;
            window
                .set_position(tauri::Position::Logical(tauri::LogicalPosition {
                    x: x.into(),
                    y: y.into(),
                }))
                .unwrap();
        } else {
            let x = (monitor_size.width as i32 - 640) / 2;
            let y = (monitor_size.height as i32 - 320) / 2;
            window
                .set_position(tauri::Position::Logical(tauri::LogicalPosition {
                    x: x.into(),
                    y: y.into(),
                }))
                .unwrap();
        }
    }
    window.show().unwrap();
}

fn init_log(handle: AppHandle) {
    log::set_boxed_logger(Box::new(FrontendLogger { app_handle: handle }))
        .map(|()| log::set_max_level(LevelFilter::Info))
        .expect("Failed to initialize logger");
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
            init_window(app.get_window("main").unwrap());
            init_log(app.app_handle());
            Ok(())
        })
        .on_window_event(move |event| match event.event() {
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                cleanup(event.window().app_handle().clone().state());
                event.window().close().unwrap();
            }
            _ => {}
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
