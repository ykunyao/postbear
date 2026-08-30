# postbear — 熊的桌角

[![CI](https://github.com/ykunyao/postbear/actions/workflows/ci.yml/badge.svg)](https://github.com/ykunyao/postbear/actions/workflows/ci.yml)

一张住在屏幕右下角的桌面便签。

我的另一个项目 [bear 日记站](https://github.com/ykunyao/bear)每天早上 8 点由 GitHub Actions 自动更新当天的天气与一句话日记；postbear 把它带到你的桌面上——熊会当着你的面，把今天的话逐字写完。

## 下载与运行

到 [Releases](https://github.com/ykunyao/postbear/releases) 页面下载 **Windows 10/11 x64** 版本，两种形态任选：

| 文件 | 形态 | 用法 |
|---|---|---|
| `postbear-vX.Y.Z-windows-x64.zip` | 绿色便携版 | 解压后双击 `postbear.exe` 即用，删文件夹即卸载 |
| `postbear-setup-vX.Y.Z.exe` | 安装器 | 下一步式安装：可选安装目录、桌面快捷方式、开机自启 |

> 两者是同一个程序的两种包装，二选一即可。若桌面上已有一只 Bear，先点便签右上角的 ✕ 关掉再启动新的。

## 功能

- 📝 新日记送达时自动逐字书写，像有人在敲键盘
- 🔴 朱红「熊」印，与网页版印章同款
- 🌦 底角落款天气、温度与「第 N 天」
- ✋ 按住卡片任意处拖动，位置自动记忆，下次原位复活
- 📐 卡片高度随文字长度自动伸缩，像真正的手写便签
- 🔇 启动不抢焦点，安安静静住在角落

🐾 在底部左下角，点击它可手动刷新今天的日记。

## 它是怎么工作的

```
GitHub Actions（每天 08:00 北京时间）
    └─ 更新 bear 日记站的 data.json
            └─ postbear 定时拉取 → 解析天气与文本 → 打字机动画呈现
```

数据双通道拉取（自定义域名 HTTPS + GitHub Pages 回退），网络失败时保持上一次的内容安静等待重试。

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
- 推送 `v*` 标签：自动构建绿色版 zip 与安装器并发布到 Releases

发个新版只要三行：改 `Cargo.toml` 版本号 → `git tag vX.Y.Z` → `git push origin vX.Y.Z`。
