import type { ReactNode } from "react";

export interface PaneDefinition {
	id: string;
	label: string;
	icon?: React.ComponentType<{ size?: number | string }>;
	render: () => ReactNode;
}

interface PaneHostProps {
	panes: PaneDefinition[];
	activePaneId: string;
	children?: ReactNode;
}

export function PaneHost({ panes, activePaneId }: PaneHostProps) {
	return (
		<div className="min-h-0 flex-1 overflow-hidden">
			{panes.map((pane) => (
				<div
					key={pane.id}
					className="h-full overflow-auto"
					style={{ display: pane.id === activePaneId ? undefined : "none" }}
				>
					{pane.render()}
				</div>
			))}
		</div>
	);
}
