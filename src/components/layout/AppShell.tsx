import { ChatOverlay } from "@/components/chat/ChatOverlay";
import {
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup,
} from "@/components/ui/resizable";
import { useWorkspaceStore } from "@/stores/workspace";
import { CenterContent } from "./CenterContent";
import { LeftSidebar } from "./LeftSidebar";
import { RightPanels } from "./RightPanels";

export function AppShell() {
	const panelSizes = useWorkspaceStore((state) => state.panelSizes);
	const setPanelSizes = useWorkspaceStore((state) => state.setPanelSizes);
	const isHydrated = useWorkspaceStore((state) => state.isHydrated);

	// 在 hydration 完成前使用默认布局，避免异步配置导致面板大小异常
	const sizes = isHydrated ? panelSizes : { left: 22, center: 52, right: 26 };

	return (
		<div className="fixed inset-0 min-h-150 min-w-225 bg-background text-foreground">
			<div className="flex h-full flex-col bg-[radial-gradient(circle_at_top_left,rgba(15,23,42,0.05),transparent_32%),linear-gradient(180deg,rgba(255,255,255,0.84),rgba(248,250,252,0.98))] p-3">
				<ResizablePanelGroup
					key={isHydrated ? "hydrated" : "loading"}
					orientation="horizontal"
					className="h-full rounded-[28px] border border-white/70 bg-workspace shadow-[0_20px_60px_rgba(15,23,42,0.08)]"
					onLayoutChanged={(layout: { [id: string]: number }) => {
						setPanelSizes({
							left: layout.left ?? sizes.left,
							center: layout.center ?? sizes.center,
							right: layout.right ?? sizes.right,
						});
					}}
				>
					<ResizablePanel
						id="left"
						defaultSize={`${sizes.left}`}
						minSize="16"
						maxSize="34"
						className="min-w-55"
					>
						<LeftSidebar />
					</ResizablePanel>

					<ResizableHandle withHandle className="bg-transparent" />

					<ResizablePanel
						id="center"
						defaultSize={`${sizes.center}`}
						minSize="34"
						className="min-w-105"
					>
						<CenterContent />
					</ResizablePanel>

					<ResizableHandle withHandle className="bg-transparent" />

					<ResizablePanel
						id="right"
						defaultSize={`${sizes.right}`}
						minSize="18"
						maxSize="36"
						className="min-w-65"
					>
						<RightPanels />
					</ResizablePanel>
				</ResizablePanelGroup>
			</div>

			<ChatOverlay />
		</div>
	);
}
