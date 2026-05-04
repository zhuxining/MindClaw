import { X } from "lucide-react";
import type { OpenTab } from "@/lib/types";
import { cn } from "@/lib/utils";
import { useTabStore } from "@/stores/tabs";

interface TabAreaProps {
	className?: string;
}

export function TabArea({ className }: TabAreaProps) {
	const openTabs = useTabStore((s) => s.openTabs);
	const activeTabId = useTabStore((s) => s.activeTabId);
	const setActiveTab = useTabStore((s) => s.setActiveTab);
	const closeTab = useTabStore((s) => s.closeTab);

	if (openTabs.length === 0) return null;

	return (
		<div
			className={cn(
				"flex shrink-0 items-center gap-1 overflow-x-auto px-2 py-1",
				className,
			)}
			style={{
				height: 36,
				borderBottom: "1px solid var(--flexoki-bg-2)",
			}}
		>
			{openTabs.map((tab) => (
				<TabItem
					key={tab.id}
					tab={tab}
					active={tab.id === activeTabId}
					onSelect={() => setActiveTab(tab.id)}
					onClose={() => closeTab(tab.id)}
				/>
			))}
		</div>
	);
}

interface TabItemProps {
	tab: OpenTab;
	active: boolean;
	onSelect: () => void;
	onClose: () => void;
}

function TabItem({ tab, active, onSelect, onClose }: TabItemProps) {
	const showClose = active || tab.dirty;

	return (
		<button
			type="button"
			onClick={onSelect}
			className={cn(
				"flex items-center gap-1.5 rounded-md px-2 py-1 text-xs transition-colors",
				active
					? "bg-background text-foreground shadow-sm"
					: "text-muted-foreground hover:text-foreground hover:bg-muted/50",
			)}
			style={{
				minWidth: 80,
				maxWidth: 160,
				border: active
					? "1px solid var(--flexoki-bg-3)"
					: "1px solid transparent",
			}}
		>
			<span className="truncate">{tab.descriptor.title}</span>
			{tab.dirty && (
				<span
					className="ml-1 h-1.5 w-1.5 rounded-full"
					style={{ background: "var(--flexoki-tx-2)" }}
				/>
			)}
			{showClose && (
				<button
					type="button"
					onClick={(e) => {
						e.stopPropagation();
						onClose();
					}}
					className="ml-1 flex h-4 w-4 items-center justify-center rounded hover:bg-muted"
				>
					<X size={12} />
				</button>
			)}
		</button>
	);
}
