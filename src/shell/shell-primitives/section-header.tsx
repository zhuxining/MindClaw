import { cn } from "@/lib/utils";

export function SectionHeader({
	title,
	description,
	actions,
	className,
}: {
	title: string;
	description?: string;
	actions?: React.ReactNode;
	className?: string;
}) {
	return (
		<div
			className={cn(
				"flex min-h-14 items-center justify-between gap-3 border-b border-border/70 px-4 py-3",
				className,
			)}
		>
			<div className="min-w-0">
				<p className="truncate text-sm font-semibold text-foreground">
					{title}
				</p>
				{description ? (
					<p className="truncate text-xs text-muted-foreground">
						{description}
					</p>
				) : null}
			</div>
			{actions ? (
				<div className="flex items-center gap-1">{actions}</div>
			) : null}
		</div>
	);
}
