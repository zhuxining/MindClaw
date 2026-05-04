import { ipc } from "@/lib/ipc";
import type { OpenedItem, VaultEntry } from "@/lib/types";

export function isMarkdownEntry(entry: VaultEntry) {
	return entry.name.toLowerCase().endsWith(".md");
}

export async function buildOpenedItemFromEntry(
	entry: VaultEntry,
): Promise<OpenedItem> {
	if (entry.is_dir) {
		throw new Error("cannot open a directory");
	}

	const lowerName = entry.name.toLowerCase();
	const title = entry.name.replace(/\.[^.]+$/, "");

	if (entry.path.startsWith("daily/") && lowerName.endsWith(".md")) {
		const date = entry.name.replace(/\.md$/i, "");
		return {
			type: "daily",
			date,
			path: entry.path,
		};
	}

	if (lowerName.endsWith(".md")) {
		return {
			type: "note",
			path: entry.path,
			title,
		};
	}

	return ipc.resolveSourceItem(entry.path);
}
