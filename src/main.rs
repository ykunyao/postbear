//! postbear —— Bear 的桌角（桌面贴纸形态）
//!
//! 无边框圆角小卡，住在屏幕右下角：平时半透明淡化，鼠标靠近才显形；
//! 顶部抓取条可整卡拖动，位置会被记住。
//! 运行：`cargo run`（或 Windows 下 `dev.cmd run`）

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use gpui::{
    div, prelude::*, px, relative, rgb, rgba, size, App, Bounds, Context, MouseButton,
    Pixels, Render, SharedString, Window, WindowBackgroundAppearance, WindowBounds,
    WindowOptions,
};
use gpui_platform::application;
use serde::{Deserialize, Serialize};

/// 数据源：自定义域名 HTTPS 为主，GitHub Pages 作回退。
const DATA_URLS: [&str; 2] = [
    "https://woaiwo.xyz/bear/data.json",
    "https://ykunyao.github.io/bear/data.json",
];

/// bear 日记站的上线日（2026-08-26，见 ykunyao/bear 的 index.html）
const LAUNCH_DATE: (i32, u32, u32) = (2026, 8, 26);

/// 贴纸尺寸与右下角边距
const CARD_SIZE: (f32, f32) = (360., 520.);
const SCREEN_MARGIN: f32 = 28.;

#[derive(Debug, Deserialize)]
struct DiaryData {
    weather: String,
    temp: String,
    text: String,
    updated_at: String,
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
                .join(if cfg!(windows) { "postbear" } else { ".postbear" })
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
            if (sx > min_x && sx < max_x && sy > min_y && sy < max_y) || display.is_none() {
                return gpui::point(sx, sy);
            }
        }
    }

    let Some(db) = display else {
        return gpui::point(px(100.), px(100.));
    };
    gpui::point(
        db.origin.x + db.size.width - sw - px(SCREEN_MARGIN),
        db.origin.y + db.size.height - sh - px(SCREEN_MARGIN),
    )
}

// ---------- 数据 ----------

struct WeatherUi {
    avatar: &'static str,
    icon: &'static str,
    mood: &'static str,
    greeting: &'static str,
}

