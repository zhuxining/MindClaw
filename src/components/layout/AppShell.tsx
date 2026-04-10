import { ChatOverlay } from "@/components/chat/ChatOverlay";
import { CenterContent } from "./CenterContent";
import { LeftSidebar } from "./LeftSidebar";
import { RightPanels } from "./RightPanels";

// TODO Phase 2: 使用 resizable.tsx 组件为左右侧边栏和右侧面板
// 三个区块添加可拖拽分隔线功能。见 PRD 01-workspace-shell.md US-06, US-09, US-10

export function AppShell() {
	return (
		<div className="fixed inset-0 flex overflow-hidden bg-background text-foreground">
			{/* 左侧：Tab 导航 + 目录 22% */}
			<div className="flex w-[22%] min-w-[200px] shrink-0 flex-col">
				<LeftSidebar />
			</div>

			<div className="w-px shrink-0 bg-border" />

			{/* 中央：内容区 */}
			<div className="flex min-w-0 flex-1 flex-col">
				<CenterContent />
			</div>

			<div className="w-px shrink-0 bg-border" />

			{/* 右侧：Pin / Tasks / Relevance 26% */}
			<div className="flex w-[26%] min-w-[240px] shrink-0 flex-col">
				<RightPanels />
			</div>

			<ChatOverlay />
		</div>
	);
}
