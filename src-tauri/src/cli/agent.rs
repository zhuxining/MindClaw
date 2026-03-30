// mindclaw agent（需 API Key）

use crate::cli::runtime::CliRuntime;
use crate::error::AppResult;

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
    // 初始化运行时（带 Agent）
    let runtime = CliRuntime::new_with_agent(args.provider).await?;

    // 如果指定了会话 ID，设置它
    if let Some(session_id) = args.session {
        runtime.set_session_id(Some(session_id)).await;
        println!(
            "📝 继续会话: {}\n",
            runtime.get_session_id().await.unwrap_or_default()
        );
    }

    // 根据是否有消息参数决定模式
    match args.message {
        Some(message) => {
            // 单消息模式
            let _output = runtime.send_single_message(&message).await?;
            // 显示会话 ID（方便后续继续）
            if let Some(session_id) = runtime.get_session_id().await {
                eprintln!("\n💾 会话 ID: {}", session_id);
            }
        }
        None => {
            // 交互式 REPL 模式
            if let Some(session_id) = runtime.get_session_id().await {
                println!("📝 当前会话: {}\n", session_id);
            }
            runtime.run_interactive().await?;
        }
    }

    Ok(())
}
