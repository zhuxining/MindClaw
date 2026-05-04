import { useQuery } from "@tanstack/react-query";
import { Brain, Star } from "lucide-react";
import { useState } from "react";
import { useOpenContent } from "@/hooks/useOpenContent";
import { ipc } from "@/lib/ipc";
import type { MemoryListItem } from "@/lib/types";

const CATEGORIES = ["preference", "pattern", "case", "constraint", "knowledge"];

export function MemoryListPane() {
	const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
	const { openFromItem } = useOpenContent();

	const { data: memories = [], isLoading } = useQuery({
		queryKey: ["memories", selectedCategory],
		queryFn: () => {
			const params: { limit: number; category?: string } = { limit: 100 };
			if (selectedCategory) params.category = selectedCategory;
			return ipc.listMemories(params);
		},
	});

	function handleOpenMemory(memory: MemoryListItem) {
		// 打开 memory 文件
		openFromItem({
			type: "note",
			path: memory.file_path,
			title: memory.key,
		});
	}

	if (isLoading) {
		return (
			<div className="flex h-full items-center justify-center p-4 text-sm text-muted-foreground">
				加载记忆列表…
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
					<p className="text-sm font-semibold text-foreground">Agent 记忆</p>
					<p className="text-xs text-muted-foreground">长期关键记忆摘要</p>
				</div>

				<div className="flex flex-wrap gap-1.5">
					<button
						type="button"
						onClick={() => setSelectedCategory(null)}
						className={`rounded-lg px-2.5 py-1.5 text-xs transition-colors ${
							selectedCategory === null
								? "bg-accent text-accent-foreground"
								: "border border-border/50 hover:bg-muted/70"
						}`}
					>
						全部
					</button>
					{CATEGORIES.map((cat) => (
						<button
							key={cat}
							type="button"
							onClick={() => setSelectedCategory(cat)}
							className={`rounded-lg px-2.5 py-1.5 text-xs transition-colors ${
								selectedCategory === cat
									? "bg-accent text-accent-foreground"
									: "border border-border/50 hover:bg-muted/70"
							}`}
						>
							{cat}
						</button>
					))}
				</div>
			</div>

			<div className="min-h-0 flex-1 overflow-y-auto p-3">
				{memories.length === 0 ? (
					<div className="py-8 text-center text-sm text-muted-foreground">
						暂无记忆（Agent 运行后生成）
					</div>
				) : (
					<ul className="space-y-2">
						{memories.map((memory) => {
							const importancePercent = Math.round(memory.importance * 100);
							const updated = new Date(memory.updated);
							const timeStr = updated.toLocaleDateString("zh-CN");

							return (
								<li key={memory.id}>
									<button
										type="button"
										onClick={() => handleOpenMemory(memory)}
										className="w-full rounded-lg border border-border/50 p-3 text-left transition-colors hover:bg-muted/50"
									>
										<div className="flex items-center gap-2">
											<Brain className="h-4 w-4 text-muted-foreground" />
											<p className="truncate text-sm font-medium">
												{memory.key}
											</p>
										</div>
										<div className="mt-1.5 flex items-center gap-3 text-xs text-muted-foreground">
											<span className="rounded bg-muted px-1.5 py-0.5">
												{memory.category}
											</span>
											<span className="flex items-center gap-1">
												<Star className="h-3 w-3" />
												{importancePercent}%
											</span>
											<span>{timeStr}</span>
										</div>
									</button>
								</li>
							);
						})}
					</ul>
				)}
			</div>
		</div>
	);
}
