//! 编辑面板：常用动作列表 + 新建 / 编辑。

use eframe::egui;

use crate::app::{self, Draft};
use crate::config::Action;
use crate::theme;
use crate::ui::{close_button, divider, field, flat_button, switch, ButtonKind};

/// 按水平 / 垂直内缩得到新的矩形。
fn inset(rect: egui::Rect, horizontal: f32, vertical: f32) -> egui::Rect {
    egui::Rect::from_min_max(
        rect.min + egui::vec2(horizontal, vertical),
        rect.max - egui::vec2(horizontal, vertical),
    )
}

/// 卡片高度。
const CARD_HEIGHT: f32 = 88.0;
/// 弹窗高度。
const MODAL_HEIGHT: f32 = 430.0;

/// 绘制编辑面板。运行在独立的「编辑面板」视口里。
pub fn show(ui: &mut egui::Ui) {
    let pal = theme::palette();

    let full = ui.max_rect();
    let margin = app::WINDOW_MARGIN;
    let panel = egui::Rect::from_min_max(
        full.min + egui::vec2(margin, margin),
        full.max - egui::vec2(margin, margin),
    );

    let frame = egui::Frame::new()
        .fill(pal.bg)
        .stroke(egui::Stroke::new(1.0, pal.border))
        .corner_radius(egui::CornerRadius::from(18u8))
        .shadow(theme::panel_shadow());
    ui.painter().add(frame.paint(panel));

    let draft_active = app::shared().draft.is_some();
    let content = panel.shrink(20.0);

    ui.scope_builder(egui::UiBuilder::new().max_rect(content), |ui| {
        if draft_active {
            // 压暗背景，突出弹窗。
            ui.painter().rect(
                panel,
                egui::CornerRadius::from(18u8),
                egui::Color32::from_black_alpha(110),
                egui::Stroke::NONE,
                egui::StrokeKind::Inside,
            );
            show_modal(ui, panel);
        } else {
            show_content(ui, panel);
        }
    });
}

/// 面板主体。
fn show_content(ui: &mut egui::Ui, panel: egui::Rect) {
    let pal = theme::palette();
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
    let width = ui.available_width();

    let (count, elevate) = {
        let sh = app::shared();
        (sh.config.actions.len(), sh.config.elevate)
    };

    // 标题栏
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(10.0, 0.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
            ui.label(
                egui::RichText::new("常用动作管理")
                    .size(17.0)
                    .strong()
                    .color(pal.text),
            );
            ui.label(
                egui::RichText::new(format!("共 {count} 个动作"))
                    .size(11.5)
                    .color(pal.text_dim),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if close_button(ui).clicked() {
                app::close_editor();
            }
        });
    });

    ui.add_space(14.0);
    divider(ui, width);
    ui.add_space(14.0);

    // 动作列表
    let footer_height = 46.0;
    let list_height = (ui.available_height() - footer_height - 14.0).max(120.0);
    let actions = app::shared().config.actions.clone();

    egui::ScrollArea::vertical()
        .max_height(list_height)
        .auto_shrink(false)
        .show(ui, |ui| {
            if actions.is_empty() {
                ui.add_space(28.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("还没有常用动作")
                            .size(13.5)
                            .color(pal.text_dim),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("点击下方「新建动作」添加第一个")
                            .size(12.0)
                            .color(pal.text_dim),
                    );
                });
            } else {
                for action in &actions {
                    card(ui, action);
                    ui.add_space(10.0);
                }
            }
        });

    ui.add_space(2.0);
    divider(ui, width);
    ui.add_space(14.0);

    // 底部
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(10.0, 0.0);
        if flat_button(ui, "新建动作", egui::vec2(96.0, 36.0), ButtonKind::Primary).clicked() {
            app::shared().draft = Some(Draft {
                id: None,
                name: String::new(),
                description: String::new(),
                command: String::new(),
            });
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut value = elevate;
            let response = switch(ui, &mut value);
            if response.clicked() && value != elevate {
                let config = {
                    let mut sh = app::shared();
                    sh.config.elevate = value;
                    sh.config.clone()
                };
                if let Err(err) = crate::config::save(&config) {
                    app::toast(err, true);
                }
            }
            ui.label(
                egui::RichText::new("以管理员身份运行")
                    .size(12.0)
                    .color(pal.text_dim),
            );
        });
    });

    // 提示消息
    let toast = app::shared().toast.clone();
    if let Some(toast) = toast {
        let (bg, fg) = if toast.error {
            (pal.danger_soft, pal.danger)
        } else {
            (pal.accent_soft, pal.accent)
        };
        let rect = egui::Rect::from_center_size(
            egui::pos2(panel.center().x, panel.bottom() - 26.0),
            egui::vec2(300.0, 32.0),
        );
        ui.painter().rect(
            rect,
            egui::CornerRadius::from(10u8),
            bg,
            egui::Stroke::NONE,
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            &toast.text,
            egui::FontId::proportional(12.0),
            fg,
        );
    }
}

