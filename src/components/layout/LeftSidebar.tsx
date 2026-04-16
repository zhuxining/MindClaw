import { DirectoryPanel } from "./DirectoryPanel";
import { TabNav } from "./TabNav";
import { PanelFrame } from "./workspace-chrome";

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
