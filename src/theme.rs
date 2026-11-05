//! 视觉主题：配色、字体与 egui 样式。
//!
//! 配色跟随系统的浅色 / 深色设置自动切换。

use std::sync::atomic::{AtomicBool, Ordering};

use eframe::egui;

/// 一套完整的界面配色。
///
/// 配色参照 Apple 的系统色与 grouped 背景层级：
/// `bg`（窗口底色）→ `surface`（卡片 / 菜单）→ `bg_soft`（控件填充）。
#[derive(Clone, Copy)]
pub struct Palette {
    /// 窗口底色（grouped background）。
    pub bg: Color32Alias,
    /// 卡片、菜单、输入框的底色（比窗口底色更「浮」一层）。
    pub surface: Color32Alias,
    /// 控件填充：次要按钮、分段控件轨道等。
    pub bg_soft: Color32Alias,
    /// 分隔线 / 卡片描边。
    pub border: Color32Alias,
    /// 窗口 / 菜单外框描边。
    pub border_strong: Color32Alias,
    /// 主要文字。
    pub text: Color32Alias,
    /// 次要文字。
    pub text_dim: Color32Alias,
    /// 三级文字（代码、占位）。
    pub text_faint: Color32Alias,
    /// 强调色（system blue）。
    pub accent: Color32Alias,
    /// 强调色（悬停）。
    pub accent_hover: Color32Alias,
    /// 强调色（淡染）。
    pub accent_soft: Color32Alias,
    /// 危险操作（system red）。
    pub danger: Color32Alias,
    /// 危险操作（淡染）。
    pub danger_soft: Color32Alias,
    /// 开关打开时的轨道色（system green）。
    pub switch_on: Color32Alias,
    /// 开关关闭时的轨道色。
    pub switch_off: Color32Alias,
    /// 分段控件里被选中的那一段。
    pub segment_selected: Color32Alias,
}

type Color32Alias = egui::Color32;

static DARK: AtomicBool = AtomicBool::new(false);

/// 当前生效的配色。
pub fn palette() -> Palette {
    if DARK.load(Ordering::Relaxed) {
        dark_palette()
    } else {
        light_palette()
    }
}

fn light_palette() -> Palette {
    Palette {
        bg: egui::Color32::from_rgb(245, 245, 247),
        surface: egui::Color32::from_rgb(255, 255, 255),
        bg_soft: egui::Color32::from_rgb(238, 238, 240),
        border: egui::Color32::from_rgb(229, 229, 234),
        border_strong: egui::Color32::from_rgb(208, 208, 214),
        text: egui::Color32::from_rgb(29, 29, 31),
        text_dim: egui::Color32::from_rgb(110, 110, 115),
        text_faint: egui::Color32::from_rgb(142, 142, 147),
        accent: egui::Color32::from_rgb(0, 122, 255),
        accent_hover: egui::Color32::from_rgb(10, 108, 224),
        accent_soft: egui::Color32::from_rgba_unmultiplied(0, 122, 255, 30),
        danger: egui::Color32::from_rgb(255, 59, 48),
        danger_soft: egui::Color32::from_rgba_unmultiplied(255, 59, 48, 28),
        switch_on: egui::Color32::from_rgb(52, 199, 89),
        switch_off: egui::Color32::from_rgb(220, 220, 225),
        segment_selected: egui::Color32::from_rgb(255, 255, 255),
    }
}

fn dark_palette() -> Palette {
    Palette {
        bg: egui::Color32::from_rgb(28, 28, 30),
        surface: egui::Color32::from_rgb(44, 44, 46),
        bg_soft: egui::Color32::from_rgb(58, 58, 60),
        border: egui::Color32::from_rgb(56, 56, 58),
        border_strong: egui::Color32::from_rgb(72, 72, 74),
        text: egui::Color32::from_rgb(245, 245, 247),
        text_dim: egui::Color32::from_rgb(152, 152, 157),
        text_faint: egui::Color32::from_rgb(118, 118, 122),
        accent: egui::Color32::from_rgb(10, 132, 255),
        accent_hover: egui::Color32::from_rgb(64, 156, 255),
        accent_soft: egui::Color32::from_rgba_unmultiplied(10, 132, 255, 52),
        danger: egui::Color32::from_rgb(255, 69, 58),
        danger_soft: egui::Color32::from_rgba_unmultiplied(255, 69, 58, 50),
        switch_on: egui::Color32::from_rgb(48, 209, 88),
        switch_off: egui::Color32::from_rgb(78, 78, 82),
        segment_selected: egui::Color32::from_rgb(99, 99, 102),
    }
}

/// 主题偏好：环境变量 `TRAYPILOT_THEME=light|dark` 优先（调试用），
/// 否则取设置里的选择（默认跟随系统）。
///
/// 注意：内部会加锁读配置，调用时不要持有 `app::shared()` 的锁。
pub fn theme_preference() -> egui::ThemePreference {
    match std::env::var("TRAYPILOT_THEME").as_deref() {
        Ok("light") => return egui::ThemePreference::Light,
        Ok("dark") => return egui::ThemePreference::Dark,
        _ => {}
    }
    use crate::config::ThemeMode;
    match crate::app::shared().config.theme {
        ThemeMode::System => egui::ThemePreference::System,
        ThemeMode::Light => egui::ThemePreference::Light,
        ThemeMode::Dark => egui::ThemePreference::Dark,
    }
}

