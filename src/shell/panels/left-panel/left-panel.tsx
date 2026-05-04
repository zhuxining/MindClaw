import { ChevronLeft, ChevronRight } from "lucide-react";
import type { ReactNode } from "react";
import { useShellStore } from "@/stores/shell";

interface LeftPanelProps {
	children?: ReactNode;
}

export function LeftPanel({ children }: LeftPanelProps) {
	const collapsed = useShellStore((s) => s.leftPanelCollapsed);
	const toggle = useShellStore((s) => s.toggleLeftPanel);

	return (
		<div className="flex h-full min-w-55 flex-col">
			<div
				className="flex shrink-0 items-center justify-between px-3"
				style={{ height: 32, borderBottom: "1px solid var(--flexoki-bg-2)" }}
			>
				<button
					type="button"
					onClick={toggle}
					className="flex h-6 w-6 items-center justify-center rounded text-muted-foreground hover:text-foreground"
					aria-label={collapsed ? "展开左侧面板" : "折叠左侧面板"}
				>
					{collapsed ? <ChevronRight size={14} /> : <ChevronLeft size={14} />}
				</button>
			</div>
			{!collapsed && children}
		</div>
	);
}
