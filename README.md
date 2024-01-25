# A Rust Client for WeChatFerry

## 一起开发
### 调试运行
```sh
# 启动运行
cargo tauri dev

# 如果遇到错误，可以启用更多调试信息
RUST_BACKTRACE=full RUST_LOG=debug cargo tauri dev
```

### 验证
#### 查看监听端口
`启动` 后，查看相关端口会显示相关信息
```sh
lsof -i :8888
```

#### 直接访问
```sh
curl http://localhost:8888/api/v1/time
```

### 定制 Logo
1. 找一张 logo 图片 `your_path/logo.png`（1024*1024 的 PNG 图片）
2. 通过命令重新生成：`cargo tauri icon your_path/logo.png`
