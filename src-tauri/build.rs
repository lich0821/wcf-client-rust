fn main() {
    let mut config = prost_build::Config::new();
    config
        .out_dir("src/wcferry")
        .type_attribute(
            ".",
            "#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]",
        )
        .compile_protos(&["src/wcferry/lib/wcf.proto"], &["."])
        .unwrap();
    tauri_build::build()
}
