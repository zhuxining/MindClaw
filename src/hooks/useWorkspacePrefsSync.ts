import { useEffect, useRef } from "react";
import { ipc } from "@/lib/ipc";
import type { WorkspacePrefs } from "@/lib/types";
import { openedItemToDescriptor } from "@/lib/types";
import { useWorkspacePrefsQuery } from "@/queries/settings";
import { useShellStore } from "@/stores/shell";
import { useTabStore } from "@/stores/tabs";

const SAVE_DELAY_MS = 300;

function serializePrefs(): WorkspacePrefs {
	const shell = useShellStore.getState();
	const tabs = useTabStore.getState();
	return {
		active_workspace_id: shell.activeWorkspaceId,
		open_tabs: tabs.openTabs,
		active_tab_id: tabs.activeTabId,
		panel_sizes: shell.panelSizes,
		last_opened_item: shell.openedItem,
	};
}

export function useWorkspacePrefsSync(enabled: boolean) {
	const { data: prefs } = useWorkspacePrefsQuery();
	const hydratedRef = useRef(false);
	const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

	// Hydration: load prefs from backend into stores
	useEffect(() => {
		if (!enabled || !prefs || hydratedRef.current) return;

		// Hydrate ShellStore
		useShellStore.getState().hydrate({
			activeWorkspaceId: prefs.active_workspace_id ?? prefs.active_tab_id,
			panelSizes: prefs.panel_sizes,
			lastOpenedItem: prefs.last_opened_item,
		});

		// Hydrate TabStore
		if (prefs.open_tabs && prefs.open_tabs.length > 0) {
			const tabStore = useTabStore.getState();
			// Restore tabs by opening each one
			for (const tab of prefs.open_tabs) {
				tabStore.openTab(tab.descriptor);
			}
			// Set active tab
			if (prefs.active_tab_id) {
				tabStore.setActiveTab(prefs.active_tab_id);
			}
		} else if (prefs.last_opened_item) {
			// Fallback: open last item as a tab
			useTabStore
				.getState()
				.openTab(openedItemToDescriptor(prefs.last_opened_item));
		}

		hydratedRef.current = true;
	}, [enabled, prefs]);

	// Auto-save: subscribe to store changes
	useEffect(() => {
		if (!enabled || !hydratedRef.current) return;

		const scheduleSave = () => {
			if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
			saveTimerRef.current = setTimeout(() => {
				if (!useShellStore.getState().isHydrated) return;
				void ipc.saveWorkspacePrefs(serializePrefs());
			}, SAVE_DELAY_MS);
		};

		// Subscribe to all relevant stores
		const unsubShell = useShellStore.subscribe(scheduleSave);
		const unsubTabs = useTabStore.subscribe(scheduleSave);

		return () => {
			unsubShell();
			unsubTabs();
			if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
		};
	}, [enabled]);
}
