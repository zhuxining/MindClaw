import type { AgentMessage, UserMessage } from "@/lib/types";
import { cn } from "@/lib/utils";

function StreamingDot() {
	return (
		<span className="ml-2 inline-flex items-center gap-1">
			{[0, 1, 2].map((index) => (
				<span
					key={index}
					className="h-1.5 w-1.5 animate-bounce rounded-full bg-current/70"
					style={{ animationDelay: `${index * 150}ms` }}
				/>
			))}
		</span>
	);
}

function PhaseLabel({ phase }: { phase: string }) {
	const labels: Record<string, string> = {
		thinking: "思考中…",
		using_tools: "处理中…",
		streaming: "生成中…",
		completed: "",
		cancelled: "已取消",
		error: "出错了",
	};
	const label = labels[phase];
	if (!label) return null;
	return <span className="text-xs text-muted-foreground italic">{label}</span>;
}

export function UserBubble({ message }: { message: UserMessage }) {
	return (
		<div className="flex justify-end">
			<div className="max-w-[86%] rounded-[22px] rounded-tr-sm bg-primary px-4 py-3 text-sm leading-7 text-primary-foreground shadow-sm">
				{message.content}
			</div>
		</div>
	);
}

export function AgentBubble({ message }: { message: AgentMessage }) {
	return (
		<div className="flex justify-start">
			<div
				className={cn(
					"max-w-[86%] rounded-[22px] rounded-tl-sm border px-4 py-3 text-sm leading-7 shadow-sm",
					message.phase === "error"
						? "border-red-200 bg-red-50 text-red-700"
						: "border-border/70 bg-background text-foreground",
				)}
			>
				<div className="mb-1 text-[11px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
					MindClaw
				</div>
				{message.content ? (
					<span className="whitespace-pre-wrap">{message.content}</span>
				) : (
					<PhaseLabel phase={message.phase} />
				)}
				{message.isStreaming && message.phase !== "error" ? (
					<StreamingDot />
				) : null}
			</div>
		</div>
	);
}
