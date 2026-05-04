use serde::{Deserialize, Serialize};

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

/// Content type for tabs in Content Host
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContentType {
    DailyNote,
    Markdown,
    Web,
    Pdf,
    Image,
    AgentSession,
    AgentDetail,
    SkillDetail,
    MemoryDetail,
    McpDetail,
    SessionDetail,
    CronDetail,
    Checklist,
    Graph,
    Settings,
}

/// Descriptor for content that can be opened in a tab
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentDescriptor {
    #[serde(rename = "type")]
    pub content_type: ContentType,
    pub path: String,
    pub title: String,
    #[serde(default)]
    pub meta: Option<serde_json::Value>,
}

/// An open tab in Content Host
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenTab {
    pub id: String,
    pub descriptor: ContentDescriptor,
    #[serde(default)]
    pub dirty: bool,
}

/// Workspace ID for Ribbon workspaces
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceId {
    Daily,
    Inbox,
    Private,
    Vault,
    Agent,
    Skills,
    Memory,
    Mcp,
    Session,
    Cron,
    Checklist,
    Graph,
    Tasks,
    Settings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspacePrefs {
    pub active_workspace_id: WorkspaceId,
    pub open_tabs: Vec<OpenTab>,
    pub active_tab_id: Option<String>,
    pub panel_sizes: WorkspacePanelSizes,
    pub last_opened_item: Option<WorkspaceOpenedItem>,
}

impl Default for WorkspacePrefs {
    fn default() -> Self {
        let today = chrono::Local::now().date_naive().to_string();
        Self {
            active_workspace_id: WorkspaceId::Daily,
            open_tabs: Vec::new(),
            active_tab_id: None,
            panel_sizes: WorkspacePanelSizes::default(),
            last_opened_item: Some(WorkspaceOpenedItem::Daily {
                date: today.clone(),
                path: format!("daily/{}.md", today),
            }),
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
