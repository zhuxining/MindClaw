import {
	Bot,
	Brain,
	CalendarDays,
	Clock,
	Filter,
	FolderOpen,
	Folders,
	Hash,
	MessageSquare,
	Plug,
	Puzzle,
	Tag,
} from "lucide-react";
import type { WorkspaceId } from "@/lib/types";
import { AgentListPane } from "@/shell/panes/agent-list-pane/agent-list-pane";
import { CalendarFilterPane } from "@/shell/panes/calendar-filter-pane/calendar-filter-pane";
import { CronJobListPane } from "@/shell/panes/cron-job-list-pane/cron-job-list-pane";
import { FileExplorerPane } from "@/shell/panes/file-explorer-pane/file-explorer-pane";
import { McpServerListPane } from "@/shell/panes/mcp-server-list-pane/mcp-server-list-pane";
import { MemoryListPane } from "@/shell/panes/memory-list-pane/memory-list-pane";
import { SavedFilterPane } from "@/shell/panes/saved-filter-pane/saved-filter-pane";
import { SessionListPane } from "@/shell/panes/session-list-pane/session-list-pane";
import { SkillListPane } from "@/shell/panes/skill-list-pane/skill-list-pane";
import { TagsFilterPane } from "@/shell/panes/tags-filter-pane/tags-filter-pane";
import { TypeFilterPane } from "@/shell/panes/type-filter-pane/type-filter-pane";
import type { WorkspaceDefinition } from "./workspace-definition";

function fileWorkspace(
	id: WorkspaceId & ("daily" | "inbox" | "private" | "vault"),
	label: string,
	scope: string,
	defaultPath: string,
	defaultTitle: string,
): WorkspaceDefinition {
	return {
		id,
		ribbonItem: { id, icon: CalendarDays, label },
		leftPanel: {
			defaultPane: "file-explorer",
			panes: [
				{
					id: "calendar-filter",
					label: "日历",
					icon: CalendarDays,
					render: () => <CalendarFilterPane />,
				},
				{
					id: "tags-filter",
					label: "标签",
					icon: Tag,
					render: () => <TagsFilterPane />,
				},
				{
					id: "type-filter",
					label: "类型",
					icon: Filter,
					render: () => <TypeFilterPane />,
				},
				{
					id: "saved-filter",
					label: "已存过滤",
					icon: Hash,
					render: () => <SavedFilterPane />,
				},
				{
					id: "file-explorer",
					label: "文件",
					icon: Folders,
					render: () => <FileExplorerPane scope={scope} />,
				},
			],
			filterToolbar: [
				{ id: "calendar-filter", icon: CalendarDays, label: "日历过滤" },
				{ id: "tags-filter", icon: Tag, label: "标签过滤" },
				{ id: "type-filter", icon: Filter, label: "类型过滤" },
				{ id: "saved-filter", icon: Hash, label: "已存过滤" },
				{ id: "file-explorer", icon: FolderOpen, label: "文件浏览" },
			],
		},
		defaultContent: {
			type: id === "daily" ? "daily-note" : "markdown",
			path: defaultPath,
			title: defaultTitle,
		},
		rightPanel: {
			defaultPane: "note-outline",
			panes: [],
			filterToolbar: [],
		},
		openBehavior: "new-tab",
	};
}

export const workspaceRegistry: Partial<
	Record<WorkspaceId, WorkspaceDefinition>
