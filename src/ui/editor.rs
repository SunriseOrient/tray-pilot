//! 编辑面板。
//!
//! 极简布局：一行小标题 + 右上角两个操作，下面是**无卡片的平铺列表**，
//! 行与行之间只有一条细线。行内操作平时隐藏，鼠标移到该行才淡入。

use eframe::egui;

use crate::app::{self, Draft};
use crate::config::{Action, ThemeMode};
use crate::theme;
use crate::ui::{
    close_button, fill_rounded, flat_button, hairline, modal_blocker, segmented, switch,
    ButtonKind,
};

/// 按水平 / 垂直内缩得到新的矩形。
fn inset(rect: egui::Rect, horizontal: f32, vertical: f32) -> egui::Rect {
    egui::Rect::from_min_max(
        rect.min + egui::vec2(horizontal, vertical),
        rect.max - egui::vec2(horizontal, vertical),
    )
}

/// 列表行高。
const ROW_HEIGHT: f32 = 64.0;
/// 设置行高。
const SETTING_ROW_HEIGHT: f32 = 48.0;
/// 新建 / 编辑弹窗高度。
const MODAL_HEIGHT: f32 = 388.0;
/// 新建 / 编辑弹窗尺寸。
const MODAL_SIZE: egui::Vec2 = egui::vec2(440.0, MODAL_HEIGHT);
/// 设置弹窗尺寸。
const SETTINGS_SIZE: egui::Vec2 = egui::vec2(400.0, 224.0);
/// 删除确认尺寸。
const CONFIRM_SIZE: egui::Vec2 = egui::vec2(320.0, 156.0);

/// 居中弹窗的矩形。
fn sheet_rect(panel: egui::Rect, size: egui::Vec2) -> egui::Rect {
    egui::Rect::from_center_size(panel.center(), size)
}

/// 按比例调整颜色的透明度（用于淡入淡出）。
fn fade(color: egui::Color32, alpha: f32) -> egui::Color32 {
    let a = (f32::from(color.a()) * alpha).round().clamp(0.0, 255.0) as u8;
    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), a)
}

/// 绘制编辑面板。
pub fn show(ui: &mut egui::Ui) {
    let pal = theme::palette();

    let panel = ui.max_rect();
    fill_rounded(ui.painter(), panel, 0, pal.bg, egui::Stroke::NONE);

    if ui.ctx().input(|i| i.viewport().close_requested()) {
        app::close_editor();
        return;
    }

    enforce_fixed_size(ui);

    let (draft_active, settings_open, confirm_delete) = {
        let sh = app::shared();
        (sh.draft.is_some(), sh.settings_open, sh.confirm_delete.clone())
    };
    let content = panel.shrink(app::PANEL_PADDING);

    ui.scope_builder(egui::UiBuilder::new().max_rect(content), |ui| {
        // 三种弹窗共用同一套结构：内容照常画在后面（被遮罩压暗）、
        // 遮罩拦截输入、弹窗盖在最上面。这样无论打开哪个弹窗，
        // 背景看起来都是一致的。
        let modal_open = draft_active || settings_open || confirm_delete.is_some();
        show_content(ui, panel, !modal_open);

        if draft_active {
            dim_background(ui, panel);
            modal_blocker(ui, panel, "tray-pilot::modal-block::draft");
            show_modal(ui, sheet_rect(panel, MODAL_SIZE));
        } else if settings_open {
            dim_background(ui, panel);
            modal_blocker(ui, panel, "tray-pilot::modal-block::settings");
            show_settings(ui, sheet_rect(panel, SETTINGS_SIZE));
        } else if confirm_delete.is_some() {
            dim_background(ui, panel);
            modal_blocker(ui, panel, "tray-pilot::modal-block::confirm");
            show_delete_confirm(ui, sheet_rect(panel, CONFIRM_SIZE));
        }
    });
}

/// 把窗口尺寸钉回预设值（跨屏 DPI 抖动兜底，见 README）。
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

fn dim_background(ui: &egui::Ui, panel: egui::Rect) {
    fill_rounded(
        ui.painter(),
        panel,
        0,
        egui::Color32::from_black_alpha(72),
        egui::Stroke::NONE,
    );
}

