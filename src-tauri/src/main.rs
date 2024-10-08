#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Local;
use local_ip_address::local_ip;
use log::{info, Level, LevelFilter, Log, Metadata, Record};
use service::global_service::{initialize_global, GLOBAL};
use wechat_config::WechatConfig;
use std::fs::{self, File};
use std::io::Write;
use std::ptr;
use std::sync::{Arc, Mutex};
use tauri::{command, AppHandle, CustomMenuItem, Manager, SystemTray, SystemTrayMenu, WindowEvent};
use winapi::{
    shared::winerror::ERROR_ALREADY_EXISTS,
    um::{
        errhandlingapi::GetLastError,
        synchapi::CreateMutexA,
        winuser::{FindWindowA, SetForegroundWindow, ShowWindow, SW_RESTORE},
    },
};

mod endpoints;
mod http_server;
mod wcferry;
mod service;
mod wechat_config;
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
    http_server_running: bool,
    http_server: HttpServer,
}

#[tauri::command]
async fn ip() -> Result<String, String> {
    let local = local_ip().map_err(|e| e.to_string())?;
    Ok(String::from(local.to_string()))
}

#[tauri::command]
async fn is_http_server_running(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<bool, String> {
    let app_state = state.inner().lock().map_err(|e| e.to_string())?;
    Ok(app_state.http_server_running)
}


// 写入配置到文件中
#[command]
fn save_wechat_config(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    config: WechatConfig,
) -> Result<bool, String> {
    // 定义文件路径
    let file_path = ".\\config.json5";

    // 尝试创建并写入文件
    let mut file = File::create(&file_path).map_err(|e| e.to_string())?;
    let json_str = serde_json::to_string(&config).unwrap();
    file.write_all(json_str.as_bytes())
        .map_err(|e| e.to_string())?;
    let global = GLOBAL.get().unwrap();
    let mut wechat_config_lock = global.wechat_config.try_lock().unwrap();
    wechat_config_lock.cburl = config.cburl.clone();
    wechat_config_lock.wsurl = config.wsurl.clone();
    wechat_config_lock.file_dir = config.file_dir.clone();
    info!("Wechat configuration update {:?}", serde_json::to_string(&config));
    Ok(true)
}


// 读取文件
#[command]
fn read_wechat_config(state: tauri::State<'_, Arc<Mutex<AppState>>>) -> Result<WechatConfig, String> {
    // 获取应用安装目录的路径
    // let install_dir = resolve_path(&app, ".", None).map_err(|e| e.to_string())?;
    // 定义文件路径
    let file_path = ".\\config.json5";

    // 尝试创建并写入文件
    let file_str = fs::read_to_string(&file_path).unwrap();

    let wechatconfig: WechatConfig = serde_json::from_str(&file_str).unwrap();
    Ok(wechatconfig)
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
        if !app_state.http_server_running {
            app_state.http_server.start(host_bytes, port, cburl)?;
            app_state.http_server_running = true;
        }
    }

    info!("服务启动，监听 http://{}:{}", host, port);
    info!("浏览器访问 http://localhost:{}/swagger/ 查看文档", port);
    Ok(())
}

#[command]
async fn stop_server(state: tauri::State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    {
        let mut app_state = state.inner().lock().map_err(|e| e.to_string())?;
        if app_state.http_server_running {
            match app_state.http_server.stop() {
                Ok(()) => {
                    app_state.http_server_running = false;
                    ()
                }
                Err(e) => {
                    log::error!("http服务关闭失败 {}", e);
                }
            }
        } else {
            info!("服务已停止");
        }
    }

    info!("服务停止");
    Ok(())
}

#[command]
async fn confirm_exit(app_handle: tauri::AppHandle) {
    let _ = stop_server(app_handle.state()).await;
    std::process::exit(0);
}



fn handle_system_tray_event(app_handle: &tauri::AppHandle, event: tauri::SystemTrayEvent) {
    match event {
        tauri::SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "quit" => {
                app_handle.emit_all("request-exit", ()).unwrap();
            }
            _ => {}
        },
        tauri::SystemTrayEvent::LeftClick { .. } => {
            if let Some(window) = app_handle.get_window("main") {
                window.show().unwrap();
                window.set_focus().unwrap();
            }
        }
        _ => {}
    }
}

fn init_log(handle: AppHandle) {
    log::set_boxed_logger(Box::new(FrontendLogger { app_handle: handle }))
        .map(|()| log::set_max_level(LevelFilter::Info))
        .expect("Failed to initialize logger");
}

#[tokio::main]
async fn main() {
    let mutex_name = b"Global\\wcfrust_app_mutex\0";
    unsafe {
        let handle = CreateMutexA(ptr::null_mut(), 0, mutex_name.as_ptr() as *const i8);
        if handle.is_null() {
            eprintln!("Failed to create mutex.");
            return;
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let window_name = "WcfRust\0".as_ptr() as *const i8;
            let hwnd = FindWindowA(ptr::null(), window_name);
            if !hwnd.is_null() {
                ShowWindow(hwnd, SW_RESTORE);
                SetForegroundWindow(hwnd);
            }
            return;
        }
    }

    let quit = CustomMenuItem::new("quit".to_string(), "退出");
    let tray_menu = SystemTrayMenu::new().add_item(quit);
    let tray = SystemTray::new().with_menu(tray_menu);

    let app = tauri::Builder::default()
        .setup(|app| {
            // init_window(app.get_window("main").unwrap());
            init_log(app.app_handle());
            initialize_global();
            // app.get_window("main").unwrap().open_devtools();
            Ok(())
        })
        .on_window_event(move |event| match event.event() {
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                if let Some(window) = event.window().get_window("main") {
                    window.hide().unwrap();
                }
            }
            _ => {}
        })
        .system_tray(tray)
        .on_system_tray_event(handle_system_tray_event)
        .manage(Arc::new(Mutex::new(AppState {
            http_server_running: false,
            http_server: HttpServer::new(),
        })))
        .invoke_handler(tauri::generate_handler![
            start_server,
            stop_server,
            confirm_exit,
            is_http_server_running,
            ip,
            save_wechat_config,
            read_wechat_config
        ]);

    app.run(tauri::generate_context!())
        .expect("error while running tauri application");
}
