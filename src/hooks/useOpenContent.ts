import type { ContentDescriptor, OpenedItem } from "@/lib/types";
import { openedItemToDescriptor } from "@/lib/types";
import { useShellStore } from "@/stores/shell";
import { useTabStore } from "@/stores/tabs";

/**
 * Hook that provides a unified open action syncing both ShellStore and TabStore.
 * Use this when opening content from legacy sources (file explorer, search results).
 */
export function useOpenContent() {
	const openItem = useShellStore((s) => s.openItem);
	const openTab = useTabStore((s) => s.openTab);

	/**
	 * Open content from legacy OpenedItem format.
	 * Syncs both ShellStore.openedItem and TabStore.openTabs.
	 */
	const openFromItem = (item: OpenedItem) => {
		openItem(item);
		openTab(openedItemToDescriptor(item));
	};

	/**
	 * Open content from ContentDescriptor format.
	 * Only updates TabStore (ShellStore.openedItem will be synced via rendering).
	 */
	const openFromDescriptor = (descriptor: ContentDescriptor) => {
		openTab(descriptor);
		// Note: ShellStore.openedItem is updated via CenterContent's renderItem logic
	};

	return { openFromItem, openFromDescriptor };
}
