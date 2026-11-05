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

/// 应用配置。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    /// 是否以管理员身份执行指令。
    #[serde(default = "default_true")]
    pub elevate: bool,
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
