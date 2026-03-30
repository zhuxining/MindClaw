// CliRuntime：最小运行时（DB + Services，无 UI）

use crate::agent::commands::AgentCommandRegistry;
use crate::agent::observer::TracingObserver;
use crate::agent::session::SessionManager;
use crate::agent::tools::{filesystem::FilesystemTool, shell::ShellTool, ToolRegistry};
use crate::agent::{
    AgentLoop, ContextPipeline, ConversationHistorySource, SystemPromptSource, UserMessageSource,
};
use crate::bus::events::{InboundMessage, OutboundPayload};
use crate::bus::MessageBus;
use crate::error::{AppError, AppResult};
use crate::models::conversation::ConversationMode;
use crate::providers::config::builtin_configs;
use crate::providers::openai_compat::OpenAICompatProvider;
use crate::providers::traits::Provider;
use crate::storage::database;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{self, AsyncWriteExt};
use tokio::sync::Mutex;

/// CLI 运行时：封装所有 Agent 依赖组件
pub struct CliRuntime {
    /// 消息总线
    pub bus: Arc<MessageBus>,
    /// Agent 循环
    pub agent: Arc<AgentLoop>,
    /// 会话管理器
    pub session_mgr: Arc<SessionManager>,
    /// 当前会话 ID（共享状态，用于接收出站消息时更新）
    current_session_id: Arc<Mutex<Option<String>>>,
}

impl CliRuntime {
    /// 创建并初始化 CLI 运行时（带 Agent）
    pub async fn new_with_agent(provider_id: Option<String>) -> AppResult<Self> {
        // 1. 初始化 tracing 日志
        init_tracing();

        // 2. 初始化数据库（使用应用数据目录）
        let data_dir = get_data_dir()?;
        let db_path = data_dir.join("mindclaw.db");
        tracing::info!(db_path = %db_path.display(), "initializing_database");
        let db = database::open(&db_path)?;

        // 3. 创建 SessionManager
        let session_mgr = Arc::new(SessionManager::new(db.clone()));

        // 4. 创建 MessageBus
        let bus = Arc::new(MessageBus::new(100));

        // 5. 初始化 Provider（支持指定 provider_id）
        let provider = init_provider_with_id(provider_id)?;

        // 6. 初始化 ToolRegistry
        let tools = init_tools(&data_dir)?;

        // 7. 创建 AgentCommandRegistry
        let agent_commands = Arc::new(AgentCommandRegistry::with_builtins());

        // 8. 创建 ContextPipeline 并添加上下文源
        let mut context_pipeline = ContextPipeline::new(128_000);
        context_pipeline.add_source(Arc::new(SystemPromptSource::new(
            "你是一个智能助手，可以帮助用户完成各种任务。",
        )));
        context_pipeline.add_source(Arc::new(ConversationHistorySource::new(5)));
        context_pipeline.add_source(Arc::new(UserMessageSource));
        let context_pipeline = Arc::new(context_pipeline);

        // 9. 创建 Observer
        let observer: Arc<dyn crate::agent::observer::AgentObserver> =
            Arc::new(TracingObserver::new());

        // 10. 创建 AgentLoop
        let agent = Arc::new(AgentLoop::new(
            bus.clone(),
            session_mgr.clone(),
            context_pipeline,
            provider,
            tools,
            agent_commands,
            observer,
        ));

        // 11. 启动 AgentLoop（在后台任务中运行）
        let agent_clone = agent.clone();
        tokio::spawn(async move {
            if let Err(e) = AgentLoop::run(agent_clone).await {
                tracing::error!(error = %e, "agent_loop_failed");
            }
        });

        // 创建共享状态
        let current_session_id = Arc::new(Mutex::new(None));

        // 12. 启动出站消息消费任务
        let bus_clone = bus.clone();
        let session_id_clone = current_session_id.clone();
        tokio::spawn(async move {
            consume_outbound_messages(bus_clone, session_id_clone).await;
        });

        Ok(Self {
            bus,
            agent,
            session_mgr,
            current_session_id,
        })
    }

    /// 启动交互式 REPL
    pub async fn run_interactive(&self) -> AppResult<()> {
        println!("🤖 MindClaw Agent - 输入消息开始对话");
        println!("提示：输入 /new 创建新会话，Ctrl+C 或输入 'exit' 退出\n");

        let stdin = io::stdin();
        let reader = io::BufReader::new(stdin);
        let mut lines = io::AsyncBufReadExt::lines(reader);

        loop {
            // 显示提示符
            print!("\n> ");
            io::stdout().flush().await.ok();

            // 读取用户输入
            let line = match lines.next_line().await {
                Ok(Some(line)) => line,
                Ok(None) => break, // EOF
                Err(e) => {
                    eprintln!("读取输入错误: {}", e);
                    continue;
                }
            };

            let trimmed = line.trim();

            // 处理退出命令
            if trimmed == "exit" || trimmed == "quit" {
                println!("👋 再见!");
                break;
            }

            // 跳过空行
            if trimmed.is_empty() {
                continue;
            }

            // 发送消息并等待响应
            if let Err(e) = self.send_message(trimmed).await {
                eprintln!("错误: {}", e);
            }
        }

        Ok(())
    }