/// 单个动作卡片。
fn card(ui: &mut egui::Ui, action: &Action) {
    let pal = theme::palette();
    let width = ui.available_width();

    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(width, CARD_HEIGHT),
        egui::Sense::hover(),
    );
    ui.painter().rect(
        rect,
        egui::CornerRadius::from(12u8),
        pal.bg_soft,
        egui::Stroke::NONE,
        egui::StrokeKind::Inside,
    );

    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inset(rect, 14.0, 11.0)));
    child.spacing_mut().item_spacing = egui::vec2(0.0, 5.0);

    child.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
        ui.label(
            egui::RichText::new(&action.name)
                .size(14.5)
                .strong()
                .color(pal.text),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let armed = app::shared().delete_armed.as_deref() == Some(action.id.as_str());
            let (label, kind) = if armed {
                ("确认删除", ButtonKind::Danger)
            } else {
                ("删除", ButtonKind::Ghost)
            };
            if flat_button(ui, label, egui::vec2(68.0, 26.0), kind).clicked() {
                if armed {
                    app::delete_action(&action.id);
                } else {
                    app::shared().delete_armed = Some(action.id.clone());
                }
            }
            if flat_button(ui, "编辑", egui::vec2(56.0, 26.0), ButtonKind::Ghost).clicked() {
                app::shared().draft = Some(Draft {
                    id: Some(action.id.clone()),
                    name: action.name.clone(),
                    description: action.description.clone(),
                    command: action.command.clone(),
                });
            }
        });
    });

    child.add(
        egui::Label::new(
            egui::RichText::new(&action.description)
                .size(12.0)
                .color(pal.text_dim),
        )
        .truncate(),
    );

    child.add(
        egui::Label::new(
            egui::RichText::new(&action.command)
                .size(11.5)
                .monospace()
                .color(pal.text_dim),
        )
        .truncate(),
    );
}

/// 新建 / 编辑动作的弹窗。
fn show_modal(ui: &mut egui::Ui, panel: egui::Rect) {
    let pal = theme::palette();

    let rect = egui::Rect::from_center_size(
        panel.center(),
        egui::vec2(480.0, MODAL_HEIGHT),
    );
    let frame = egui::Frame::new()
        .fill(pal.bg)
        .stroke(egui::Stroke::new(1.0, pal.border))
        .corner_radius(egui::CornerRadius::from(16u8))
        .shadow(theme::panel_shadow());
    ui.painter().add(frame.paint(rect));

    let Some(mut draft) = app::shared().draft.take() else {
        return;
    };
    let is_new = draft.id.is_none();

    let mut cancel = false;
    let mut save = false;

    let inner = rect.shrink(20.0);
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(if is_new { "新建动作" } else { "编辑动作" })
                    .size(16.0)
                    .strong()
                    .color(pal.text),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if close_button(ui).clicked() {
                    cancel = true;
                }
            });
        });

        ui.add_space(12.0);
        divider(ui, inner.width());
        ui.add_space(12.0);

        let fields_height = MODAL_HEIGHT - 40.0 - 30.0 - 12.0 - 12.0 - 34.0 - 14.0;
        egui::ScrollArea::vertical()
            .max_height(fields_height)
            .auto_shrink(false)
            .show(ui, |ui| {
                field(ui, "名称", &mut draft.name, false);
                ui.add_space(12.0);
                field(ui, "说明", &mut draft.description, false);
                ui.add_space(12.0);
                field(ui, "指令", &mut draft.command, true);
            });

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if flat_button(ui, "保存", egui::vec2(84.0, 34.0), ButtonKind::Primary).clicked() {
                    save = true;
                }
                if flat_button(ui, "取消", egui::vec2(84.0, 34.0), ButtonKind::Ghost).clicked() {
                    cancel = true;
                }
            });
        });
    });

    if cancel {
        app::shared().draft = None;
    } else if save {
        match app::save_action(&draft) {
            Ok(()) => {
                app::shared().draft = None;
                app::toast("已保存".to_owned(), false);
            }
            Err(err) => {
                app::shared().draft = Some(draft);
                app::toast(err, true);
            }
        }
    } else {
        app::shared().draft = Some(draft);
    }
}
