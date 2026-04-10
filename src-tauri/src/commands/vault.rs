use crate::error::{AppError, AppResult};
use crate::runtime::AppRuntime;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 目录条目（返回给前端，与 config::VaultEntry 不同）
#[derive(Debug, Serialize)]
pub struct VaultDirEntry {
    pub name: String,
    /// 相对于 vault 根目录的路径
    pub path: String,
    pub is_dir: bool,
    /// Unix 毫秒时间戳
    pub modified_ms: i64,
}

/// 规范化并验证路径未逃逸 root
fn validate_path_child(root: &Path, candidate: &Path) -> AppResult<PathBuf> {
    let canonical_root = root
        .canonicalize()
        .map_err(|e| AppError::Internal(format!("cannot resolve root: {e}")))?;
    let canonical_candidate = candidate
        .canonicalize()
        .map_err(|e| AppError::Internal(format!("invalid path: {e}")))?;
    if !canonical_candidate.starts_with(&canonical_root) {
        return Err(AppError::Internal("path escapes root directory".into()));
    }
    Ok(canonical_candidate)
}

/// 列出指定目录的内容（一层，不递归）
///
/// 安全限制：路径必须是 vault 路径或其子目录。
#[tauri::command]
pub async fn list_dir(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    path: String,
) -> AppResult<Vec<VaultDirEntry>> {
    let vault_root = runtime.config().vault_path.clone();
    let target = PathBuf::from(&path);
    let target = validate_path_child(&vault_root, &target)?;

    let mut entries = Vec::new();
    let read_dir =
        std::fs::read_dir(&target).map_err(|e| AppError::Internal(format!("list_dir: {e}")))?;

    for entry in read_dir.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.starts_with('.') {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let modified_ms = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let entry_path = entry.path();
        let relative = entry_path
            .strip_prefix(&vault_root)
            .unwrap_or(&entry_path)
            .to_string_lossy()
            .to_string();

        entries.push(VaultDirEntry {
            name: file_name,
            path: relative,
            is_dir: metadata.is_dir(),
            modified_ms,
        });
    }

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok(entries)
}

/// 列出 vault 子目录内容（一层，不递归）
///
/// - `path`: 相对 vault 根目录的路径，空字符串或 None 表示 vault 根目录
/// - 自动过滤以 `.` 开头的隐藏文件
/// - 返回结果：文件夹优先，同类按名称字母序排列
#[tauri::command]
pub async fn list_vault_dir(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    path: Option<String>,
) -> AppResult<Vec<VaultDirEntry>> {
    let vault_root = runtime.config().vault_path.clone();
    let target = match path.as_deref() {
        None | Some("") => vault_root.clone(),
        Some(p) => vault_root.join(p),
    };

    // 验证路径未通过 `..` 逃逸 vault 根目录
    let target = validate_path_child(&vault_root, &target)?;

    let mut entries = Vec::new();
    let read_dir = std::fs::read_dir(&target)
        .map_err(|e| AppError::Internal(format!("list_vault_dir: {e}")))?;

    for entry in read_dir {
        let entry = entry.map_err(|e| AppError::Internal(e.to_string()))?;
        let file_name = entry.file_name().to_string_lossy().to_string();

        if file_name.starts_with('.') {
            continue;
        }

        let metadata = entry
            .metadata()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let modified_ms = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let relative = entry
            .path()
            .strip_prefix(&vault_root)
            .unwrap_or(&entry.path())
            .to_string_lossy()
            .to_string();

        entries.push(VaultDirEntry {
            name: file_name,
            path: relative,
            is_dir: metadata.is_dir(),
            modified_ms,
        });
    }

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok(entries)
}
