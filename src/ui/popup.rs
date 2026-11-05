//! 托盘弹出菜单：常用动作列表 + 编辑 / 退出。

use std::time::{Duration, Instant};

use eframe::egui;

use crate::app;
use crate::theme;
use crate::ui::{divider, flat_button, ButtonKind};

/// 绘制弹出菜单。运行在独立的「弹出菜单」视口里。
pub fn show(ui: &mut egui::Ui) {
    let pal = theme::palette();

    let full = ui.max_rect();
    let margin = app::WINDOW_MARGIN;
    let panel = egui::Rect::from_min_max(
        full.min + egui::vec2(margin, margin),
        full.max - egui::vec2(margin, margin),
    );

    // 面板：圆角 + 柔和阴影。
    let frame = egui::Frame::new()
        .fill(pal.bg)
        .stroke(egui::Stroke::new(1.0, pal.border))
        .corner_radius(egui::CornerRadius::from(16u8))
        .shadow(theme::panel_shadow());
    ui.painter().add(frame.paint(panel));

    let content = panel.shrink(app::PANEL_PADDING);
    ui.scope_builder(egui::UiBuilder::new().max_rect(content), |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        let width = ui.available_width();

        let (actions, status) = {
            let sh = app::shared();
            (sh.config.actions.clone(), sh.popup.as_ref().and_then(|p| p.status.clone()))
        };

        // 头部
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
            ui.label(egui::RichText::new("常用动作").size(12.5).color(pal.text_dim));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("{} 项", actions.len()))
                        .size(11.5)
                        .color(pal.text_dim),
                );
            });
        });
        ui.add_space(9.0);
        divider(ui, width);
        ui.add_space(9.0);

        // 动作列表
        if actions.is_empty() {
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new("还没有常用动作，点击下方「编辑」添加")
                    .size(12.0)
                    .color(pal.text_dim),
            );
            ui.add_space(10.0);
        } else {
            for action in &actions {
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(width, app::POPUP_ITEM_HEIGHT),
                    egui::Sense::hover(),
                );
                let response = ui.interact(rect, ui.make_persistent_id(&action.id), egui::Sense::click());

                if response.hovered() {
                    ui.painter().rect(
                        rect,
                        egui::CornerRadius::from(10u8),
                        pal.accent_soft,
                        egui::Stroke::NONE,
                        egui::StrokeKind::Inside,
                    );
                }

                let mut item = ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_max(
                    rect.min + egui::vec2(10.0, 6.0),
                    rect.max - egui::vec2(10.0, 6.0),
                )));
                item.spacing_mut().item_spacing = egui::vec2(0.0, 2.0);
                item.add(
                    egui::Label::new(
                        egui::RichText::new(&action.name)
                            .size(14.0)
                            .strong()
                            .color(pal.text),
                    )
                    .truncate(),
                );
                item.add(
                    egui::Label::new(
                        egui::RichText::new(&action.description)
                            .size(11.5)
                            .color(pal.text_dim),
                    )
                    .truncate(),
                );

                if response.clicked() {
                    app::run_action(&action.id);
                }
            }
        }

        ui.add_space(9.0);
        divider(ui, width);
        ui.add_space(9.0);

        // 底部：编辑 / 退出（或执行状态提示）
        if let Some((text, _)) = &status {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 34.0), egui::Sense::hover());
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(12.5),
                pal.accent,
            );
        } else {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
                if flat_button(ui, "编辑", egui::vec2(88.0, 34.0), ButtonKind::Ghost).clicked() {
                    app::close_popup();
                    app::open_editor(ui.ctx());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if flat_button(ui, "退出", egui::vec2(88.0, 34.0), ButtonKind::Danger).clicked() {
                        app::quit(ui.ctx());
                    }
                });
            });
        }
    });

    // 失焦自动关闭（避免刚弹出就被误判为失焦）。
    let focused = ui.ctx().input(|i| i.viewport().focused.unwrap_or(false));
    let now = Instant::now();
    let mut should_close = false;
    {
        let mut sh = app::shared();
        if let Some(popup) = sh.popup.as_mut() {
            if focused {
                popup.focused_once = true;
            } else if popup.focused_once
                && now.saturating_duration_since(popup.opened_at) > Duration::from_millis(350)
            {
                should_close = true;
            }
        }
    }
    if should_close {
        app::close_popup();
    }
}
