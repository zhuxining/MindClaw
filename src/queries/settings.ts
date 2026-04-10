import { useQuery } from "@tanstack/react-query";
import { ipc } from "@/lib/ipc";
import { queryKeys } from "./keys";

export function useSettingsQuery() {
	return useQuery({
		queryKey: queryKeys.settings.all,
		queryFn: () => ipc.getSettings(),
		staleTime: 60_000,
	});
}
