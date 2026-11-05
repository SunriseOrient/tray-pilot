//! 共用界面组件，视觉参照 Apple 的 macOS / iOS 设计语言：
//! 系统蓝强调色、圆角填充按钮、iOS 风格开关、分段选择器、分组卡片。

pub mod editor;
pub mod popup;

use eframe::egui;

use crate::theme;

/// 按钮风格。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    /// 主按钮：系统蓝实心，白字。
    Primary,
    /// 次要按钮：灰色填充，正文色文字。
    Ghost,
    /// 纯文字按钮：蓝色文字，悬停时淡蓝底（用于行内的「编辑」）。
    Plain,
    /// 纯文字危险按钮：红色文字（用于行内的「删除」）。
    DangerPlain,
    /// 危险主按钮：红色实心，白字（用于删除确认）。
    DangerSolid,
}

/// 圆角矩形（统一的 `rect` 调用，省得到处写 `StrokeKind`）。
pub fn fill_rounded(
    painter: &egui::Painter,
    rect: egui::Rect,
    radius: u8,
    fill: egui::Color32,
    stroke: egui::Stroke,
) {
    painter.rect(
        rect,
        egui::CornerRadius::from(radius),
        fill,
        stroke,
        egui::StrokeKind::Inside,
    );
}

/// 自绘的按钮。
pub fn flat_button(
    ui: &mut egui::Ui,
    label: &str,
    size: egui::Vec2,
    kind: ButtonKind,
) -> egui::Response {
    let pal = theme::palette();
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let hovered = response.hovered();
    let pressed = response.is_pointer_button_down_on();

    let (bg, fg) = match kind {
        ButtonKind::Primary => (
            if pressed || hovered { pal.accent_hover } else { pal.accent },
            egui::Color32::WHITE,
        ),
        ButtonKind::Ghost => (
            if pressed || hovered {
                pal.bg_soft.lerp_to_gamma(pal.text, 0.07)
            } else {
                pal.bg_soft
            },
            pal.text,
        ),
        ButtonKind::Plain => (
            if hovered { pal.accent_soft } else { egui::Color32::TRANSPARENT },
            pal.accent,
        ),
        ButtonKind::DangerPlain => (
            if hovered { pal.danger_soft } else { egui::Color32::TRANSPARENT },
            pal.danger,
        ),
        ButtonKind::DangerSolid => (
            if pressed || hovered {
                pal.danger.lerp_to_gamma(egui::Color32::BLACK, 0.12)
            } else {
                pal.danger
            },
            egui::Color32::WHITE,
        ),
    };

    fill_rounded(ui.painter(), rect, 8, bg, egui::Stroke::NONE);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(13.5),
        fg,
    );

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

/// 分组卡片：白色（深色模式下为深灰）圆角底，细描边。
/// 返回卡片矩形，调用方在里面自行排版每一行。
pub fn group_card(ui: &mut egui::Ui, width: f32, height: f32) -> egui::Rect {
    let pal = theme::palette();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    fill_rounded(
        ui.painter(),
        rect,
        12,
        pal.surface,
        egui::Stroke::new(1.0, pal.border),
    );
    rect
}

/// 卡片内两行之间的分隔线：左侧缩进 16，和 iOS 的列表一致。
pub fn row_separator(ui: &egui::Ui, row: egui::Rect) {
    let pal = theme::palette();
    ui.painter().hline(
        (row.left() + 16.0)..=row.right(),
        row.bottom(),
        egui::Stroke::new(1.0, pal.border),
    );
}

/// 菜单里的分组分隔线（上下各留一点空白）。
pub fn menu_separator(ui: &mut egui::Ui, width: f32) {
    ui.add_space(5.0);
    divider(ui, width);
    ui.add_space(5.0);
}

/// macOS 菜单风格的条目：单行文字，悬停时整行系统蓝高亮、文字变白。
pub fn menu_item(ui: &mut egui::Ui, label: &str, id: &str, width: f32) -> egui::Response {
    let pal = theme::palette();
    let height = crate::app::MENU_ITEM_HEIGHT;

    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let response = ui.interact(rect, ui.make_persistent_id(id), egui::Sense::click());
    let hovered = response.hovered();

    if hovered {
        fill_rounded(ui.painter(), rect, 5, pal.accent, egui::Stroke::NONE);
    }

    let color = if hovered { egui::Color32::WHITE } else { pal.text };
    let font = egui::FontId::proportional(13.5);
    let text = ellipsize(ui, label, font.clone(), width - 24.0);
    let galley = ui.painter().layout_no_wrap(text, font, color);
    ui.painter().galley(
        egui::pos2(rect.left() + 12.0, rect.center().y - galley.size().y * 0.5),
        galley,
        color,
    );

    response
}

