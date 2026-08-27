//! postbear —— Bear 的桌角（桌面宠物形态）
//!
//! 一个悬浮在屏幕右下角的小家伙：大白脸上下呼吸蹦跳，头顶飘着天气小气泡，
//! 脚下两只爪印交替踏步。按住任意处拖动；点左爪刷新；无任何文字。
//! 运行：`cargo run`（或 Windows 下 `dev.cmd run`）

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use gpui::{
    div, prelude::*, px, rgb, rgba, size, Animation, AnimationExt as _, App, Bounds, Context,
    MouseButton, Pixels, Render, Window, WindowBackgroundAppearance, WindowBounds, WindowKind,
    WindowOptions, bounce, ease_in_out,
};
use gpui_platform::application;
use serde::{Deserialize, Serialize};

/// 数据源：自定义域名 HTTPS 为主，GitHub Pages 作回退。
const DATA_URLS: [&str; 2] = [
    "https://woaiwo.xyz/bear/data.json",
    "https://ykunyao.github.io/bear/data.json",
];

/// 宠物活动区尺寸与右下角边距
const PET_SIZE: (f32, f32) = (150., 200.);
const SCREEN_MARGIN: f32 = 36.;

#[derive(Debug, Deserialize)]
struct DiaryData {
    weather: String,
    // data.json 的其余字段（temp/text/updated_at）宠物 UI 不展示，交给 serde 忽略
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
    let sw = px(PET_SIZE.0);
    let sh = px(PET_SIZE.1);

    if let Some(saved) = load_origin() {
        let sx = px(saved.x as f32);
        let sy = px(saved.y as f32);
        if let Some(db) = display {
            let min_x = db.origin.x - sw + px(120.);
            let max_x = db.origin.x + db.size.width - px(120.);
            let min_y = db.origin.y - sh + px(80.);
            let max_y = db.origin.y + db.size.height - px(80.);
            if sx > min_x && sx < max_x && sy > min_y && sy < max_y {
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

fn weather_display(weather: &str) -> (&'static str, &'static str) {
    match weather {
        "sunny" => ("😊", "☀"),
        "cloudy" => ("😌", "☁"),
        "rain" => ("😴", "☂"),
        "snow" => ("❄", "❅"),
        "fog" => ("🌫", "≈"),
        "thunder" => ("⛈", "⚡"),
        _ => ("👧", ""),
    }
}

fn fetch_diary_weather() -> Result<String, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .build();
    let mut last_err = String::from("no data url");
    for url in DATA_URLS {
        match agent.get(url).call() {
            Ok(resp) => match resp.into_string() {
                Ok(body) => match serde_json::from_str::<DiaryData>(&body) {
                    Ok(data) => return Ok(data.weather),
                    Err(e) => last_err = format!("parse {url}: {e}"),
                },
                Err(e) => last_err = format!("read {url}: {e}"),
            },
            Err(e) => last_err = format!("request {url}: {e}"),
        }
    }
    Err(last_err)
}

enum Status {
    Idle,
    Fetching,
}

struct BearState {
    status: Status,
    avatar: &'static str,
    weather_icon: &'static str,
    last_bounds_save: Instant,
}

impl BearState {
    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.status = Status::Fetching;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { fetch_diary_weather() })
                .await;
            let _ = this.update(cx, |state, cx| {
                if let Ok(weather) = result {
                    let (avatar, icon) = weather_display(&weather);
                    state.avatar = avatar;
                    state.weather_icon = icon;
                }
                state.status = Status::Idle;
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for BearState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let fetching = matches!(self.status, Status::Fetching);

        div()
            .relative()
            .flex()
            .flex_col()
            .items_center()
            .size_full()
            // 整个活动区皆可拖动；小按钮各自拦住冒泡
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, window, _| window.start_window_move()),
            )
            // 头部：大脸呼吸蹦跳，右上角天气气泡
            .child(
                div()
                    .relative()
                    .mt(px(24.))
                    .w(px(96.))
                    .h(px(86.))
                    // 天气气泡徽章
                    .child(
                        div()
                            .absolute()
                            .top(px(0.))
                            .right(px(0.))
                            .size(px(28.))
                            .rounded_full()
                            .bg(rgba(0xFFFFFFC8))
                            .border_1()
                            .border_color(rgb(0xCDBFA2))
                            .when(fetching, |this| this.opacity(0.35))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(14.))
                            .text_color(rgb(0x6E6350))
                            .child(self.weather_icon),
                    )
                    // 大脸：以 bounce 缓动循环起伏
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .flex()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(62.))
                                    .line_height(px(76.))
                                    .child(self.avatar)
                                    .with_animation(
                                        "pet-bob",
                                        Animation::new(Duration::from_millis(2200))
                                            .repeat()
                                            .with_easing(bounce(ease_in_out)),
                                        |face, delta| face.top(px(-8. * delta)),
                                    ),
                            ),
                    ),
            )
            // 弹性空隙把爪印压到底部
            .child(div().flex_1())
            // 脚下：两只交替踏步的爪印；左爪兼任刷新
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap(px(16.))
                    .pb(px(16.))
                    .child(paw("paw-a", "paw-hop-a", 1400, fetching, true, cx))
                    .child(paw("paw-b", "paw-hop-b", 1700, false, false, cx)),
            )
            // 隐蔽的退出点：右下角一枚极淡的小叉
            .child(
                div()
                    .id("quit")
                    .absolute()
                    .bottom(px(2.))
                    .right(px(5.))
                    .cursor_pointer()
                    .opacity(0.22)
                    .hover(|this| this.opacity(0.9))
                    .text_size(px(11.))
                    .text_color(rgb(0xB7A989))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(|_, _, window, _| window.remove_window()))
                    .child("✕"),
            )
    }
}

