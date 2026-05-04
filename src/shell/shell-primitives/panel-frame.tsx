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
