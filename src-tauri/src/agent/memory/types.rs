use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCategory {
    UserFact,
    Preference,
    WorkContext,
    Relationship,
    Goal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub key: String,
    pub category: MemoryCategory,
    pub content: String,
    pub importance: f32,
    pub embedding: Option<Vec<f32>>,
    pub created_at: i64,
    pub updated_at: i64,
}
