fn main() {
    let app = tauri::Builder::default();

    app.run(tauri::generate_context!())
        .expect("error while running tauri application");
}
