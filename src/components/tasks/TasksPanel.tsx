import { Plus } from "lucide-react";
import { useState } from "react";
import type { Task } from "@/lib/types";
import { useTasksQuery, useUpdateTaskStatusMutation } from "@/queries/tasks";
import { CreateTaskDialog } from "./CreateTaskDialog";
import { TaskItem } from "./TaskItem";

function todayStr() {
	return new Date().toISOString().slice(0, 10);
}

export function TasksPanel() {
	const [createOpen, setCreateOpen] = useState(false);
	const { data: tasks = [], isLoading } = useTasksQuery();
	const updateStatus = useUpdateTaskStatusMutation();

	const today = todayStr();
	const activeTasks = tasks.filter(
		(t) => t.status !== "done" && t.status !== "cancelled",
	);

	const todayTasks = activeTasks.filter((t) => t.due_date === today);
	const inProgressTasks = activeTasks.filter(
		(t) => t.status === "in_progress" && t.due_date !== today,
	);
	const todoTasks = activeTasks.filter(
		(t) => t.status === "todo" && t.due_date !== today,
	);

	function handleToggle(task: Task) {
		updateStatus.mutate({ id: task.id, status: "done" });
	}

	return (
		<div className="flex h-full flex-col">
			<div className="flex items-center justify-between border-b border-border px-3 py-2">
				<span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
					任务
				</span>
				<button
					type="button"
					onClick={() => setCreateOpen(true)}
					title="新建任务"
					className="rounded p-0.5 text-muted-foreground hover:text-foreground transition-colors"
				>
					<Plus className="h-4 w-4" />
				</button>
			</div>

			<div className="flex-1 overflow-y-auto">
				{isLoading ? (
					<div className="px-3 py-2 text-xs text-muted-foreground">加载中…</div>
				) : activeTasks.length === 0 ? (
					<div className="px-3 py-2 text-xs text-muted-foreground">
						暂无任务
					</div>
				) : (
					<>
						<TaskGroup
							label="今日到期"
							tasks={todayTasks}
							onToggle={handleToggle}
						/>
						<TaskGroup
							label="进行中"
							tasks={inProgressTasks}
							onToggle={handleToggle}
						/>
						<TaskGroup label="待办" tasks={todoTasks} onToggle={handleToggle} />
					</>
				)}
			</div>

			<CreateTaskDialog
				open={createOpen}
				onClose={() => setCreateOpen(false)}
			/>
		</div>
	);
}

function TaskGroup({
	label,
	tasks,
	onToggle,
}: {
	label: string;
	tasks: Task[];
	onToggle: (task: Task) => void;
}) {
	if (tasks.length === 0) return null;

	return (
		<div>
			<div className="px-3 py-1.5 text-xs text-muted-foreground font-medium">
				{label} <span className="opacity-60">({tasks.length})</span>
			</div>
			{tasks.map((task) => (
				<TaskItem key={task.id} task={task} onToggle={onToggle} />
			))}
		</div>
	);
}
