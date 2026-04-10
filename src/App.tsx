import { useQuery } from "@tanstack/react-query";
import { AppShell } from "@/components/layout/AppShell";
import { VaultSetup } from "@/components/layout/VaultSetup";
import { useAgentEvents } from "@/hooks/useAgentEvents";
import { ipc } from "@/lib/ipc";

export default function App() {
	useAgentEvents();

	const { data: settings, isLoading } = useQuery({
		queryKey: ["settings"],
		queryFn: () => ipc.getSettings(),
		staleTime: Number.POSITIVE_INFINITY,
	});

	if (isLoading) {
		return (
			<div className="flex h-screen w-screen items-center justify-center bg-background text-muted-foreground text-sm">
				加载中…
			</div>
		);
	}

	if (!settings?.vault_path) {
		return <VaultSetup />;
	}

	return <AppShell />;
}
