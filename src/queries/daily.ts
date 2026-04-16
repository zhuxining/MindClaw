import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { todayLocalDate } from "@/lib/date";
import { ipc } from "@/lib/ipc";
import { queryKeys } from "./keys";

export function useDailyQuery(date: string) {
	return useQuery({
		queryKey: queryKeys.daily.byDate(date),
		queryFn: () => ipc.getDaily(date),
	});
}

export function useSaveDailyMutation() {
	const qc = useQueryClient();
	return useMutation({
		mutationFn: ({ date, content }: { date: string; content: string }) =>
			ipc.saveDaily(date, content),
		onSuccess: (_, { date }) => {
			qc.invalidateQueries({ queryKey: queryKeys.daily.byDate(date) });
		},
	});
}

export function todayDate(): string {
	return todayLocalDate();
}
