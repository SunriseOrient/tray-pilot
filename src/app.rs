//! 应用状态机：托盘事件 → 弹出菜单 / 编辑面板 → 执行动作。

use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use eframe::egui;
use tray_icon::TrayIcon;

use crate::config::{Action, AppConfig};
use crate::{exec, theme};

/// 弹出菜单面板宽度（点）。
pub const POPUP_WIDTH: f32 = 250.0;
/// 菜单条目高度（点）。
pub const MENU_ITEM_HEIGHT: f32 = 30.0;
/// 菜单内容内边距（点）。
pub const MENU_PADDING: f32 = 6.0;
/// 面板内边距（点）。
pub const PANEL_PADDING: f32 = 20.0;
/// 窗口四周留给阴影的空白（点）。
///
/// 保持为 0：窗口与面板完全重合，否则四周会留下一圈透明区域——
/// 它虽然看不见，却会挡住下面程序的点击，看起来像凭空多出一块窗体。
pub const WINDOW_MARGIN: f32 = 0.0;

/// 弹出菜单窗口标题。
pub const POPUP_TITLE: &str = "TrayPilot · 常用动作";
/// 编辑面板窗口标题。
pub const EDITOR_TITLE: &str = "TrayPilot · 常用动作管理";

/// 弹出菜单视口 ID。
pub fn popup_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("tray-pilot::popup")
}

/// 编辑面板视口 ID。
pub fn editor_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("tray-pilot::editor")
}

/// 弹出菜单的运行状态。
pub struct Popup {
    /// 窗口位置（逻辑像素）。
    pub pos: egui::Pos2,
    /// 窗口尺寸（逻辑像素）。
    pub size: egui::Vec2,
    /// 打开时间。
    pub opened_at: Instant,
    /// 是否曾经获得过焦点（用于失焦自动关闭）。
    pub focused_once: bool,
    /// 状态提示，例如「已提交：重启 IIS」。
    pub status: Option<(String, Instant)>,
    /// 到点后自动关闭。
    pub close_at: Option<Instant>,
    /// 已经渲染的帧数：第 2 帧时窗口已存在，可以安全地请求焦点。
    pub frames: u32,
}

/// 编辑面板的运行状态。
pub struct Editor {
    pub pos: egui::Pos2,
    pub size: egui::Vec2,
    /// 已经渲染的帧数。
    pub frames: u32,
}

/// 正在编辑 / 新建的动作。
pub struct Draft {
    /// `Some` 表示编辑已有动作，`None` 表示新建。
    pub id: Option<String>,
    pub name: String,
    pub description: String,
    pub command: String,
}

/// 短暂的提示消息。
#[derive(Clone)]
pub struct Toast {
    pub text: String,
    pub until: Instant,
    pub error: bool,
}

/// 全局共享状态。所有访问都发生在主线程，锁只用于满足 `Send + Sync` 要求。
pub struct Shared {
    pub config: AppConfig,
    pub popup: Option<Popup>,
    pub editor: Option<Editor>,
    pub draft: Option<Draft>,
    pub toast: Option<Toast>,
    /// 已经点过一次「删除」的动作 ID，需要再点一次才真正删除。
    pub delete_armed: Option<String>,
    pub quit: bool,
    pending: Vec<TrayCommand>,
}

static SHARED: LazyLock<Mutex<Shared>> = LazyLock::new(|| {
    Mutex::new(Shared {
        config: AppConfig::default(),
        popup: None,
        editor: None,
        draft: None,
        toast: None,
        delete_armed: None,
        quit: false,
        pending: Vec::new(),
    })
});

/// 取得共享状态（忽略锁中毒）。
pub fn shared() -> MutexGuard<'static, Shared> {
    SHARED.lock().unwrap_or_else(|err| err.into_inner())
}

/// 来自托盘图标的事件。
pub enum TrayCommand {
    /// 左键或右键点击托盘图标。
    ShowPopup {
        /// 托盘图标矩形（物理像素）：left, top, right, bottom。
        tray: (f64, f64, f64, f64),
    },
}

