//! Skills commands
//!
//! IPC commands for SkillsRegistry

use crate::agent::skills::SkillMetadata;
use crate::error::AppResult;
use crate::runtime::AppRuntime;
use std::sync::Arc;

/// 列出所有技能
#[tauri::command]
pub async fn list_skills(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
) -> AppResult<Vec<SkillMetadata>> {
    let registry = runtime.skills_registry().lock().await;
    Ok(registry.list().into_iter().cloned().collect())
}

/// 搜索技能
#[tauri::command]
pub async fn search_skills(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    query: String,
) -> AppResult<Vec<SkillMetadata>> {
    let registry = runtime.skills_registry().lock().await;
    Ok(registry.search(&query).into_iter().cloned().collect())
}

/// 激活技能（加载完整 SKILL.md）
#[tauri::command]
pub async fn activate_skill(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    name: String,
) -> AppResult<crate::agent::skills::SkillManifest> {
    let registry = runtime.skills_registry().lock().await;
    registry.activate(&name).await
}
