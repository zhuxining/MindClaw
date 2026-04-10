use crate::error::{AppError, AppResult};
use crate::runtime::AppRuntime;
use std::sync::Arc;

/// 获取指定日期的日记（不存在时返回空字符串）
#[tauri::command]
pub async fn get_daily(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    date: String,
) -> AppResult<String> {
    let path = runtime
        .config()
        .vault_path
        .join("daily")
        .join(format!("{date}.md"));

    if !path.exists() {
        return Ok(String::new());
    }

    std::fs::read_to_string(&path).map_err(|e| AppError::Storage(format!("read daily {date}: {e}")))
}

/// 保存日记（自动创建 daily/ 目录）
#[tauri::command]
pub async fn save_daily(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    date: String,
    content: String,
) -> AppResult<()> {
    let dir = runtime.config().vault_path.join("daily");
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::Storage(format!("create daily dir: {e}")))?;

    std::fs::write(dir.join(format!("{date}.md")), content)
        .map_err(|e| AppError::Storage(format!("write daily {date}: {e}")))
}

/// 读取 vault 内任意文件（相对路径）
#[tauri::command]
pub async fn read_note(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    path: String,
) -> AppResult<String> {
    let vault_root = &runtime.config().vault_path;
    let full_path = vault_root.join(&path);

    // 防止路径逃逸
    let canonical_root = vault_root
        .canonicalize()
        .map_err(|e| AppError::Internal(format!("resolve vault root: {e}")))?;
    let canonical_path = full_path
        .canonicalize()
        .map_err(|e| AppError::NotFound(format!("file not found: {path}: {e}")))?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(AppError::Internal("path escapes vault".into()));
    }

    if !canonical_path.exists() {
        return Ok(String::new());
    }

    std::fs::read_to_string(&canonical_path)
        .map_err(|e| AppError::Storage(format!("read note {path}: {e}")))
}

/// 保存 vault 内任意文件（相对路径，自动创建父目录）
#[tauri::command]
pub async fn save_note(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    path: String,
    content: String,
) -> AppResult<()> {
    let vault_root = &runtime.config().vault_path;
    let full_path = vault_root.join(&path);

    // 防止路径逃逸（通过父目录 canonicalize）
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::Storage(format!("create parent dir: {e}")))?;
        let canonical_root = vault_root
            .canonicalize()
            .map_err(|e| AppError::Internal(format!("resolve vault root: {e}")))?;
        let canonical_parent = parent
            .canonicalize()
            .map_err(|e| AppError::Internal(format!("resolve parent: {e}")))?;
        if !canonical_parent.starts_with(&canonical_root) {
            return Err(AppError::Internal("path escapes vault".into()));
        }
    }

    std::fs::write(&full_path, content)
        .map_err(|e| AppError::Storage(format!("write note {path}: {e}")))
}
