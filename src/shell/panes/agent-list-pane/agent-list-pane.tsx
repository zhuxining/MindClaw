import { Bot, Cpu, MemoryStick, Settings2, Wrench } from "lucide-react";
import type { AppSettings } from "@/lib/types";

interface AgentListPaneProps {
	settings: AppSettings | null;
}

export function AgentListPane({ settings }: AgentListPaneProps) {
	const agent = settings?.agent;

	return (
		<div className="flex h-full flex-col">
			<div
				className="border-b p-4"
				style={{ borderColor: "var(--flexoki-bg-2)" }}
			>
				<p className="text-sm font-semibold text-foreground">Agent 配置</p>
				<p className="text-xs text-muted-foreground">查看当前 Agent 运行配置</p>
			</div>

			<div className="min-h-0 flex-1 overflow-y-auto p-3">
				<div className="space-y-4">
					{/* Main Agent */}
					<div className="rounded-lg border border-border/50 p-4">
						<div className="flex items-center gap-2 mb-3">
							<Bot className="h-5 w-5 text-primary" />
							<p className="text-sm font-semibold">Main Agent</p>
						</div>
						<div className="space-y-2 text-sm">
							<div className="flex items-center gap-2">
								<Cpu className="h-4 w-4 text-muted-foreground" />
								<span className="text-muted-foreground">模型:</span>
								<span className="font-medium">
									{agent?.provider ?? "openai"} / {agent?.model_id ?? "默认"}
								</span>
							</div>
							<div className="flex items-center gap-2">
								<span className="text-muted-foreground">模型层级:</span>
								<span className="rounded bg-muted px-1.5 py-0.5 text-xs">
									{agent?.model_tier ?? "auto"}
								</span>
							</div>
							<div className="flex items-center gap-2">
								<span className="text-muted-foreground">每轮最大 Token:</span>
								<span>{agent?.max_tokens_per_turn ?? 8192}</span>
							</div>
							<div className="flex items-center gap-2">
								<MemoryStick className="h-4 w-4 text-muted-foreground" />
								<span className="text-muted-foreground">记忆:</span>
								<span>{agent?.enable_memory ? "启用" : "禁用"}</span>
							</div>
							<div className="flex items-center gap-2">
								<Wrench className="h-4 w-4 text-muted-foreground" />
								<span className="text-muted-foreground">工具:</span>
								<span>{agent?.enable_tools ? "启用" : "禁用"}</span>
							</div>
						</div>
					</div>

					{/* SubAgent */}
					<div className="rounded-lg border border-border/50 p-4">
						<div className="flex items-center gap-2 mb-3">
							<Cpu className="h-5 w-5 text-muted-foreground" />
							<p className="text-sm font-semibold">SubAgent</p>
							<span className="rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
								后台任务
							</span>
						</div>
						<div className="space-y-2 text-sm text-muted-foreground">
							<p>• 固定配置：轻量模型</p>
							<p>• 最大迭代：15 次</p>
							<p>• Temperature: 0.0</p>
							<p>• 不允许嵌套 SubAgent</p>
						</div>
					</div>

					{/* Settings Link */}
					<div className="rounded-lg border border-primary/30 bg-primary/5 p-3">
						<div className="flex items-center gap-2">
							<Settings2 className="h-4 w-4 text-primary" />
							<p className="text-sm text-primary">
								在 Settings 工作域修改 Agent 配置
							</p>
						</div>
					</div>
				</div>
			</div>
		</div>
	);
}
