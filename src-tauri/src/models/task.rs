use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low,
    #[default]
    Medium,
    High,
}

/// Frontmatter 部分（用于 serde_yaml 读写，不含 file_path 和 body）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFrontmatter {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    /// YYYY-MM-DD，可选
    pub due_date: Option<String>,
    pub tags: Vec<String>,
    /// ISO8601 时间戳
    pub created: String,
    /// ISO8601 时间戳
    pub updated: String,
}

/// 完整的 Task（含文件正文和运行时路径）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    /// YYYY-MM-DD，可选
    pub due_date: Option<String>,
    pub tags: Vec<String>,
    /// ISO8601 时间戳
    pub created: String,
    /// ISO8601 时间戳
    pub updated: String,
    /// Frontmatter 之后的正文（可选）
    pub body: Option<String>,
    /// vault 内相对路径（运行时填充，不写入 Frontmatter）
    #[serde(skip)]
    pub file_path: PathBuf,
}

impl Task {
    pub fn to_frontmatter(&self) -> TaskFrontmatter {
        TaskFrontmatter {
            id: self.id.clone(),
            title: self.title.clone(),
            status: self.status.clone(),
            priority: self.priority.clone(),
            due_date: self.due_date.clone(),
            tags: self.tags.clone(),
            created: self.created.clone(),
            updated: self.updated.clone(),
        }
    }
}
