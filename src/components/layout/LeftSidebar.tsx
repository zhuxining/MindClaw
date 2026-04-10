import { DirectoryPanel } from "./DirectoryPanel";
import { TabNav } from "./TabNav";

export function LeftSidebar() {
	return (
		<div className="flex h-full flex-col border-r border-border">
			<TabNav />
			<div className="flex-1 overflow-hidden">
				<DirectoryPanel />
			</div>
		</div>
	);
}
