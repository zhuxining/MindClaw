// mindclaw agent（需 API Key）

use crate::bus::events::OutboundPayload;
use crate::bus::MessageBus;
use crate::error::AppResult;
use crate::runtime::AppRuntime;
use std::sync::Arc;
use tokio::io::{self, AsyncWriteExt};
use tokio::sync::Mutex;

/// Agent 子命令参数
#[derive(Debug, clap::Args)]
pub struct AgentArgs {
    /// 指定已有会话 ID 继续对话
    #[arg(short, long, value_name = "SESSION_ID")]
    pub session: Option<String>,
    /// 指定 Provider（claude/deepseek/openai）
    #[arg(short, long, value_name = "PROVIDER")]
    pub provider: Option<String>,
    /// 指定模型 ID
    #[arg(short = 'o', long, value_name = "MODEL")]
    pub model: Option<String>,
    /// 发送单条消息后立即退出
    #[arg(short, long, value_name = "MESSAGE")]
    pub message: Option<String>,
}

/// 执行 agent 命令
pub async fn execute(args: AgentArgs) -> AppResult<()> {
    init_tracing();

    // 构建并启动 AppRuntime
    let mut builder = AppRuntime::builder();
    if let Some(ref id) = args.provider {
        builder = builder.provider(id.clone());
    }
    if let Some(ref id) = args.model {
        builder = builder.model(id.clone());
    }
    let runtime = Arc::new(builder.build().await?);
    runtime.start().await?;

    // 会话 ID 跟踪
    let current_session_id = Arc::new(Mutex::new(args.session.clone()));

    // 启动终端出站消息消费
    let bus = runtime.bus().clone();
    let session_id_for_consumer = current_session_id.clone();
    tokio::spawn(async move {
        consume_outbound_messages(bus, session_id_for_consumer).await;
    });

    if let Some(ref session_id) = args.session {
        println!("📝 继续会话: {}\n", session_id);
    }

    match args.message {
        Some(message) => {
            // 单消息模式
            let session_id = current_session_id.lock().await.clone();
            runtime
                .send_message(&message, "cli_user", "terminal", session_id)
                .await?;

            // 等待响应完成（简单方式：等 outbound Done）
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            // outbound consumer 会自动打印

            if let Some(session_id) = current_session_id.lock().await.as_ref() {
                eprintln!("\n💾 会话 ID: {}", session_id);
            }
        }
        None => {
            // 交互式 REPL
            if let Some(session_id) = current_session_id.lock().await.as_ref() {
                println!("📝 当前会话: {}\n", session_id);
            }
            run_interactive(&runtime, &current_session_id).await?;
        }
    }

    runtime.shutdown().await;
    Ok(())
}

/// 交互式 REPL
async fn run_interactive(
    runtime: &Arc<AppRuntime>,
    current_session_id: &Arc<Mutex<Option<String>>>,
) -> AppResult<()> {
    println!("🤖 MindClaw Agent - 输入消息开始对话");
    println!("提示：输入 /new 创建新会话，Ctrl+C 或输入 'exit' 退出\n");

    let stdin = io::stdin();
    let reader = io::BufReader::new(stdin);
    let mut lines = io::AsyncBufReadExt::lines(reader);

    loop {
        print!("\n> ");
        io::stdout().flush().await.ok();

        let line = match lines.next_line().await {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(e) => {
                eprintln!("读取输入错误: {}", e);
                continue;
            }
        };

        let trimmed = line.trim();

        if trimmed == "exit" || trimmed == "quit" {
            println!("👋 再见!");
            break;
        }

        if trimmed.is_empty() {
            continue;
        }

        // /new 重置会话
        if trimmed == "/new" {
            *current_session_id.lock().await = None;
        }

        let session_id = current_session_id.lock().await.clone();
        if let Err(e) = runtime
            .send_message(trimmed, "cli_user", "terminal", session_id)
            .await
        {
            eprintln!("错误: {}", e);
        }
    }

    Ok(())
}

/// 消费出站消息并打印到终端
async fn consume_outbound_messages(bus: Arc<MessageBus>, session_id: Arc<Mutex<Option<String>>>) {
    match bus.take_outbound_rx().await {
        Ok(mut rx) => {
            while let Some(msg) = rx.recv().await {
                *session_id.lock().await = Some(msg.session_id.clone());

                match msg.payload {
                    OutboundPayload::Chunk { content } => {
                        print!("{}", content);
                        io::stdout().flush().await.ok();
                    }
                    OutboundPayload::Done => {
                        println!();
                    }
                    OutboundPayload::Error { message, .. } => {
                        eprintln!("\n❌ 错误: {}", message);
                    }
                    OutboundPayload::Status { status } => {
                        tracing::debug!(status = ?status, "status_update");
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
