//! 编辑面板：常用动作列表 + 新建 / 编辑 / 设置。
//!
//! 布局参照 macOS / iOS：大标题、右上角的操作按钮、分组圆角列表，
//! 弹窗是压暗背景之上的 sheet。

use eframe::egui;

use crate::app::{self, Draft};
use crate::config::{Action, ThemeMode};
use crate::theme;
use crate::ui::{
    divider, field, fill_rounded, flat_button, group_card, row_separator, segmented, switch,
    ButtonKind,
};

/// 按水平 / 垂直内缩得到新的矩形。
fn inset(rect: egui::Rect, horizontal: f32, vertical: f32) -> egui::Rect {
    egui::Rect::from_min_max(
        rect.min + egui::vec2(horizontal, vertical),
        rect.max - egui::vec2(horizontal, vertical),
    )
}

/// 列表里每一行的高度。
const ROW_HEIGHT: f32 = 78.0;
/// 设置里每一行的高度。
const SETTING_ROW_HEIGHT: f32 = 58.0;
/// 新建 / 编辑弹窗高度。
const MODAL_HEIGHT: f32 = 430.0;

/// 绘制编辑面板。运行在独立的「编辑面板」视口里。
pub fn show(ui: &mut egui::Ui) {
    let pal = theme::palette();

    // 原生窗口：客户区直接铺窗口底色，标题栏与圆角由系统绘制。
    let panel = ui.max_rect();
    fill_rounded(ui.painter(), panel, 0, pal.bg, egui::Stroke::NONE);

    // 系统标题栏的关闭按钮
    if ui.ctx().input(|i| i.viewport().close_requested()) {
        app::close_editor();
        return;
    }

    // 尺寸兜底：见 enforce_fixed_size 的说明。
    enforce_fixed_size(ui);

    let (draft_active, settings_open, confirm_delete) = {
        let sh = app::shared();
        (sh.draft.is_some(), sh.settings_open, sh.confirm_delete.clone())
    };
    let content = panel.shrink(app::PANEL_PADDING);

    ui.scope_builder(egui::UiBuilder::new().max_rect(content), |ui| {
        if draft_active {
            // 压暗背景，突出弹窗。
            dim_background(ui, panel);
            show_modal(ui, panel);
        } else {
            show_content(ui, panel);
            if settings_open {
                dim_background(ui, panel);
                show_settings(ui, panel);
            } else if confirm_delete.is_some() {
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
/// 正确的物理尺寸上。
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

/// 把整个面板压暗，突出上层的弹窗。
fn dim_background(ui: &egui::Ui, panel: egui::Rect) {
    fill_rounded(
        ui.painter(),
        panel,
        0,
        egui::Color32::from_black_alpha(96),
        egui::Stroke::NONE,
    );
}

/// 弹窗外框：窗口底色 + 圆角 + 细描边 + 柔和投影（sheet 的浮起感）。
fn paint_sheet(ui: &egui::Ui, rect: egui::Rect, radius: u8) {
    let pal = theme::palette();
    let frame = egui::Frame::new()
        .fill(pal.bg)
        .stroke(egui::Stroke::new(1.0, pal.border_strong))
        .corner_radius(egui::CornerRadius::from(radius))
        .shadow(egui::Shadow {
            offset: [0, 12],
            blur: 32,
            spread: 0,
            color: egui::Color32::from_black_alpha(72),
        });
    ui.painter().add(frame.paint(rect));
}

/// 面板主体：大标题 + 右上角操作区 + 动作列表。
fn show_content(ui: &mut egui::Ui, panel: egui::Rect) {
    let pal = theme::palette();
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
    let width = ui.available_width();

    let count = app::shared().config.actions.len();

    // 顶栏：左边大标题 + 数量，所有操作按钮集中在右上角。
    let (header, _) = ui.allocate_exact_size(egui::vec2(width, 54.0), egui::Sense::hover());
    ui.painter().text(
        egui::pos2(header.left(), header.top() + 16.0),
        egui::Align2::LEFT_CENTER,
        "常用动作",
        egui::FontId::proportional(25.0),
        pal.text,
    );
    ui.painter().text(
        egui::pos2(header.left() + 1.0, header.top() + 42.0),
        egui::Align2::LEFT_CENTER,
        format!("{count} 个动作"),
        egui::FontId::proportional(12.5),
        pal.text_dim,
    );

    let mut actions_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(header)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    actions_ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
    // right_to_left：先加的在最右边。主操作放最右。
    if flat_button(&mut actions_ui, "+ 新建动作", egui::vec2(100.0, 32.0), ButtonKind::Primary)
        .clicked()
    {
        app::shared().draft = Some(Draft {
            id: None,
            name: String::new(),
            description: String::new(),
            command: String::new(),
        });
    }
    if flat_button(&mut actions_ui, "设置", egui::vec2(64.0, 32.0), ButtonKind::Ghost).clicked() {
        app::shared().settings_open = true;
    }

    ui.add_space(18.0);

    // 动作列表：一张分组卡片，占满剩余空间（内容多时可滚动）。
    let actions = app::shared().config.actions.clone();
    let list_height = ui.available_height().max(120.0);

    egui::ScrollArea::vertical()
        .max_height(list_height)
        .auto_shrink(false)
        .show(ui, |ui| {
            let width = ui.available_width();
            if actions.is_empty() {
                let card = group_card(ui, width, 150.0);
                ui.painter().text(
                    egui::pos2(card.center().x, card.center().y - 10.0),
                    egui::Align2::CENTER_CENTER,
                    "还没有常用动作",
                    egui::FontId::proportional(14.0),
                    pal.text,
                );
                ui.painter().text(
                    egui::pos2(card.center().x, card.center().y + 14.0),
                    egui::Align2::CENTER_CENTER,
                    "点击右上角「+ 新建动作」添加第一个",
                    egui::FontId::proportional(12.5),
                    pal.text_dim,
                );
                return;
            }

            let card = group_card(ui, width, actions.len() as f32 * ROW_HEIGHT);
            for (i, action) in actions.iter().enumerate() {
                let row = egui::Rect::from_min_size(
                    egui::pos2(card.left(), card.top() + i as f32 * ROW_HEIGHT),
                    egui::vec2(card.width(), ROW_HEIGHT),
                );
                action_row(ui, row, action);
                if i + 1 < actions.len() {
                    row_separator(ui, row);
                }
            }
            ui.add_space(8.0);
        });

    // 提示消息（HUD 风格的胶囊）
    let toast = app::shared().toast.clone();
    if let Some(toast) = toast {
        let bg = if toast.error {
            pal.danger
        } else {
            egui::Color32::from_rgba_unmultiplied(40, 40, 42, 235)
        };
        let rect = egui::Rect::from_center_size(
            egui::pos2(panel.center().x, panel.bottom() - 30.0),
            egui::vec2(230.0, 36.0),
        );
        fill_rounded(ui.painter(), rect, 18, bg, egui::Stroke::NONE);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            &toast.text,
            egui::FontId::proportional(13.0),
            egui::Color32::WHITE,
        );
    }
}

/// 列表里的一行：左边是名称 / 说明 / 指令，右边是「编辑」「删除」文字按钮。
fn action_row(ui: &mut egui::Ui, row: egui::Rect, action: &Action) {
    let pal = theme::palette();

    let text_rect = egui::Rect::from_min_max(
        egui::pos2(row.left() + 16.0, row.top() + 13.0),
        egui::pos2(row.right() - 126.0, row.bottom() - 13.0),
    );
    let mut text = ui.new_child(egui::UiBuilder::new().max_rect(text_rect));
    text.spacing_mut().item_spacing = egui::vec2(0.0, 3.0);
    text.add(
        egui::Label::new(egui::RichText::new(&action.name).size(14.5).color(pal.text)).truncate(),
    );
    if !action.description.is_empty() {
        text.add(
            egui::Label::new(
                egui::RichText::new(&action.description)
                    .size(12.5)
                    .color(pal.text_dim),
            )
            .truncate(),
        );
    }
    text.add(
        egui::Label::new(
            egui::RichText::new(&action.command)
                .size(11.5)
                .monospace()
                .color(pal.text_faint),
        )
        .truncate(),
    );

    let mut buttons = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inset(row, 10.0, 0.0))
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    buttons.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
    if flat_button(&mut buttons, "删除", egui::vec2(52.0, 30.0), ButtonKind::DangerPlain).clicked()
    {
        app::shared().confirm_delete = Some(action.id.clone());
    }
    if flat_button(&mut buttons, "编辑", egui::vec2(52.0, 30.0), ButtonKind::Plain).clicked() {
        app::shared().draft = Some(Draft {
            id: Some(action.id.clone()),
            name: action.name.clone(),
            description: action.description.clone(),
            command: action.command.clone(),
        });
    }
}

/// 设置：管理员运行、开机启动、主题。
fn show_settings(ui: &mut egui::Ui, panel: egui::Rect) {
    let pal = theme::palette();

    let rect = egui::Rect::from_center_size(panel.center(), egui::vec2(440.0, 324.0));
    paint_sheet(ui, rect, 14);

    let (elevate, autostart, mode) = {
        let sh = app::shared();
        (sh.config.elevate, sh.config.autostart, sh.config.theme)
    };

    let inner = inset(rect, app::PANEL_PADDING, app::PANEL_PADDING);
    let mut done = false;
    let mut new_elevate = elevate;
    let mut new_autostart = autostart;
    let mut new_mode = mode;

    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        let width = ui.available_width();

        ui.label(egui::RichText::new("设置").size(19.0).color(pal.text));
        ui.add_space(16.0);

        // 一张分组卡片，三行。
        let card = group_card(ui, width, SETTING_ROW_HEIGHT * 3.0);
        let row_rect = |i: usize| {
            egui::Rect::from_min_size(
                egui::pos2(card.left(), card.top() + i as f32 * SETTING_ROW_HEIGHT),
                egui::vec2(card.width(), SETTING_ROW_HEIGHT),
            )
        };

        setting_switch(
            ui,
            row_rect(0),
            "以管理员身份运行",
            "执行动作时通过 UAC 提权",
            &mut new_elevate,
        );
        row_separator(ui, row_rect(0));
        setting_switch(
            ui,
            row_rect(1),
            "开机启动",
            "登录 Windows 后自动运行并常驻托盘",
            &mut new_autostart,
        );
        row_separator(ui, row_rect(1));

        // 第三行：主题 + 分段选择器
        let theme_row = row_rect(2);
        ui.painter().text(
            egui::pos2(theme_row.left() + 16.0, theme_row.center().y),
            egui::Align2::LEFT_CENTER,
            "主题",
            egui::FontId::proportional(14.0),
            pal.text,
        );
        let mut seg_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(inset(theme_row, 14.0, 0.0))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        let labels: Vec<&str> = ThemeMode::ALL.iter().map(|m| m.label()).collect();
        let selected = ThemeMode::ALL.iter().position(|m| *m == mode).unwrap_or(0);
        if let Some(i) = segmented(&mut seg_ui, &labels, selected) {
            new_mode = ThemeMode::ALL[i];
        }

        // 右下角「完成」
        let done_rect = egui::Rect::from_min_size(
            egui::pos2(inner.right() - 84.0, inner.bottom() - 34.0),
            egui::vec2(84.0, 34.0),
        );
        let mut done_ui = ui.new_child(egui::UiBuilder::new().max_rect(done_rect));
        if flat_button(&mut done_ui, "完成", egui::vec2(84.0, 34.0), ButtonKind::Primary).clicked()
        {
            done = true;
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

/// 设置里的一行：左边标题 + 说明，右边开关。
fn setting_switch(ui: &mut egui::Ui, row: egui::Rect, title: &str, hint: &str, value: &mut bool) {
    let pal = theme::palette();

    ui.painter().text(
        egui::pos2(row.left() + 16.0, row.center().y - 9.0),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(14.0),
        pal.text,
    );
    ui.painter().text(
        egui::pos2(row.left() + 16.0, row.center().y + 11.0),
        egui::Align2::LEFT_CENTER,
        hint,
        egui::FontId::proportional(12.0),
        pal.text_dim,
    );

    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inset(row, 14.0, 0.0))
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    switch(&mut child, value);
}

/// 删除确认：macOS 风格的警示框，居中文字 + 并排两个按钮。
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

    let rect = egui::Rect::from_center_size(panel.center(), egui::vec2(330.0, 172.0));
    paint_sheet(ui, rect, 14);

    let inner = inset(rect, 24.0, 22.0);
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new("删除动作").size(16.0).color(pal.text));
            ui.add_space(8.0);
            ui.add(
                egui::Label::new(
                    egui::RichText::new(format!("确定要删除「{name}」吗？该操作无法撤销。"))
                        .size(12.5)
                        .color(pal.text_dim),
                )
                .wrap(),
            );
        });

        // 两个等宽按钮固定在底部。
        let gap = 10.0;
        let btn_w = (inner.width() - gap) / 2.0;
        let top = inner.bottom() - 34.0;
        let cancel_rect =
            egui::Rect::from_min_size(egui::pos2(inner.left(), top), egui::vec2(btn_w, 34.0));
        let delete_rect = egui::Rect::from_min_size(
            egui::pos2(inner.left() + btn_w + gap, top),
            egui::vec2(btn_w, 34.0),
        );
        let mut cancel_ui = ui.new_child(egui::UiBuilder::new().max_rect(cancel_rect));
        if flat_button(&mut cancel_ui, "取消", egui::vec2(btn_w, 34.0), ButtonKind::Ghost).clicked()
        {
            app::shared().confirm_delete = None;
        }
        let mut delete_ui = ui.new_child(egui::UiBuilder::new().max_rect(delete_rect));
        if flat_button(&mut delete_ui, "删除", egui::vec2(btn_w, 34.0), ButtonKind::DangerSolid)
            .clicked()
        {
            app::delete_action(&id);
            app::toast("已删除".to_owned(), false);
        }
    });

    // Esc 取消
    if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
        app::shared().confirm_delete = None;
    }
}

