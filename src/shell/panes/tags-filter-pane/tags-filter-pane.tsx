import { useQuery } from "@tanstack/react-query";
import { Check, Search, X } from "lucide-react";
import { useState } from "react";
import { Input } from "@/components/ui/input";
import { ipc } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useFilterStore } from "@/stores/filter";

export function TagsFilterPane() {
	const [searchQuery, setSearchQuery] = useState("");
	const selectedTags = useFilterStore((s) => s.selectedTags);
	const addTag = useFilterStore((s) => s.addTag);
	const removeTag = useFilterStore((s) => s.removeTag);
	const clearFilters = useFilterStore((s) => s.clearFilters);

	const { data: allTags = [], isLoading } = useQuery({
		queryKey: ["all-tags"],
		queryFn: () => ipc.listAllTags(),
	});

	const filteredTags =
		searchQuery.trim().length > 0
			? allTags.filter((tag) =>
					tag.toLowerCase().includes(searchQuery.trim().toLowerCase()),
				)
			: allTags;

	if (isLoading) {
		return (
			<div className="flex h-full items-center justify-center p-4 text-sm text-muted-foreground">
				加载标签…
			</div>
		);
	}

	if (allTags.length === 0) {
		return (
			<div className="flex h-full items-center justify-center p-4 text-sm text-muted-foreground">
				暂无标签（索引需同步）
			</div>
		);
	}

	return (
		<div className="flex h-full flex-col">
			<div
				className="border-b p-4"
				style={{ borderColor: "var(--flexoki-bg-2)" }}
			>
				<div className="mb-3">
					<p className="text-sm font-semibold text-foreground">标签过滤</p>
					<p className="text-xs text-muted-foreground">选择标签筛选笔记</p>
				</div>

				<div className="relative">
					<Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
					<Input
						value={searchQuery}
						onChange={(e) => setSearchQuery(e.target.value)}
						placeholder="搜索标签"
						className="h-9 rounded-lg pl-9 text-sm"
					/>
				</div>
			</div>

			<div className="min-h-0 flex-1 overflow-y-auto p-3">
				<ul className="space-y-1">
					{filteredTags.map((tag) => {
						const isSelected = selectedTags.includes(tag);
						return (
							<li key={tag}>
								<button
									type="button"
									onClick={() => (isSelected ? removeTag(tag) : addTag(tag))}
									className={cn(
										"w-full rounded-lg px-3 py-2 text-left text-sm transition-colors",
										isSelected
											? "bg-accent text-accent-foreground"
											: "hover:bg-muted/70",
									)}
								>
									<div className="flex items-center gap-2">
										<div
											className={cn(
												"flex h-4 w-4 items-center justify-center rounded border",
												isSelected
													? "border-primary bg-primary text-primary-foreground"
													: "border-muted-foreground/30",
											)}
										>
											{isSelected && <Check className="h-3 w-3" />}
										</div>
										<span className="truncate">{tag}</span>
									</div>
								</button>
							</li>
						);
					})}
				</ul>
			</div>

			{selectedTags.length > 0 && (
				<div
					className="border-t p-3"
					style={{ borderColor: "var(--flexoki-bg-2)" }}
				>
					<div className="flex items-center justify-between">
						<p className="text-xs text-muted-foreground">
							已选 {selectedTags.length} 个标签
						</p>
						<button
							type="button"
							onClick={clearFilters}
							className="flex items-center gap-1 rounded-lg px-2 py-1 text-xs text-muted-foreground transition-colors hover:bg-muted/70 hover:text-foreground"
						>
							<X className="h-3 w-3" />
							清除
						</button>
					</div>
				</div>
			)}
		</div>
	);
}