    /// 发送单条消息并等待完成
    pub async fn send_single_message(&self, content: &str) -> AppResult<String> {
        // 创建临时 MessageBus 用于收集输出
        let temp_bus = Arc::new(MessageBus::new(100));

        // 启动收集任务
        let bus_clone = temp_bus.clone();
        let collect_handle = tokio::spawn(async move {
            let mut rx = bus_clone.take_outbound_rx().await?;
            let mut output = String::new();

            while let Some(msg) = rx.recv().await {
                match msg.payload {
                    OutboundPayload::Chunk { content } => {
                        print!("{}", content);
                        io::stdout().flush().await.ok();
                        output.push_str(&content);
                    }
                    OutboundPayload::Done => {
                        println!();
                        break;
                    }
                    OutboundPayload::Error { message, .. } => {
                        return Err(AppError::Provider(message));
                    }
                    _ => {}
                }
            }
            Ok::<String, AppError>(output)
        });

        // 发送消息
        self.send_message(content).await?;

        // 等待收集完成
        collect_handle
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
    }

    /// 发送消息到 Agent
    async fn send_message(&self, content: &str) -> AppResult<()> {
        let request_id = uuid::Uuid::new_v4().to_string();
        let session_id = self.current_session_id.lock().await.clone();

        let message = InboundMessage {
            id: uuid::Uuid::new_v4().to_string(),
            request_id,
            session_id,
            sender: "cli_user".to_string(),
            channel: "terminal".to_string(),
            mode: ConversationMode::Chat,
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        self.bus.publish_inbound(message).await?;

        // 如果是 /new 命令，重置会话 ID
        if content.trim() == "/new" {
            *self.current_session_id.lock().await = None;
        }

        Ok(())
    }

    /// 设置当前会话 ID
    pub async fn set_session_id(&self, session_id: Option<String>) {
        *self.current_session_id.lock().await = session_id;
    }

    /// 获取当前会话 ID
    pub async fn get_session_id(&self) -> Option<String> {
        self.current_session_id.lock().await.clone()
    }
}

/// 消费出站消息（打印到终端）
async fn consume_outbound_messages(bus: Arc<MessageBus>, session_id: Arc<Mutex<Option<String>>>) {
    match bus.take_outbound_rx().await {
        Ok(mut rx) => {
            while let Some(msg) = rx.recv().await {
                // 更新会话 ID
                *session_id.lock().await = Some(msg.session_id.clone());

                match msg.payload {
                    OutboundPayload::Chunk { content } => {
                        print!("{}", content);
                        io::stdout().flush().await.ok();
                    }
                    OutboundPayload::Done => {
                        println!(); // 响应结束，换行
                                    // 可选：发送完成信号
                    }
                    OutboundPayload::Error { message, .. } => {
                        eprintln!("\n❌ 错误: {}", message);
                    }
                    OutboundPayload::Status { phase } => {
                        // 可选：显示状态更新
                        tracing::debug!(phase = ?phase, "status_update");
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "failed_to_take_outbound_rx");
        }
    }
}

/// 初始化 tracing 日志
fn init_tracing() {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

/// 获取应用数据目录
fn get_data_dir() -> AppResult<PathBuf> {
    use directories::BaseDirs;
    let base = BaseDirs::new()
        .ok_or_else(|| AppError::Internal("cannot determine base directories".to_string()))?;
    Ok(base.config_dir().join("mindclaw"))
}

/// 初始化 Provider（支持指定 provider_id，默认 deepseek）
fn init_provider_with_id(provider_id: Option<String>) -> AppResult<Arc<dyn Provider>> {
    let configs = builtin_configs();

    // 确定使用哪个 provider
    let provider_name = provider_id.as_deref().unwrap_or("deepseek");

    let config = configs
        .iter()
        .find(|c| c.name == provider_name)
        .ok_or_else(|| {
            AppError::Provider(format!(
                "Provider '{}' not found in builtin configs",
                provider_name
            ))
        })?;

    // 从环境变量读取 API Key
    let provider = OpenAICompatProvider::from_env(config, None).map_err(|e| {
        AppError::Provider(format!(
            "failed to initialize {} provider: {}",
            provider_name, e
        ))
    })?;

    tracing::info!(provider = %provider_name, model = %provider.model_id(), "provider_initialized");

    Ok(Arc::new(provider))
}

/// 初始化 ToolRegistry
fn init_tools(data_dir: &std::path::Path) -> AppResult<Arc<ToolRegistry>> {
    let mut tools = ToolRegistry::new();

    // 注册内置工具
    tools.register(Arc::new(ShellTool));
    tools.register(Arc::new(FilesystemTool::new(data_dir.join("vault"))));

    tracing::info!(tool_count = %tools.len(), "tools_initialized");

    Ok(Arc::new(tools))
}
