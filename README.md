# postbear

[![CI](https://github.com/ykunyao/postbear/actions/workflows/ci.yml/badge.svg)](https://github.com/ykunyao/postbear/actions/workflows/ci.yml)

[bear 日记站](https://github.com/ykunyao/bear) 的 Windows 桌面客户端。从日记站拉取当天数据，以一张便签的形态常驻屏幕右下角。

## 功能

- 朱红「熊」印 + 打字机逐字动画呈现当日内容
- 底角落款天气、温度与「第 N 天」
- 卡片高度随文字长度自动调整（160–480 px）
- 按住卡片任意处拖动，位置持久化到 `%APPDATA%\postbear\position.json`
- 左下角爪印为刷新按钮；启动不抢占系统焦点
- 无边框窗口 + 系统级模糊背景（`WindowBackgroundAppearance::Blurred`）

## 下载与运行

到 [Releases](https://github.com/ykunyao/postbear/releases) 页面下载 Windows 10/11 x64 版本，两种形态任选：

| 文件 | 形态 | 用法 |
|---|---|---|
| `postbear-vX.Y.Z-windows-x64.zip` | 便携版 | 解压后双击 `postbear.exe` 即用，删文件夹即卸载 |
| `postbear-setup-vX.Y.Z.exe` | 安装器 | 向导式安装，可选安装目录、桌面快捷方式、开机自启 |

> 两者是同一个程序的两种包装，二选一即可。若桌面上已有一只，先点便签右上角的 ✕ 关掉再启动新的。

数据来自公开的 bear 日记站（自定义域名 HTTPS 为主，GitHub Pages 回退），无需任何配置。

## 从源码构建

依赖：Windows 10/11、Rust stable（MSVC 工具链）。

```bash
git clone https://github.com/ykunyao/postbear.git
cd postbear
dev.cmd run        # 推荐入口：编译并运行
dev.cmd build      # 仅编译
```

技术栈：[Rust](https://www.rust-lang.org/) + [GPUI](https://github.com/zed-industries/zed)（Zed 编辑器的 GPU 加速 UI 框架，锁定 rev 保证可复现构建）/ ureq / serde。

> `dev.cmd` 是本仓库的开发包装脚本：国内网络直连 github.com 拉 git 依赖时常超时，它会临时改道 SSH 后再调用 cargo。云端 CI 不需要它。

## CI / 发布

- push 到 main：自动跑格式检查 + 编译 + clippy（零警告门槛）
- 推送 `v*` 标签：自动构建便携版与安装器并发布到 Releases

发个新版只要三行：改 `Cargo.toml` 版本号 → `git tag vX.Y.Z` → `git push origin vX.Y.Z`。
