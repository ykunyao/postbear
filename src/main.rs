//! postbear —— Bear 的桌角
//!
//! 每天从 bear 日记站拉取 data.json，在桌面窗口里逐字"写"出今天的日记。
//! 运行：`cargo run`

use std::time::Duration;

use gpui::{
    div, prelude::*, px, relative, rgb, size, App, Bounds, Context, Render, SharedString,
    TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use gpui_platform::application;
use serde::Deserialize;

/// 数据源：先试 GitHub Pages（HTTPS），失败回退自定义域名直连（仅 HTTP）
const DATA_URLS: [&str; 2] = [
    "https://ykunyao.github.io/bear/data.json",
    "http://woaiwo.xyz/bear/data.json",
];

/// bear 日记站的上线日（2026-08-26，见 ykunyao/bear 的 index.html）
const LAUNCH_DATE: (i32, u32, u32) = (2026, 8, 26);

#[derive(Debug, Deserialize)]
struct DiaryData {
    weather: String,
    temp: String,
    text: String,
    updated_at: String,
}

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
    chrono::Local::now().format("%b %d, %Y").to_string().to_uppercase()
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
        self.weather_line =
            SharedString::from(format!("{} {}°", ui.mood, data.temp));
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
        // SharedString 是 Arc 包装，克隆廉价；直接 as_deref 会把 &self 借用泄进 'static 元素树
        let updated_label: SharedString =
            self.updated_at.clone().unwrap_or_else(|| "----".into());
        let status_line = match &self.status {
            Status::Idle => "— faithfully archived · with GitHub Actions —".to_string(),
            Status::Fetching => "fetching data.json …".to_string(),
            Status::Failed(err) => format!("fetch failed: {err}"),
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xF3EBD8))
            .p(px(22.))
            .gap(px(10.))
            .text_color(rgb(0x3D3528))
            // 顶部记录行
            .child(
                div()
                    .flex()
                    .justify_between()
                    .text_size(px(12.))
                    .text_color(rgb(0x6E6350))
                    .child(
                        div().child(
                            div()
                                .child("REC. NO. ")
                                .child(
                                    div()
                                        .text_color(rgb(0xB0432E))
                                        .child(format!("#{}", self.day_no)),
                                ),
                        ),
                    )
                    .child(div().child(updated_label)),
            )
            // 报头：头像 + 标题 + 问候
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(2.))
                    .child(div().text_size(px(44.)).child(self.avatar))
                    .child(
                        div()
                            .text_size(px(20.))
                            .text_color(rgb(0x3D3528))
                            .child("· BEAR 的今日日记 ·"),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(0xB0432E))
                            .child(self.tagline),
                    ),
            )
            // 元信息条
            .child(
                div()
                    .flex()
                    .justify_between()
                    .px(px(4.))
                    .py(px(6.))
                    .border_b_1()
                    .border_t_1()
                    .border_color(rgb(0xCDBFA2))
                    .text_size(px(12.))
                    .text_color(rgb(0x6E6350))
                    .child(format!("DATE {}", today_label()))
                    .child(format!("WX {} {}", self.weather_line, self.icon)),
            )
            // 日记卡片
            .child(
                div()
                    .flex_1()
                    .bg(rgb(0xEDE3CC))
                    .rounded(px(4.))
                    .border_1()
                    .border_color(rgb(0xCDBFA2))
                    .p(px(16.))
                    .id("diary-card")
                    .overflow_y_scroll()
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(0xB0432E))
                                    .child("今日日记"),
                            )
                            .child(
                                div().text_size(px(15.)).line_height(relative(1.9)).child(
                                    if still_typing {
                                        format!("{typed}▌")
                                    } else {
                                        typed.clone()
                                    },
                                ),
                            ),
                    ),
            )
            // 刷新按钮 + 状态
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .id("refresh")
                            .cursor_pointer()
                            .px(px(14.))
                            .py(px(5.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(rgb(0xCDBFA2))
                            .active(|this| this.opacity(0.7))
                            .on_click(cx.listener(|this, _, _, cx| this.refresh(cx)))
                            .text_size(px(13.))
                            .child("↻ 刷新"),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0x6E6350))
                            .child(status_line),
                    ),
            )
            // 小脚印
            .child(
                div()
                    .flex()
                    .justify_center()
                    .gap(px(12.))
                    .text_size(px(14.))
                    .text_color(rgb(0xB7A989))
                    .child("🐾")
                    .child("🐾")
                    .child("🐾")
                    .child("🐾")
                    .child("🐾"),
            )
    }
}

fn main() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(360.), px(520.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("postbear · Bear 的桌角".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|cx| {
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
                    };
                    state.begin_typing("Bear 正在被 Actions 叫醒……", cx);
                    state.refresh(cx);
                    state
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
