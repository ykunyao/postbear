//! postbear —— 熊的桌角（桌面贴纸形态）
//!
//! 一张住在屏幕右下角的便签：一枚熊印 + 今天的一句话，仅此而已。
//! 按住卡片任意空白处即可拖动；点小脚印刷新；位置自动记忆。
//! 运行：`cargo run`（或 Windows 下 `dev.cmd run`）

use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use gpui::{
    App, AssetSource, Bounds, Context, MouseButton, Pixels, Render, SharedString, Window,
    WindowBackgroundAppearance, WindowBounds, WindowControlArea, WindowOptions, div, point,
    prelude::*, px, relative, rgb, rgba, size, svg,
};
use gpui_platform::application;
use serde::{Deserialize, Serialize};

/// 内嵌资源：爪印与网页版同款 SVG，编译期打进 exe
struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        let bytes: &[u8] = match path {
            "paw.svg" => include_bytes!("../assets/paw.svg"),
            _ => return Ok(None),
        };
        Ok(Some(Cow::Borrowed(bytes)))
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        let _ = path;
        Ok(Vec::new())
    }
}

/// 数据源：自定义域名 HTTPS 为主，GitHub Pages 作回退。
const DATA_URLS: [&str; 2] = [
    "https://woaiwo.xyz/bear/data.json",
    "https://ykunyao.github.io/bear/data.json",
];

/// bear 日记站的上线日（2026-08-26，见 ykunyao/bear 的 index.html）
const LAUNCH_DATE: (i32, u32, u32) = (2026, 8, 26);

/// 贴纸尺寸与右下角边距
const CARD_SIZE: (f32, f32) = (300., 190.);
const SCREEN_MARGIN: f32 = 32.;

#[derive(Debug, Deserialize)]
struct DiaryData {
    weather: String,
    temp: String,
    text: String,
    // data.json 里还有 updated_at，贴纸 UI 用不到，交给 serde 忽略
}

// ---------- 位置持久化 ----------

#[derive(Serialize, Deserialize)]
struct SavedOrigin {
    x: f64,
    y: f64,
}

fn state_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("HOME"))
        .map(|base| {
            PathBuf::from(base)
                .join(if cfg!(windows) {
                    "postbear"
                } else {
                    ".postbear"
                })
                .join("position.json")
        })
}

fn load_origin() -> Option<SavedOrigin> {
    let body = fs::read_to_string(state_path()?).ok()?;
    serde_json::from_str(&body).ok()
}

fn save_origin(origin: &SavedOrigin) {
    let Some(path) = state_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(body) = serde_json::to_string_pretty(origin) {
        let _ = fs::write(path, body);
    }
}

/// 初始窗口原点：优先用上次保存的位置；首次运行放主屏右下角；保存值落出屏幕则回默认。
fn initial_origin(display: Option<Bounds<Pixels>>) -> gpui::Point<Pixels> {
    let sw = px(CARD_SIZE.0);
    let sh = px(CARD_SIZE.1);

    if let Some(saved) = load_origin() {
        let sx = px(saved.x as f32);
        let sy = px(saved.y as f32);
        if let Some(db) = display {
            let min_x = db.origin.x - sw + px(120.);
            let max_x = db.origin.x + db.size.width - px(120.);
            let min_y = db.origin.y - sh + px(80.);
            let max_y = db.origin.y + db.size.height - px(80.);
            if sx > min_x && sx < max_x && sy > min_y && sy < max_y {
                return point(sx, sy);
            }
        }
    }

    let Some(db) = display else {
        return point(px(100.), px(100.));
    };
    point(
        db.origin.x + db.size.width - sw - px(SCREEN_MARGIN),
        db.origin.y + db.size.height - sh - px(SCREEN_MARGIN),
    )
}

// ---------- 数据 ----------

fn weather_name(weather: &str) -> &'static str {
    match weather {
        "sunny" => "晴",
        "cloudy" => "多云",
        "rain" => "雨",
        "snow" => "雪",
        "fog" => "雾",
        "thunder" => "雷雨",
        _ => "—",
    }
}