/// 弹窗外框：浮起层底色 + 圆角 + 细描边，投影很轻。
fn paint_sheet(ui: &egui::Ui, rect: egui::Rect, radius: u8) {
    let pal = theme::palette();
    let frame = egui::Frame::new()
        .fill(pal.surface)
        .stroke(egui::Stroke::new(1.0, pal.border_strong))
        .corner_radius(egui::CornerRadius::from(radius))
        .shadow(egui::Shadow {
            offset: [0, 16],
            blur: 40,
            spread: 0,
            color: egui::Color32::from_black_alpha(56),
        });
    ui.painter().add(frame.paint(rect));
}

/// 面板主体。
fn show_content(ui: &mut egui::Ui, panel: egui::Rect, interactive: bool) {
    let pal = theme::palette();
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
    let width = ui.available_width();
    let count = app::shared().config.actions.len();

    // ── 顶栏：左侧小标题，右侧操作 ──────────────────────────────
    let (header, _) = ui.allocate_exact_size(egui::vec2(width, 30.0), egui::Sense::hover());
    ui.painter().text(
        egui::pos2(header.left(), header.center().y),
        egui::Align2::LEFT_CENTER,
        format!("{count} 个动作"),
        egui::FontId::proportional(12.0),
        pal.text_dim,
    );

    let mut actions_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(header)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    actions_ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
    // right_to_left：先加的在最右边。主操作最右。
    if flat_button(&mut actions_ui, "新建", egui::vec2(62.0, 26.0), ButtonKind::Primary).clicked() {
        app::shared().draft = Some(Draft {
            id: None,
            name: String::new(),
            description: String::new(),
            command: String::new(),
        });
    }
    if flat_button(&mut actions_ui, "设置", egui::vec2(52.0, 26.0), ButtonKind::Ghost).clicked() {
        app::shared().settings_open = true;
    }

    ui.add_space(10.0);
    hairline(ui, width, 0.0);
    ui.add_space(2.0);

    // ── 列表：平铺，行间细线，无卡片 ────────────────────────────
    let actions = app::shared().config.actions.clone();
    let list_height = ui.available_height().max(100.0);

    egui::ScrollArea::vertical()
        .max_height(list_height)
        .auto_shrink(false)
        .show(ui, |ui| {
            let width = ui.available_width();
            if actions.is_empty() {
                ui.add_space(48.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("还没有动作")
                            .size(13.0)
                            .color(pal.text_dim),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("点右上角「新建」开始")
                            .size(12.0)
                            .color(pal.text_faint),
                    );
                });
                return;
            }

            for (i, action) in actions.iter().enumerate() {
                action_row(ui, width, action, interactive);
                if i + 1 < actions.len() {
                    hairline(ui, width, 0.0);
                }
            }
        });

    // 提示：底部居中的小字，不打断视线
    let toast = app::shared().toast.clone();
    if let Some(toast) = toast {
        let color = if toast.error { pal.danger } else { pal.text_dim };
        ui.painter().text(
            egui::pos2(panel.center().x, panel.bottom() - 22.0),
            egui::Align2::CENTER_CENTER,
            &toast.text,
            egui::FontId::proportional(12.0),
            color,
        );
    }
}

/// 一行动作：名称 + 说明；悬停时右侧淡入「编辑 / 删除」，并显示命令提示。
fn action_row(ui: &mut egui::Ui, width: f32, action: &Action, interactive: bool) {
    let pal = theme::palette();

    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, ROW_HEIGHT), egui::Sense::hover());

    // 悬停只看「指针是否在这个矩形里」，不看 egui 的命中测试结果。
    //
    // 用 `Response::hovered()` 会闪：鼠标移到行内的「编辑/删除」上时，按钮成为
    // 命中目标，行本身就「不再悬停」→ 按钮开始淡出 → 淡出后指针又落回行上 →
    // 按钮再淡入，如此反复。几何判断不受层级影响，所以状态是稳定的。
    let hovered = interactive
        && ui
            .ctx()
            .input(|i| i.pointer.interact_pos())
            .is_some_and(|p| rect.contains(p));

    // 悬停：极淡的底色，不用卡片描边。
    if hovered {
        fill_rounded(
            ui.painter(),
            rect.expand2(egui::vec2(8.0, 0.0)),
            8,
            pal.bg_soft,
            egui::Stroke::NONE,
        );
    }

    let mut text = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inset(rect, 2.0, 0.0))
            .layout(egui::Layout::top_down(egui::Align::LEFT)),
    );
    text.spacing_mut().item_spacing = egui::vec2(0.0, 2.0);
    text.add_space(13.0);
    text.add(
        egui::Label::new(egui::RichText::new(&action.name).size(13.5).color(pal.text)).truncate(),
    );
    if !action.description.is_empty() {
        text.add(
            egui::Label::new(
                egui::RichText::new(&action.description)
                    .size(12.0)
                    .color(pal.text_dim),
            )
            .truncate(),
        );
    }

    // 行内操作：平时透明，悬停淡入。
    let t = ui.ctx().animate_bool_with_time(
        egui::Id::unique(("row-actions", &action.id)),
        hovered,
        0.12,
    );
    if t > 0.01 {
        let mut buttons = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(inset(rect, 2.0, 0.0))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        buttons.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);

        // 用自绘按钮，但整体乘上淡入系数。
        let del = row_action(&mut buttons, "删除", ButtonKind::DangerPlain, t);
        if del {
            app::shared().confirm_delete = Some(action.id.clone());
        }
        let edit = row_action(&mut buttons, "编辑", ButtonKind::Plain, t);
        if edit {
            app::shared().draft = Some(Draft {
                id: Some(action.id.clone()),
                name: action.name.clone(),
                description: action.description.clone(),
                command: action.command.clone(),
            });
        }
    }

    // 命令放在悬停提示里，列表保持清爽。
    // 挂在左侧文字区域上而不是整行：鼠标移到右侧按钮时提示不会跟着闪。
    if !action.command.is_empty() && interactive {
        text.response()
            .on_hover_text(egui::RichText::new(&action.command).monospace().size(12.0));
    }
}

