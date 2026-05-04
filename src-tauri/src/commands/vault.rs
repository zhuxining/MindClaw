use crate::error::{AppError, AppResult};
use crate::models::note::VaultNote;
use crate::models::settings::WorkspaceOpenedItem;
use crate::runtime::AppRuntime;
use crate::services::note::NoteIndex;
use serde::Serialize;
use std::collections::HashSet;
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

/// Memory 列表项
#[derive(Debug, Serialize)]
pub struct MemoryListItem {
    pub id: String,
    pub key: String,
    pub category: String,
    pub importance: f64,
    pub file_path: String,
    pub updated: String,
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

fn relative_path(root: &Path, entry_path: &Path) -> String {
    entry_path
        .strip_prefix(root)
        .unwrap_or(entry_path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn modified_ms(metadata: &std::fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn build_entry(root: &Path, entry: std::fs::DirEntry) -> Option<VaultDirEntry> {
    let file_name = entry.file_name().to_string_lossy().to_string();
    if file_name.starts_with('.') {
        return None;
    }

    let metadata = entry.metadata().ok()?;
    Some(VaultDirEntry {
        name: file_name,
        path: relative_path(root, &entry.path()),
        is_dir: metadata.is_dir(),
        modified_ms: modified_ms(&metadata),
    })
}

fn sort_entries(entries: &mut [VaultDirEntry]) {
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });
}

fn is_image_ext(ext: &str) -> bool {
    matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp")
}

fn parse_url_from_resource_file(content: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if let Some(url) = line.strip_prefix("URL=") {
            return Some(url.trim().to_string());
        }
        if line.starts_with("http://") || line.starts_with("https://") {
            return Some(line.to_string());
        }
    }

    for marker in ["https://", "http://"] {
        if let Some(idx) = content.find(marker) {
            let rest = &content[idx..];
            let end = rest
                .find(|c: char| c.is_whitespace() || c == '<' || c == '"' || c == '\'')
                .unwrap_or(rest.len());
            return Some(rest[..end].to_string());
        }
    }

    None
}

fn display_title(path: &str) -> String {
    let file_name = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path);

    match file_name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem.to_string(),
        _ => file_name.to_string(),
    }
}

fn note_to_vault_note(note: NoteIndex) -> VaultNote {
    VaultNote {
        id: note.id,
        title: note.title,
        topic: note.file_path.clone(),
        content: String::new(),
        wikilinks: Vec::new(),
        tags: note.tags,
        source_url: None,
        created_at: 0,
        updated_at: 0,
    }
}

fn stem(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(path)
        .to_string()
}

fn title_tokens(value: &str) -> HashSet<String> {
    value
        .split(|c: char| !c.is_alphanumeric() && !('\u{4E00}'..='\u{9FFF}').contains(&c))
        .map(str::trim)
        .filter(|token| token.len() >= 2)
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

fn score_note(
    current_tags: &HashSet<String>,
    current_tokens: &HashSet<String>,
    current_path: &str,
    candidate: &NoteIndex,
) -> i32 {
    let candidate_tags: HashSet<_> = candidate
        .tags
        .iter()
        .map(|tag| tag.to_ascii_lowercase())
        .collect();
    let shared_tags = current_tags.intersection(&candidate_tags).count() as i32;

    let candidate_tokens = title_tokens(&candidate.title);
    let shared_tokens = current_tokens.intersection(&candidate_tokens).count() as i32;

    let daily_bonus =
        if current_path.starts_with("daily/") && candidate.file_path.starts_with("daily/") {
            2
        } else {
            0
        };

    let source_bonus =
        if current_path.starts_with("source/") || candidate.file_path.starts_with("source/") {
            1
        } else {
            0
        };

    shared_tags * 10 + shared_tokens * 4 + daily_bonus + source_bonus
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
    let target = validate_path_child(&vault_root, &PathBuf::from(&path))?;

    let mut entries = std::fs::read_dir(&target)
        .map_err(|e| AppError::Internal(format!("list_dir: {e}")))?
        .flatten()
        .filter_map(|entry| build_entry(&vault_root, entry))
        .collect::<Vec<_>>();

    sort_entries(&mut entries);
    Ok(entries)
}

/// 列出 vault 子目录内容（一层，不递归）
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
    let target = validate_path_child(&vault_root, &target)?;

    let mut entries = std::fs::read_dir(&target)
        .map_err(|e| AppError::Internal(format!("list_vault_dir: {e}")))?
        .flatten()
        .filter_map(|entry| build_entry(&vault_root, entry))
        .collect::<Vec<_>>();

    sort_entries(&mut entries);
    Ok(entries)
}

