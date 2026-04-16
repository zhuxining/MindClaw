import { openUrl } from "@tauri-apps/plugin-opener";
import {
	ArrowLeft,
	ArrowRight,
	ExternalLink,
	ImageIcon,
	Link2,
	NotebookText,
	Pin,
	PinOff,
} from "lucide-react";
import { useMemo, useState } from "react";
import { NoteEditor } from "@/components/editor/NoteEditor";
import { Button } from "@/components/ui/button";
import { formatLocalDate } from "@/lib/date";
import type { EditorSaveState } from "@/lib/types";
import { todayDate } from "@/queries/daily";
import { useSettingsQuery } from "@/queries/settings";
import { useWorkspaceStore } from "@/stores/workspace";
import {
	ImagePreview,
	openSourceExternally,
	PdfViewer,
	WebPreview,
} from "./source-preview";
import {
	ContentHeader,
	EmptyState,
	PanelFrame,
	StatusBadge,
} from "./workspace-chrome";

function offsetDate(date: string, days: number): string {
	const current = new Date(date);
	current.setDate(current.getDate() + days);
	return formatLocalDate(current);
}

export function CenterContent() {
	const openedItem = useWorkspaceStore((state) => state.openedItem);
	const openItem = useWorkspaceStore((state) => state.openItem);
	const pinnedNote = useWorkspaceStore((state) => state.pinnedNote);
	const setPinnedNote = useWorkspaceStore((state) => state.setPinnedNote);
	const { data: settings } = useSettingsQuery();
	const [saveState, setSaveState] = useState<EditorSaveState>("idle");

	const isPinned =
		openedItem !== null &&
		openedItem.type === "note" &&
		pinnedNote?.path === openedItem.path;

	const header = useMemo(() => {
		if (!openedItem) return null;

		const base = {
			subtitle: openedItem.path,
			status: <StatusBadge state={saveState} />,
		};

		switch (openedItem.type) {
			case "daily":
				return {
					...base,
					eyebrow: "Daily Note",
					title: openedItem.date,
					leading: <NotebookText className="h-5 w-5 text-muted-foreground" />,
					actions: (
						<>
							<Button
								variant="outline"
								size="sm"
								onClick={() =>
									openItem({
										type: "daily",
										date: offsetDate(openedItem.date, -1),
										path: `daily/${offsetDate(openedItem.date, -1)}.md`,
									})
								}
							>
								<ArrowLeft className="h-4 w-4" />
								前一天
							</Button>
							<Button
								variant="outline"
								size="sm"
								disabled={openedItem.date >= todayDate()}
								onClick={() =>
									openItem({
										type: "daily",
										date: offsetDate(openedItem.date, 1),
										path: `daily/${offsetDate(openedItem.date, 1)}.md`,
									})
								}
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
					title: openedItem.title,
					leading: <NotebookText className="h-5 w-5 text-muted-foreground" />,
					actions: (
						<Button
							variant={isPinned ? "secondary" : "outline"}
							size="sm"
							onClick={() =>
								setPinnedNote(
									isPinned
										? null
										: { path: openedItem.path, title: openedItem.title },
								)
							}
						>
							{isPinned ? (
								<PinOff className="h-4 w-4" />
							) : (
								<Pin className="h-4 w-4" />
							)}
							{isPinned ? "取消 Pin" : "Pin 到侧栏"}
						</Button>
					),
				};
			case "source-web":
				return {
					...base,
					eyebrow: "Resource Preview",
					title: openedItem.title,
					leading: <Link2 className="h-5 w-5 text-muted-foreground" />,
					actions: (
						<Button
							variant="outline"
							size="sm"
							onClick={() => void openUrl(openedItem.url)}
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
					title: openedItem.title,
					leading: <NotebookText className="h-5 w-5 text-muted-foreground" />,
					actions: settings?.vault_path ? (
						<Button
							variant="outline"
							size="sm"
							onClick={() =>
								void openSourceExternally(settings.vault_path, openedItem.path)
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
					title: openedItem.title,
					leading: <ImageIcon className="h-5 w-5 text-muted-foreground" />,
					actions: settings?.vault_path ? (
						<Button
							variant="outline"
							size="sm"
							onClick={() =>
								void openSourceExternally(settings.vault_path, openedItem.path)
							}
						>
							<ExternalLink className="h-4 w-4" />
							系统打开
						</Button>
					) : null,
				};
		}
	}, [
		isPinned,
		openedItem,
		openItem,
		saveState,
		setPinnedNote,
		settings?.vault_path,
	]);

	return (
		<div className="h-full p-3 pl-1.5">
			<PanelFrame className="overflow-hidden">
				{openedItem && header ? (
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
					{!openedItem ? (
						<EmptyState
							title="从左侧选择一个文件"
							description="Daily 会默认打开，其他内容也可以从目录树或搜索结果进入。"
						/>
					) : null}

					{openedItem?.type === "daily" ? (
						<NoteEditor
							key={openedItem.path}
							date={openedItem.date}
							path={openedItem.path}
							onSaveStateChange={setSaveState}
						/>
					) : null}

					{openedItem?.type === "note" ? (
						<NoteEditor
							key={openedItem.path}
							path={openedItem.path}
							onSaveStateChange={setSaveState}
						/>
					) : null}

					{openedItem?.type === "source-web" ? (
						<WebPreview url={openedItem.url} />
					) : null}

					{openedItem?.type === "source-pdf" ? (
						<PdfViewer path={openedItem.path} />
					) : null}

					{openedItem?.type === "source-image" ? (
						<ImagePreview path={openedItem.path} title={openedItem.title} />
					) : null}
				</div>
			</PanelFrame>
		</div>
	);
}