/// 行内文字按钮，`alpha` 控制淡入程度。
fn row_action(ui: &mut egui::Ui, label: &str, kind: ButtonKind, alpha: f32) -> bool {
    let pal = theme::palette();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(46.0, 26.0), egui::Sense::click());
    let hot = response.hovered();

    if hot {
        let bg = match kind {
            ButtonKind::DangerPlain => pal.danger_soft,
            _ => pal.accent_soft,
        };
        fill_rounded(ui.painter(), rect, 6, fade(bg, alpha), egui::Stroke::NONE);
    }
    let base = match kind {
        ButtonKind::DangerPlain => pal.danger,
        _ => pal.text_dim,
    };
    let color = fade(base, alpha);

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(12.0),
        color,
    );

    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.clicked()
}

/// 设置。
fn show_settings(ui: &mut egui::Ui, rect: egui::Rect) {
    let pal = theme::palette();
    paint_sheet(ui, rect, 12);

    let (elevate, autostart, mode) = {
        let sh = app::shared();
        (sh.config.elevate, sh.config.autostart, sh.config.theme)
    };

    let inner = inset(rect, 20.0, 18.0);
    let mut done = false;
    let mut new_elevate = elevate;
    let mut new_autostart = autostart;
    let mut new_mode = mode;

    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        let width = ui.available_width();

        // 标题栏：左边标题，右上角关闭按钮。
        let (head, _) = ui.allocate_exact_size(egui::vec2(width, 24.0), egui::Sense::hover());
        ui.painter().text(
            egui::pos2(head.left(), head.center().y),
            egui::Align2::LEFT_CENTER,
            "设置",
            egui::FontId::proportional(13.0),
            pal.text_dim,
        );
        let mut lead = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(head)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        if close_button(&mut lead).clicked() {
            done = true;
        }
        ui.add_space(12.0);
        hairline(ui, width, 0.0);

        let rows_top = ui.cursor().top();
        let row = |i: usize| {
            egui::Rect::from_min_size(
                egui::pos2(inner.left(), rows_top + i as f32 * SETTING_ROW_HEIGHT),
                egui::vec2(inner.width(), SETTING_ROW_HEIGHT),
            )
        };

        setting_switch(ui, row(0), "以管理员身份运行", "", &mut new_elevate);
        hairline(ui, width, 0.0);
        setting_switch(ui, row(1), "开机启动", "", &mut new_autostart);
        hairline(ui, width, 0.0);

        // 主题行：标题 + 右侧分段选择器。
        let theme_row = row(2);
        ui.painter().text(
            egui::pos2(theme_row.left(), theme_row.center().y),
            egui::Align2::LEFT_CENTER,
            "主题",
            egui::FontId::proportional(13.0),
            pal.text,
        );
        let mut seg_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_max(
                    egui::pos2(theme_row.left(), theme_row.top()),
                    theme_row.max,
                ))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        let labels: Vec<&str> = ThemeMode::ALL.iter().map(|m| m.label()).collect();
        let selected = ThemeMode::ALL.iter().position(|m| *m == mode).unwrap_or(0);
        if let Some(i) = segmented(&mut seg_ui, &labels, selected) {
            new_mode = ThemeMode::ALL[i];
        }
    });

    if new_elevate != elevate {
        app::update_config(|c| c.elevate = new_elevate);
    }
    if new_autostart != autostart {
        match crate::autostart::set_enabled(new_autostart) {
            Ok(()) => app::update_config(|c| c.autostart = new_autostart),
            Err(err) => app::toast(err, true),
        }
    }
    if new_mode != mode {
        app::update_config(|c| c.theme = new_mode);
    }

    if done || ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
        app::shared().settings_open = false;
    }
}

