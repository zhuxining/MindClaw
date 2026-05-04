import { openUrl } from "@tauri-apps/plugin-opener";
import {
	ArrowLeft,
	ArrowRight,
	ExternalLink,
	ImageIcon,
	Link2,
	NotebookText,
} from "lucide-react";
import { useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { ChatWindow } from "@/features/agent-session/chat-window";
import { NoteEditor } from "@/features/editor/note-editor";
import {
	ImagePreview,
	openSourceExternally,
	PdfViewer,
	WebPreview,
} from "@/features/resource-preview/source-preview";
import { formatLocalDate } from "@/lib/date";
import type { EditorSaveState } from "@/lib/types";
import { descriptorToOpenedItem, openedItemToDescriptor } from "@/lib/types";
import { todayDate } from "@/queries/daily";
import { useSettingsQuery } from "@/queries/settings";
import {
	ContentHeader,
	EmptyState,
	PanelFrame,
	StatusBadge,
} from "@/shell/shell-primitives";
import { useShellStore } from "@/stores/shell";
import { useTabStore } from "@/stores/tabs";

function offsetDate(date: string, days: number): string {
	const current = new Date(date);
	current.setDate(current.getDate() + days);
	return formatLocalDate(current);
}

export function CenterContent() {
	const activeTabId = useTabStore((s) => s.activeTabId);
	const openTabs = useTabStore((s) => s.openTabs);
	const openTab = useTabStore((s) => s.openTab);
	const markDirty = useTabStore((s) => s.markDirty);
	const markClean = useTabStore((s) => s.markClean);
	const closeTab = useTabStore((s) => s.closeTab);

	const openedItem = useShellStore((s) => s.openedItem);
	const openItem = useShellStore((s) => s.openItem);

	const { data: settings } = useSettingsQuery();
	const [saveState, setSaveState] = useState<EditorSaveState>("idle");

	// Sync save state to tab dirty flag
	const handleSaveStateChange = useMemo(() => {
		return (state: EditorSaveState) => {
			setSaveState(state);
			const tabId = useTabStore.getState().activeTabId;
			if (!tabId) return;
			if (state === "saved") {
				markClean(tabId);
			} else if (state === "saving" || state === "error") {
				markDirty(tabId);
			}
		};
	}, [markClean, markDirty]);

	// Derive active content from tab or shell state
	const activeContent = useMemo(() => {
		// If we have an active tab, prefer it
		if (activeTabId) {
			const activeTab = openTabs.find((t) => t.id === activeTabId);
			if (activeTab) return activeTab.descriptor;
		}
		// Fall back to shell's openedItem (legacy mode)
		if (openedItem) {
			return openedItemToDescriptor(openedItem);
		}
		return null;
	}, [activeTabId, openTabs, openedItem]);

	// Get the legacy OpenedItem for rendering (needed for type-specific props)
	const renderItem = useMemo(() => {
		if (!activeContent) return null;
		// If openedItem matches, use it directly
		if (openedItem) {
			const itemDescriptor = openedItemToDescriptor(openedItem);
			if (itemDescriptor.path === activeContent.path) {
				return openedItem;
			}
		}
		// Convert from descriptor
		return descriptorToOpenedItem(activeContent);
	}, [activeContent, openedItem]);

	const header = useMemo(() => {
		if (!renderItem) return null;

		const base = {
			subtitle: renderItem.path,
			status: <StatusBadge state={saveState} />,
		};

		switch (renderItem.type) {
			case "daily":
				return {
					...base,
					eyebrow: "Daily Note",
					title: renderItem.date,
					leading: <NotebookText className="h-5 w-5 text-muted-foreground" />,
					actions: (
						<>
							<Button
								variant="outline"
								size="sm"
								onClick={() => {
									const prevDate = offsetDate(renderItem.date, -1);
									const prevItem = {
										type: "daily" as const,
										date: prevDate,
										path: `daily/${prevDate}.md`,
									};
									openItem(prevItem);
									openTab({
										type: "daily-note",
										path: prevItem.path,
										title: prevDate,
										meta: { date: prevDate },
									});
								}}
							>
								<ArrowLeft className="h-4 w-4" />
								前一天
							</Button>
							<Button
								variant="outline"
								size="sm"
								disabled={renderItem.date >= todayDate()}
								onClick={() => {
									const nextDate = offsetDate(renderItem.date, 1);
									const nextItem = {
										type: "daily" as const,
										date: nextDate,
										path: `daily/${nextDate}.md`,
									};
									openItem(nextItem);
									openTab({
										type: "daily-note",
										path: nextItem.path,
										title: nextDate,
										meta: { date: nextDate },
									});
								}}
							>
								后一天
								<ArrowRight className="h-4 w-4" />
							</Button>
						</>
					),
				};
			case "note":
				return {
					...base,
					eyebrow: "Markdown Note",
					title: renderItem.title,
					leading: <NotebookText className="h-5 w-5 text-muted-foreground" />,
					actions: null,
				};
			case "source-web":
				return {
					...base,
					eyebrow: "Resource Preview",
					title: renderItem.title,
					leading: <Link2 className="h-5 w-5 text-muted-foreground" />,
					actions: (
						<Button
							variant="outline"
							size="sm"
							onClick={() => void openUrl(renderItem.url)}
						>
							<ExternalLink className="h-4 w-4" />
							浏览器打开
						</Button>
					),
				};
			case "source-pdf":
				return {
					...base,
					eyebrow: "PDF Preview",
					title: renderItem.title,
					leading: <NotebookText className="h-5 w-5 text-muted-foreground" />,
					actions: settings?.vault_path ? (
						<Button
							variant="outline"
							size="sm"
							onClick={() =>
								void openSourceExternally(settings.vault_path, renderItem.path)
							}
						>
							<ExternalLink className="h-4 w-4" />
							系统打开
						</Button>
					) : null,
				};
			case "source-image":
				return {
					...base,
					eyebrow: "Image Preview",
					title: renderItem.title,
					leading: <ImageIcon className="h-5 w-5 text-muted-foreground" />,
					actions: settings?.vault_path ? (
						<Button
							variant="outline"
							size="sm"
							onClick={() =>
								void openSourceExternally(settings.vault_path, renderItem.path)
							}
						>
							<ExternalLink className="h-4 w-4" />
							系统打开
						</Button>
					) : null,
				};
		}
	}, [renderItem, openItem, openTab, saveState, settings?.vault_path]);

	return (
		<div className="h-full p-3 pl-1.5">
			<PanelFrame className="overflow-hidden">
				{renderItem && header ? (
					<ContentHeader
						eyebrow={header.eyebrow}
						title={header.title}
						subtitle={header.subtitle}
						status={header.status}
						actions={header.actions}
						leading={header.leading}
					/>
				) : null}

				<div className="min-h-0 flex-1 overflow-hidden">
					{!activeContent ? (
						<EmptyState
							title="从左侧选择一个文件"
							description="Daily 会默认打开，其他内容也可以从目录树或搜索结果进入。"
						/>
					) : null}

					{activeContent?.type === "agent-session" ? (
						<div className="flex h-full items-center justify-center p-4">
							<ChatWindow
								onClose={() => {
									if (activeTabId) {
										closeTab(activeTabId);
									}
								}}
							/>
						</div>
					) : null}

					{renderItem?.type === "daily" ? (
						<NoteEditor
							key={renderItem.path}
							date={renderItem.date}
							path={renderItem.path}
							onSaveStateChange={handleSaveStateChange}
						/>
					) : null}

					{renderItem?.type === "note" ? (
						<NoteEditor
							key={renderItem.path}
							path={renderItem.path}
							onSaveStateChange={handleSaveStateChange}
						/>
					) : null}

					{renderItem?.type === "source-web" ? (
						<WebPreview url={renderItem.url} />
					) : null}

					{renderItem?.type === "source-pdf" ? (
						<PdfViewer path={renderItem.path} />
					) : null}

					{renderItem?.type === "source-image" ? (
						<ImagePreview path={renderItem.path} title={renderItem.title} />
					) : null}
				</div>
			</PanelFrame>
		</div>
	);
}
