# postbear — Bear 的桌角

一只住在屏幕角落的 Bear：[bear 日记站](https://github.com/ykunyao/bear)的桌面伴侣应用。
每天早上 8 点，GitHub Actions 在网页端更新 `data.json`，postbear 负责把它带到你的桌面上——当着你的面，逐字写完今天的日记。

## 状态

✅ **MVP 已可运行**：360×520 日记小窗，启动后自动拉取数据并逐字"写"出今天的日记。

- 拉取 `data.json`：先试 GitHub Pages（HTTPS），失败自动回退自定义域名直连
- 打字机逐字动画（带回合号取消机制，连点刷新不会出现两套动画互踩）
- REC NO #N 天数徽章 / 天气表情 + 温度条 / 当日问候语 / ↻ 手动刷新

| 里程碑 | 内容 |
|---|---|
| ~~MVP~~ | ✅ 窗口骨架 + 数据拉取 + 打字机动画 |
| V1.1 | 无边框置顶小窗、拖拽记位、新日记轮询重播动画 |
| V2 | 本地日记存档翻页、系统通知、打字机音效 |

## 技术栈

- **Rust** + [GPUI](https://github.com/zed-industries/zed)（Zed 编辑器的 GPU 加速 UI 框架）
- 数据源：`https://ykunyao.github.io/bear/data.json`（公开数据契约，与网页版共享）
- HTTP：`ureq`；序列化：`serde_json`

## 开发

```bash
dev.cmd run         # Windows 推荐入口：自动为本机不稳的 github HTTPS 改道 SSH 再调 cargo
# 或直接：
cargo run           # 构建运行；gpui 锁定在固定 rev，依赖进过缓存后无需网络
```

> gpui / gpui_platform 以 git rev 方式锁定 Zed 主干代码。若首次构建时拉取 github 超时，
> 用 `dev.cmd` 替代 `cargo` 即可（原理见脚本内注释与 `.cargo/config.toml`）。

技术备忘：gpui 在 edition 2024 下有保留关键字 `gen`（generator 预留）；`overflow_y_scroll()`
等滚动方法只存在于带 `.id(...)` 的 Stateful 元素上；元素树要求内容 `'static`，借用 self 的
数据需克隆为 owned 值。