/// 设置里的一行：左边标签，右边开关。
fn setting_switch(ui: &mut egui::Ui, row: egui::Rect, title: &str, hint: &str, value: &mut bool) {
    let pal = theme::palette();

    ui.painter().text(
        egui::pos2(row.left(), row.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(13.0),
        pal.text,
    );
    if !hint.is_empty() {
        ui.painter().text(
            egui::pos2(row.left() + 1.0, row.center().y + 14.0),
            egui::Align2::LEFT_CENTER,
            hint,
            egui::FontId::proportional(11.5),
            pal.text_dim,
        );
    }

    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(row)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    switch(&mut child, value);
}

/// 删除确认。
fn show_delete_confirm(ui: &mut egui::Ui, rect: egui::Rect) {
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

    paint_sheet(ui, rect, 12);

    let inner = inset(rect, 22.0, 20.0);
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new("删除动作").size(14.0).color(pal.text));
            ui.add_space(6.0);
            ui.add(
                egui::Label::new(
                    egui::RichText::new(format!("「{name}」将被永久删除"))
                        .size(12.5)
                        .color(pal.text_dim),
                )
                .wrap(),
            );
        });

        let gap = 8.0;
        let btn_w = (inner.width() - gap) / 2.0;
        let top = inner.bottom() - 32.0;
        let cancel_rect =
            egui::Rect::from_min_size(egui::pos2(inner.left(), top), egui::vec2(btn_w, 32.0));
        let delete_rect = egui::Rect::from_min_size(
            egui::pos2(inner.left() + btn_w + gap, top),
            egui::vec2(btn_w, 32.0),
        );
        let mut cancel_ui = ui.new_child(egui::UiBuilder::new().max_rect(cancel_rect));
        if flat_button(&mut cancel_ui, "取消", egui::vec2(btn_w, 32.0), ButtonKind::Ghost).clicked()
        {
            app::shared().confirm_delete = None;
        }
        let mut delete_ui = ui.new_child(egui::UiBuilder::new().max_rect(delete_rect));
        if flat_button(&mut delete_ui, "删除", egui::vec2(btn_w, 32.0), ButtonKind::DangerSolid)
            .clicked()
        {
            app::delete_action(&id);
            app::toast("已删除".to_owned(), false);
        }
    });

    if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
        app::shared().confirm_delete = None;
    }
}

/// 新建 / 编辑动作。
fn show_modal(ui: &mut egui::Ui, rect: egui::Rect) {
    let pal = theme::palette();
    paint_sheet(ui, rect, 12);

    let Some(mut draft) = app::shared().draft.take() else {
        return;
    };
    let is_new = draft.id.is_none();

    let mut cancel = false;
    let mut save = false;

    let inner = inset(rect, 22.0, 20.0);
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(if is_new { "新建动作" } else { "编辑动作" })
                    .size(13.0)
                    .color(pal.text_dim),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if flat_button(ui, "保存", egui::vec2(62.0, 24.0), ButtonKind::Primary).clicked() {
                    save = true;
                }
                if flat_button(ui, "取消", egui::vec2(52.0, 24.0), ButtonKind::Ghost).clicked() {
                    cancel = true;
                }
            });
        });
        ui.add_space(12.0);
        hairline(ui, inner.width(), 0.0);
        ui.add_space(14.0);

        let fields_height = inner.height() - 24.0 - 12.0 - 1.0 - 14.0 - 14.0;
        egui::ScrollArea::vertical()
            .max_height(fields_height)
            .auto_shrink(false)
            .show(ui, |ui| {
                crate::ui::field(ui, "名称", &mut draft.name, false);
                ui.add_space(14.0);
                crate::ui::field(ui, "说明", &mut draft.description, false);
                ui.add_space(14.0);
                crate::ui::field(ui, "指令", &mut draft.command, true);
            });
    });

    if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
        cancel = true;
    }

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