fn weather_display(weather: &str) -> WeatherUi {
    let (avatar, icon, mood, greeting) = match weather {
        "sunny" => ("😊", "☀", "CLEAR", "今天天气晴朗，Bear 心情很好"),
        "cloudy" => ("😌", "☁", "CLOUDY", "今天云很多，但 Bear 心情不赖"),
        "rain" => ("😴", "☂", "RAIN", "下雨天，Bear 最适合窝着发呆"),
        "snow" => ("❄", "❅", "SNOW", "下雪了，Bear 想看窗外的世界"),
        "fog" => ("🌫", "≡", "FOG", "今天有雾，Bear 有点看不清路"),
        "thunder" => ("⛈", "⚡", "STORM", "打雷了，Bear 缩在被窝里"),
        _ => ("👧", "", "?", "Bear 在记录今天的天气"),
    };
    WeatherUi {
        avatar,
        icon,
        mood,
        greeting,
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
    (today - launch).num_days().max(0)
}

fn today_label() -> String {
    chrono::Local::now()
        .format("%b %d, %Y")
        .to_string()
        .to_uppercase()
}

enum Status {
    Idle,
    Fetching,
    Failed(String),
}

struct BearState {
    day_no: i64,
    status: Status,

    avatar: &'static str,
    icon: &'static str,
    mood: &'static str,
    tagline: &'static str,
    weather_line: SharedString,
    updated_at: Option<SharedString>,

    // 打字机：round 是回合号，旧动画发现自己过期就自动退场
    full_text: SharedString,
    typed_len: usize,
    round: u64,

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

        cx.spawn(async move |this, cx| loop {
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
        })
        .detach();
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.round += 1; // 作废进行中的打字动画
        self.status = Status::Fetching;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { fetch_diary() }).await;
            let _ = this.update(cx, |state, cx| match result {
                Ok(data) => state.apply(data, cx),
                Err(err) => {
                    state.status = Status::Failed(err);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn apply(&mut self, data: DiaryData, cx: &mut Context<Self>) {
        let ui = weather_display(&data.weather);
        self.avatar = ui.avatar;
        self.icon = ui.icon;
        self.mood = ui.mood;
        self.tagline = ui.greeting;
        self.weather_line = SharedString::from(format!("{} {}°", ui.mood, data.temp));
        self.updated_at = Some(SharedString::from(data.updated_at));
        self.status = Status::Idle;
        cx.notify();
        self.begin_typing(data.text, cx);
    }
}

impl Render for BearState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let typed: String = self.full_text.chars().take(self.typed_len).collect();
        let still_typing = self.typed_len < self.full_text.chars().count();
        let status_line = match &self.status {
            Status::Idle => "— with GitHub Actions —".to_string(),
            Status::Fetching => "fetching …".to_string(),
            Status::Failed(err) => format!("failed: {err}"),
        };

        // 纸面配色 + 半透明贴纸质感（背景为系统级模糊）
        let paper = rgba(0xF3EBD8D9); // 米黄，约 85% 不透明度
        let card = rgba(0xEDE3CCF0);
        let ink = rgb(0x3D3528);
        let ink_soft = rgb(0x6E6350);
        let accent = rgb(0xB0432E);
        let line = rgb(0xCDBFA2);

        div()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .rounded(px(14.))
            .border_1()
            .border_color(line)
            .bg(paper)
            .text_color(ink)
            .shadow_lg()
            // 贴纸的呼吸：平时半透明，靠近才完全显形
            .opacity(0.62)
            .hover(|this| this.opacity(1.0))
            // 抓取条：按住即可拖动整卡（原生窗口移动），右侧独立 ✕
            .child(
                div()
                    .id("grab-bar")
                    .h(px(26.))
                    .w_full()
                    .flex()
                    .items_center()
                    .pl(px(14.))
                    .pr(px(6.))
                    .bg(rgba(0xEDE3CC99))
                    .border_b_1()
                    .border_color(line)
                    .text_size(px(11.))
                    .text_color(ink_soft)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_, _, window, _| window.start_window_move()),
                    )
                    .child(format!("⠿  REC. NO. #{}", self.day_no))
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .justify_center()
                            .child(self.updated_at.as_deref().unwrap_or("").to_string()),
                    )
                    .child(
                        div()
                            .id("close")
                            .cursor_pointer()
                            .px(px(7.))
                            .rounded(px(4.))
                            .hover(|this| this.bg(rgba(0xB0432E33)))
                            .text_color(accent)
                            .on_click(cx.listener(|_, _, window, _| window.remove_window()))
                            .child("✕"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .px(px(20.))
                    .py(px(14.))
                    .gap(px(9.))
                    // 报头
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(1.))
                            .child(div().text_size(px(40.)).child(self.avatar))
                            .child(
                                div()
                                    .text_size(px(17.))
                                    .text_color(ink)
                                    .child("· BEAR 的今日日记 ·"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(accent)
                                    .child(self.tagline),
                            ),
                    )
                    // 元信息条
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .px(px(4.))
                            .py(px(5.))
                            .border_b_1()
                            .border_t_1()
                            .border_color(line)
                            .text_size(px(11.))
                            .text_color(ink_soft)
                            .child(format!("DATE {}", today_label()))
                            .child(format!("WX {} {}", self.weather_line, self.icon)),
                    )
                    // 日记卡片
                    .child(
                        div()
                            .flex_1()
                            .bg(card)
                            .rounded(px(6.))
                            .border_1()
                            .border_color(line)
                            .p(px(14.))
                            .id("diary-card")
                            .overflow_y_scroll()
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .text_size(px(10.5))
                                            .text_color(accent)
                                            .child("今日日记"),
                                    )
                                    .child(
                                        div().text_size(px(14.5)).line_height(relative(1.85)).child(
                                            if still_typing {
                                                format!("{typed}▌")
                                            } else {
                                                typed.clone()
                                            },
                                        ),
                                    ),
                            ),
                    )
                    // 刷新行
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .id("refresh")
                                    .cursor_pointer()
                                    .px(px(12.))
                                    .py(px(4.))
                                    .rounded(px(4.))
                                    .border_1()
                                    .border_color(line)
                                    .active(|this| this.opacity(0.7))
                                    .hover(|this| this.bg(rgba(0xCDBFA244)))
                                    .on_click(cx.listener(|this, _, _, cx| this.refresh(cx)))
                                    .text_size(px(12.))
                                    .child("↻ 刷新"),
                            )
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(ink_soft)
                                    .child(status_line),
                            ),
                    )
                    // 小脚印
                    .child(
                        div()
                            .flex()
                            .justify_center()
                            .gap(px(11.))
                            .text_size(px(13.))
                            .text_color(rgb(0xB7A989))
                            .child("🐾")
                            .child("🐾")
                            .child("🐾")
                            .child("🐾")
                            .child("🐾"),
                    ),
            )
    }
}

fn main() {
    application().run(|cx: &mut App| {
        let display = cx.primary_display().map(|d| d.bounds());
        let origin = initial_origin(display);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin,
                    size: size(px(CARD_SIZE.0), px(CARD_SIZE.1)),
                })),
                titlebar: None,
                focus: false,
                is_resizable: false,
                ..Default::default()
            },
            |window, cx| {
                window.set_background_appearance(WindowBackgroundAppearance::Blurred);
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
                        avatar: "👧",
                        icon: "",
                        mood: "",
                        tagline: "一只叫 Bear 的女孩，每天醒来写一行字",
                        weather_line: "--".into(),
                        updated_at: None,
                        full_text: "".into(),
                        typed_len: 0,
                        round: 0,
                        last_bounds_save: Instant::now(),
                    };
                    state.begin_typing("Bear 正在被 Actions 叫醒……", cx);
                    state.refresh(cx);
                    state
                })
            },
        )
        .unwrap();
        cx.activate(false); // 贴纸不打扰：启动不抢焦点
    });
}