/// 文本超出宽度时截断并加省略号。
fn ellipsize(ui: &egui::Ui, text: &str, font: egui::FontId, max_width: f32) -> String {
    let painter = ui.painter();
    let galley = painter.layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE);
    if galley.size().x <= max_width {
        return text.to_owned();
    }

    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let candidate: String = chars.iter().collect();
        let measured =
            painter.layout_no_wrap(candidate.clone(), font.clone(), egui::Color32::WHITE);
        if measured.size().x <= max_width - 12.0 {
            return format!("{candidate}…");
        }
    }
    String::new()
}

/// iOS 风格开关：绿色轨道，白色圆形滑块带轻投影，切换时有过渡动画。
pub fn switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let pal = theme::palette();
    let size = egui::vec2(46.0, 28.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if response.clicked() {
        *on = !*on;
    }

    // 0.0 = 关，1.0 = 开，中间值是过渡动画。
    let t = ui.ctx().animate_bool_with_time(response.id, *on, 0.16);

    let track = pal.switch_off.lerp_to_gamma(pal.switch_on, t);
    fill_rounded(ui.painter(), rect, 14, track, egui::Stroke::NONE);

    let radius = 11.5;
    let travel = rect.width() - 2.0 * (radius + 2.5);
    let knob_center = egui::pos2(rect.left() + radius + 2.5 + travel * t, rect.center().y);

    // 滑块下面一层极淡的投影。
    ui.painter().circle_filled(
        knob_center + egui::vec2(0.0, 1.5),
        radius,
        egui::Color32::from_black_alpha(36),
    );
    ui.painter()
        .circle_filled(knob_center, radius, egui::Color32::WHITE);

    response
}

/// 分段选择器（iOS 的 Segmented Control）。
///
/// 返回被点击的那一段的下标；没有点击则返回 `None`。
pub fn segmented(ui: &mut egui::Ui, labels: &[&str], selected: usize) -> Option<usize> {
    let pal = theme::palette();
    let seg_w = 70.0;
    let height = 30.0;
    let pad = 2.0;
    let total = egui::vec2(seg_w * labels.len() as f32 + pad * 2.0, height);

    let (rect, _) = ui.allocate_exact_size(total, egui::Sense::hover());
    fill_rounded(ui.painter(), rect, 8, pal.bg_soft, egui::Stroke::NONE);

    let mut clicked = None;
    for (i, label) in labels.iter().enumerate() {
        let seg = egui::Rect::from_min_size(
            egui::pos2(rect.left() + pad + seg_w * i as f32, rect.top() + pad),
            egui::vec2(seg_w, height - pad * 2.0),
        );
        let response = ui.interact(seg, ui.scope_id().with(("segment", i)), egui::Sense::click());
        let is_selected = i == selected;

        if is_selected {
            // 选中的一段「浮」起来：白底（深色模式为灰）+ 细投影。
            ui.painter().add(
                egui::Shadow {
                    offset: [0, 1],
                    blur: 4,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(34),
                }
                .as_shape(seg, egui::CornerRadius::from(6u8)),
            );
            fill_rounded(ui.painter(), seg, 6, pal.segment_selected, egui::Stroke::NONE);
        }

        let color = if is_selected || response.hovered() {
            pal.text
        } else {
            pal.text_dim
        };
        ui.painter().text(
            seg.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(12.5),
            color,
        );

        if response.clicked() {
            clicked = Some(i);
        }
    }
    clicked
}

/// 带标签的输入框：标签在上，输入区是白底圆角 + 细描边（深色模式为深灰）。
pub fn field(ui: &mut egui::Ui, label: &str, text: &mut String, multiline: bool) {
    let pal = theme::palette();
    ui.label(egui::RichText::new(label).size(12.5).color(pal.text_dim));
    ui.add_space(6.0);

    let frame = egui::Frame::new()
        .fill(pal.surface)
        .stroke(egui::Stroke::new(1.0, pal.border_strong))
        .corner_radius(egui::CornerRadius::from(8u8))
        .inner_margin(egui::Margin::symmetric(11, 8));

    frame.show(ui, |ui| {
        // 外框已经画好了，TextEdit 自己的底和描边要关掉，否则会套出两层边框。
        if multiline {
            ui.add(
                egui::TextEdit::multiline(text)
                    .frame(egui::Frame::NONE).margin(egui::Margin::ZERO)
                    .desired_width(f32::INFINITY)
                    .desired_rows(5)
                    .font(egui::FontId::monospace(12.0))
                    .hint_text("例如：Get-Service | Where-Object { $_.Status -eq 'Stopped' }"),
            );
        } else {
            ui.add(
                egui::TextEdit::singleline(text)
                    .frame(egui::Frame::NONE).margin(egui::Margin::ZERO)
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
