import { cn } from "@/lib/utils";

export interface PaneFilterEntry {
	id: string;
	icon: React.ComponentType<{ size?: number | string }>;
	label: string;
}

interface PaneFilterToolbarProps {
	filters: PaneFilterEntry[];
	activeFilter: string;
	onFilterChange: (id: string) => void;
}

export function PaneFilterToolbar({
	filters,
	activeFilter,
	onFilterChange,
}: PaneFilterToolbarProps) {
	return (
		<div
			className="flex shrink-0 items-center gap-0.5 border-t px-2"
			style={{ height: 40, borderColor: "var(--flexoki-bg-2)" }}
		>
			{filters.map((f) => (
				<button
					key={f.id}
					type="button"
					onClick={() => onFilterChange(f.id)}
					className={cn(
						"flex h-7 w-7 items-center justify-center rounded-md border-0 transition-colors",
						activeFilter === f.id
							? "text-accent"
							: "text-muted-foreground hover:text-foreground",
					)}
					style={{
						backgroundColor:
							activeFilter === f.id ? "var(--flexoki-bg-2)" : "transparent",
					}}
					aria-label={f.label}
				>
					<f.icon size={14} />
				</button>
			))}
		</div>
	);
}
