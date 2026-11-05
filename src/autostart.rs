//! 开机自启。
//!
//! 写的是当前用户的 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`，
//! 这样不需要管理员权限，也不会影响其他用户。

use windows::core::{w, HSTRING};
use windows::Win32::System::Registry::{
    RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ,
};

/// 注册表值名。
const VALUE_NAME: &str = "TrayPilot";

/// 注册表里的自启项命令。
fn command_line() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    // 路径可能含空格，统一加引号。
    Some(format!("\"{}\"", exe.display()))
}

/// 当前是否已开启开机自启。
///
/// 除了判断注册表项存在，还会比对路径：程序被移动过之后，注册表里残留的是
/// 旧路径，这时应当视为「未开启」，否则用户会以为开着、实际启动不了。
pub fn is_enabled() -> bool {
    let Some(expected) = command_line() else {
        return false;
    };

    let mut buffer = vec![0u16; 1024];
    let mut size = (buffer.len() * std::mem::size_of::<u16>()) as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Run"),
            &HSTRING::from(VALUE_NAME),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if status.is_err() {
        return false;
    }

    let len = (size as usize / std::mem::size_of::<u16>()).saturating_sub(1);
    let stored = String::from_utf16_lossy(&buffer[..len.min(buffer.len())]);
    stored.trim().eq_ignore_ascii_case(expected.trim())
}

/// 开启或关闭开机自启。
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let run_key = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");

    if enabled {
        let Some(cmd) = command_line() else {
            return Err("无法获取程序路径".to_owned());
        };
        let wide: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();
        let status = unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                run_key,
                &HSTRING::from(VALUE_NAME),
                REG_SZ.0,
                Some(wide.as_ptr().cast()),
                (wide.len() * std::mem::size_of::<u16>()) as u32,
            )
        };
        if status.is_err() {
            return Err(format!("写入注册表失败（错误码 {}）", status.0));
        }
    } else {
        let status = unsafe {
            RegDeleteKeyValueW(
                HKEY_CURRENT_USER,
                run_key,
                &HSTRING::from(VALUE_NAME),
            )
        };
        // 值本来就不存在时也算成功。
        if status.is_err() && status.0 != 2 {
            return Err(format!("删除注册表项失败（错误码 {}）", status.0));
        }
    }

    Ok(())
}
