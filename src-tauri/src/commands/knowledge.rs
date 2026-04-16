use crate::error::{AppError, AppResult};
use crate::models::note::KnowledgeEntry;
use crate::runtime::AppRuntime;
use crate::services::note::NoteIndex;
use std::collections::HashSet;
use std::sync::Arc;

fn note_to_knowledge(n: NoteIndex) -> KnowledgeEntry {
    KnowledgeEntry {
        id: n.id,
        title: n.title,
        topic: n.file_path.clone(),
        content: String::new(),
        wikilinks: Vec::new(),
        tags: n.tags,
        source_url: None,
        created_at: 0,
        updated_at: 0,
    }
}

fn stem(path: &str) -> String {
    std::path::Path::new(path)
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

/// 搜索知识库（按标题/标签关键词）
#[tauri::command]
pub async fn search_knowledge(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    query: String,
) -> AppResult<Vec<KnowledgeEntry>> {
    let results = runtime.services().note.search(&query, 20).await?;
    Ok(results.into_iter().map(note_to_knowledge).collect())
}

/// 获取与当前笔记语义上更相关的笔记
#[tauri::command]
pub async fn get_relevant_notes(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    path: String,
) -> AppResult<Vec<KnowledgeEntry>> {
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

    // Pre-compute current note's tokens to avoid O(n²) HashSet creation
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
        .map(|(_, entry)| note_to_knowledge(entry))
        .collect())
}

/// 获取知识条目（含正文）
#[tauri::command]
pub async fn get_knowledge(
    runtime: tauri::State<'_, Arc<AppRuntime>>,
    id: String,
) -> AppResult<KnowledgeEntry> {
    let all = runtime.services().note.search("", 10_000).await?;
    let entry = all
        .into_iter()
        .find(|n| n.id == id)
        .ok_or_else(|| AppError::NotFound(format!("knowledge entry not found: {id}")))?;

    let content = runtime.services().note.read(&entry.file_path).await?;
    Ok(KnowledgeEntry {
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
