import type { ConversationMode } from "@/lib/types";
import { cn } from "@/lib/utils";

const MODES: {
	id: ConversationMode;
	label: string;
	description: string;
	accent: string;
}[] = [
	{
		id: "companion",
		label: "陪伴",
		description: "接住情绪",
		accent: "data-[active=true]:bg-sky-50 data-[active=true]:text-sky-700",
	},
	{
		id: "reflection",
		label: "反思",
		description: "轻推提问",
		accent:
			"data-[active=true]:bg-emerald-50 data-[active=true]:text-emerald-700",
	},
	{
		id: "challenge",
		label: "挑战",
		description: "直接指出",
		accent: "data-[active=true]:bg-rose-50 data-[active=true]:text-rose-700",
	},
	{
		id: "knowledge",
		label: "知识",
		description: "共建笔记",
		accent:
			"data-[active=true]:bg-violet-50 data-[active=true]:text-violet-700",
	},
	{
		id: "vault",
		label: "树洞",
		description: "不进知识库",
		accent: "data-[active=true]:bg-amber-50 data-[active=true]:text-amber-700",
	},
];

interface ModeSelectorProps {
	mode: ConversationMode;
	onChange: (mode: ConversationMode) => void;
}

export function ModeSelector({ mode, onChange }: ModeSelectorProps) {
	return (
		<div className="grid grid-cols-5 gap-2">
			{MODES.map((currentMode) => {
				const active = mode === currentMode.id;
				return (
					<button
						key={currentMode.id}
						type="button"
						onClick={() => onChange(currentMode.id)}
						data-active={active}
						className={cn(
							"rounded-2xl border border-border/70 bg-muted/50 px-2 py-2 text-left transition-all hover:border-border hover:bg-muted",
							currentMode.accent,
							active && "shadow-sm",
						)}
					>
						<div className="text-sm font-medium">{currentMode.label}</div>
						<div className="mt-0.5 text-[11px] text-muted-foreground">
							{currentMode.description}
						</div>
					</button>
				);
			})}
		</div>
	);
}
