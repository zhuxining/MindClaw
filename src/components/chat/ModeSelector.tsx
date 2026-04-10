import type { ConversationMode } from "@/lib/types";
import { cn } from "@/lib/utils";

const MODES: { id: ConversationMode; label: string }[] = [
	{ id: "companion", label: "陪伴" },
	{ id: "reflection", label: "反思" },
	{ id: "challenge", label: "挑战" },
	{ id: "knowledge", label: "知识" },
	{ id: "vault", label: "树洞" },
];

interface ModeSelectorProps {
	mode: ConversationMode;
	onChange: (mode: ConversationMode) => void;
}

export function ModeSelector({ mode, onChange }: ModeSelectorProps) {
	return (
		<div className="flex gap-1 rounded-lg bg-muted p-1">
			{MODES.map((m) => (
				<button
					key={m.id}
					type="button"
					onClick={() => onChange(m.id)}
					className={cn(
						"flex-1 rounded-md px-2 py-1 text-xs font-medium transition-all",
						mode === m.id
							? "bg-background text-foreground shadow-sm"
							: "text-muted-foreground hover:text-foreground",
					)}
				>
					{m.label}
				</button>
			))}
		</div>
	);
}