/// 系统当前是否为深色模式。
fn is_dark(ctx: &egui::Context) -> bool {
    match ctx.options(|o| o.theme_preference) {
        egui::ThemePreference::Dark => true,
        egui::ThemePreference::Light => false,
        egui::ThemePreference::System => matches!(ctx.system_theme(), Some(egui::Theme::Dark)),
    }
}

/// 系统当前是否为深色模式（不依赖 egui 上下文，用于启动时创建托盘图标）。
pub fn system_is_dark() -> bool {
    matches!(theme_preference(), egui::ThemePreference::Dark)
        || (matches!(theme_preference(), egui::ThemePreference::System) && registry_prefers_dark())
}

/// 读取注册表里的系统应用主题设置。
fn registry_prefers_dark() -> bool {
    use windows::core::w;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD,
    };

    unsafe {
        let mut value: u32 = 1;
        let mut size = std::mem::size_of::<u32>() as u32;
        let path = windows::core::HSTRING::from(
            r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
        );
        let status = RegGetValueW(
            HKEY_CURRENT_USER,
            &path,
            w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut value as *mut u32).cast()),
            Some(&mut size),
        );
        let _ = RegCloseKey(HKEY_CURRENT_USER);
        if status.is_err() {
            return false;
        }
        value == 0
    }
}

/// 把主题应用到 egui。每帧调用一次，系统主题变化时自动跟随。
pub fn apply(ctx: &egui::Context) {
    // 设置里改了主题就立刻生效（原生标题栏也会跟着变）。
    let wanted = theme_preference();
    if ctx.options(|o| o.theme_preference) != wanted {
        ctx.set_theme(wanted);
    }

    let dark = is_dark(ctx);
    DARK.store(dark, Ordering::Relaxed);
    let pal = palette();

    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.panel_fill = pal.bg;
    visuals.window_fill = pal.bg;
    visuals.extreme_bg_color = pal.surface;
    visuals.faint_bg_color = pal.bg_soft;
    visuals.weak_text_color = Some(pal.text_dim);
    visuals.hyperlink_color = pal.accent;
    visuals.code_bg_color = pal.bg_soft;
    visuals.text_edit_bg_color = Some(pal.surface);
    visuals.selection.bg_fill = pal.accent_soft;
    visuals.selection.stroke = egui::Stroke::new(1.0, pal.accent);
    visuals.window_corner_radius = egui::CornerRadius::from(16u8);
    visuals.window_stroke = egui::Stroke::new(1.0, pal.border);
    visuals.window_shadow = egui::Shadow::NONE;

    let widget = |fill: egui::Color32, stroke: egui::Color32, fg: egui::Color32| egui::style::WidgetVisuals {
        bg_fill: fill,
        weak_bg_fill: fill,
        bg_stroke: egui::Stroke::new(1.0, stroke),
        corner_radius: egui::CornerRadius::from(8u8),
        fg_stroke: egui::Stroke::new(1.0, fg),
        expansion: 0.0,
    };

    visuals.widgets.noninteractive = widget(pal.bg_soft, pal.border, pal.text_dim);
    visuals.widgets.inactive = widget(pal.bg_soft, pal.border, pal.text);
    visuals.widgets.hovered = widget(pal.accent_soft, pal.accent, pal.text);
    visuals.widgets.active = widget(pal.accent_soft, pal.accent, pal.text);
    visuals.widgets.open = widget(pal.bg_soft, pal.border, pal.text);

    ctx.set_visuals_of(
        if dark {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        },
        visuals,
    );

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        style.spacing.interact_size = egui::vec2(24.0, 24.0);
    });
}

/// 安装字体：拉丁字体 + 中文字体回退 + 等宽字体。
///
/// egui 自带字体不含中文字形，因此必须从系统字体目录加载中文字体。
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    if let Some(data) = load_font(&["segoeui.ttf", "tahoma.ttf", "arial.ttf"]) {
        fonts.font_data.insert("latin".to_owned(), std::sync::Arc::new(data));
        insert_family(&mut fonts, egui::FontFamily::Proportional, "latin", 0);
    }

    if let Some(data) = load_font(&["Deng.ttf", "simhei.ttf", "msyh.ttc", "simsun.ttc"]) {
        fonts.font_data.insert("cjk".to_owned(), std::sync::Arc::new(data));
        insert_family(&mut fonts, egui::FontFamily::Proportional, "cjk", 1);
        insert_family(&mut fonts, egui::FontFamily::Monospace, "cjk", 1);
    }

    if let Some(data) = load_font(&["consola.ttf", "cour.ttf"]) {
        fonts.font_data.insert("mono".to_owned(), std::sync::Arc::new(data));
        insert_family(&mut fonts, egui::FontFamily::Monospace, "mono", 0);
    }

    ctx.set_fonts(fonts);
}

/// 把字体插入到字体族列表的指定位置。
fn insert_family(
    fonts: &mut egui::FontDefinitions,
    family: egui::FontFamily,
    name: &str,
    index: usize,
) {
    let list = fonts.families.entry(family).or_default();
    if list.iter().any(|n| n == name) {
        return;
    }
    let index = index.min(list.len());
    list.insert(index, name.to_owned());
}

/// 从系统字体目录加载第一个可用的字体文件。
fn load_font(candidates: &[&str]) -> Option<egui::FontData> {
    let dir = std::env::var_os("WINDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Windows"));

    for name in candidates {
        let path = dir.join("Fonts").join(name);
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        // TTC 集合无法被解析，这里提前校验，避免装入坏字体。
        if ab_glyph::FontVec::try_from_vec(bytes.clone()).is_ok() {
            return Some(egui::FontData::from_owned(bytes));
        }
    }
    None
}
