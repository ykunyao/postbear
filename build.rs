// Windows 下把 assets/postbear.ico 内嵌为 exe 的图标资源
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/postbear.ico")
            .compile()
            .expect("failed to compile Windows resources");
    }
}
