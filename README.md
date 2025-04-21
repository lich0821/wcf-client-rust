# WeChatFerry HTTP 客户端（基于 Rust）

又一个 [WeChatFerry](https://github.com/lich0821/WeChatFerry) 的 HTTP 客户端。[🙋 FAQ](https://mp.weixin.qq.com/s/YvgFFhF6D-79kXDzRqtg6w)

<details><summary><font color="#FF0000" size="5">免责声明【必读】</font></summary>

本工具仅供学习和技术研究使用，不得用于任何商业或非法行为，否则后果自负。

本工具的作者不对本工具的安全性、完整性、可靠性、有效性、正确性或适用性做任何明示或暗示的保证，也不对本工具的使用或滥用造成的任何直接或间接的损失、责任、索赔、要求或诉讼承担任何责任。

本工具的作者保留随时修改、更新、删除或终止本工具的权利，无需事先通知或承担任何义务。

本工具的使用者应遵守相关法律法规，尊重微信的版权和隐私，不得侵犯微信或其他第三方的合法权益，不得从事任何违法或不道德的行为。

本工具的使用者在下载、安装、运行或使用本工具时，即表示已阅读并同意本免责声明。如有异议，请立即停止使用本工具，并删除所有相关文件。

</details>

<details><summary>点击查看功能清单</summary>

- 查询登录状态
- 获取登录账号信息
- 获取消息类型
- 获取联系人
- 获取可查询数据库
- 获取数据库所有表
- 获取语音消息
- 发送文本消息（可 @）
- 发送图片消息
- 发送文件消息
- 发送卡片消息
- 拍一拍群友
- 转发消息
- 开启接收消息
- 关闭接收消息
- 查询数据库
- 获取朋友圈消息
- 下载图片、视频、文件
- 解密图片
- 添加群成员
- 删除群成员
- 邀请群成员

</details>

| ![碲矿](https://s2.loli.net/2023/09/25/fub5VAPSa8srwyM.jpg) | ![赞赏](https://s2.loli.net/2023/09/25/gkh9uWZVOxzNPAX.jpg) |
| :---------------------------------------------------------: | :---------------------------------------------------------: |
|                   后台回复 `WCF` 加群交流                   |                       如果你觉得有用                        |

## 快速开始

> ℹ️ 如果跑过机器人，先将机器人停止，然后退出微信，再开始，以避免奇奇怪怪的问题。

- 安装微信 `3.9.12.51`（[这里能找到](https://github.com/lich0821/WeChatFerry/releases/latest)）
- 下载 [最新版安装包](https://github.com/lich0821/wcf-client-rust/releases/latest)
- 安装
- 运行
- 启动，按日志提示操作

### 回调示例

如果不懂回调，玩这个其实不是很合适。尽管如此，这里还是提供一个示例。

```py
#! /usr/bin/env python3
# -*- coding: utf-8 -*-

import uvicorn
from fastapi import Body, FastAPI
from pydantic import BaseModel


class Msg(BaseModel):
    is_self: bool
    is_group: bool
    id: int
    type: int
    ts: int
    roomid: str
    content: str
    sender: str
    sign: str
    thumb: str
    extra: str
    xml: str


def msg_cb(msg: Msg = Body(description="微信消息")):
    """示例回调方法，简单打印消息"""
    print(f"收到消息：{msg}")
    return {"status": 0, "message": "成功"}


if __name__ == "__main__":
    app = FastAPI()
    app.add_api_route("/callback", msg_cb, methods=["POST"])
    uvicorn.run(app, host="0.0.0.0", port=8000)
```

## 版本更新

### v39.5.1 (2025.04.21)

- 跟随主版本更新

### v39.4.5 (2025.04.10)

- 跟随主版本更新
- 用户信息增加别名

### v39.4.1 (2025.03.08)

- 跟随主版本更新

### v39.3.3-5 (2025.01.26)

- 修复获取群昵称

## 一起开发（🚫非开发者不用往下看）

### 搭建开发环境

#### 安装 Rustup

访问 Rust 官方网站，下载并运行 Rustup 的安装程序。

在安装过程中，选择默认配置即可，这将安装最新稳定版本的 Rust，包括 `rustc` 、 `cargo` 和 `rustup` 自身。

#### 安装 tauri-cli

```sh
cargo install tauri-cli@1.6.5
```

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

1. 找一张 logo 图片 `your_path/logo.png`（1024\*1024 的 PNG 图片）
2. 通过命令重新生成：`cargo tauri icon your_path/logo.png`

## 感谢各位大佬贡献代码

<a href="https://github.com/lich0821/wcf-client-rust/graphs/contributors">![](https://contrib.rocks/image?repo=lich0821/wcf-client-rust)</a>
