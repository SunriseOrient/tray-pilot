//! 以管理员身份执行 PowerShell 指令。
//!
//! 指令被包装成一段脚本后，以 **UTF-16LE Base64** 的形式通过 `-EncodedCommand`
//! 传给 PowerShell —— 这样可以彻底绕开引号、空格、百分号等所有转义问题。
//! 脚本自身的标准输出与错误输出被重定向到日志文件，控制台窗口保持隐藏。

use std::path::{Path, PathBuf};

use base64::Engine;
use windows::core::HSTRING;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{SW_HIDE, SW_SHOWNORMAL};

use crate::config::Action;

/// 日志文件路径：`%APPDATA%\TrayPilot\logs\run.log`
pub fn log_path() -> PathBuf {
    crate::config::log_dir().join("run.log")
}

/// 执行一个动作。`elevate` 为真时通过 UAC 以管理员身份运行。
pub fn run(action: &Action, elevate: bool) -> Result<(), String> {
    let log = log_path();
    if let Some(dir) = log.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("创建日志目录失败：{e}"))?;
    }

    let script = build_script(action, &log);
    let encoded = encode_utf16le_base64(&script);
    let params = format!(
        "-NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -EncodedCommand {encoded}"
    );

    launch(&params, elevate)
}

/// 拼装最终执行的 PowerShell 脚本。
fn build_script(action: &Action, log: &Path) -> String {
    let log = ps_quote(&log.to_string_lossy());
    let name = ps_quote(&action.name);

    format!(
        r#"$ErrorActionPreference = 'Continue'
$log = {log}
$name = {name}
"[{{0}}] ===== {{1}} =====" -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss'), $name | Out-File -FilePath $log -Append -Encoding utf8
$global:LASTEXITCODE = $null
$global:Error.Clear()
& {{
{command}
}} *>&1 | Out-File -FilePath $log -Append -Encoding utf8
$code = $LASTEXITCODE
if ($null -eq $code) {{ if ($global:Error.Count -gt 0) {{ $code = 1 }} else {{ $code = 0 }} }}
"[{{0}}] ----- 结束，退出码 {{1}} -----" -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss'), $code | Out-File -FilePath $log -Append -Encoding utf8
"#,
        log = log,
        name = name,
        command = action.command,
    )
}

/// PowerShell 单引号字符串转义。
fn ps_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// UTF-16LE → Base64。
fn encode_utf16le_base64(text: &str) -> String {
    let bytes: Vec<u8> = text
        .encode_utf16()
        .flat_map(|unit| unit.to_le_bytes())
        .collect();
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// 启动 PowerShell。`runas` 动词会触发 UAC 提权提示。
fn launch(params: &str, elevate: bool) -> Result<(), String> {
    let verb = if elevate { "runas" } else { "open" };
    let show = if elevate { SW_HIDE } else { SW_SHOWNORMAL };

    let result = unsafe {
        ShellExecuteW(
            None,
            &HSTRING::from(verb),
            &HSTRING::from(powershell_exe()),
            &HSTRING::from(params),
            windows::core::PCWSTR::null(),
            show,
        )
    };

    // ShellExecuteW 返回值 > 32 表示成功；1223 表示用户取消了 UAC。
    let code = result.0 as isize;
    if code > 32 {
        Ok(())
    } else if code == 1223 {
        Err("已取消管理员权限请求".to_owned())
    } else {
        Err(format!("启动 PowerShell 失败（错误码 {code}）"))
    }
}

/// 优先使用 PowerShell 7，找不到时回退到系统自带的 Windows PowerShell。
fn powershell_exe() -> &'static str {
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            if dir.join("pwsh.exe").is_file() {
                return "pwsh.exe";
            }
        }
    }
    "powershell.exe"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Action;

    #[test]
    fn base64_uses_utf16le() {
        // "Hi" → 48 00 69 00
        assert_eq!(encode_utf16le_base64("Hi"), "SABpAA==");
    }

    #[test]
    fn script_contains_command_and_log_path() {
        let action = Action::new("测试", "说明", "Write-Host 'hi'");
        let script = build_script(&action, &PathBuf::from(r"C:\tmp\run.log"));
        assert!(script.contains("Write-Host 'hi'"));
        assert!(script.contains(r"C:\tmp\run.log"));
        assert!(script.contains("测试"));
    }

    #[test]
    fn single_quotes_are_escaped() {
        assert_eq!(ps_quote("a'b"), "'a''b'");
    }
}
