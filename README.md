# WeChatFerry HTTP 客户端（基于 Rust）
## 快速开始
* 下载 [最新版安装包](https://github.com/lich0821/wcf-client-rust/releases/latest)
* 安装
* 运行
* 启动

## 版本更新
### v39.0.12.0 (2024.02.05)
* 检查登录状态
* 查询登录账号 wxid
* 获取登录账号信息
* 获取通信录
* 列出所有可查询数据库
* 查询数据库的表信息
* 查询消息类型映射表
* 接收消息
* 刷新朋友圈
* 发送文本消息
* 发送图片消息

|![碲矿](https://s2.loli.net/2023/09/25/fub5VAPSa8srwyM.jpg)|![赞赏](https://s2.loli.net/2023/09/25/gkh9uWZVOxzNPAX.jpg)|
|:-:|:-:|
|后台回复 `WeChatFerry` 加群交流|如果你觉得有用|

## 一起开发（非开发者不要往下看🛑）
### 搭建开发环境
#### 安装 Rustup
访问 Rust 官方网站，下载并运行 Rustup 的安装程序。

在安装过程中，选择默认配置即可，这将安装最新稳定版本的 Rust，包括 `rustc` 、 `cargo` 和 `rustup` 自身。

#### 安装 Protoc
下载适用于 Windows 的 `protoc` 二进制文件。

解压到一个目录，并将该目录添加到你的系统环境变量 PATH 中，这样你就可以在命令行中直接运行 `protoc` 命令了。

#### 验证安装
打开命令行或终端，运行以下命令以确认 Rust 和 Cargo 已正确安装：
```sh
rustc --version
cargo --version
protoc --version
```

当前项目开发环境如下：
```txt
rustc 1.75.0 (82e1608df 2023-12-21)
cargo 1.75.0 (1d8b05cdd 2023-11-20)
libprotoc 22.2
```

### 调试运行
```sh
# 启动运行
cargo tauri dev

# 如果遇到错误，可以启用更多调试信息
RUST_BACKTRACE=full RUST_LOG=debug cargo tauri dev
```

### 验证
点击 `启动`，然后访问 [http://localhost:10010/swagger/](http://localhost:10010/swagger/)。

### 定制 Logo
1. 找一张 logo 图片 `your_path/logo.png`（1024*1024 的 PNG 图片）
2. 通过命令重新生成：`cargo tauri icon your_path/logo.png`