/// 新建 / 编辑动作的弹窗。
fn show_modal(ui: &mut egui::Ui, panel: egui::Rect) {
    let pal = theme::palette();

    let rect = egui::Rect::from_center_size(panel.center(), egui::vec2(480.0, MODAL_HEIGHT));
    paint_sheet(ui, rect, 14);

    let Some(mut draft) = app::shared().draft.take() else {
        return;
    };
    let is_new = draft.id.is_none();

    let mut cancel = false;
    let mut save = false;

    let inner = rect.shrink(app::PANEL_PADDING);
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

        ui.label(
            egui::RichText::new(if is_new { "新建动作" } else { "编辑动作" })
                .size(19.0)
                .color(pal.text),
        );
        ui.add_space(14.0);
        divider(ui, inner.width());
        ui.add_space(14.0);

        let fields_height = inner.height() - 24.0 - 14.0 - 1.0 - 14.0 - 14.0 - 34.0;
        egui::ScrollArea::vertical()
            .max_height(fields_height)
            .auto_shrink(false)
            .show(ui, |ui| {
                field(ui, "名称", &mut draft.name, false);
                ui.add_space(14.0);
                field(ui, "说明", &mut draft.description, false);
                ui.add_space(14.0);
                field(ui, "指令", &mut draft.command, true);
            });

        // 按钮固定在右下角：取消在左，保存（主操作）在最右。
        let save_rect = egui::Rect::from_min_size(
            egui::pos2(inner.right() - 84.0, inner.bottom() - 34.0),
            egui::vec2(84.0, 34.0),
        );
        let cancel_rect = egui::Rect::from_min_size(
            egui::pos2(inner.right() - 84.0 - 10.0 - 84.0, inner.bottom() - 34.0),
            egui::vec2(84.0, 34.0),
        );
        let mut cancel_ui = ui.new_child(egui::UiBuilder::new().max_rect(cancel_rect));
        if flat_button(&mut cancel_ui, "取消", egui::vec2(84.0, 34.0), ButtonKind::Ghost).clicked()
        {
            cancel = true;
        }
        let mut save_ui = ui.new_child(egui::UiBuilder::new().max_rect(save_rect));
        if flat_button(&mut save_ui, "保存", egui::vec2(84.0, 34.0), ButtonKind::Primary).clicked()
        {
            save = true;
        }
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
