use env_logger::Env;
use log::{error, info};
use std::sync::{Arc, Mutex};
use tauri::command;
use tauri::Manager;
use tauri::SystemTray;
use tauri::{CustomMenuItem, SystemTrayMenu, SystemTrayMenuItem};
use tokio::sync::oneshot;
use warp::Filter;

struct AppState {
    shutdown_tx: Option<oneshot::Sender<()>>,
}

#[command]
async fn start_server(state: tauri::State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    info!("Starting server...");

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let (addr, server) = warp::serve(warp::path!("hello" / "world").map(|| "Hello, world!"))
        .bind_with_graceful_shutdown(([127, 0, 0, 1], 8888), async {
            shutdown_rx.await.ok();
        });

    tokio::spawn(async move {
        server.await;
    });

    let mut app_state = state.inner().lock().unwrap();
    app_state.shutdown_tx = Some(shutdown_tx);

    info!("Server started at http://{}", addr);
    Ok(())
}

#[command]
async fn stop_server(state: tauri::State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    info!("Stopping server...");

    let mut app_state = state.inner().lock().unwrap();
    if let Some(shutdown_tx) = app_state.shutdown_tx.take() {
        if shutdown_tx.send(()).is_err() {
            error!("Failed to send shutdown signal");
        }
    }

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
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    info!("Starting main...");
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
        .system_tray(tray)
        .on_system_tray_event(handle_system_tray_event)
        .manage(Arc::new(Mutex::new(AppState { shutdown_tx: None })))
        .invoke_handler(tauri::generate_handler![start_server, stop_server]);

    app.run(tauri::generate_context!())
        .expect("error while running tauri application");
}
