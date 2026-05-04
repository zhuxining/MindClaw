import { cn } from "@/lib/utils";

export function EmptyState({
	title,
	description,
	action,
	className,
}: {
	title: string;
	description?: string;
	action?: React.ReactNode;
	className?: string;
}) {
	return (
		<div
			className={cn(
				"flex h-full min-h-0 flex-col items-center justify-center px-6 py-8 text-center",
				className,
			)}
		>
			<div className="max-w-xs space-y-2">
				<p className="text-sm font-medium text-foreground">{title}</p>
				{description ? (
					<p className="text-xs leading-6 text-muted-foreground">
						{description}
					</p>
				) : null}
				{action ? <div className="pt-2">{action}</div> : null}
			</div>
		</div>
	);
}
