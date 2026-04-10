import type { AgentMessage, UserMessage } from "@/lib/types";
import { cn } from "@/lib/utils";

function StreamingDot() {
	return (
		<span className="inline-flex items-center gap-0.5 ml-1">
			{[0, 1, 2].map((i) => (
				<span
					key={i}
					className="h-1 w-1 rounded-full bg-current opacity-70 animate-bounce"
					style={{ animationDelay: `${i * 150}ms` }}
				/>
			))}
		</span>
	);
}

function PhaseLabel({ phase }: { phase: string }) {
	const labels: Record<string, string> = {
		thinking: "思考中",
		using_tools: "使用工具",
		streaming: "",
		completed: "",
		cancelled: "已取消",
		error: "出错",
	};
	const label = labels[phase];
	if (!label) return null;
	return <span className="text-xs text-muted-foreground italic">{label}</span>;
}

export function UserBubble({ message }: { message: UserMessage }) {
	return (
		<div className="flex justify-end">
			<div className="max-w-[85%] rounded-2xl rounded-tr-sm bg-primary px-3 py-2 text-sm text-primary-foreground">
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
					"max-w-[85%] rounded-2xl rounded-tl-sm px-3 py-2 text-sm",
					message.phase === "error"
						? "bg-destructive/10 text-destructive"
						: "bg-muted text-foreground",
				)}
			>
				{message.content ? (
					<span className="whitespace-pre-wrap">{message.content}</span>
				) : (
					<PhaseLabel phase={message.phase} />
				)}
				{message.isStreaming && message.phase !== "error" && <StreamingDot />}
			</div>
		</div>
	);
}
