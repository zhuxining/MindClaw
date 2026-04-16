import { useQuery } from "@tanstack/react-query";
import { LayoutList, Search, TreePine } from "lucide-react";
import { useMemo, useState } from "react";
import { Input } from "@/components/ui/input";
import { ipc } from "@/lib/ipc";
import type { DirectoryViewMode, NoteItem } from "@/lib/types";
import { cn } from "@/lib/utils";
import {
	tabToVaultPath,
	useTabViewMode,
	useWorkspaceStore,
} from "@/stores/workspace";
import { FlatView } from "./FlatView";
import { TreeView } from "./TreeView";

const TAB_META: Record<string, { title: string; description: string }> = {
	daily: { title: "Daily Notes", description: "按日期浏览最近记录" },
	vault: { title: "Vault Browser", description: "浏览整个知识库" },
	source: { title: "Resources", description: "外部链接、PDF 与图片" },
	private: { title: "Private Notes", description: "仅自己可见的内容" },
};

export function DirectoryPanel() {
	const activeTabId = useWorkspaceStore((state) => state.activeTabId);
	const pinnedDirTabs = useWorkspaceStore((state) => state.pinnedDirTabs);
	const setDirViewMode = useWorkspaceStore((state) => state.setDirViewMode);
	const viewMode = useTabViewMode(activeTabId);
	const [query, setQuery] = useState("");

	const vaultPath = tabToVaultPath(activeTabId, pinnedDirTabs);
	const activeMeta = useMemo(() => {
		if (activeTabId in TAB_META) return TAB_META[activeTabId];
		const pinned = pinnedDirTabs.find((tab) => tab.id === activeTabId);
		return {
			title: pinned?.label ?? "Pinned Folder",
			description: pinned?.dirPath ?? "固定目录",
		};
	}, [activeTabId, pinnedDirTabs]);

	const showSearchResults = activeTabId === "vault" && query.trim().length >= 2;

	return (
		<div className="flex h-full min-h-0 flex-col">
			<div className="space-y-3 border-b border-border/70 px-4 py-4">
				<div className="space-y-1">
					<p className="text-sm font-semibold text-foreground">
						{activeMeta.title}
					</p>
					<p className="text-xs text-muted-foreground">
						{vaultPath ? `/${vaultPath}` : "Vault 根目录"} ·{" "}
						{activeMeta.description}
					</p>
				</div>

				<div className="flex items-center gap-2">
					<div className="relative flex-1">
						<Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
						<Input
							value={query}
							onChange={(event) => setQuery(event.target.value)}
							placeholder={
								activeTabId === "vault"
									? "搜索标题或标签"
									: "按文件名筛选当前列表"
							}
							className="h-10 rounded-xl border-border/70 bg-background pl-9 text-sm"
						/>
					</div>

					<ViewToggle
						mode={viewMode}
						onChange={(mode) => setDirViewMode(activeTabId, mode)}
					/>
				</div>
			</div>

			<div className="min-h-0 flex-1 overflow-hidden">
				{showSearchResults ? (
					<VaultSearchResults query={query} />
				) : viewMode === "tree" ? (
					<TreeView path={vaultPath} query={query} />
				) : (
					<FlatView path={vaultPath} tabId={activeTabId} query={query} />
				)}
			</div>
		</div>
	);
}

function ViewToggle({
	mode,
	onChange,
}: {
	mode: DirectoryViewMode;
	onChange: (mode: DirectoryViewMode) => void;
}) {
	return (
		<div className="flex items-center gap-1 rounded-xl border border-border/70 bg-muted/60 p-1">
			<button
				type="button"
				title="平铺"
				onClick={() => onChange("flat")}
				className={cn(
					"flex h-8 w-8 items-center justify-center rounded-lg transition-colors",
					mode === "flat"
						? "bg-background text-foreground shadow-sm"
						: "text-muted-foreground hover:text-foreground",
				)}
			>
				<LayoutList className="h-4 w-4" />
			</button>
			<button
				type="button"
				title="树状"
				onClick={() => onChange("tree")}
				className={cn(
					"flex h-8 w-8 items-center justify-center rounded-lg transition-colors",
					mode === "tree"
						? "bg-background text-foreground shadow-sm"
						: "text-muted-foreground hover:text-foreground",
				)}
			>
				<TreePine className="h-4 w-4" />
			</button>
		</div>
	);
}

function VaultSearchResults({ query }: { query: string }) {
	const openItem = useWorkspaceStore((state) => state.openItem);
	const { data: results = [], isLoading } = useQuery({
		queryKey: ["vault-search", query],
		queryFn: () => ipc.searchKnowledge(query.trim()),
	});

	if (isLoading) {
		return (
			<div className="px-4 py-4 text-sm text-muted-foreground">搜索中…</div>
		);
	}

	if (results.length === 0) {
		return (
			<div className="px-4 py-6 text-sm text-muted-foreground">
				未找到相关笔记
			</div>
		);
	}

	return (
		<div className="h-full overflow-y-auto px-3 py-3">
			<div className="mb-3 text-xs font-medium text-muted-foreground">
				标题 / 标签匹配结果
			</div>
			<ul className="space-y-1.5">
				{results.map((entry) => {
					const item: NoteItem = {
						type: "note",
						path: entry.topic,
						title: entry.title,
					};
					return (
						<li key={entry.id}>
							<button
								type="button"
								onClick={() => openItem(item)}
								className="w-full rounded-xl border border-transparent px-3 py-2 text-left transition-colors hover:border-border hover:bg-muted/60"
							>
								<p className="truncate text-sm font-medium text-foreground">
									{entry.title}
								</p>
								<p className="truncate text-xs text-muted-foreground">
									{entry.topic}
								</p>
							</button>
						</li>
					);
				})}
			</ul>
		</div>
	);
}
