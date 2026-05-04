import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";

interface RibbonButtonProps {
	icon: React.ComponentType<{
		size?: number | string;
		strokeWidth?: number | string;
	}>;
	label: string;
	active?: boolean;
	onClick: () => void;
}

export function RibbonButton({
	icon: Icon,
	label,
	active,
	onClick,
}: RibbonButtonProps) {
	return (
		<Tooltip>
			<TooltipTrigger
				onClick={onClick}
				className={`flex h-7 w-7 items-center justify-center rounded-md border-0 transition-colors ${
					active ? "text-accent" : "text-muted-foreground hover:text-foreground"
				}`}
				style={{
					backgroundColor: active ? "var(--flexoki-bg-2)" : "transparent",
				}}
				aria-label={label}
			>
				<Icon size={16} strokeWidth={1.5} />
			</TooltipTrigger>
			<TooltipContent side="right" sideOffset={8}>
				{label}
			</TooltipContent>
		</Tooltip>
	);
}
