import { Button } from "@/components/ui/button";
import type { EditorSaveState } from "@/lib/types";
import { cn } from "@/lib/utils";

export function PanelFrame({
	className,
	children,
}: {
	className?: string;
	children: React.ReactNode;
}) {
	return (
		<section
			className={cn(
				"flex h-full min-h-0 flex-col rounded-2xl border border-border/70 bg-surface shadow-[0_1px_0_rgba(15,23,42,0.03)]",
				className,
			)}
		>
			{children}
		</section>
	);
}

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

export function PanelAction({
	children,
	className,
	...props
}: React.ComponentProps<typeof Button>) {
	return (
		<Button
			variant="ghost"
			size="icon-sm"
			className={cn("text-muted-foreground hover:text-foreground", className)}
			{...props}
		>
			{children}
		</Button>
	);
}

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

export function StatusBadge({
	state,
}: {
	state: EditorSaveState | "todo" | "in_progress" | "done" | "cancelled";
}) {
	const copy: Record<string, string> = {
		idle: "就绪",
		saving: "保存中",
		saved: "已保存",
		error: "保存失败",
		todo: "待办",
		in_progress: "进行中",
		done: "已完成",
		cancelled: "已取消",
	};

	return (
		<span
			className={cn(
				"inline-flex h-6 items-center rounded-full border px-2.5 text-[11px] font-medium",
				state === "saved" &&
					"border-emerald-200 bg-emerald-50 text-emerald-700",
				state === "saving" && "border-sky-200 bg-sky-50 text-sky-700",
				state === "error" && "border-red-200 bg-red-50 text-red-700",
				state === "idle" && "border-border bg-muted/70 text-muted-foreground",
				state === "todo" && "border-border bg-muted/70 text-muted-foreground",
				state === "in_progress" &&
					"border-amber-200 bg-amber-50 text-amber-700",
				state === "done" && "border-emerald-200 bg-emerald-50 text-emerald-700",
				state === "cancelled" &&
					"border-border bg-muted/70 text-muted-foreground",
			)}
		>
			{copy[state]}
		</span>
	);
}

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
