import { ArrowLeft, CalendarClock, Plus } from "lucide-react";
import { useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { todayLocalDate } from "@/lib/date";
import type { Task } from "@/lib/types";
import {
	useTaskQuery,
	useTasksQuery,
	useUpdateTaskStatusMutation,
} from "@/queries/tasks";
import {
	EmptyState,
	PanelFrame,
	SectionHeader,
	StatusBadge,
} from "@/shell/shell-primitives";
import { CreateTaskDialog } from "./create-task-dialog";
import { TaskItem } from "./task-item";

function todayStr() {
	return todayLocalDate();
}

export function TasksPanel() {
	const [createOpen, setCreateOpen] = useState(false);
	const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
	const { data: tasks = [], isLoading } = useTasksQuery();
	const updateStatus = useUpdateTaskStatusMutation();
	const detailQuery = useTaskQuery(selectedTaskId ?? "");

	const grouped = useMemo(() => {
		const today = todayStr();
		const activeTasks: Task[] = [];
		const todayTasks: Task[] = [];
		const inProgressTasks: Task[] = [];
		const todoTasks: Task[] = [];

		for (const task of tasks) {
			if (task.status === "done" || task.status === "cancelled") continue;
			activeTasks.push(task);

			if (task.due_date === today) {
				todayTasks.push(task);
			} else if (task.status === "in_progress") {
				inProgressTasks.push(task);
			} else if (task.status === "todo") {
				todoTasks.push(task);
			}
		}

		return { activeTasks, todayTasks, inProgressTasks, todoTasks };
	}, [tasks]);

	function handleToggle(task: Task) {
		updateStatus.mutate({ id: task.id, status: "done" });
		if (selectedTaskId === task.id) {
			setSelectedTaskId(null);
		}
	}

	const selectedTask = detailQuery.data;

	return (
		<PanelFrame className="overflow-hidden">
			<SectionHeader
				title="Tasks"
				description={
					selectedTask ? "任务详情" : `${grouped.activeTasks.length} 条活跃任务`
				}
				actions={
					selectedTask ? (
						<Button
							variant="ghost"
							size="sm"
							onClick={() => setSelectedTaskId(null)}
						>
							<ArrowLeft className="h-4 w-4" />
							返回列表
						</Button>
					) : (
						<Button size="sm" onClick={() => setCreateOpen(true)}>
							<Plus className="h-4 w-4" />
							新建
						</Button>
					)
				}
			/>

			<div className="min-h-0 flex-1 overflow-y-auto py-3">
				{selectedTask ? (
					<TaskDetail task={selectedTask} />
				) : isLoading ? (
					<div className="px-4 py-3 text-sm text-muted-foreground">
						加载任务中…
					</div>
				) : grouped.activeTasks.length === 0 ? (
					<EmptyState
						title="暂无任务"
						description="把需要推进的事情记在这里，右侧面板会持续跟随你的工作流。"
						action={
							<Button size="sm" onClick={() => setCreateOpen(true)}>
								<Plus className="h-4 w-4" />
								创建第一条任务
							</Button>
						}
					/>
				) : (
					<>
						<TaskGroup
							label="今日到期"
							tasks={grouped.todayTasks}
							onToggle={handleToggle}
							onOpen={(task) => setSelectedTaskId(task.id)}
						/>
						<TaskGroup
							label="进行中"
							tasks={grouped.inProgressTasks}
							onToggle={handleToggle}
							onOpen={(task) => setSelectedTaskId(task.id)}
						/>
						<TaskGroup
							label="待办"
							tasks={grouped.todoTasks}
							onToggle={handleToggle}
							onOpen={(task) => setSelectedTaskId(task.id)}
						/>
					</>
				)}
			</div>

			<CreateTaskDialog
				open={createOpen}
				onClose={() => setCreateOpen(false)}
			/>
		</PanelFrame>
	);
}

function TaskGroup({
	label,
	tasks,
	onToggle,
	onOpen,
}: {
	label: string;
	tasks: Task[];
	onToggle: (task: Task) => void;
	onOpen: (task: Task) => void;
}) {
	if (tasks.length === 0) return null;

	return (
		<div className="mb-4">
			<div className="mb-2 px-4 text-xs font-semibold uppercase tracking-[0.12em] text-muted-foreground">
				{label} <span className="opacity-60">({tasks.length})</span>
			</div>
			<div className="space-y-1">
				{tasks.map((task) => (
					<TaskItem
						key={task.id}
						task={task}
						onToggle={onToggle}
						onOpen={onOpen}
					/>
				))}
			</div>
		</div>
	);
}

function TaskDetail({ task }: { task: Task }) {
	return (
		<div className="space-y-4 px-4 pb-4">
			<div className="rounded-2xl border border-border/70 bg-muted/40 p-4">
				<div className="flex items-start justify-between gap-3">
					<div>
						<h3 className="text-base font-semibold text-foreground">
							{task.title}
						</h3>
						<p className="mt-1 text-xs text-muted-foreground">{task.id}</p>
					</div>
					<StatusBadge state={task.status} />
				</div>

				<div className="mt-4 grid grid-cols-2 gap-3 text-xs text-muted-foreground">
					<div className="rounded-xl border border-border/70 bg-background px-3 py-2">
						<p className="mb-1 font-medium text-foreground">优先级</p>
						<p>{task.priority}</p>
					</div>
					<div className="rounded-xl border border-border/70 bg-background px-3 py-2">
						<p className="mb-1 font-medium text-foreground">截止时间</p>
						<p>{task.due_date ?? "未设置"}</p>
					</div>
				</div>
			</div>

			<div className="rounded-2xl border border-border/70 bg-background px-4 py-4">
				<div className="mb-3 flex items-center gap-2 text-sm font-medium text-foreground">
					<CalendarClock className="h-4 w-4 text-muted-foreground" />
					任务说明
				</div>
				{task.body ? (
					<p className="whitespace-pre-wrap text-sm leading-7 text-foreground">
						{task.body}
					</p>
				) : (
					<p className="text-sm text-muted-foreground">
						这条任务还没有补充说明。
					</p>
				)}
			</div>
		</div>
	);
}
