use crate::models::conversation::ConversationMode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Professional,
    Student,
    Researcher,
    Creator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelTierPreference {
    /// 优先省钱（Haiku）
    Economy,
    /// 优先质量（Sonnet）
    Quality,
    /// 自动按任务选择
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPreference {
    /// 提供商名称（如 "openai", "deepseek", "claude"）
    pub provider: String,
    /// 具体模型 ID，None 则使用提供商的默认模型
    pub model_id: Option<String>,
    pub model_tier: ModelTierPreference,
    pub max_tokens_per_turn: u32,
    pub enable_memory: bool,
    pub enable_tools: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub vault_path: String,
    pub user_role: Option<UserRole>,
    pub agent: AgentPreference,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DirectoryViewMode {
    Tree,
    #[default]
    Flat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedDirTab {
    pub id: String,
    #[serde(rename = "dirPath")]
    pub dir_path: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedNote {
    pub path: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacePanelSizes {
    pub left: f32,
    pub center: f32,
    pub right: f32,
}

impl Default for WorkspacePanelSizes {
    /// NOTE: These defaults are duplicated in TypeScript (src/stores/workspace.ts).
    /// When changing values, update BOTH locations to maintain consistency.
    fn default() -> Self {
        Self {
            left: 22.0,
            center: 52.0,
            right: 26.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceRightPanelHeights {
    pub pin: f32,
    pub tasks: f32,
    #[serde(rename = "relatedContent", alias = "relevance")]
    pub related_content: f32,
}

impl Default for WorkspaceRightPanelHeights {
    /// NOTE: These defaults are duplicated in TypeScript (src/stores/workspace.ts).
    /// When changing values, update BOTH locations to maintain consistency.
    fn default() -> Self {
        Self {
            pin: 20.0,
            tasks: 50.0,
            related_content: 30.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum WorkspaceOpenedItem {
    Daily {
        date: String,
        path: String,
    },
    Note {
        path: String,
        title: String,
    },
    SourceWeb {
        path: String,
        title: String,
        url: String,
    },
    SourcePdf {
        path: String,
        title: String,
    },
    SourceImage {
        path: String,
        title: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspacePrefs {
    pub active_tab_id: String,
    pub pinned_dir_tabs: Vec<PinnedDirTab>,
    pub dir_view_mode: HashMap<String, DirectoryViewMode>,
    pub panel_sizes: WorkspacePanelSizes,
    pub right_panel_heights: WorkspaceRightPanelHeights,
    pub last_opened_item: Option<WorkspaceOpenedItem>,
    pub pinned_note: Option<PinnedNote>,
    pub chat_mode: ConversationMode,
}

impl Default for WorkspacePrefs {
    fn default() -> Self {
        Self {
            active_tab_id: "daily".to_string(),
            pinned_dir_tabs: Vec::new(),
            dir_view_mode: HashMap::new(),
            panel_sizes: WorkspacePanelSizes::default(),
            right_panel_heights: WorkspaceRightPanelHeights::default(),
            last_opened_item: Some(WorkspaceOpenedItem::Daily {
                date: chrono::Local::now().date_naive().to_string(),
                path: format!("daily/{}.md", chrono::Local::now().date_naive()),
            }),
            pinned_note: None,
            chat_mode: ConversationMode::Companion,
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            vault_path: "~/MindClaw/vault".to_string(),
            user_role: None,
            agent: AgentPreference {
                provider: "openai".to_string(),
                model_id: None,
                model_tier: ModelTierPreference::Auto,
                max_tokens_per_turn: 8192,
                enable_memory: true,
                enable_tools: true,
            },
            language: "zh-CN".to_string(),
        }
    }
}
