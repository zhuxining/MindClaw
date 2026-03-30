use crate::error::{AppError, AppResult};
use crate::runtime::AppRuntime;
use std::sync::Arc;

/// 获取指定日期的日记
#[tauri::command]
pub async fn get_daily(
    _runtime: tauri::State<'_, Arc<AppRuntime>>,
    _date: String,
) -> AppResult<String> {
    // TODO: 委托给 runtime.services().daily
    Err(AppError::NotFound(
        "daily service not yet implemented".into(),
    ))
}

/// 保存日记
#[tauri::command]
pub async fn save_daily(
    _runtime: tauri::State<'_, Arc<AppRuntime>>,
    _date: String,
    _content: String,
) -> AppResult<()> {
    // TODO: 委托给 runtime.services().daily
    Err(AppError::NotFound(
        "daily service not yet implemented".into(),
    ))
}
