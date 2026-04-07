//! Markdown 文件读写工具
//!
//! 提供 YAML Frontmatter 解析/序列化，以及 vault 文件路径生成规则。
//! 所有写操作使用原子写（写临时文件后 rename），防止部分写入损坏文件。

use crate::agent::memory::MemoryCategory;
use crate::error::{AppError, AppResult};
use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};

// ─── Frontmatter 解析 ─────────────────────────────────────────────────────────

/// 解析 Markdown 文件内容，返回 `(frontmatter, body)`。
///
/// 格式要求：文件以 `---` 开头，第二个 `---` 结束 Frontmatter，之后为正文。
pub fn parse_frontmatter<T: DeserializeOwned>(content: &str) -> AppResult<(T, String)> {
    let content = content.trim_start_matches('\u{feff}'); // strip BOM

    if !content.starts_with("---") {
        return Err(AppError::Storage(
            "markdown file missing frontmatter".into(),
        ));
    }

    // 找到第二个 ---
    let rest = &content[3..];
    let end = rest
        .find("\n---")
        .ok_or_else(|| AppError::Storage("markdown frontmatter not closed".into()))?;

    let yaml_str = &rest[..end];
    let body_start = end + 4; // skip "\n---"
                              // 跳过 \n---\n 之后的换行
    let body = rest
        .get(body_start..)
        .unwrap_or("")
        .trim_start_matches('\n')
        .to_string();

    let frontmatter: T = serde_yaml::from_str(yaml_str)
        .map_err(|e| AppError::Storage(format!("parse frontmatter yaml: {e}")))?;

    Ok((frontmatter, body))
}

/// 将 Frontmatter 和正文序列化为 Markdown 文件内容。
pub fn render_markdown<T: Serialize>(frontmatter: &T, body: &str) -> AppResult<String> {
    let yaml = serde_yaml::to_string(frontmatter)
        .map_err(|e| AppError::Storage(format!("serialize frontmatter: {e}")))?;
    // serde_yaml 输出不含 "---\n" 前缀（v0.9）
    let content = if body.is_empty() {
        format!("---\n{yaml}---\n")
    } else {
        format!("---\n{yaml}---\n\n{body}")
    };
    Ok(content)
}

// ─── 异步文件读写 ──────────────────────────────────────────────────────────────

/// 从文件读取并解析 Frontmatter，返回 `(frontmatter, body)`。
pub async fn read_file<T: DeserializeOwned>(path: &Path) -> AppResult<(T, String)> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| AppError::Storage(format!("read {}: {e}", path.display())))?;
    parse_frontmatter(&content)
}

/// 原子写：将 Frontmatter 和正文写入文件（写临时文件后 rename）。
pub async fn write_file<T: Serialize>(path: &Path, frontmatter: &T, body: &str) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| AppError::Storage(format!("create dir {}: {e}", parent.display())))?;
    }
    let content = render_markdown(frontmatter, body)?;
    let tmp = path.with_extension("md.tmp");
    tokio::fs::write(&tmp, &content)
        .await
        .map_err(|e| AppError::Storage(format!("write tmp {}: {e}", tmp.display())))?;
    tokio::fs::rename(&tmp, path)
        .await
        .map_err(|e| AppError::Storage(format!("rename to {}: {e}", path.display())))
}

// ─── 路径生成规则 ──────────────────────────────────────────────────────────────

/// 生成 slug：保留中文和字母数字，其余替换为连字符，压缩连续连字符，去首尾。
pub fn title_to_slug(title: &str) -> String {
    let mut slug = String::new();
    let mut last_was_hyphen = false;
    for ch in title.chars() {
        if ch.is_alphanumeric() || is_cjk(ch) {
            slug.push(ch);
            last_was_hyphen = false;
        } else if !last_was_hyphen && !slug.is_empty() {
            slug.push('-');
            last_was_hyphen = true;
        }
    }
    let slug = slug.trim_end_matches('-').to_string();
    if slug.is_empty() {
        "untitled".to_string()
    } else {
        slug
    }
}

fn is_cjk(ch: char) -> bool {
    matches!(ch,
        '\u{4E00}'..='\u{9FFF}'
        | '\u{3400}'..='\u{4DBF}'
        | '\u{F900}'..='\u{FAFF}'
        | '\u{3040}'..='\u{30FF}'
        | '\u{AC00}'..='\u{D7AF}'
    )
}

/// 生成 Task 文件路径：`{tasks_dir}/{YYYY-MM-DD}-{slug}.md`
///
/// `created` 格式应为 ISO8601（取前 10 个字符作为 YYYY-MM-DD）。
/// 若文件已存在，在 slug 后追加 id 短后缀避免冲突。
pub fn task_file_path(tasks_dir: &Path, title: &str, created: &str, id: &str) -> PathBuf {
    let date = &created[..created.len().min(10)];
    let slug = title_to_slug(title);
    let candidate = tasks_dir.join(format!("{date}-{slug}.md"));
    if candidate.exists() {
        let short_id = &id[..id.len().min(8)];
        tasks_dir.join(format!("{date}-{slug}-{short_id}.md"))
    } else {
        candidate
    }
}

/// 生成 Memory 文件路径：`{memory_dir}/{category_kebab}-{key}.md`
pub fn memory_file_path(memory_dir: &Path, category: &MemoryCategory, key: &str) -> PathBuf {
    let cat = category_to_kebab(category);
    let key_slug = title_to_slug(key);
    memory_dir.join(format!("{cat}-{key_slug}.md"))
}

fn category_to_kebab(category: &MemoryCategory) -> &'static str {
    match category {
        MemoryCategory::UserFact => "user-fact",
        MemoryCategory::Preference => "preference",
        MemoryCategory::WorkContext => "work-context",
        MemoryCategory::Relationship => "relationship",
        MemoryCategory::Goal => "goal",
    }
}

// ─── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestFm {
        id: String,
        title: String,
    }

    #[test]
    fn test_round_trip() {
        let fm = TestFm {
            id: "abc".to_string(),
            title: "Hello World".to_string(),
        };
        let content = render_markdown(&fm, "正文内容").unwrap();
        let (parsed, body): (TestFm, String) = parse_frontmatter(&content).unwrap();
        assert_eq!(parsed, fm);
        assert_eq!(body, "正文内容");
    }

    #[test]
    fn test_slug() {
        assert_eq!(title_to_slug("写季度报告"), "写季度报告");
        assert_eq!(title_to_slug("Hello World!"), "Hello-World");
        assert_eq!(title_to_slug(""), "untitled");
    }
}
