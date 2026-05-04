import {
	BookmarkPlus,
	Bot,
	Brain,
	CalendarDays,
	CalendarPlus2,
	ClipboardPlus,
	FilePlus,
	FolderLock,
	FolderRoot,
	Inbox,
	ListTodo,
	MessageSquarePlus,
	MessagesSquare,
	Network,
	Plug,
	Settings,
	TimerReset,
	Zap,
} from "lucide-react";
import { useCallback } from "react";
import type { ContentDescriptor, WorkspaceId } from "@/lib/types";
import { useChatStore } from "@/stores/chat";
import { useShellStore } from "@/stores/shell";
import { useTabStore } from "@/stores/tabs";
import { RibbonButton } from "./ribbon-button";

interface RibbonEntry {
	id: string;
	icon: React.ComponentType<{
		size?: number | string;
		strokeWidth?: number | string;
	}>;
	label: string;
	workspace?: WorkspaceId;
	action?: boolean;
}

const ENTRIES: RibbonEntry[] = [
	{ id: "daily", icon: CalendarDays, label: "每日", workspace: "daily" },
	{ id: "inbox", icon: Inbox, label: "收件箱", workspace: "inbox" },
	{ id: "private", icon: FolderLock, label: "私密", workspace: "private" },
	{ id: "vault", icon: FolderRoot, label: "保险库", workspace: "vault" },
	{ id: "sep-1", icon: () => null, label: "", action: false },
	{ id: "open-today", icon: CalendarPlus2, label: "打开今日", action: true },
	{ id: "new-note", icon: FilePlus, label: "新建笔记", action: true },
	{
		id: "new-session",
		icon: MessageSquarePlus,
		label: "新建会话",
		action: true,
	},
	{ id: "new-task", icon: ClipboardPlus, label: "新建任务", action: true },
	{ id: "add-link", icon: BookmarkPlus, label: "添加链接", action: true },
	{ id: "sep-2", icon: () => null, label: "", action: false },
	{ id: "tasks", icon: ListTodo, label: "任务", workspace: "tasks" },
	{ id: "graph", icon: Network, label: "图谱", workspace: "graph" },
	{ id: "sep-3", icon: () => null, label: "", action: false },
	{ id: "agent", icon: Bot, label: "智能体", workspace: "agent" },
	{ id: "skills", icon: Zap, label: "技能", workspace: "skills" },
	{ id: "memory", icon: Brain, label: "记忆", workspace: "memory" },
	{ id: "mcp", icon: Plug, label: "MCP", workspace: "mcp" },
	{ id: "session", icon: MessagesSquare, label: "会话", workspace: "session" },
	{ id: "cron", icon: TimerReset, label: "Cron", workspace: "cron" },
	{ id: "sep-4", icon: () => null, label: "", action: false },
	{ id: "settings", icon: Settings, label: "设置", workspace: "settings" },
];

function getTodayDate(): string {
	return new Date().toISOString().split("T")[0];
}

function generateNoteTitle(): string {
	const now = new Date();
	const timestamp = now.toISOString().replace(/[:.]/g, "-").slice(0, 19);
	return `note-${timestamp}`;
}

export function Ribbon() {
	const activeId = useShellStore((s) => s.activeWorkspaceId);
	const setWorkspace = useShellStore((s) => s.setActiveWorkspace);
	const openItem = useShellStore((s) => s.openItem);
	const openTab = useTabStore((s) => s.openTab);
	const setSessionId = useChatStore((s) => s.setSessionId);
	const clearMessages = useChatStore((s) => s.clearMessages);

	const handleAction = useCallback(
		(actionId: string) => {
			switch (actionId) {
				case "open-today": {
					const today = getTodayDate();
					const path = `daily/${today}.md`;
					const item = { type: "daily" as const, date: today, path };
					const descriptor: ContentDescriptor = {
						type: "daily-note",
						path,
						title: today,
						meta: { date: today },
					};
					openItem(item);
					openTab(descriptor);
					setWorkspace("daily");
					break;
				}
				case "new-note": {
					const title = generateNoteTitle();
					const path = `notes/${title}.md`;
					const item = { type: "note" as const, path, title };
					const descriptor: ContentDescriptor = {
						type: "markdown",
						path,
						title,
					};
					openItem(item);
					openTab(descriptor);
					setWorkspace("vault");
					break;
				}
				case "new-session": {
					setSessionId(null);
					clearMessages();
					const descriptor: ContentDescriptor = {
						type: "agent-session",
						path: "",
						title: "新会话",
					};
					openTab(descriptor);
					setWorkspace("session");
					break;
				}
				case "new-task": {
					// TODO: 打开任务创建对话框
					setWorkspace("tasks");
					break;
				}
				case "add-link": {
					// TODO: 打开链接添加对话框
					break;
				}
			}
		},
		[openItem, openTab, setSessionId, clearMessages, setWorkspace],
	);

	return (
		<nav
			className="flex w-9 shrink-0 flex-col items-center gap-[6px] border-r py-2"
			style={{ borderColor: "var(--flexoki-bg-2)" }}
		>
			{ENTRIES.map((entry) => {
				if (entry.id.startsWith("sep-")) {
					return (
						<div
							key={entry.id}
							className="my-1 h-px w-6"
							style={{ backgroundColor: "var(--flexoki-bg-2)" }}
						/>
					);
				}

				return (
					<RibbonButton
						key={entry.id}
						icon={entry.icon}
						label={entry.label}
						active={entry.workspace === activeId}
						onClick={() => {
							if (entry.workspace) {
								setWorkspace(entry.workspace);
							} else if (entry.action) {
								handleAction(entry.id);
							}
						}}
					/>
				);
			})}
		</nav>
	);
}
