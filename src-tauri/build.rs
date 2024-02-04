fn main() {
    let mut config = prost_build::Config::new();
    config
        .out_dir("src/wcferry")
        .type_attribute(
            ".",
            "#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]",
        )
        .field_attribute(".wcf.WxMsg.type", "#[serde(rename = \"type\")]")
        .field_attribute(
            ".wcf.TextMsg.msg",
            "#[schema(example = \"换行用\n就可以了\n @昵称随便写\")]",
        )
        .field_attribute(
            ".wcf.TextMsg.receiver",
            "#[schema(example = \"88888888888@chatroom\")]",
        )
        .field_attribute(
            ".wcf.TextMsg.aters",
            "#[schema(example = \"wxid_88888888888888\")]",
        )
        .compile_protos(&["src/wcferry/lib/wcf.proto"], &["."])
        .unwrap();
    tauri_build::build()
}
