import { X } from "lucide-react";
import { cn } from "@/lib/utils";
import { type DateRange, useFilterStore } from "@/stores/filter";

const QUICK_OPTIONS = [
	{ label: "今天", getRange: () => getDateRange("today") },
	{ label: "本周", getRange: () => getDateRange("this-week") },
	{ label: "本月", getRange: () => getDateRange("this-month") },
	{ label: "最近 30 天", getRange: () => getDateRange("last-30-days") },
];

function formatDate(date: Date): string {
	return date.toISOString().split("T")[0];
}

function getDateRange(preset: string): DateRange {
	const now = new Date();
	const today = formatDate(now);

	switch (preset) {
		case "today":
			return { from: today, to: today };
		case "this-week": {
			const dayOfWeek = now.getDay();
			const startOfWeek = new Date(now);
			startOfWeek.setDate(now.getDate() - dayOfWeek);
			return { from: formatDate(startOfWeek), to: today };
		}
		case "this-month": {
			const startOfMonth = new Date(now.getFullYear(), now.getMonth(), 1);
			return { from: formatDate(startOfMonth), to: today };
		}
		case "last-30-days": {
			const start = new Date(now);
			start.setDate(now.getDate() - 30);
			return { from: formatDate(start), to: today };
		}
		default:
			return { from: today, to: today };
	}
}

export function CalendarFilterPane() {
	const dateRange = useFilterStore((s) => s.dateRange);
	const setDateRange = useFilterStore((s) => s.setDateRange);
	const clearFilters = useFilterStore((s) => s.clearFilters);

	function handleQuickSelect(preset: string) {
		const range = getDateRange(preset);
		setDateRange(range);
	}

	function handleFromDateChange(e: React.ChangeEvent<HTMLInputElement>) {
		const from = e.target.value;
		if (dateRange) {
			setDateRange({ ...dateRange, from });
		} else {
			setDateRange({ from, to: from });
		}
	}

	function handleToDateChange(e: React.ChangeEvent<HTMLInputElement>) {
		const to = e.target.value;
		if (dateRange) {
			setDateRange({ ...dateRange, to });
		} else {
			setDateRange({ from: to, to });
		}
	}

	return (
		<div className="flex h-full flex-col">
			<div
				className="border-b p-4"
				style={{ borderColor: "var(--flexoki-bg-2)" }}
			>
				<div className="mb-3">
					<p className="text-sm font-semibold text-foreground">日期过滤</p>
					<p className="text-xs text-muted-foreground">按修改日期筛选笔记</p>
				</div>

				<div className="flex flex-wrap gap-2">
					{QUICK_OPTIONS.map((opt) => {
						const isActive =
							dateRange &&
							dateRange.from === opt.getRange().from &&
							dateRange.to === opt.getRange().to;
						return (
							<button
								key={opt.label}
								type="button"
								onClick={() => handleQuickSelect(opt.label)}
								className={cn(
									"rounded-lg px-3 py-1.5 text-xs transition-colors",
									isActive
										? "bg-accent text-accent-foreground"
										: "border border-border/50 hover:bg-muted/70",
								)}
							>
								{opt.label}
							</button>
						);
					})}
				</div>
			</div>

			<div className="p-4">
				<div className="space-y-3">
					<div>
						<label
							htmlFor="date-from"
							className="mb-1 block text-xs text-muted-foreground"
						>
							起始日期
						</label>
						<input
							id="date-from"
							type="date"
							value={dateRange?.from ?? ""}
							onChange={handleFromDateChange}
							className="w-full rounded-lg border border-border/50 bg-background px-3 py-2 text-sm outline-none focus:border-primary"
						/>
					</div>
					<div>
						<label
							htmlFor="date-to"
							className="mb-1 block text-xs text-muted-foreground"
						>
							结束日期
						</label>
						<input
							id="date-to"
							type="date"
							value={dateRange?.to ?? ""}
							onChange={handleToDateChange}
							className="w-full rounded-lg border border-border/50 bg-background px-3 py-2 text-sm outline-none focus:border-primary"
						/>
					</div>
				</div>
			</div>

			{dateRange && (
				<div
					className="border-t p-3"
					style={{ borderColor: "var(--flexoki-bg-2)" }}
				>
					<div className="flex items-center justify-between">
						<p className="text-xs text-muted-foreground">
							{dateRange.from} 至 {dateRange.to}
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
