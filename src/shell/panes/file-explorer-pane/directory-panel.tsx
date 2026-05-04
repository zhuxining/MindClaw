import { useQuery } from "@tanstack/react-query";
import { Filter, LayoutList, Search, TreePine, X } from "lucide-react";
import { useMemo, useState } from "react";
import { Input } from "@/components/ui/input";
import { useOpenContent } from "@/hooks/useOpenContent";
import { ipc } from "@/lib/ipc";
import type { DirectoryViewMode, NoteItem, VaultNote } from "@/lib/types";
import { cn } from "@/lib/utils";
import { type FilterQuery, useFilterStore } from "@/stores/filter";
import {
	useShellStore,
	workspaceIdToScope,
	workspaceIdToViewMode,
} from "@/stores/shell";
import { FlatView } from "./flat-view";
import { TreeView } from "./tree-view";

const SCOPE_META: Record<string, { title: string; description: string }> = {
	daily: { title: "Daily Notes", description: "按日期浏览最近记录" },
	vault: { title: "Vault Browser", description: "浏览整个 Vault" },
	inbox: { title: "收件箱", description: "待处理笔记" },
	private: { title: "Private Notes", description: "仅自己可见的内容" },
};

export function DirectoryPanel() {
	const activeId = useShellStore((s) => s.activeWorkspaceId);
	const [viewMode, setViewMode] = useState<DirectoryViewMode>("flat");
	const [query, setQuery] = useState("");

	const vaultPath = workspaceIdToScope(activeId);
	const defaultMode = workspaceIdToViewMode(activeId);
	const effectiveMode =
		viewMode === "tree" || defaultMode === "tree" ? viewMode : viewMode;

	// 过滤状态
	const selectedTags = useFilterStore((s) => s.selectedTags);
	const dateRange = useFilterStore((s) => s.dateRange);
	const selectedTypes = useFilterStore((s) => s.selectedTypes);
	const hasActiveFilters = useFilterStore((s) => s.hasActiveFilters());
	const clearFilters = useFilterStore((s) => s.clearFilters);

	const activeMeta = useMemo(() => {
		return (
			SCOPE_META[activeId] ?? {
				title: activeId,
				description: vaultPath || "Vault 根目录",
			}
		);
	}, [activeId, vaultPath]);

	const showSearchResults = activeId === "vault" && query.trim().length >= 2;
	const showFilteredResults = hasActiveFilters && !showSearchResults;

	// 构建过滤查询对象
	const filterQuery: FilterQuery = useMemo(() => {
		const q: FilterQuery = {};
		if (selectedTags.length > 0) q.tags = selectedTags;
		if (dateRange) q.dateRange = dateRange;
		if (selectedTypes.length > 0) q.types = selectedTypes;
		return q;
	}, [selectedTags, dateRange, selectedTypes]);

	return (
		<div className="flex h-full min-h-0 flex-col">
			<div
				className="space-y-3 border-b px-4 py-4"
				style={{ borderColor: "var(--flexoki-bg-2)" }}
			>
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
								activeId === "vault" ? "搜索标题或标签" : "按文件名筛选当前列表"
							}
							className="h-10 rounded-xl pl-9 text-sm"
						/>
					</div>

					<ViewToggle
						mode={effectiveMode}
						onChange={(mode) => setViewMode(mode)}
					/>
				</div>

				{showFilteredResults && (
					<div className="flex items-center gap-2 rounded-lg bg-accent/50 px-3 py-2">
						<Filter className="h-4 w-4 text-primary" />
						<p className="text-xs text-muted-foreground">
							过滤已激活
							{selectedTags.length > 0 && ` · ${selectedTags.length} 个标签`}
							{dateRange && ` · 日期范围`}
							{selectedTypes.length > 0 && ` · ${selectedTypes.length} 个类型`}
						</p>
						<button
							type="button"
							onClick={clearFilters}
							className="ml-auto rounded-lg p-1 text-muted-foreground transition-colors hover:bg-muted/70 hover:text-foreground"
						>
							<X className="h-3 w-3" />
						</button>
					</div>
				)}
			</div>

			<div className="min-h-0 flex-1 overflow-hidden">
				{showSearchResults ? (
					<VaultSearchResults query={query} />
				) : showFilteredResults ? (
					<FilteredResults filterQuery={filterQuery} />
				) : effectiveMode === "tree" ? (
					<TreeView path={vaultPath} query={query} />
				) : (
					<FlatView path={vaultPath} tabId={activeId} query={query} />
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
		<div
			className="flex items-center gap-1 rounded-xl border p-1"
			style={{ borderColor: "var(--flexoki-bg-2)" }}
		>
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
	const { openFromItem } = useOpenContent();
	const { data: results = [], isLoading } = useQuery({
		queryKey: ["vault-search", query],
		queryFn: () => ipc.searchVault(query.trim()),
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
								onClick={() => openFromItem(item)}
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

function FilteredResults({ filterQuery }: { filterQuery: FilterQuery }) {
	const { openFromItem } = useOpenContent();
	const { data: results = [], isLoading } = useQuery({
		queryKey: ["filtered-notes", filterQuery],
		queryFn: () => {
			const params: {
				tags?: string[];
				date_from?: string;
				date_to?: string;
				limit: number;
			} = { limit: 100 };
			if (filterQuery.tags) params.tags = filterQuery.tags;
			if (filterQuery.dateRange?.from)
				params.date_from = filterQuery.dateRange.from;
			if (filterQuery.dateRange?.to) params.date_to = filterQuery.dateRange.to;
			return ipc.listNotesByFilter(params);
		},
	});

	if (isLoading) {
		return (
			<div className="px-4 py-4 text-sm text-muted-foreground">
				加载过滤结果…
			</div>
		);
	}

	if (results.length === 0) {
		return (
			<div className="px-4 py-6 text-sm text-muted-foreground">
				无符合条件的笔记
			</div>
		);
	}

	return (
		<div className="h-full overflow-y-auto px-3 py-3">
			<div className="mb-3 text-xs font-medium text-muted-foreground">
				过滤结果 · {results.length} 条
			</div>
			<ul className="space-y-1.5">
				{results.map((entry: VaultNote) => {
					const item: NoteItem = {
						type: "note",
						path: entry.topic,
						title: entry.title,
					};
					return (
						<li key={entry.id}>
							<button
								type="button"
								onClick={() => openFromItem(item)}
								className="w-full rounded-xl border border-transparent px-3 py-2 text-left transition-colors hover:border-border hover:bg-muted/60"
							>
								<p className="truncate text-sm font-medium text-foreground">
									{entry.title}
								</p>
								<p className="truncate text-xs text-muted-foreground">
									{entry.topic}
								</p>
								{entry.tags.length > 0 && (
									<div className="mt-1 flex gap-1">
										{entry.tags.slice(0, 3).map((tag: string) => (
											<span
												key={tag}
												className="rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground"
											>
												{tag}
											</span>
										))}
									</div>
								)}
							</button>
						</li>
					);
				})}
			</ul>
		</div>
	);
}
