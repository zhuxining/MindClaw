import { TasksPanel } from "@/components/tasks/TasksPanel";
import { PinPanel } from "./PinPanel";
import { RelevancePanel } from "./RelevancePanel";

export function RightPanels() {
	return (
		<div className="flex h-full flex-col overflow-hidden">
			{/* Pin：20% */}
			<div className="h-[20%] overflow-hidden">
				<PinPanel />
			</div>

			<div className="h-px shrink-0 bg-border" />

			{/* Tasks：50% */}
			<div className="flex-1 overflow-hidden">
				<TasksPanel />
			</div>

			<div className="h-px shrink-0 bg-border" />

			{/* Relevance：30% */}
			<div className="h-[30%] overflow-hidden">
				<RelevancePanel />
			</div>
		</div>
	);
}
