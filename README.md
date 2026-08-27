# postbear — Bear 的桌角

一只住在屏幕角落的 Bear：[bear 日记站](https://github.com/ykunyao/bear)的桌面伴侣应用。
每天早上 8 点，GitHub Actions 在网页端更新 `data.json`，postbear 负责把它带到你的桌面上——当着你的面，逐字写完今天的日记。

## 状态

🚧 项目刚起步，当前是空的仓库骨架。

| 阶段 | 内容 |
|---|---|
| MVP | 拉取 data.json + 打字机动画 + 天气/天数徽章 + 手动刷新 |
| V1.1 | 无边框置顶小窗、拖拽记位、新日记到达时重播动画 |
| V2 | 本地日记存档翻页、系统通知、打字机音效 |

## 技术栈

- **Rust** + [GPUI](https://github.com/zed-industries/zed)（Zed 编辑器的 GPU 加速 UI 框架）
- 数据源：`https://ykunyao.github.io/bear/data.json`（公开数据契约，与网页版共享）
- HTTP：`ureq`；序列化：`serde_json`

## 开发

```bash
cargo run          # 启动桌面窗口（待实现）
```
