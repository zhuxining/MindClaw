// CLI 独立二进制入口（不启动 Tauri）

use clap::{Parser, Subcommand};

/// MindClaw - AI 驱动的个人知识管理与助手工具
#[derive(Parser)]
#[command(name = "mindclaw")]
#[command(about = "MindClaw CLI - 与 Agent 对话")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 与 Agent 交互式对话
    Agent {
        /// 指定已有会话 ID 继续对话
        #[arg(short, long, value_name = "SESSION_ID")]
        session: Option<String>,
        /// 指定 Provider（claude/deepseek/openai）
        #[arg(short, long, value_name = "PROVIDER")]
        provider: Option<String>,
        /// 指定模型 ID
        #[arg(short = 'o', long, value_name = "MODEL")]
        model: Option<String>,
        /// 发送单条消息后立即退出
        #[arg(short, long, value_name = "MESSAGE")]
        message: Option<String>,
    },
    /// 查看系统状态
    Status,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Agent {
            session,
            provider,
            model,
            message,
        } => {
            let args = mindclaw_lib::cli::agent::AgentArgs {
                session,
                provider,
                model,
                message,
            };
            if let Err(e) = mindclaw_lib::cli::agent::execute(args).await {
                eprintln!("❌ 错误: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Status => {
            println!("🤖 MindClaw Agent 状态");
            println!("运行状态: 未连接 Tauri 运行时");
            println!("使用 'mindclaw agent' 启动 Agent 对话");
        }
    }
}
