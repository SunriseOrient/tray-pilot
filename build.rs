//! 构建脚本：为 Windows 可执行文件嵌入图标与版本信息。
//!
//! 只有 .exe 文件本身带资源段，资源管理器、任务栏、开始菜单才会显示图标；
//! Rust 编译器不会自动做这件事，需要把 .rc 编译进链接阶段。

fn main() {
    println!("cargo:rerun-if-changed=assets/tray-pilot.rc");
    println!("cargo:rerun-if-changed=assets/app.ico");

    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/app.ico");
        res.set("FileDescription", "TrayPilot");
        res.set("ProductName", "TrayPilot");
        res.set("OriginalFilename", "tray-pilot.exe");
        // 只对 release 或不限制时嵌入，避免拖慢增量编译。
        if let Err(err) = res.compile() {
            // 没有资源编译器（比如没装 Windows SDK）时不让构建失败，
            // 只是 exe 不带图标。
            println!("cargo:warning=嵌入 Windows 资源失败：{err}");
        }
    }
}
