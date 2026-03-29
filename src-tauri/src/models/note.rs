use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyNote {
    pub date: String, // YYYY-MM-DD
    pub content: String,
    pub tasks: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub id: String,
    pub title: String,
    pub topic: String,
    pub content: String,
    pub wikilinks: Vec<String>,
    pub tags: Vec<String>,
    pub source_url: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