/// 供托盘事件回调调用。
pub fn push_tray_command(cmd: TrayCommand) {
    shared().pending.push(cmd);
}

/// 主应用。
pub struct TrayPilotApp {
    /// 必须一直存活，否则托盘图标会消失。
    _tray: Box<TrayIcon>,
    /// 是否已经处理过启动预览。
    previewed: bool,
}

impl TrayPilotApp {
    pub fn new(tray: TrayIcon) -> Self {
        Self {
            _tray: Box::new(tray),
            previewed: false,
        }
    }

    /// 通过环境变量 `TRAYPILOT_PREVIEW` 在启动时预览某个窗口，便于开发调试。
    ///
    /// 可选值：`popup`（模拟一次托盘点击）、`editor`（直接打开编辑面板）。
    fn maybe_preview(&mut self, ctx: &egui::Context) {
        if self.previewed {
            return;
        }
        self.previewed = true;

        let Ok(mode) = std::env::var("TRAYPILOT_PREVIEW") else {
            return;
        };

        match mode.as_str() {
            "popup" => {
                // 模拟一次托盘点击：图标位于屏幕右下角。
                let monitor = ctx
                    .input(|i| i.viewport().monitor_size)
                    .unwrap_or(egui::vec2(1920.0, 1080.0));
                let scale = dpi_scale(0.0, 0.0);
                let icon_x = f64::from(monitor.x) * scale - 120.0;
                let icon_y = f64::from(monitor.y) * scale - 48.0;
                push_tray_command(TrayCommand::ShowPopup {
                    tray: (icon_x, icon_y, icon_x + 40.0, icon_y + 40.0),
                });
            }
            "editor" => open_editor(ctx),
            "modal" => {
                open_editor(ctx);
                shared().draft = Some(Draft {
                    id: None,
                    name: String::new(),
                    description: String::new(),
                    command: String::new(),
                });
            }
            _ => {}
        }
    }

    /// 处理托盘事件与各类计时状态。
    fn handle_commands(&mut self, ctx: &egui::Context) {
        let mut sh = shared();

        if sh.quit {
            drop(sh);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        let commands = std::mem::take(&mut sh.pending);
        for cmd in commands {
            match cmd {
                TrayCommand::ShowPopup { tray } => {
                    let count = sh.config.actions.len();
                    let (pos, size) = popup_spec(ctx, tray, count);
                    sh.popup = Some(Popup {
                        pos,
                        size,
                        opened_at: Instant::now(),
                        focused_once: false,
                        status: None,
                        close_at: None,
                        frames: 0,
                    });
                }
            }
        }

        // 提示信息过期
        let now = Instant::now();
        if sh.toast.as_ref().is_some_and(|t| now >= t.until) {
            sh.toast = None;
        }

        // 弹出菜单到点关闭
        let expired = sh
            .popup
            .as_ref()
            .is_some_and(|p| p.close_at.is_some_and(|at| now >= at));
        if expired {
            sh.popup = None;
        }
    }

    /// 创建 / 维持子窗口。
    fn show_children(&self, ctx: &egui::Context) {
        let (popup, editor) = {
            let mut sh = shared();
            let popup = sh.popup.as_mut().map(|p| {
                p.frames += 1;
                (p.pos, p.size, p.frames)
            });
            let editor = sh.editor.as_mut().map(|e| {
                e.frames += 1;
                (e.pos, e.size, e.frames)
            });
            (popup, editor)
        };

        if let Some((pos, size, frames)) = popup {
            let builder = egui::ViewportBuilder::default()
                .with_title(POPUP_TITLE)
                .with_decorations(false)
                .with_transparent(true)
                .with_resizable(false)
                .with_always_on_top()
                .with_inner_size(size)
                .with_position(pos);
            ctx.show_viewport_deferred(popup_id(), builder, |ui, _| crate::ui::popup::show(ui));
            // 窗口创建后需要显式显示并激活，否则不会被系统合成到屏幕上。
            if frames == 2 {
                crate::win::activate(POPUP_TITLE, true);
            }
        }

        if let Some((pos, size, frames)) = editor {
            let builder = egui::ViewportBuilder::default()
                .with_title(EDITOR_TITLE)
                .with_decorations(false)
                .with_transparent(true)
                .with_resizable(false)
                .with_inner_size(size)
                .with_position(pos);
            ctx.show_viewport_deferred(editor_id(), builder, |ui, _| crate::ui::editor::show(ui));
            if frames == 2 {
                crate::win::activate(EDITOR_TITLE, false);
            }
        }
    }
}

impl eframe::App for TrayPilotApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        theme::apply(ctx);
        self.maybe_preview(ctx);
        self.handle_commands(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // 根窗口只是 egui 上下文的宿主，始终保持隐藏。
        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Visible(false));
        self.show_children(&ctx);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // 透明背景：圆角与阴影由 egui 自己绘制。
        [0.0, 0.0, 0.0, 0.0]
    }
}

