//! Windows 平台辅助：把窗口显示出来并置于前台。
//!
//! egui / winit 创建的子窗口在某些情况下不会被系统合成到屏幕上
//! （窗口存在、可以命中测试，但画面是透明的）。这里用 Win32 API
//! 显式地显示并激活窗口，保证菜单能立刻显示在其它窗口之上。

use std::cell::RefCell;

use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::System::Threading::AttachThreadInput;
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow, SetWindowPos, ShowWindow,
    HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, SW_RESTORE,
};

thread_local! {
    /// 要查找的窗口标题（UTF-16）。
    static TARGET_TITLE: RefCell<Vec<u16>> = const { RefCell::new(Vec::new()) };
    /// 找到的窗口句柄。
    static FOUND: RefCell<Option<HWND>> = const { RefCell::new(None) };
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
        force_foreground(hwnd);
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
