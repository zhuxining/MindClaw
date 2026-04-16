import { useEffect, useRef, useState } from "react";
import { TasksPanel } from "@/components/tasks/TasksPanel";
import {
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup,
} from "@/components/ui/resizable";
import { useWorkspaceStore } from "@/stores/workspace";
import { PinPanel } from "./PinPanel";
import { RelevancePanel } from "./RelevancePanel";

export function RightPanels() {
	const rightPanelHeights = useWorkspaceStore(
		(state) => state.rightPanelHeights,
	);
	const collapseRelevanceRef = useRef(false);
	const [collapseRelevance, setCollapseRelevance] = useState(false);

	useEffect(() => {
		function syncViewport() {
			const shouldCollapse = window.innerHeight < 600;
			collapseRelevanceRef.current = shouldCollapse;
			setCollapseRelevance(shouldCollapse);
		}

		syncViewport();
		window.addEventListener("resize", syncViewport);
		return () => window.removeEventListener("resize", syncViewport);
	}, []);

	return (
		<div className="h-full p-3 pl-1.5">
			<ResizablePanelGroup
				orientation="vertical"
				className="h-full"
				onLayoutChanged={(layout: { [id: string]: number }) => {
					const state = useWorkspaceStore.getState();
					const heights = state.rightPanelHeights;
					state.setRightPanelHeights({
						pin: layout.pin ?? heights.pin,
						tasks: layout.tasks ?? heights.tasks,
						relevance: collapseRelevanceRef.current
							? heights.relevance
							: (layout.relevance ?? heights.relevance),
					});
				}}
			>
				<ResizablePanel
					id="pin"
					defaultSize={`${rightPanelHeights.pin}`}
					minSize="16"
					className="min-h-[120px]"
				>
					<PinPanel />
				</ResizablePanel>

				<ResizableHandle withHandle className="bg-transparent py-1" />

				<ResizablePanel
					id="tasks"
					defaultSize={`${collapseRelevance ? 80 : rightPanelHeights.tasks}`}
					minSize="28"
					className="min-h-[220px]"
				>
					<TasksPanel />
				</ResizablePanel>

				{collapseRelevance ? null : (
					<>
						<ResizableHandle withHandle className="bg-transparent py-1" />
						<ResizablePanel
							id="relevance"
							defaultSize={`${rightPanelHeights.relevance}`}
							minSize="18"
							className="min-h-[140px]"
						>
							<RelevancePanel />
						</ResizablePanel>
					</>
				)}
			</ResizablePanelGroup>
		</div>
	);
}
