export function ContentHeader({
	title,
	subtitle,
	eyebrow,
	status,
	actions,
	leading,
}: {
	title: string;
	subtitle?: string;
	eyebrow?: string;
	status?: React.ReactNode;
	actions?: React.ReactNode;
	leading?: React.ReactNode;
}) {
	return (
		<header className="flex min-h-18 items-center justify-between gap-4 border-b border-border/70 bg-elevated/60 px-6 py-4 backdrop-blur-sm">
			<div className="flex min-w-0 items-center gap-3">
				{leading ? <div className="shrink-0">{leading}</div> : null}
				<div className="min-w-0 space-y-1">
					{eyebrow ? (
						<p className="text-[11px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">
							{eyebrow}
						</p>
					) : null}
					<div className="flex min-w-0 items-center gap-2">
						<h1 className="truncate text-lg font-semibold text-foreground">
							{title}
						</h1>
						{status}
					</div>
					{subtitle ? (
						<p className="truncate text-sm text-muted-foreground">{subtitle}</p>
					) : null}
				</div>
			</div>
			{actions ? (
				<div className="flex items-center gap-2">{actions}</div>
			) : null}
		</header>
	);
}
