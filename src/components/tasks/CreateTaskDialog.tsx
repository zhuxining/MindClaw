import { useState } from "react";
import {
	Dialog,
	DialogContent,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import type { TaskPriority } from "@/lib/types";
import { useCreateTaskMutation } from "@/queries/tasks";

interface CreateTaskDialogProps {
	open: boolean;
	onClose: () => void;
}

export function CreateTaskDialog({ open, onClose }: CreateTaskDialogProps) {
	const [title, setTitle] = useState("");
	const [priority, setPriority] = useState<TaskPriority>("medium");
	const [dueDate, setDueDate] = useState("");
	const [titleError, setTitleError] = useState(false);

	const createTask = useCreateTaskMutation();

	function handleSubmit() {
		if (!title.trim()) {
			setTitleError(true);
			return;
		}
		createTask.mutate(
			{
				title: title.trim(),
				priority,
				...(dueDate ? { due_date: dueDate } : {}),
			},
			{
				onSuccess: () => {
					setTitle("");
					setPriority("medium");
					setDueDate("");
					setTitleError(false);
					onClose();
				},
			},
		);
	}

	function handleKeyDown(e: React.KeyboardEvent) {
		if (e.key === "Enter") handleSubmit();
		if (e.key === "Escape") onClose();
	}

	return (
		<Dialog open={open} onOpenChange={(v) => !v && onClose()}>
			<DialogContent className="max-w-sm">
				<DialogHeader>
					<DialogTitle>新建任务</DialogTitle>
				</DialogHeader>

				<div className="flex flex-col gap-4">
					<div>
						<input
							autoFocus
							type="text"
							placeholder="任务标题（必填）"
							value={title}
							onChange={(e) => {
								setTitle(e.target.value);
								setTitleError(false);
							}}
							onKeyDown={handleKeyDown}
							className={`w-full rounded border px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring ${
								titleError
									? "border-destructive focus:ring-destructive"
									: "border-border"
							}`}
						/>
						{titleError && (
							<p className="mt-1 text-xs text-destructive">请输入任务标题</p>
						)}
					</div>

					<div className="flex items-center gap-3">
						<label
							htmlFor="task-priority"
							className="text-xs text-muted-foreground w-12"
						>
							优先级
						</label>
						<select
							id="task-priority"
							value={priority}
							onChange={(e) => setPriority(e.target.value as TaskPriority)}
							className="flex-1 rounded border border-border px-2 py-1.5 text-sm outline-none focus:ring-1 focus:ring-ring bg-background"
						>
							<option value="high">高</option>
							<option value="medium">中</option>
							<option value="low">低</option>
						</select>
					</div>

					<div className="flex items-center gap-3">
						<label
							htmlFor="task-due-date"
							className="text-xs text-muted-foreground w-12"
						>
							截止日
						</label>
						<input
							id="task-due-date"
							type="date"
							value={dueDate}
							onChange={(e) => setDueDate(e.target.value)}
							className="flex-1 rounded border border-border px-2 py-1.5 text-sm outline-none focus:ring-1 focus:ring-ring bg-background"
						/>
					</div>

					<div className="flex justify-end gap-2">
						<button
							type="button"
							onClick={onClose}
							className="rounded px-3 py-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors"
						>
							取消
						</button>
						<button
							type="button"
							onClick={handleSubmit}
							disabled={createTask.isPending}
							className="rounded bg-primary px-3 py-1.5 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50 transition-colors"
						>
							创建
						</button>
					</div>
				</div>
			</DialogContent>
		</Dialog>
	);
}