/// 计算弹出菜单窗口的位置与尺寸。
fn popup_spec(
    ctx: &egui::Context,
    tray: (f64, f64, f64, f64),
    count: usize,
) -> (egui::Pos2, egui::Vec2) {
    let panel_h = f64::from(popup_panel_height(count));
    let win_w = f64::from(POPUP_WIDTH) + 2.0 * f64::from(WINDOW_MARGIN);
    let win_h = panel_h + 2.0 * f64::from(WINDOW_MARGIN);

    // 以托盘图标所在显示器的缩放比例换算物理像素 → 逻辑像素。
    let scale = dpi_scale((tray.0 + tray.2) * 0.5, (tray.1 + tray.3) * 0.5);
    let icon_center_x = (tray.0 + tray.2) * 0.5;

    // 以托盘图标为中心，向上弹出。
    let mut phys_x = icon_center_x - win_w * scale * 0.5;
    let phys_y = tray.1 - 8.0 - win_h * scale;

    // 不要超出屏幕右边缘。
    let monitor = ctx
        .input(|i| i.viewport().monitor_size)
        .unwrap_or(egui::vec2(1920.0, 1080.0));
    let max_x = f64::from(monitor.x) * scale - win_w * scale - 8.0;
    phys_x = phys_x.min(max_x).max(8.0);

    (
        egui::pos2((phys_x / scale) as f32, (phys_y / scale) as f32),
        egui::vec2(win_w as f32, win_h as f32),
    )
}

/// 弹出菜单面板的高度（点）。
pub fn popup_panel_height(count: usize) -> f32 {
    // 动作条目 + 分组分隔线 + 「编辑」「退出」两项
    let content = count as f32 * MENU_ITEM_HEIGHT
        + if count > 0 { 11.0 } else { 0.0 }
        + 2.0 * MENU_ITEM_HEIGHT;
    content + 2.0 * MENU_PADDING
}

/// 计算编辑面板窗口的位置与尺寸（居中）。
fn editor_spec(ctx: &egui::Context) -> (egui::Pos2, egui::Vec2) {
    let monitor = ctx
        .input(|i| i.viewport().monitor_size)
        .unwrap_or(egui::vec2(1920.0, 1080.0));

    let panel_w = (monitor.x * 0.46).clamp(460.0, 660.0);
    let panel_h = (monitor.y * 0.56).clamp(360.0, 520.0);
    let win_w = panel_w + 2.0 * WINDOW_MARGIN;
    let win_h = panel_h + 2.0 * WINDOW_MARGIN;

    (
        egui::pos2((monitor.x - win_w) * 0.5, (monitor.y - win_h) * 0.5),
        egui::vec2(win_w, win_h),
    )
}

