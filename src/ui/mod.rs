//! 共用界面组件：按钮、分隔线、开关等。

pub mod editor;
pub mod popup;

use eframe::egui;

use crate::theme;

/// 按钮风格。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    /// 主按钮，强调色实心。
    Primary,
    /// 次要按钮，描边。
    Ghost,
    /// 危险操作，描边 + 危险色。
    Danger,
}

/// 自绘的扁平圆角按钮，完全掌控外观。
pub fn flat_button(
    ui: &mut egui::Ui,
    label: &str,
    size: egui::Vec2,
    kind: ButtonKind,
) -> egui::Response {
    let pal = theme::palette();
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    let (mut bg, mut fg, border) = match kind {
        ButtonKind::Primary => (pal.accent, egui::Color32::WHITE, pal.accent),
        ButtonKind::Ghost => (egui::Color32::TRANSPARENT, pal.text_dim, pal.border),
        ButtonKind::Danger => (egui::Color32::TRANSPARENT, pal.danger, pal.border),
    };

    if response.hovered() {
        match kind {
            ButtonKind::Primary => bg = pal.accent_hover,
            ButtonKind::Ghost => {
                bg = pal.bg_soft;
                fg = pal.text;
            }
            ButtonKind::Danger => {
                bg = pal.danger_soft;
                fg = pal.danger;
            }
        }
    }

    let stroke = if kind == ButtonKind::Primary {
        egui::Stroke::NONE
    } else {
        egui::Stroke::new(1.0, border)
    };

    ui.painter().rect(
        rect,
        egui::CornerRadius::from(8u8),
        bg,
        stroke,
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(13.0),
        fg,
    );

    response
}

/// 关闭按钮：一个手绘的叉号，避免依赖特殊字形。
pub fn close_button(ui: &mut egui::Ui) -> egui::Response {
    let pal = theme::palette();
    let size = egui::vec2(28.0, 28.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if response.hovered() {
        ui.painter().rect(
            rect,
            egui::CornerRadius::from(8u8),
            pal.bg_soft,
            egui::Stroke::NONE,
            egui::StrokeKind::Inside,
        );
    }

    let color = if response.hovered() {
        pal.text
    } else {
        pal.text_dim
    };
    let stroke = egui::Stroke::new(1.6, color);
    let c = rect.center();
    let r = 4.5;
    ui.painter()
        .line_segment([c - egui::vec2(r, r), c + egui::vec2(r, r)], stroke);
    ui.painter()
        .line_segment([c + egui::vec2(r, -r), c + egui::vec2(-r, r)], stroke);

    response
}

/// 水平分隔线。
pub fn divider(ui: &mut egui::Ui, width: f32) {
    let pal = theme::palette();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 1.0), egui::Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        egui::Stroke::new(1.0, pal.border),
    );
}

/// 开关。
pub fn switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let pal = theme::palette();
    let size = egui::vec2(38.0, 22.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if response.clicked() {
        *on = !*on;
    }

    let track = if *on { pal.accent } else { pal.bg_soft };
    ui.painter().rect(
        rect,
        egui::CornerRadius::from(11u8),
        track,
        egui::Stroke::new(1.0, if *on { pal.accent } else { pal.border }),
        egui::StrokeKind::Inside,
    );

    let knob_x = if *on {
        rect.right() - 11.0
    } else {
        rect.left() + 11.0
    };
    ui.painter()
        .circle_filled(egui::pos2(knob_x, rect.center().y), 8.0, egui::Color32::WHITE);

    response
}

/// 带图标背景的输入框（egui 自带的 TextEdit 外观略旧，这里统一成圆角描边风格）。
pub fn field(ui: &mut egui::Ui, label: &str, text: &mut String, multiline: bool) {
    let pal = theme::palette();
    ui.label(egui::RichText::new(label).size(12.0).color(pal.text_dim));
    ui.add_space(5.0);

    let frame = egui::Frame::new()
        .fill(pal.bg_soft)
        .stroke(egui::Stroke::new(1.0, pal.border))
        .corner_radius(egui::CornerRadius::from(9u8))
        .inner_margin(egui::Margin::symmetric(10, 8));

    frame.show(ui, |ui| {
        if multiline {
            ui.add(
                egui::TextEdit::multiline(text)
                    .desired_width(f32::INFINITY)
                    .desired_rows(5)
                    .font(egui::FontId::monospace(12.0))
                    .hint_text("例如：Get-Service | Where-Object { $_.Status -eq 'Stopped' }"),
            );
        } else {
            ui.add(
                egui::TextEdit::singleline(text)
                    .desired_width(f32::INFINITY)
                    .hint_text(match label {
                    "名称" => "给这个动作起个名字",
                    "说明" => "一句话描述它的作用（可留空）",
                    _ => "",
                }),
            );
        }
    });
}
