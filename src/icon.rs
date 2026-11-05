//! 应用图标资源。
//!
//! 图形来自 `pilot-Fill.svg`（P 形定位针 + 四角星镂空），在构建前离线光栅化，
//! 以**原始 RGBA** 的形式嵌入二进制——这样运行时既不需要 SVG 解析器，
//! 也不需要 PNG 解码器。
//!
//! 颜色按用途区分：
//! - 托盘图标：Windows 任务栏可能是浅色或深色，因此准备了两套单色版本，
//!   运行时按系统主题切换（深色任务栏用白字，浅色任务栏用近黑 #1C1C1E）。
//! - 窗口 / 任务栏图标：靛蓝 → 紫罗兰渐变圆角方块 + 白色图形，与界面强调色一致。

/// 托盘图标边长（像素）。
pub const SIZE: u32 = 32;

/// 应用图标边长（像素）。
pub const APP_ICON_SIZE: u32 = 256;

/// 深色任务栏上的托盘图标（白色图形）。
static TRAY_ON_DARK: &[u8] = include_bytes!("../assets/tray_on_dark.raw");
/// 浅色任务栏上的托盘图标（近黑图形）。
static TRAY_ON_LIGHT: &[u8] = include_bytes!("../assets/tray_on_light.raw");
/// 窗口 / 任务栏图标（渐变底色）。
static APP_ICON: &[u8] = include_bytes!("../assets/app_icon.raw");

/// 取托盘图标的 RGBA 数据。`dark_taskbar` 表示任务栏是深色的。
pub fn tray_icon_rgba(dark_taskbar: bool) -> Vec<u8> {
    if dark_taskbar {
        TRAY_ON_DARK.to_vec()
    } else {
        TRAY_ON_LIGHT.to_vec()
    }
}

/// 取应用图标的 RGBA 数据。
pub fn app_icon_rgba() -> Vec<u8> {
    APP_ICON.to_vec()
}