fn fetch_diary() -> Result<DiaryData, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .build();
    let mut last_err = String::from("no data url");
    for url in DATA_URLS {
        match agent.get(url).call() {
            Ok(resp) => match resp.into_string() {
                Ok(body) => match serde_json::from_str::<DiaryData>(&body) {
                    Ok(data) => return Ok(data),
                    Err(e) => last_err = format!("parse {url}: {e}"),
                },
                Err(e) => last_err = format!("read {url}: {e}"),
            },
            Err(e) => last_err = format!("request {url}: {e}"),
        }
    }
    Err(last_err)
}

fn day_number() -> i64 {
    let launch = chrono::NaiveDate::from_ymd_opt(LAUNCH_DATE.0, LAUNCH_DATE.1, LAUNCH_DATE.2)
        .expect("valid launch date");
    let today = chrono::Local::now().date_naive();
    // 上线当天即第 1 天，与网页版口径一致
    (today - launch).num_days().max(0) + 1
}

enum Status {
    Idle,
    Fetching,
}

/// 由全文估算正文行数，进而推卡片高度（内容决定高度）。
/// CJK 字宽 ≈ 字号，ASCII ≈ 半个字号；误差用缓冲兜住。
fn estimate_lines(text: &str) -> f32 {
    const INNER_W: f32 = CARD_SIZE.0 - 32.; // px(16) 左右内边距
    let mut width = 0.;
    for ch in text.chars() {
        let code = ch as u32;
        if ch == '\n' {
            width += INNER_W;
        } else if code >= 0x2E80 {
            // CJK 及全角区
            width += 15.;
        } else {
            width += 7.6;
        }
    }
    (width / INNER_W).ceil().clamp(1., 12.)
}

/// header(印章区) + pt6 + n×15px 行高 1.85 + footer + 缓冲
fn card_height_for(text: &str) -> f32 {
    (154. + 32. * estimate_lines(text)).clamp(200., 480.)
}

struct BearState {
    day_no: i64,
    status: Status,

    weather_name: &'static str,
    temp: SharedString,

    // 打字机：round 是回合号，旧动画发现自己过期就自动退场
    full_text: SharedString,
    typed_len: usize,
    round: u64,

    // 动态高度：期望值随文本变化，render 时应用到窗口
    desired_h: f32,
    applied_h: Option<f32>,

    // 拖动位置的节流落盘
    last_bounds_save: Instant,
}

impl BearState {
    fn begin_typing(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.round += 1;
        let round = self.round;
        self.full_text = text.into();
        self.typed_len = 0;
        cx.notify();

        cx.spawn(async move |this, cx| {
            loop {
                let continue_typing = this
                    .update(cx, |state, cx| {
                        if state.round != round {
                            return false; // 已被新一轮顶替
                        }
                        let total = state.full_text.chars().count();
                        state.typed_len = usize::min(state.typed_len + 1, total);
                        cx.notify();
                        state.typed_len < total
                    })
                    .unwrap_or(false);
                if !continue_typing {
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(45))
                    .await;
            }
        })
        .detach();
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.round += 1; // 作废进行中的打字动画
        self.status = Status::Fetching;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { fetch_diary() }).await;
            let _ = this.update(cx, |state, cx| {
                if let Ok(data) = result {
                    state.weather_name = weather_name(&data.weather);
                    state.temp = SharedString::from(data.temp);
                    state.desired_h = card_height_for(&data.text);
                    state.apply_text(data.text, cx);
                }
                state.status = Status::Idle;
                cx.notify();
            });
        })
        .detach();
    }

    fn apply_text(&mut self, text: String, cx: &mut Context<Self>) {
        cx.notify();
        self.begin_typing(text, cx);
    }
}

