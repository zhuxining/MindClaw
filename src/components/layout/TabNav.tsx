import { BookOpen, Database, FileText, FolderOpen, X } from "lucide-react";
import {
	ContextMenu,
	ContextMenuContent,
	ContextMenuItem,
	ContextMenuTrigger,
} from "@/components/ui/context-menu";
import type { BuiltinTabId } from "@/lib/types";
import { cn } from "@/lib/utils";
import { useWorkspaceStore } from "@/stores/workspace";

const BUILTIN_TABS: {
	id: BuiltinTabId;
	label: string;
	icon: React.ComponentType<{ className?: string }>;
}[] = [
	{ id: "daily", label: "日记", icon: BookOpen },
	{ id: "private", label: "私密", icon: FileText },
	{ id: "vault", label: "库", icon: Database },
	{ id: "source", label: "资源", icon: FolderOpen },
];

export function TabNav() {
	const activeTabId = useWorkspaceStore((s) => s.activeTabId);
	const pinnedDirTabs = useWorkspaceStore((s) => s.pinnedDirTabs);
	const setActiveTab = useWorkspaceStore((s) => s.setActiveTab);
	const unpinDirTab = useWorkspaceStore((s) => s.unpinDirTab);

	return (
		<div className="flex flex-col border-b border-border">
			<div className="flex flex-wrap items-center gap-0.5 px-1 pb-1 pt-1">
				{BUILTIN_TABS.map(({ id, label, icon: Icon }) => (
					<TabButton
						key={id}
						active={activeTabId === id}
						onClick={() => setActiveTab(id)}
						label={label}
					>
						<Icon className="h-3.5 w-3.5" />
					</TabButton>
				))}

				{pinnedDirTabs.map((tab) => (
					<ContextMenu key={tab.id}>
						<ContextMenuTrigger>
							<TabButton
								active={activeTabId === tab.id}
								onClick={() => setActiveTab(tab.id)}
								label={tab.label}
							>
								<FolderOpen className="h-3.5 w-3.5" />
							</TabButton>
						</ContextMenuTrigger>
						<ContextMenuContent>
							<ContextMenuItem
								className="text-destructive"
								onClick={() => unpinDirTab(tab.id)}
							>
								<X className="mr-2 h-4 w-4" />
								关闭
							</ContextMenuItem>
						</ContextMenuContent>
					</ContextMenu>
				))}
			</div>
		</div>
	);
}

function TabButton({
	active,
	onClick,
	label,
	children,
}: {
	active: boolean;
	onClick: () => void;
	label: string;
	children: React.ReactNode;
}) {
	return (
		<button
			type="button"
			onClick={onClick}
			title={label}
			className={cn(
				"flex items-center gap-1.5 rounded px-2 py-1.5 text-xs font-medium transition-colors",
				active
					? "bg-accent text-accent-foreground"
					: "text-muted-foreground hover:bg-accent/50 hover:text-foreground",
			)}
		>
			{children}
			<span className="max-w-20 truncate">{label}</span>
		</button>
	);
}
