import { DirectoryPanel } from "./DirectoryPanel";
import { PanelFrame } from "./shell-primitives";
import { TabNav } from "./TabNav";

export function LeftSidebar() {
	return (
		<div className="h-full p-3 pr-1.5">
			<PanelFrame className="overflow-hidden">
				<TabNav />
				<div className="min-h-0 flex-1 overflow-hidden">
					<DirectoryPanel />
				</div>
			</PanelFrame>
		</div>
	);
}
