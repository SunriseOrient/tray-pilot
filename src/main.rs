//! TrayPilot —— Windows 托盘常用动作工具。
//!
//! 左键或右键点击托盘图标即可弹出常用动作列表；选择「编辑」会从屏幕正中
//! 打开管理面板。指令通过 PowerShell 以管理员身份执行，控制台隐藏，
//! 输出写入 `%APPDATA%\TrayPilot\logs\run.log`。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod exec;
mod icon;
mod theme;
mod ui;
mod win;

use std::sync::{Arc, OnceLock};

use eframe::egui;
use tray_icon::{TrayIconBuilder, TrayIconEvent};

/// 生成托盘图标。`dark_taskbar` 为真时使用白色图形。
pub fn build_tray_icon(dark_taskbar: bool) -> tray_icon::Icon {
    tray_icon::Icon::from_rgba(
        icon::tray_icon_rgba(dark_taskbar),
        icon::SIZE,
        icon::SIZE,
    )
    .expect("托盘图标数据无效")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 载入配置
    {
        let mut shared = app::shared();
        shared.config = config::load();
    }

    // 托盘图标：按当前系统主题选择深色 / 浅色任务栏对应的那一套。
    let tray_icon = build_tray_icon(theme::system_is_dark());
    let tray = TrayIconBuilder::new()
        .with_icon(tray_icon)
        .with_tooltip("TrayPilot · 常用动作")
        .build()?;

    // 托盘事件 → 应用命令
    let ctx_slot: Arc<OnceLock<egui::Context>> = Arc::new(OnceLock::new());
    let handler_ctx = Arc::clone(&ctx_slot);
    TrayIconEvent::set_event_handler(Some(move |event| {
        if let TrayIconEvent::Click {
            button,
            position,
            rect,
            ..
        } = event
        {
            if matches!(
                button,
                tray_icon::MouseButton::Left | tray_icon::MouseButton::Right
            ) {
                let left = rect.position.x;
                let top = rect.position.y;
                let _ = position;
                app::push_tray_command(app::TrayCommand::ShowPopup {
                    tray: (
                        left,
                        top,
                        left + f64::from(rect.size.width),
                        top + f64::from(rect.size.height),
                    ),
                });

                // 唤醒被隐藏的主窗口，让它立刻处理这条命令。
                if let Some(ctx) = handler_ctx.get() {
                    ctx.request_repaint();
                }
            }
        }
    }));

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("TrayPilot")
            .with_inner_size(egui::vec2(1.0, 1.0))
            .with_visible(false)
            .with_decorations(false)
            .with_resizable(false)
            .with_taskbar(false),
        ..Default::default()
    };

    eframe::run_native(
        "TrayPilot",
        native_options,
        Box::new(move |cc| {
            let _ = ctx_slot.set(cc.egui_ctx.clone());
            theme::install_fonts(&cc.egui_ctx);
            cc.egui_ctx.set_theme(theme::theme_preference());
            Ok(Box::new(app::TrayPilotApp::new(tray, theme::system_is_dark())))
        }),
    )?;
    Ok(())
}