impl Render for BearState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 内容决定高度：期望高度变化时，在帧首应用一次窗口尺寸
        if self.applied_h != Some(self.desired_h) {
            self.applied_h = Some(self.desired_h);
            window.resize(size(px(CARD_SIZE.0), px(self.desired_h)));
        }

        let typed: String = self.full_text.chars().take(self.typed_len).collect();
        let still_typing = self.typed_len < self.full_text.chars().count();
        let fetching = matches!(self.status, Status::Fetching);

        // 米黄纸 + 恒定观感：不做悬停变色，透明度固定避免突变
        let paper = rgba(0xF3EBD8F7);
        let ink = rgb(0x3D3528);
        let ink_soft = rgb(0x6E6350);
        let line = rgb(0xCDBFA2);

        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .rounded(px(12.))
            .border_1()
            .border_color(line)
            .bg(paper)
            .text_color(ink)
            .shadow_md()
            // 原生标题栏热区：按住卡上任何非按钮处交给系统拖动
            .window_control_area(WindowControlArea::Drag)
            // 隐蔽的关闭钮，常驻右上角
            .child(
                div()
                    .id("close")
                    .absolute()
                    .top(px(6.))
                    .right(px(8.))
                    .cursor_pointer()
                    .px(px(6.))
                    .rounded(px(4.))
                    .text_size(px(13.))
                    .text_color(rgb(0xB7A989))
                    .hover(|this| this.text_color(rgb(0xB0432E)))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(|_, _, window, _| window.remove_window()))
                    .child("✕"),
            )
            // 顶部：朱红「熊」印，与网页版同款
            .child(
                div().flex().justify_center().pt(px(12.)).child(
                    div()
                        .size(px(56.))
                        .bg(rgb(0xB0432E))
                        .rounded(px(6.))
                        .shadow_sm()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .size(px(46.))
                                .border_1()
                                .border_color(rgb(0xF6EFDD))
                                .rounded(px(3.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(26.))
                                .text_color(rgb(0xF6EFDD))
                                .font_family("KaiTi")
                                .child("熊"),
                        ),
                ),
            )
            // 正文：紧跟头像起笔，写不下时区内滚动，外形永远紧凑
            .child(
                div()
                    .id("diary-flow")
                    .flex_1()
                    .overflow_y_scroll()
                    .w_full()
                    .px(px(16.))
                    .pt(px(6.))
                    .child(
                        div()
                            .w_full()
                            .text_size(px(15.))
                            .line_height(relative(1.85))
                            .text_color(ink)
                            .child(if still_typing {
                                format!("{typed}▌")
                            } else {
                                typed.clone()
                            }),
                    ),
            )
            // 底行：左脚印=刷新入口，右天气温度与天数
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(14.))
                    .pb(px(10.))
                    .child(
                        div()
                            .id("paw")
                            .cursor_pointer()
                            .p(px(2.))
                            .when(fetching, |this| this.opacity(0.35))
                            .hover(|this| this.opacity(0.65))
                            .active(|this| this.opacity(0.9))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .on_click(cx.listener(|this, _, _, cx| this.refresh(cx)))
                            .child(
                                svg()
                                    .w(px(17.))
                                    .h(px(14.))
                                    .path("paw.svg")
                                    .text_color(rgb(0x8A7C63)),
                            ),
                    )
                    .child(div().text_size(px(11.)).text_color(ink_soft).child(format!(
                        "{} {}° · 第 {} 天",
                        self.weather_name, self.temp, self.day_no
                    ))),
            )
    }
}

fn main() {
    application().with_assets(Assets).run(|cx: &mut App| {
        let display = cx.primary_display().map(|d| d.bounds());
        let origin = initial_origin(display);
        // 启动即按占位文本给高度，避免开窗后跳变
        let initial_h = card_height_for("熊正在被 Actions 叫醒……");

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin,
                    size: size(px(CARD_SIZE.0), px(initial_h)),
                })),
                titlebar: None,
                focus: false,
                is_resizable: false,
                ..Default::default()
            },
            |window, cx| {
                window.set_background_appearance(WindowBackgroundAppearance::Blurred);
                window.set_window_title("postbear · 熊的桌角");
                cx.new(|cx| {
                    // 窗口尺寸固定，bounds 变化即用户拖动：节流落盘新位置
                    cx.observe_window_bounds(window, |state: &mut BearState, window, _| {
                        if state.last_bounds_save.elapsed() >= Duration::from_millis(500) {
                            state.last_bounds_save = Instant::now();
                            let b = window.bounds();
                            save_origin(&SavedOrigin {
                                x: b.origin.x.to_f64(),
                                y: b.origin.y.to_f64(),
                            });
                        }
                    })
                    .detach();

                    let mut state = BearState {
                        day_no: day_number(),
                        status: Status::Idle,
                        weather_name: "—",
                        temp: "--".into(),
                        full_text: "".into(),
                        typed_len: 0,
                        round: 0,
                        desired_h: initial_h,
                        applied_h: None,
                        last_bounds_save: Instant::now(),
                    };
                    state.begin_typing("熊正在被 Actions 叫醒……", cx);
                    state.refresh(cx);
                    state
                })
            },
        )
        .unwrap();
        cx.activate(false); // 贴纸不打扰：启动不抢焦点
    });
}
