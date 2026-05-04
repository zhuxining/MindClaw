import { Check, X } from "lucide-react";
import { cn } from "@/lib/utils";
import { type ContentItemType, useFilterStore } from "@/stores/filter";

const TYPE_OPTIONS: {
	value: ContentItemType;
	label: string;
	description: string;
}[] = [
	{ value: "note", label: "笔记", description: "普通 Markdown 笔记" },
	{ value: "daily", label: "日记", description: "Daily Notes" },
	{ value: "task", label: "任务", description: "Checklist 任务项" },
	{ value: "resource", label: "资源", description: "PDF/Web 等外部资源" },
];

export function TypeFilterPane() {
	const selectedTypes = useFilterStore((s) => s.selectedTypes);
	const addType = useFilterStore((s) => s.addType);
	const removeType = useFilterStore((s) => s.removeType);
	const clearFilters = useFilterStore((s) => s.clearFilters);

	return (
		<div className="flex h-full flex-col">
			<div
				className="border-b p-4"
				style={{ borderColor: "var(--flexoki-bg-2)" }}
			>
				<div className="mb-3">
					<p className="text-sm font-semibold text-foreground">类型过滤</p>
					<p className="text-xs text-muted-foreground">按内容类型筛选</p>
				</div>
			</div>

			<div className="min-h-0 flex-1 overflow-y-auto p-3">
				<ul className="space-y-1">
					{TYPE_OPTIONS.map((opt) => {
						const isSelected = selectedTypes.includes(opt.value);
						return (
							<li key={opt.value}>
								<button
									type="button"
									onClick={() =>
										isSelected ? removeType(opt.value) : addType(opt.value)
									}
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
										<div>
											<p className="font-medium">{opt.label}</p>
											<p className="text-xs text-muted-foreground">
												{opt.description}
											</p>
										</div>
									</div>
								</button>
							</li>
						);
					})}
				</ul>
			</div>

			{selectedTypes.length > 0 && (
				<div
					className="border-t p-3"
					style={{ borderColor: "var(--flexoki-bg-2)" }}
				>
					<div className="flex items-center justify-between">
						<p className="text-xs text-muted-foreground">
							已选 {selectedTypes.length} 个类型
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
