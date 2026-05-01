import { BookOpen, Database, FileText, FolderOpen, Pin, X } from "lucide-react";
import { Button } from "@/components/ui/button";
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
	description: string;
	icon: React.ComponentType<{ className?: string }>;
}[] = [
	{ id: "daily", label: "日记", description: "每日记录", icon: BookOpen },
	{ id: "vault", label: "Vault", description: "全库浏览", icon: Database },
	{ id: "source", label: "资源", description: "链接与附件", icon: FolderOpen },
	{ id: "private", label: "私密", description: "Agent 不可见", icon: FileText },
];

export function TabNav() {
	const activeTabId = useWorkspaceStore((state) => state.activeTabId);
	const pinnedDirTabs = useWorkspaceStore((state) => state.pinnedDirTabs);
	const setActiveTab = useWorkspaceStore((state) => state.setActiveTab);
	const unpinDirTab = useWorkspaceStore((state) => state.unpinDirTab);

	return (
		<div className="border-b border-border/70 px-4 pb-4 pt-4">
			<div className="mb-4 flex items-center justify-between">
				<div>
					<p className="text-[11px] font-semibold uppercase tracking-[0.16em] text-muted-foreground">
						Workspace
					</p>
					<h2 className="mt-1 text-sm font-semibold text-foreground">导航</h2>
				</div>
				<div className="rounded-full border border-border/70 bg-muted/60 px-2.5 py-1 text-[11px] text-muted-foreground">
					4 + {pinnedDirTabs.length}
				</div>
			</div>

			<div className="grid grid-cols-2 gap-2">
				{BUILTIN_TABS.map(({ id, label, description, icon: Icon }) => (
					<TabButton
						key={id}
						active={activeTabId === id}
						onClick={() => setActiveTab(id)}
						label={label}
						description={description}
						icon={<Icon className="h-4 w-4" />}
					/>
				))}
			</div>

			{pinnedDirTabs.length > 0 ? (
				<div className="mt-4 space-y-2">
					<p className="text-[11px] font-semibold uppercase tracking-[0.16em] text-muted-foreground">
						已固定目录
					</p>
					<div className="space-y-1.5">
						{pinnedDirTabs.map((tab) => (
							<ContextMenu key={tab.id}>
								<ContextMenuTrigger>
									<TabButton
										active={activeTabId === tab.id}
										onClick={() => setActiveTab(tab.id)}
										label={tab.label}
										description={tab.dirPath}
										icon={<Pin className="h-4 w-4" />}
										className="w-full"
									/>
								</ContextMenuTrigger>
								<ContextMenuContent>
									<ContextMenuItem
										className="text-destructive"
										onClick={() => unpinDirTab(tab.id)}
									>
										<X className="mr-2 h-4 w-4" />
										取消固定
									</ContextMenuItem>
								</ContextMenuContent>
							</ContextMenu>
						))}
					</div>
				</div>
			) : null}
		</div>
	);
}

function TabButton({
	active,
	onClick,
	label,
	description,
	icon,
	className,
}: {
	active: boolean;
	onClick: () => void;
	label: string;
	description: string;
	icon: React.ReactNode;
	className?: string;
}) {
	return (
		<Button
			type="button"
			variant="ghost"
			onClick={onClick}
			className={cn(
				"h-auto items-start justify-start rounded-xl border px-3 py-3 text-left",
				active
					? "border-accent bg-accent/80 text-accent-foreground shadow-[0_8px_24px_rgba(15,23,42,0.08)]"
					: "border-border/60 bg-transparent text-muted-foreground hover:border-border hover:bg-muted/70 hover:text-foreground",
				className,
			)}
		>
			<div className="flex w-full items-start gap-3">
				<div
					className={cn(
						"mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border",
						active
							? "border-white/70 bg-white/70 text-foreground"
							: "border-border/70 bg-muted/70 text-muted-foreground",
					)}
				>
					{icon}
				</div>
				<div className="min-w-0">
					<div className="truncate text-sm font-medium">{label}</div>
					<div className="truncate text-xs text-muted-foreground">
						{description}
					</div>
				</div>
			</div>
		</Button>
	);
}
