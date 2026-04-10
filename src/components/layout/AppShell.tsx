import { ChatOverlay } from "@/components/chat/ChatOverlay";
import {
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup,
} from "@/components/ui/resizable";
import { CenterContent } from "./CenterContent";
import { LeftSidebar } from "./LeftSidebar";
import { RightPanels } from "./RightPanels";

export function AppShell() {
	return (
		<div className="fixed inset-0 bg-background text-foreground">
			<ResizablePanelGroup orientation="horizontal" className="h-full">
				{/* 左侧：Tab 导航 + 目录 */}
				<ResizablePanel defaultSize={22} minSize={15} maxSize={40}>
					<LeftSidebar />
				</ResizablePanel>

				<ResizableHandle />

				{/* 中央：内容区 */}
				<ResizablePanel defaultSize={52} minSize={30}>
					<CenterContent />
				</ResizablePanel>

				<ResizableHandle />

				{/* 右侧：Pin / Tasks / Relevance */}
				<ResizablePanel defaultSize={26} minSize={16} maxSize={45}>
					<RightPanels />
				</ResizablePanel>
			</ResizablePanelGroup>

			<ChatOverlay />
		</div>
	);
}
