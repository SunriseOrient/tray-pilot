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

    // 原生窗口：客户区直接铺主题底色，标题栏与圆角由系统绘制。
    let panel = ui.max_rect();
    ui.painter().rect(
        panel,
        egui::CornerRadius::ZERO,
        pal.bg,
        egui::Stroke::NONE,
        egui::StrokeKind::Inside,
    );

    // 系统标题栏的关闭按钮
    if ui.ctx().input(|i| i.viewport().close_requested()) {
        app::close_editor();
        return;
    }

    // 尺寸兜底：见 enforce_fixed_size 的说明。
    enforce_fixed_size(ui);

    let (draft_active, confirm_delete) = {
        let sh = app::shared();
        (sh.draft.is_some(), sh.confirm_delete.clone())
    };
    let content = panel.shrink(app::PANEL_PADDING);

    ui.scope_builder(egui::UiBuilder::new().max_rect(content), |ui| {
        if draft_active {
            // 压暗背景，突出弹窗。
            dim_background(ui, panel);
            show_modal(ui, panel);
        } else {
            show_content(ui, panel);
            if confirm_delete.is_some() {
                dim_background(ui, panel);
                show_delete_confirm(ui, panel);
            }
        }
    });
}

/// 把窗口尺寸钉回预设值。
///
/// 窗口本身已经关掉了 `WS_SIZEBOX`，用户拖不动边框；但当窗口在两块缩放比例
/// 不同的显示器之间拖动时，Windows 会反复发送 `WM_DPICHANGED`（窗口横跨交界
/// 线时会来回翻转）。winit 每次都用「记住的旧缩放比例」去换算新尺寸，一旦这个
/// 记录滞后，尺寸就会被反复乘以缩放比，表现为宽度越拖越长或越拖越窄。
///
/// 这里每帧检查一次实际尺寸：只要不是最大化状态且尺寸偏离预设值，就重新请求
/// 一次预设尺寸。`InnerSize` 用的是逻辑点，所以无论当前在哪块屏幕都会落到
/// 正确的物理尺寸上。加 250ms 节流，避免和系统的 DPI 调整互相拉扯。
fn enforce_fixed_size(ui: &mut egui::Ui) {
    let ctx = ui.ctx();
    let Some(desired) = app::shared().editor.as_ref().map(|e| e.size) else {
        return;
    };

    let (size, maximized) = ctx.input(|i| {
        let vp = i.viewport();
        (vp.inner_rect.map(|r| r.size()), vp.maximized.unwrap_or(false))
    });
    if maximized {
        return;
    }
    let Some(size) = size else {
        return;
    };

    const TOLERANCE: f32 = 2.5;
    let off_x = (size.x - desired.x).abs();
    let off_y = (size.y - desired.y).abs();
    if off_x <= TOLERANCE && off_y <= TOLERANCE {
        return;
    }

    // 偏差很大（跨屏瞬间被多乘了一次缩放）时立刻纠正，避免肉眼看到闪一下。
    let badly_off = off_x > desired.x * 0.1 || off_y > desired.y * 0.1;
    let now = ctx.input(|i| i.time);
    let id = egui::Id::unique("tray-pilot::editor-size-fix");
    let last = ctx.data(|d| d.get_temp::<f64>(id)).unwrap_or(f64::NEG_INFINITY);
    if !badly_off && now - last < 0.1 {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(id, now));
    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(desired));
}

/// 把整个面板压暗，突出上层的模态框。
fn dim_background(ui: &egui::Ui, panel: egui::Rect) {
    ui.painter().rect(
        panel,
        egui::CornerRadius::ZERO,
        egui::Color32::from_black_alpha(110),
        egui::Stroke::NONE,
        egui::StrokeKind::Inside,
    );
}

/// 删除确认框。
fn show_delete_confirm(ui: &mut egui::Ui, panel: egui::Rect) {
    let Some(id) = app::shared().confirm_delete.clone() else {
        return;
    };
    let name = {
        let sh = app::shared();
        sh.config
            .actions
            .iter()
            .find(|a| a.id == id)
            .map(|a| a.name.clone())
            .unwrap_or_default()
    };
    let pal = theme::palette();

    let rect = egui::Rect::from_center_size(panel.center(), egui::vec2(380.0, 152.0));
    let frame = egui::Frame::new()
        .fill(pal.bg)
        .stroke(egui::Stroke::new(1.0, pal.border_strong))
        .corner_radius(egui::CornerRadius::from(12u8));
    ui.painter().add(frame.paint(rect));

    let inner = inset(rect, 22.0, 20.0);
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

        ui.label(
            egui::RichText::new("删除动作")
                .size(15.5)
                .strong()
                .color(pal.text),
        );
        ui.add_space(10.0);
        ui.add(
            egui::Label::new(
                egui::RichText::new(format!("确定要删除「{name}」吗？"))
                    .size(13.0)
                    .color(pal.text),
            )
            .wrap(),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("该操作无法撤销。")
                .size(12.0)
                .color(pal.text_dim),
        );

        // 按钮固定在对话框底部，避免宽度自适应时上下留白不均。
        let row = egui::Rect::from_min_max(
            egui::pos2(inner.left(), inner.bottom() - 32.0),
            inner.max,
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(row), |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if flat_button(ui, "删除", egui::vec2(84.0, 32.0), ButtonKind::DangerSolid)
                    .clicked()
                {
                    app::delete_action(&id);
                    app::toast("已删除".to_owned(), false);
                }
                if flat_button(ui, "取消", egui::vec2(84.0, 32.0), ButtonKind::Ghost).clicked() {
                    app::shared().confirm_delete = None;
                }
            });
        });
    });

    // Esc 取消
    if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
        app::shared().confirm_delete = None;
    }
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

    ui.label(
        egui::RichText::new(format!("共 {count} 个动作"))
            .size(12.0)
            .color(pal.text_dim),
    );

    ui.add_space(10.0);
    divider(ui, width);
    ui.add_space(12.0);

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
            if flat_button(ui, "删除", egui::vec2(68.0, 26.0), ButtonKind::Ghost).clicked() {
                app::shared().confirm_delete = Some(action.id.clone());
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
        .stroke(egui::Stroke::new(1.0, pal.border_strong))
        .corner_radius(egui::CornerRadius::from(12u8));
    ui.painter().add(frame.paint(rect));

    let Some(mut draft) = app::shared().draft.take() else {
        return;
    };
    let is_new = draft.id.is_none();

    let mut cancel = false;
    let mut save = false;

    let inner = rect.shrink(app::PANEL_PADDING);
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