/// 递归列出当前目录范围内的所有文件（用于 Flat View）
#[tauri::command]
pub async fn list_vault_files_recursive(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    path: Option<String>,
) -> AppResult<Vec<VaultDirEntry>> {
    let vault_root = runtime.config().vault_path.clone();
    let target = match path.as_deref() {
        None | Some("") => vault_root.clone(),
        Some(p) => vault_root.join(p),
    };
    let target = validate_path_child(&vault_root, &target)?;

    let mut files = Vec::new();
    let mut stack = vec![target];

    while let Some(dir) = stack.pop() {
        let read_dir = match std::fs::read_dir(&dir) {
            Ok(read_dir) => read_dir,
            Err(_) => continue,
        };

        for entry in read_dir.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.starts_with('.') {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };

            if metadata.is_dir() {
                stack.push(entry.path());
                continue;
            }

            files.push(VaultDirEntry {
                name: file_name,
                path: relative_path(&vault_root, &entry.path()),
                is_dir: false,
                modified_ms: modified_ms(&metadata),
            });
        }
    }

    files.sort_by(|a, b| {
        b.modified_ms
            .cmp(&a.modified_ms)
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(files)
}

/// 解析 source/ 下资源文件的展示类型
#[tauri::command]
pub async fn resolve_source_item(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    path: String,
) -> AppResult<WorkspaceOpenedItem> {
    let vault_root = runtime.config().vault_path.clone();
    let target = validate_path_child(&vault_root, &vault_root.join(&path))?;
    if target.is_dir() {
        return Err(AppError::Validation("source item must be a file".into()));
    }

    let title = display_title(&path);
    let ext = target
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "md" => Ok(WorkspaceOpenedItem::Note { path, title }),
        "pdf" => Ok(WorkspaceOpenedItem::SourcePdf { path, title }),
        "url" | "webloc" | "txt" => {
            let content = std::fs::read_to_string(&target)
                .map_err(|e| AppError::Storage(format!("read source item: {e}")))?;
            let url = parse_url_from_resource_file(&content)
                .ok_or_else(|| AppError::Validation("cannot parse url from source item".into()))?;
            Ok(WorkspaceOpenedItem::SourceWeb { path, title, url })
        }
        _ if is_image_ext(&ext) => Ok(WorkspaceOpenedItem::SourceImage { path, title }),
        _ => {
            let content = std::fs::read_to_string(&target).unwrap_or_default();
            if let Some(url) = parse_url_from_resource_file(&content) {
                Ok(WorkspaceOpenedItem::SourceWeb { path, title, url })
            } else {
                Ok(WorkspaceOpenedItem::Note { path, title })
            }
        }
    }
}

/// 搜索 Vault 笔记（按标题/标签关键词）
#[tauri::command]
pub async fn search_vault(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    query: String,
) -> AppResult<Vec<VaultNote>> {
    let results = runtime.services().note.search(&query, 20).await?;
    Ok(results.into_iter().map(note_to_vault_note).collect())
}

/// 获取与当前笔记语义上更相关的笔记
#[tauri::command]
pub async fn get_relevant_notes(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    path: String,
) -> AppResult<Vec<VaultNote>> {
    let all = runtime.services().note.search("", 10_000).await?;

    let current = all
        .iter()
        .find(|entry| entry.file_path == path)
        .cloned()
        .unwrap_or_else(|| {
            let file_stem = stem(&path);
            NoteIndex {
                id: path.clone(),
                title: file_stem,
                tags: Vec::new(),
                file_path: path.clone(),
                modified_at: String::new(),
            }
        });

    let current_tags: HashSet<_> = current
        .tags
        .iter()
        .map(|tag| tag.to_ascii_lowercase())
        .collect();
    let current_tokens = title_tokens(&current.title);

    let mut scored = all
        .into_iter()
        .filter(|entry| entry.file_path != path)
        .filter_map(|entry| {
            let score = score_note(&current_tags, &current_tokens, &current.file_path, &entry);
            if score <= 0 {
                return None;
            }
            Some((score, entry))
        })
        .collect::<Vec<_>>();

    scored.sort_by(|(left_score, left_entry), (right_score, right_entry)| {
        right_score
            .cmp(left_score)
            .then_with(|| right_entry.modified_at.cmp(&left_entry.modified_at))
    });

    Ok(scored
        .into_iter()
        .take(5)
        .map(|(_, entry)| note_to_vault_note(entry))
        .collect())
}

