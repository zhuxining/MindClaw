import { useMemo } from "react";
import type { ContentType } from "@/lib/types";
import { useTabStore } from "@/stores/tabs";

/**
 * Inspector context determines what panes are shown in Right Panel
 * based on the active tab's content type.
 */
export interface InspectorContext {
	/** Content type of the active tab */
	contentType: ContentType | null;
	/** Whether the content supports outline view */
	hasOutline: boolean;
	/** Whether the content supports frontmatter editing */
	hasFrontmatter: boolean;
	/** Whether the content has related files */
	hasRelatedContent: boolean;
	/** Whether this is an Agent Session */
	isAgentSession: boolean;
}

const MARKDOWN_TYPES: ContentType[] = ["daily-note", "markdown"];
const AGENT_TYPES: ContentType[] = ["agent-session", "agent-detail"];
const EMPTY_CONTEXT: InspectorContext = {
	contentType: null,
	hasOutline: false,
	hasFrontmatter: false,
	hasRelatedContent: false,
	isAgentSession: false,
};

/**
 * Hook that derives InspectorContext from the active tab.
 */
export function useInspectorContext(): InspectorContext {
	const activeTabId = useTabStore((s) => s.activeTabId);
	const openTabs = useTabStore((s) => s.openTabs);

	return useMemo(() => {
		if (!activeTabId) return EMPTY_CONTEXT;

		const activeTab = openTabs.find((t) => t.id === activeTabId);
		if (!activeTab) return EMPTY_CONTEXT;

		const contentType = activeTab.descriptor.type;

		// Markdown content: show outline, frontmatter, related files
		if (MARKDOWN_TYPES.includes(contentType)) {
			return {
				contentType,
				hasOutline: true,
				hasFrontmatter: true,
				hasRelatedContent: true,
				isAgentSession: false,
			};
		}

		// Agent Session: show agent-specific panes (future)
		if (AGENT_TYPES.includes(contentType)) {
			return {
				contentType,
				hasOutline: false,
				hasFrontmatter: false,
				hasRelatedContent: false,
				isAgentSession: true,
			};
		}

		// Other content types: no inspector panes
		return {
			contentType,
			hasOutline: false,
			hasFrontmatter: false,
			hasRelatedContent: false,
			isAgentSession: false,
		};
	}, [activeTabId, openTabs]);
}
