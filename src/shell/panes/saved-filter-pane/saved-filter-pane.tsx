import { Bookmark, Check, X } from "lucide-react";
import { useState } from "react";
import { cn } from "@/lib/utils";
import { useFilterStore } from "@/stores/filter";

export function SavedFilterPane() {
	const savedFilters = useFilterStore((s) => s.savedFilters);
	const activeSavedFilterId = useFilterStore((s) => s.activeSavedFilterId);
	const hasActiveFilters = useFilterStore((s) => s.hasActiveFilters);
	const loadFilter = useFilterStore((s) => s.loadFilter);
	const deleteFilter = useFilterStore((s) => s.deleteFilter);
	const saveFilter = useFilterStore((s) => s.saveFilter);
	const clearFilters = useFilterStore((s) => s.clearFilters);

	const [isSaving, setIsSaving] = useState(false);
	const [filterName, setFilterName] = useState("");

	function handleSave() {
		if (filterName.trim().length === 0) return;
		saveFilter(filterName.trim());
		setFilterName("");
		setIsSaving(false);
	}

	function formatQueryPreview(
		filter: ReturnType<typeof useFilterStore.getState>["savedFilters"][0],
	) {
		const parts: string[] = [];
		if (filter.query.tags?.length) {
			parts.push(filter.query.tags.join(", "));
		}
		if (filter.query.dateRange) {
			parts.push(
				`${filter.query.dateRange.from} ~ ${filter.query.dateRange.to}`,
			);
		}
		if (filter.query.types?.length) {
			parts.push(filter.query.types.join(", "));
		}
		return parts.length > 0 ? parts.join(" · ") : "无过滤条件";
	}

	return (
		<div className="flex h-full flex-col">
			<div
				className="border-b p-4"
				style={{ borderColor: "var(--flexoki-bg-2)" }}
			>
				<div className="mb-3">
					<p className="text-sm font-semibold text-foreground">已存过滤</p>
					<p className="text-xs text-muted-foreground">保存常用的过滤组合</p>
				</div>

				{isSaving ? (
					<div className="space-y-2">
						<input
							type="text"
							value={filterName}
							onChange={(e) => setFilterName(e.target.value)}
							placeholder="过滤名称"
							className="w-full rounded-lg border border-border/50 bg-background px-3 py-2 text-sm outline-none focus:border-primary"
						/>
						<div className="flex gap-2">
							<button
								type="button"
								onClick={handleSave}
								disabled={filterName.trim().length === 0}
								className="flex-1 rounded-lg bg-primary px-3 py-1.5 text-sm text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-50"
							>
								保存
							</button>
							<button
								type="button"
								onClick={() => {
									setIsSaving(false);
									setFilterName("");
								}}
								className="rounded-lg border border-border/50 px-3 py-1.5 text-sm transition-colors hover:bg-muted/70"
							>
								取消
							</button>
						</div>
					</div>
				) : (
					<button
						type="button"
						onClick={() => setIsSaving(true)}
						disabled={!hasActiveFilters()}
						className="w-full rounded-lg border border-border/50 px-3 py-2 text-sm transition-colors hover:bg-muted/70 disabled:opacity-50"
					>
						<Bookmark className="mr-2 inline h-4 w-4" />
						保存当前过滤
					</button>
				)}
			</div>

			<div className="min-h-0 flex-1 overflow-y-auto p-3">
				{savedFilters.length === 0 ? (
					<div className="py-8 text-center text-sm text-muted-foreground">
						暂无保存的过滤组合
					</div>
				) : (
					<ul className="space-y-2">
						{savedFilters.map((filter) => {
							const isActive = activeSavedFilterId === filter.id;
							return (
								<li key={filter.id}>
									<div
										className={cn(
											"rounded-lg border p-3 transition-colors",
											isActive
												? "border-primary bg-accent/50"
												: "border-border/50 hover:bg-muted/50",
										)}
									>
										<div className="mb-1 flex items-center gap-2">
											{isActive && <Check className="h-4 w-4 text-primary" />}
											<p className="text-sm font-medium">{filter.name}</p>
										</div>
										<p className="text-xs text-muted-foreground">
											{formatQueryPreview(filter)}
										</p>
										<div className="mt-2 flex gap-2">
											<button
												type="button"
												onClick={() => loadFilter(filter.id)}
												className="rounded-lg px-2 py-1 text-xs transition-colors hover:bg-muted/70"
											>
												加载
											</button>
											<button
												type="button"
												onClick={() => deleteFilter(filter.id)}
												className="rounded-lg px-2 py-1 text-xs text-destructive transition-colors hover:bg-destructive/10"
											>
												删除
											</button>
										</div>
									</div>
								</li>
							);
						})}
					</ul>
				)}
			</div>

			{activeSavedFilterId && (
				<div
					className="border-t p-3"
					style={{ borderColor: "var(--flexoki-bg-2)" }}
				>
					<button
						type="button"
						onClick={clearFilters}
						className="flex items-center gap-1 rounded-lg px-2 py-1 text-xs text-muted-foreground transition-colors hover:bg-muted/70 hover:text-foreground"
					>
						<X className="h-3 w-3" />
						清除当前过滤
					</button>
				</div>
			)}
		</div>
	);
}