/// 打开编辑面板。
pub fn open_editor(ctx: &egui::Context) {
    let (pos, size) = editor_spec(ctx);
    let mut sh = shared();
    sh.editor = Some(Editor {
        pos,
        size,
        frames: 0,
    });
}

/// 关闭编辑面板。
pub fn close_editor() {
    shared().editor = None;
}

/// 关闭弹出菜单。
pub fn close_popup() {
    shared().popup = None;
}

/// 设置弹出菜单上的状态提示，并在一段时间后自动关闭。
pub fn set_popup_status(text: String) {
    let now = Instant::now();
    let mut sh = shared();
    if let Some(popup) = sh.popup.as_mut() {
        popup.status = Some((text, now));
        popup.close_at = Some(now + Duration::from_millis(1100));
    }
}

/// 显示一条提示消息。
pub fn toast(text: String, error: bool) {
    let mut sh = shared();
    sh.toast = Some(Toast {
        text,
        until: Instant::now() + Duration::from_secs(5),
        error,
    });
}

/// 执行一个动作（异步，避免 UAC 对话框阻塞界面）。
pub fn run_action(id: &str) {
    let (action, elevate) = {
        let sh = shared();
        match sh.config.actions.iter().find(|a| a.id == id) {
            Some(action) => (action.clone(), sh.config.elevate),
            None => return,
        }
    };

    let label = action.name.clone();
    set_popup_status(format!("正在执行：{label}"));

    std::thread::spawn(move || {
        if let Err(err) = exec::run(&action, elevate) {
            let mut sh = shared();
            if sh.popup.is_some() {
                if let Some(popup) = sh.popup.as_mut() {
                    popup.status = Some((err.clone(), Instant::now()));
                    popup.close_at = Some(Instant::now() + Duration::from_millis(2200));
                }
            } else {
                sh.toast = Some(Toast {
                    text: err,
                    until: Instant::now() + Duration::from_secs(6),
                    error: true,
                });
            }
        }
    });
}

/// 保存一个新建 / 修改后的动作。
pub fn save_action(draft: &Draft) -> Result<(), String> {
    let name = draft.name.trim().to_owned();
    let command = draft.command.trim().to_owned();
    if name.is_empty() {
        return Err("名称不能为空".to_owned());
    }
    if command.is_empty() {
        return Err("指令不能为空".to_owned());
    }

    let config = {
        let mut sh = shared();
        match &draft.id {
            Some(id) => {
                if let Some(action) = sh.config.actions.iter_mut().find(|a| &a.id == id) {
                    action.name = name;
                    action.description = draft.description.trim().to_owned();
                    action.command = command;
                } else {
                    sh.config.actions.push(Action {
                        id: id.clone(),
                        name,
                        description: draft.description.trim().to_owned(),
                        command,
                    });
                }
            }
            None => sh.config.actions.push(Action::new(
                &name,
                draft.description.trim(),
                &command,
            )),
        }
        sh.config.clone()
    };

    crate::config::save(&config)
}

/// 删除一个动作。
pub fn delete_action(id: &str) {
    let config = {
        let mut sh = shared();
        sh.config.actions.retain(|a| a.id != id);
        sh.delete_armed = None;
        sh.config.clone()
    };
    if let Err(err) = crate::config::save(&config) {
        toast(err, true);
    }
}

/// 退出应用。
pub fn quit(ctx: &egui::Context) {
    shared().quit = true;
    ctx.request_repaint();
}

/// 读取指定物理坐标所在显示器的缩放比例。
fn dpi_scale(x: f64, y: f64) -> f64 {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    use windows::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTONEAREST};

    unsafe {
        let point = POINT {
            x: x.round() as i32,
            y: y.round() as i32,
        };
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        let mut dpi_x: u32 = 96;
        let mut dpi_y: u32 = 96;
        let _ = MDT_EFFECTIVE_DPI;
        let hr = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
        if hr.is_ok() {
            return f64::from(dpi_x) / 96.0;
        }
    }
    1.0
}