/// 获取 Vault 笔记条目（含正文）
#[tauri::command]
pub async fn get_vault_note(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    id: String,
) -> AppResult<VaultNote> {
    let all = runtime.services().note.search("", 10_000).await?;
    let entry = all
        .into_iter()
        .find(|note| note.id == id)
        .ok_or_else(|| AppError::NotFound(format!("vault note not found: {id}")))?;

    let content = runtime.services().note.read(&entry.file_path).await?;
    Ok(VaultNote {
        id: entry.id,
        title: entry.title,
        topic: entry.file_path,
        content,
        wikilinks: Vec::new(),
        tags: entry.tags,
        source_url: None,
        created_at: 0,
        updated_at: 0,
    })
}

/// 提取所有唯一标签（用于 TagsFilterPane）
#[tauri::command]
pub async fn list_all_tags(runtime: tauri::State<'_, Arc<AppRuntime>>) -> AppResult<Vec<String>> {
    runtime.services().note.list_all_tags().await
}

/// 按过滤条件查询笔记
#[tauri::command]
pub async fn list_notes_by_filter(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    tags: Option<Vec<String>>,
    date_from: Option<String>,
    date_to: Option<String>,
    limit: Option<usize>,
) -> AppResult<Vec<VaultNote>> {
    let limit = limit.unwrap_or(100);
    let results = runtime
        .services()
        .note
        .filter_notes(
            tags.as_deref(),
            date_from.as_deref(),
            date_to.as_deref(),
            limit,
        )
        .await?;
    Ok(results.into_iter().map(note_to_vault_note).collect())
}

/// 列出所有 Memory
#[tauri::command]
pub async fn list_memories(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    category: Option<String>,
    limit: Option<usize>,
) -> AppResult<Vec<MemoryListItem>> {
    let limit = limit.unwrap_or(100);
    let vault_db = runtime.vault_db();

    let conn = vault_db.lock().await;

    let items = if let Some(cat) = category {
        let mut stmt = conn.prepare(
            "SELECT id, key, category, importance, file_path, updated FROM memories_index WHERE category = ?1 ORDER BY importance DESC LIMIT ?2"
        ).map_err(|e| AppError::Storage(format!("prepare list memories: {e}")))?;

        let rows = stmt
            .query_map(rusqlite::params![cat, limit as i64], |row| {
                Ok(MemoryListItem {
                    id: row.get::<_, String>(0)?,
                    key: row.get::<_, String>(1)?,
                    category: row.get::<_, String>(2)?,
                    importance: row.get::<_, f64>(3)?,
                    file_path: row.get::<_, String>(4)?,
                    updated: row.get::<_, String>(5)?,
                })
            })
            .map_err(|e| AppError::Storage(format!("query memories: {e}")))?;

        rows.filter_map(|r| r.ok()).collect()
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, key, category, importance, file_path, updated FROM memories_index ORDER BY importance DESC LIMIT ?1"
        ).map_err(|e| AppError::Storage(format!("prepare list memories: {e}")))?;

        let rows = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                Ok(MemoryListItem {
                    id: row.get::<_, String>(0)?,
                    key: row.get::<_, String>(1)?,
                    category: row.get::<_, String>(2)?,
                    importance: row.get::<_, f64>(3)?,
                    file_path: row.get::<_, String>(4)?,
                    updated: row.get::<_, String>(5)?,
                })
            })
            .map_err(|e| AppError::Storage(format!("query memories: {e}")))?;

        rows.filter_map(|r| r.ok()).collect()
    };

    Ok(items)
}