> = {
	daily: fileWorkspace("daily", "每日", "daily", "", "每日笔记"),
	inbox: fileWorkspace("inbox", "收件箱", "inbox", "", "收件箱"),
	private: fileWorkspace("private", "私密", "private", "", "私密"),
	vault: fileWorkspace("vault", "保险库", "", "", "保险库"),
	session: {
		id: "session",
		ribbonItem: { id: "session", icon: MessageSquare, label: "会话" },
		leftPanel: {
			defaultPane: "session-list",
			panes: [
				{
					id: "session-list",
					label: "会话列表",
					icon: MessageSquare,
					render: () => <SessionListPane />,
				},
			],
			filterToolbar: [
				{ id: "session-list", icon: MessageSquare, label: "会话列表" },
			],
		},
		defaultContent: {
			type: "agent-session",
			path: "",
			title: "新会话",
		},
		rightPanel: {
			defaultPane: "note-outline",
			panes: [],
			filterToolbar: [],
		},
		openBehavior: "new-tab",
	},
	memory: {
		id: "memory",
		ribbonItem: { id: "memory", icon: Brain, label: "记忆" },
		leftPanel: {
			defaultPane: "memory-list",
			panes: [
				{
					id: "memory-list",
					label: "记忆列表",
					icon: Brain,
					render: () => <MemoryListPane />,
				},
			],
			filterToolbar: [{ id: "memory-list", icon: Brain, label: "记忆列表" }],
		},
		defaultContent: {
			type: "markdown",
			path: "agent/memory/",
			title: "Agent 记忆",
		},
		rightPanel: {
			defaultPane: "note-outline",
			panes: [],
			filterToolbar: [],
		},
		openBehavior: "new-tab",
	},
	skills: {
		id: "skills",
		ribbonItem: { id: "skills", icon: Puzzle, label: "技能" },
		leftPanel: {
			defaultPane: "skill-list",
			panes: [
				{
					id: "skill-list",
					label: "技能列表",
					icon: Puzzle,
					render: () => <SkillListPane />,
				},
			],
			filterToolbar: [{ id: "skill-list", icon: Puzzle, label: "技能列表" }],
		},
		defaultContent: {
			type: "markdown",
			path: "skills/",
			title: "Agent 技能",
		},
		rightPanel: {
			defaultPane: "note-outline",
			panes: [],
			filterToolbar: [],
		},
		openBehavior: "new-tab",
	},
	agent: {
		id: "agent",
		ribbonItem: { id: "agent", icon: Bot, label: "Agent" },
		leftPanel: {
			defaultPane: "agent-list",
			panes: [
				{
					id: "agent-list",
					label: "Agent 配置",
					icon: Bot,
					render: () => <AgentListPane settings={null} />,
				},
			],
			filterToolbar: [{ id: "agent-list", icon: Bot, label: "Agent 配置" }],
		},
		defaultContent: {
			type: "agent-session",
			path: "",
			title: "Agent",
		},
		rightPanel: {
			defaultPane: "note-outline",
			panes: [],
			filterToolbar: [],
		},
		openBehavior: "new-tab",
	},
	mcp: {
		id: "mcp",
		ribbonItem: { id: "mcp", icon: Plug, label: "MCP" },
		leftPanel: {
			defaultPane: "mcp-server-list",
			panes: [
				{
					id: "mcp-server-list",
					label: "MCP Server",
					icon: Plug,
					render: () => <McpServerListPane />,
				},
			],
			filterToolbar: [
				{ id: "mcp-server-list", icon: Plug, label: "MCP Server" },
			],
		},
		defaultContent: {
			type: "markdown",
			path: "",
			title: "MCP",
		},
		rightPanel: {
			defaultPane: "note-outline",
			panes: [],
			filterToolbar: [],
		},
		openBehavior: "new-tab",
	},
	cron: {
		id: "cron",
		ribbonItem: { id: "cron", icon: Clock, label: "定时" },
		leftPanel: {
			defaultPane: "cron-job-list",
			panes: [
				{
					id: "cron-job-list",
					label: "定时任务",
					icon: Clock,
					render: () => <CronJobListPane />,
				},
			],
			filterToolbar: [{ id: "cron-job-list", icon: Clock, label: "定时任务" }],
		},
		defaultContent: {
			type: "markdown",
			path: "",
			title: "定时任务",
		},
		rightPanel: {
			defaultPane: "note-outline",
			panes: [],
			filterToolbar: [],
		},
		openBehavior: "new-tab",
	},
};

export function getWorkspace(id: WorkspaceId): WorkspaceDefinition | undefined {
	return workspaceRegistry[id];
}
