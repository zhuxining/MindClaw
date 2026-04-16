import { useEffect, useRef } from "react";
import { ipc } from "@/lib/ipc";
import { useWorkspacePrefsQuery } from "@/queries/settings";
import { useChatStore } from "@/stores/chat";
import { useWorkspaceStore, workspacePrefsFromState } from "@/stores/workspace";

const SAVE_DELAY_MS = 300;

export function useWorkspacePrefsSync(enabled: boolean) {
	const { data: prefs } = useWorkspacePrefsQuery();
	const hydratedRef = useRef(false);
	const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

	// Hydration: load prefs from backend into stores
	useEffect(() => {
		if (!enabled || !prefs || hydratedRef.current) return;

		useWorkspaceStore.getState().hydrateFromPrefs(prefs);
		useChatStore.getState().setMode(prefs.chat_mode);
		hydratedRef.current = true;
	}, [enabled, prefs]);

	// Auto-save: subscribe to workspace store changes
	useEffect(() => {
		if (!enabled || !hydratedRef.current) return;

		const scheduleSave = () => {
			if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
			saveTimerRef.current = setTimeout(() => {
				const state = useWorkspaceStore.getState();
				if (!state.isHydrated) return;
				void ipc.saveWorkspacePrefs(
					workspacePrefsFromState(state, useChatStore.getState().mode),
				);
			}, SAVE_DELAY_MS);
		};

		const unsubscribe = useWorkspaceStore.subscribe(scheduleSave);

		return () => {
			unsubscribe();
			if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
		};
	}, [enabled]);
}
