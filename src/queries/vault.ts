import { useQuery } from "@tanstack/react-query";
import { ipc } from "@/lib/ipc";
import { queryKeys } from "./keys";

/** 列出 vault 内子目录（相对路径） */
export function useVaultDirQuery(path?: string) {
	return useQuery({
		queryKey: queryKeys.vault.dir(path),
		queryFn: () => ipc.listVaultDir(path),
		staleTime: 30_000,
	});
}

export function useVaultFlatQuery(path?: string) {
	return useQuery({
		queryKey: queryKeys.vault.flat(path),
		queryFn: () => ipc.listVaultFilesRecursive(path),
		staleTime: 30_000,
	});
}
