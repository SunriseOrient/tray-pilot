//! 动作配置与持久化。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// 一个「常用动作」。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Action {
    /// 稳定标识，用于编辑 / 删除时定位。
    pub id: String,
    /// 名称：显示在托盘菜单里。
    pub name: String,
    /// 说明：显示在名称下方。
    #[serde(default)]
    pub description: String,
    /// 指令：交给 PowerShell 执行的脚本内容。
    pub command: String,
}

impl Action {
    pub fn new(name: &str, description: &str, command: &str) -> Self {
        Self {
            id: new_id(),
            name: name.to_owned(),
            description: description.to_owned(),
            command: command.to_owned(),
        }
    }
}

/// 主题模式。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    /// 跟随系统。
    #[default]
    System,
    /// 浅色。
    Light,
    /// 深色。
    Dark,
}

impl ThemeMode {
    /// 供界面显示的中文名。
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "跟随系统",
            Self::Light => "浅色",
            Self::Dark => "深色",
        }
    }

    /// 三种模式，用于切换顺序。
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];
}

/// 应用配置。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    /// 是否以管理员身份执行指令。
    #[serde(default = "default_true")]
    pub elevate: bool,
    /// 是否随系统启动。
    #[serde(default)]
    pub autostart: bool,
    /// 界面主题。
    #[serde(default)]
    pub theme: ThemeMode,
    /// 动作列表。
    #[serde(default)]
    pub actions: Vec<Action>,
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            elevate: true,
            autostart: false,
            theme: ThemeMode::System,
            actions: vec![
                Action::new(
                    "刷新 DNS 缓存",
                    "清理本地 DNS 解析缓存，解决域名无法访问的问题",
                    "ipconfig /flushdns",
                ),
                Action::new(
                    "清理临时文件",
                    "删除当前用户目录下超过 7 天的临时文件",
                    "Get-ChildItem -Path $env:TEMP -Force -ErrorAction SilentlyContinue | Where-Object { $_.LastWriteTime -lt (Get-Date).AddDays(-7) } | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue",
                ),
            ],
        }
    }
}

/// 数据目录：`%APPDATA%\TrayPilot`
pub fn data_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata).join("TrayPilot");
        }
    }
    directories::ProjectDirs::from("com", "tray-pilot", "TrayPilot")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("TrayPilot"))
}

pub fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

pub fn log_dir() -> PathBuf {
    data_dir().join("logs")
}

/// 读取配置；文件缺失或损坏时回退到默认配置。
pub fn load() -> AppConfig {
    let path = config_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return AppConfig::default();
    };
    match serde_json::from_str::<AppConfig>(&text) {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("配置文件解析失败（{err}），使用默认配置：{}", path.display());
            AppConfig::default()
        }
    }
}

/// 原子写入配置：先写临时文件，再替换正式文件。
pub fn save(cfg: &AppConfig) -> Result<(), String> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败：{e}"))?;

    let path = config_path();
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(cfg).map_err(|e| format!("序列化失败：{e}"))?;
    std::fs::write(&tmp, text).map_err(|e| format!("写入临时文件失败：{e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("保存配置失败：{e}"))?;
    Ok(())
}

/// 生成一个足够唯一的动作 ID。
pub fn new_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("a{nanos:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_config_without_new_fields_still_loads() {
        // 旧版本写出的配置没有 autostart / theme，必须能正常读取并取默认值。
        let json = r#"{ "elevate": false, "actions": [] }"#;
        let cfg: AppConfig = serde_json::from_str(json).unwrap();
        assert!(!cfg.elevate);
        assert!(!cfg.autostart);
        assert_eq!(cfg.theme, ThemeMode::System);
    }

    #[test]
    fn theme_mode_round_trips_as_lowercase() {
        let cfg = AppConfig {
            theme: ThemeMode::Dark,
            ..AppConfig::default()
        };
        let text = serde_json::to_string(&cfg).unwrap();
        assert!(text.contains(r#""theme":"dark""#));
        let back: AppConfig = serde_json::from_str(&text).unwrap();
        assert_eq!(back.theme, ThemeMode::Dark);
    }
}