/// 一只原地踏步的爪印。interactive 时点击触发刷新。
fn paw(
    hit_id: &'static str,
    anim_id: &'static str,
    period_ms: u64,
    dimmed: bool,
    interactive: bool,
    cx: &mut Context<BearState>,
) -> impl IntoElement {
    div()
        .id(hit_id)
        .relative()
        .w(px(26.))
        .h(px(22.))
        .when(dimmed, |this| this.opacity(0.45))
        .when(interactive, |this| {
            this.cursor_pointer()
                .hover(|this| this.opacity(0.75))
                .active(|this| this.opacity(1.0))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(|this, _, _, cx| this.refresh(cx)))
        })
        .child(
            div()
                .absolute()
                .inset_0()
                .flex()
                .justify_center()
                .items_end()
                .child(
                    div()
                        .text_size(px(17.))
                        .child("🐾")
                        .with_animation(
                            anim_id,
                            Animation::new(Duration::from_millis(period_ms))
                                .repeat()
                                .with_easing(bounce(ease_in_out)),
                            |paw_el, delta| paw_el.top(px(-5. * delta)),
                        ),
                ),
        )
}

fn main() {
    application().run(|cx: &mut App| {
        let display = cx.primary_display().map(|d| d.bounds());
        let origin = initial_origin(display);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin,
                    size: size(px(PET_SIZE.0), px(PET_SIZE.1)),
                })),
                titlebar: None,
                focus: false,
                is_resizable: false,
                kind: WindowKind::PopUp, // Windows 上即 WS_EX_TOOLWINDOW|TOPMOST：无任务栏图标、恒在顶层
                ..Default::default()
            },
            |window, cx| {
                window.set_background_appearance(WindowBackgroundAppearance::Transparent);
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

                    let (_avatar, icon) = weather_display("");
                    BearState {
                        status: Status::Idle,
                        avatar: "👧",
                        weather_icon: icon,
                        last_bounds_save: Instant::now(),
                    }
                })
            },
        )
        .unwrap();
        cx.activate(false); // 宠物不打扰：启动不抢焦点
    });
}
