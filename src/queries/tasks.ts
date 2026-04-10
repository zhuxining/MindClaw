import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ipc } from "@/lib/ipc";
import type { CreateTaskParams, Task } from "@/lib/types";
import { queryKeys } from "./keys";

export function useTaskQuery(id: string) {
	return useQuery({
		queryKey: ["tasks", "detail", id],
		queryFn: () => ipc.getTask(id),
		enabled: !!id,
	});
}

export function useTasksQuery(status?: string) {
	return useQuery({
		queryKey: queryKeys.tasks.list(status),
		queryFn: () => ipc.listTasks(status),
	});
}

export function useCreateTaskMutation() {
	const qc = useQueryClient();
	return useMutation({
		mutationFn: (params: CreateTaskParams) => ipc.createTask(params),
		onSuccess: () => {
			qc.invalidateQueries({ queryKey: queryKeys.tasks.all });
		},
	});
}

export function useUpdateTaskMutation() {
	const qc = useQueryClient();
	return useMutation({
		mutationFn: ({
			id,
			...params
		}: { id: string } & Partial<CreateTaskParams & { status: string }>) =>
			ipc.updateTask(id, params),
		onSuccess: () => {
			qc.invalidateQueries({ queryKey: queryKeys.tasks.all });
		},
	});
}

export function useDeleteTaskMutation() {
	const qc = useQueryClient();
	return useMutation({
		mutationFn: (id: string) => ipc.deleteTask(id),
		onSuccess: () => {
			qc.invalidateQueries({ queryKey: queryKeys.tasks.all });
		},
	});
}

export function useUpdateTaskStatusMutation() {
	const qc = useQueryClient();
	return useMutation({
		mutationFn: ({ id, status }: { id: string; status: string }) =>
			ipc.updateTaskStatus(id, status),
		onMutate: async ({ id, status }) => {
			// Cancel outgoing refetches so they don't overwrite our optimistic update
			await qc.cancelQueries({ queryKey: queryKeys.tasks.all });

			// Snapshot current cache
			const previous = qc.getQueriesData({ queryKey: queryKeys.tasks.list() });

			// Optimistically update all task list queries
			qc.setQueriesData(
				{ queryKey: queryKeys.tasks.list() },
				(old: Task[] | undefined) => {
					if (!old) return old;
					return old
						.map((t) =>
							t.id === id ? { ...t, status: status as Task["status"] } : t,
						)
						.filter((t) => t.status !== "done"); // done tasks disappear from the panel
				},
			);

			return { previous };
		},
		onError: (_err, _vars, context) => {
			// Roll back on failure
			if (context?.previous) {
				for (const [key, value] of context.previous) {
					qc.setQueryData(key, value);
				}
			}
		},
		onSettled: () => {
			qc.invalidateQueries({ queryKey: queryKeys.tasks.all });
		},
	});
}
