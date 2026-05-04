import { ChevronLeft, ChevronRight } from "lucide-react";
import type { ReactNode } from "react";
import { useShellStore } from "@/stores/shell";

interface RightPanelProps {
	children?: ReactNode;
}

export function RightPanel({ children }: RightPanelProps) {
	const collapsed = useShellStore((s) => s.rightPanelCollapsed);
	const toggle = useShellStore((s) => s.toggleRightPanel);

	return (
		<div
			className="flex h-full min-w-65 flex-col border-l"
			style={{ borderColor: "var(--flexoki-bg-2)" }}
		>
			<div
				className="flex shrink-0 items-center justify-between px-3"
				style={{ height: 32, borderBottom: "1px solid var(--flexoki-bg-2)" }}
			>
				<button
					type="button"
					onClick={toggle}
					className="flex h-6 w-6 items-center justify-center rounded text-muted-foreground hover:text-foreground"
					aria-label={collapsed ? "展开右侧面板" : "折叠右侧面板"}
				>
					{collapsed ? <ChevronLeft size={14} /> : <ChevronRight size={14} />}
				</button>
			</div>
			{!collapsed && children}
		</div>
	);
}
