//! 共用界面组件。
//!
//! 极简取向：没有一个常驻的彩色控件。按钮靠**字重和反相**区分主次，
//! 分组靠**细分隔线**，悬停才有底色反馈。

pub mod editor;
pub mod popup;

use eframe::egui;

use crate::theme;

/// 按钮风格。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    /// 主操作：实心（浅色近黑 / 深色近白），反相文字。
    Primary,
    /// 次要操作：平时无底，悬停出现淡底。
    Ghost,
    /// 纯文字：平时淡，悬停变正文色。
    Plain,
    /// 危险文字：平时淡，悬停变红。
    DangerPlain,
    /// 危险主操作：红色实心。
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

/// 自绘按钮。`radius` 固定 7——比胶囊克制，比直角柔和。
pub fn flat_button(
    ui: &mut egui::Ui,
    label: &str,
    size: egui::Vec2,
    kind: ButtonKind,
) -> egui::Response {
    let pal = theme::palette();
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let hot = response.hovered() || response.is_pointer_button_down_on();

    let (bg, fg) = match kind {
        ButtonKind::Primary => (
            if hot { pal.accent_hover } else { pal.accent },
            pal.on_accent,
        ),
        ButtonKind::Ghost => (
            if hot { pal.bg_soft } else { egui::Color32::TRANSPARENT },
            if hot { pal.text } else { pal.text_dim },
        ),
        ButtonKind::Plain => (
            if hot { pal.accent_soft } else { egui::Color32::TRANSPARENT },
            if hot { pal.text } else { pal.text_dim },
        ),
        ButtonKind::DangerPlain => (
            if hot { pal.danger_soft } else { egui::Color32::TRANSPARENT },
            if hot { pal.danger } else { pal.text_dim },
        ),
        ButtonKind::DangerSolid => (
            if hot {
                pal.danger.lerp_to_gamma(egui::Color32::BLACK, 0.14)
            } else {
                pal.danger
            },
            egui::Color32::WHITE,
        ),
    };

    if bg != egui::Color32::TRANSPARENT {
        fill_rounded(ui.painter(), rect, 7, bg, egui::Stroke::NONE);
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(13.0),
        fg,
    );

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// 关闭按钮：手绘的叉号，悬停时出现淡底。
///
/// 不用字形，避免不同系统字体把「×」画得大小不一。
pub fn close_button(ui: &mut egui::Ui) -> egui::Response {
    let pal = theme::palette();
    let size = egui::vec2(24.0, 24.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if response.hovered() {
        fill_rounded(ui.painter(), rect, 6, pal.bg_soft, egui::Stroke::NONE);
    }

    let color = if response.hovered() {
        pal.text
    } else {
        pal.text_dim
    };
    let stroke = egui::Stroke::new(1.4, color);
    let c = rect.center();
    let r = 4.0;
    ui.painter()
        .line_segment([c - egui::vec2(r, r), c + egui::vec2(r, r)], stroke);
    ui.painter()
        .line_segment([c + egui::vec2(r, -r), c + egui::vec2(-r, r)], stroke);

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// 模态弹窗的输入拦截层。
///
/// 弹窗打开后，下面被压暗的内容仍然会响应鼠标和键盘——那会让用户误以为
/// 还能操作。这里在整个面板上铺一层不可见的交互区：它是在背景内容**之后**
/// 添加的，因此在同一层里盖在上面，命中测试优先命中它，背景就收不到点击了。
/// 弹窗自身的控件再在它之后添加，所以照常可用。
///
/// 遮罩只拦截输入，不关闭弹窗——关窗走标题栏的 × 或 Esc，避免误触。
pub fn modal_blocker(ui: &mut egui::Ui, panel: egui::Rect, id: &str) {
    let _ = ui.interact(panel, egui::Id::unique(id), egui::Sense::click_and_drag());
}

/// 细分隔线。`inset` 是左侧留白，用来做列表的分组缩进。
pub fn hairline(ui: &mut egui::Ui, width: f32, inset: f32) {
    let pal = theme::palette();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 1.0), egui::Sense::hover());
    ui.painter().hline(
        (rect.left() + inset)..=rect.right(),
        rect.center().y,
        egui::Stroke::new(1.0, pal.border),
    );
}

/// 菜单里的分组分隔线。
pub fn menu_separator(ui: &mut egui::Ui, width: f32) {
    ui.add_space(4.0);
    hairline(ui, width, 0.0);
    ui.add_space(4.0);
}

/// 菜单条目的内边距。
const MENU_PAD_V: f32 = 7.0;
const MENU_PAD_H: f32 = 10.0;
/// 菜单条目：标题字号。
const MENU_NAME_SIZE: f32 = 13.0;
/// 菜单条目：说明字号。
const MENU_DESC_SIZE: f32 = 11.5;

/// 菜单条目的高度。
///
/// 说明文字会自动换行，所以高度必须按**同样的换行宽度**实际排版后测量，
/// 否则窗口高度和内容对不上，多出来的部分会被窗口裁掉。
pub fn menu_item_height(ctx: &egui::Context, desc: Option<&str>, width: f32) -> f32 {
    let painter = ctx.debug_painter();
    let name_font = egui::FontId::proportional(MENU_NAME_SIZE);

    let Some(desc) = desc.filter(|d| !d.trim().is_empty()) else {
        return crate::app::MENU_ITEM_HEIGHT;
    };

    let name_h = painter
        .layout_no_wrap("测".to_owned(), name_font, egui::Color32::WHITE)
        .size()
        .y;
    let desc_h = painter
        .layout(
            desc.to_owned(),
            egui::FontId::proportional(MENU_DESC_SIZE),
            egui::Color32::WHITE,
            width - MENU_PAD_H * 2.0,
        )
        .size()
        .y;

    MENU_PAD_V * 2.0 + name_h + 3.0 + desc_h
}

/// 菜单条目：标题 + 说明，悬停时淡底。不用彩色高亮。
pub fn menu_item(
    ui: &mut egui::Ui,
    title: &str,
    desc: Option<&str>,
    id: &str,
    width: f32,
) -> egui::Response {
    let pal = theme::palette();
    let height = menu_item_height(ui.ctx(), desc, width);

    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let response = ui
        .interact(rect, ui.make_persistent_id(id), egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let hovered = response.hovered();

    if hovered {
        fill_rounded(ui.painter(), rect, 5, pal.bg_soft, egui::Stroke::NONE);
    }

    let name_font = egui::FontId::proportional(MENU_NAME_SIZE);
    let name = ellipsize(ui, title, name_font.clone(), width - MENU_PAD_H * 2.0);
    let galley = ui.painter().layout_no_wrap(name, name_font, pal.text);
    let name_h = galley.size().y;

    // 标题贴在顶部内边距之下；说明跟在标题后面自动换行。
    ui.painter().galley(
        egui::pos2(rect.left() + MENU_PAD_H, rect.top() + MENU_PAD_V),
        galley,
        pal.text,
    );

    if let Some(desc) = desc.filter(|d| !d.trim().is_empty()) {
        let galley = ui.painter().layout(
            desc.to_owned(),
            egui::FontId::proportional(MENU_DESC_SIZE),
            pal.text_dim,
            width - MENU_PAD_H * 2.0,
        );
        ui.painter().galley(
            egui::pos2(
                rect.left() + MENU_PAD_H,
                rect.top() + MENU_PAD_V + name_h + 3.0,
            ),
            galley,
            pal.text_dim,
        );
    }

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

/// 开关：近乎单色。开=深色轨道+白色滑块，关=浅灰轨道+白色滑块。
pub fn switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let pal = theme::palette();
    let size = egui::vec2(40.0, 24.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if response.clicked() {
        *on = !*on;
    }

    let t = ui.ctx().animate_bool_with_time(response.id, *on, 0.15);
    let track = pal.switch_off.lerp_to_gamma(pal.switch_on, t);
    fill_rounded(ui.painter(), rect, 12, track, egui::Stroke::NONE);

    let radius = 9.5;
    let travel = rect.width() - 2.0 * (radius + 2.5);
    let center = egui::pos2(rect.left() + radius + 2.5 + travel * t, rect.center().y);
    ui.painter()
        .circle_filled(center, radius, egui::Color32::WHITE);

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// 分段选择器。选中段是一块略亮的圆角底。
///
/// 返回被点击的那一段的下标；没有点击则返回 `None`。
pub fn segmented(ui: &mut egui::Ui, labels: &[&str], selected: usize) -> Option<usize> {
    let pal = theme::palette();
    let seg_w = 62.0;
    let height = 28.0;
    let pad = 2.0;
    let total = egui::vec2(seg_w * labels.len() as f32 + pad * 2.0, height);

    let (rect, _) = ui.allocate_exact_size(total, egui::Sense::hover());
    fill_rounded(ui.painter(), rect, 7, pal.bg_soft, egui::Stroke::NONE);

    let mut clicked = None;
    for (i, label) in labels.iter().enumerate() {
        let seg = egui::Rect::from_min_size(
            egui::pos2(rect.left() + pad + seg_w * i as f32, rect.top() + pad),
            egui::vec2(seg_w, height - pad * 2.0),
        );
        let response = ui.interact(seg, ui.scope_id().with(("segment", i)), egui::Sense::click());
        let is_selected = i == selected;

        if is_selected {
            fill_rounded(ui.painter(), seg, 5, pal.segment_selected, egui::Stroke::NONE);
        }

        let color = if is_selected || response.hovered() {
            pal.text
        } else {
            pal.text_faint
        };
        ui.painter().text(
            seg.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(12.0),
            color,
        );

        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.clicked() {
            clicked = Some(i);
        }
    }
    clicked
}

/// 带标签的输入框。外框是一条淡底 + 无描边，聚焦时才出现描边。
pub fn field(ui: &mut egui::Ui, label: &str, text: &mut String, multiline: bool) {
    let pal = theme::palette();
    ui.label(egui::RichText::new(label).size(12.0).color(pal.text_dim));
    ui.add_space(5.0);

    let frame = egui::Frame::new()
        .fill(pal.bg_soft)
        .corner_radius(egui::CornerRadius::from(7u8))
        .inner_margin(egui::Margin::symmetric(10, 7));

    frame.show(ui, |ui| {
        if multiline {
            ui.add(
                egui::TextEdit::multiline(text)
                    .frame(egui::Frame::NONE)
                    .margin(egui::Margin::ZERO)
                    .desired_width(f32::INFINITY)
                    .desired_rows(5)
                    .font(egui::FontId::monospace(12.0))
                    .hint_text("例如：Get-Service | Where-Object { $_.Status -eq 'Stopped' }"),
            );
        } else {
            ui.add(
                egui::TextEdit::singleline(text)
                    .frame(egui::Frame::NONE)
                    .margin(egui::Margin::ZERO)
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
