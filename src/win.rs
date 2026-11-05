//! Windows 平台辅助：把窗口显示出来并置于前台。
//!
//! egui / winit 创建的子窗口在某些情况下不会被系统合成到屏幕上
//! （窗口存在、可以命中测试，但画面是透明的）。这里用 Win32 API
//! 显式地显示并激活窗口，保证菜单能立刻显示在其它窗口之上。

use std::cell::RefCell;
use std::collections::HashMap;

use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::System::Threading::AttachThreadInput;
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsWindow, IsWindowVisible, SetForegroundWindow, SetWindowPos,
    ShowWindow, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, SW_RESTORE,
};

thread_local! {
    /// 要查找的窗口标题（UTF-16）。
    static TARGET_TITLE: RefCell<Vec<u16>> = const { RefCell::new(Vec::new()) };
    /// 找到的窗口句柄。
    static FOUND: RefCell<Option<HWND>> = const { RefCell::new(None) };
    /// 标题 → 窗口句柄的缓存，避免每帧枚举所有窗口。
    static CACHE: RefCell<HashMap<String, HWND>> = RefCell::new(HashMap::new());
}

unsafe extern "system" fn enum_proc(hwnd: HWND, _lparam: LPARAM) -> BOOL {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };

    if pid != std::process::id() {
        return true.into();
    }
    if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
        return true.into();
    }

    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len == 0 {
        return true.into();
    }
    let mut buffer = vec![0u16; len as usize + 1];
    let written = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    if written <= 0 {
        return true.into();
    }

    let matched = TARGET_TITLE.with(|target| {
        let target = target.borrow();
        let title = &target[..target.len() - 1];
        buffer[..written as usize] == *title
    });

    if matched {
        FOUND.with(|found| *found.borrow_mut() = Some(hwnd));
        return false.into(); // 找到后停止枚举
    }
    true.into()
}

/// 强制把窗口置于前台。
///
/// 普通的 `SetForegroundWindow` 会被 Windows 的前台锁拦下（用户刚点击的是
/// 别的程序时），这里借用 `AttachThreadInput` 技巧绕过限制——托盘菜单必须
/// 立刻拿到焦点，否则第一次点击只会被用来激活窗口，按钮收不到。
fn force_foreground(hwnd: HWND) {
    unsafe {
        let mut foreground_thread = 0u32;
        GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut foreground_thread));
        let mut own_thread = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut own_thread));

        let attached = foreground_thread != 0 && foreground_thread != own_thread;
        if attached {
            let _ = AttachThreadInput(own_thread, foreground_thread, true);
        }
        let _ = SetForegroundWindow(hwnd);
        let _ = BringWindowToTop(hwnd);
        if attached {
            let _ = AttachThreadInput(own_thread, foreground_thread, false);
        }
    }
}

/// 显示并激活标题为 `title` 的窗口。
///
/// `topmost` 为真时把窗口置顶（托盘菜单需要始终显示在其它窗口之上）。
pub fn activate(title: &str, topmost: bool) {
    TARGET_TITLE.with(|target| {
        *target.borrow_mut() = title.encode_utf16().chain(std::iter::once(0)).collect()
    });
    FOUND.with(|found| *found.borrow_mut() = None);

    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(0));
    }

    let Some(hwnd) = FOUND.with(|found| *found.borrow()) else {
        return;
    };

    unsafe {
        // SW_RESTORE 会触发一次重绘，让窗口真正被系统合成出来。
        let _ = ShowWindow(hwnd, SW_RESTORE);
        // 预览模式只为看界面，不要抢用户当前窗口的焦点。
        if std::env::var_os("TRAYPILOT_PREVIEW").is_none() {
            force_foreground(hwnd);
        }
        if topmost {
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE,
            );
        }
    }
}

/// 按标题取窗口句柄（带缓存）。
pub fn window_handle(title: &str) -> Option<HWND> {
    if let Some(hwnd) = CACHE.with(|cache| cache.borrow().get(title).copied()) {
        if unsafe { IsWindow(Some(hwnd)) }.as_bool() {
            return Some(hwnd);
        }
    }
    let hwnd = find_window(title)?;
    CACHE.with(|cache| cache.borrow_mut().insert(title.to_owned(), hwnd));
    Some(hwnd)
}

/// 该窗口当前是否是系统前台窗口。
///
/// 比 egui 的焦点状态更可靠：点击其它程序、桌面或任务栏后都能立刻反映出来。
pub fn is_foreground(title: &str) -> bool {
    match window_handle(title) {
        Some(hwnd) => hwnd == unsafe { GetForegroundWindow() },
        None => false,
    }
}

/// 枚举本进程窗口，按标题查找（不含缓存）。
fn find_window(title: &str) -> Option<HWND> {
    TARGET_TITLE.with(|target| {
        *target.borrow_mut() = title.encode_utf16().chain(std::iter::once(0)).collect()
    });
    FOUND.with(|found| *found.borrow_mut() = None);

    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(0));
    }
    FOUND.with(|found| *found.borrow())
}

/// 鼠标左键或右键当前是否处于按下状态。
pub fn mouse_down() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON};
    unsafe {
        let left = GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16 & 0x8000 != 0;
        let right = GetAsyncKeyState(VK_RBUTTON.0 as i32) as u16 & 0x8000 != 0;
        left || right
    }
}

/// 鼠标指针当前是否落在该窗口的客户区之外。
pub fn cursor_outside(title: &str) -> bool {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect};

    let Some(hwnd) = window_handle(title) else {
        return true;
    };
    let mut rect = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
        return true;
    }
    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point) }.is_err() {
        return true;
    }
    let (x, y) = (point.x, point.y);
    x < rect.left || x >= rect.right || y < rect.top || y >= rect.bottom
}
