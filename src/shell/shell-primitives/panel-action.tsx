import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

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
