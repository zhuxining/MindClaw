import { AppShell } from "@/app/app-shell";
import { VaultSetup } from "@/app/vault-setup";
import { useAgentEvents } from "@/hooks/useAgentEvents";
import { useWorkspacePrefsSync } from "@/hooks/useWorkspacePrefsSync";
import { useSettingsQuery } from "@/queries/settings";

export default function App() {
	useAgentEvents();
	const { data: settings, isLoading } = useSettingsQuery();
	useWorkspacePrefsSync(Boolean(settings?.vault_path));

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
