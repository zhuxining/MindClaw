import { invoke } from "@tauri-apps/api/core";
import { MessageSquare, RefreshCw, Trash2 } from "lucide-react";
import { useCallback, useEffect, useRef } from "react";
import { Button } from "../components/ui/button";
import { Card } from "../components/ui/card";
import { ScrollArea } from "../components/ui/scroll-area";
import type { ChannelMessage } from "../lib/types";
import { useMessageStore } from "../stores/message-store";

function formatTime(timestamp: number): string {
	const date = new Date(timestamp * 1000);
	return date.toLocaleTimeString("zh-CN", {
		hour: "2-digit",
		minute: "2-digit",
	});
}

function formatDate(timestamp: number): string {
	const date = new Date(timestamp * 1000);
	const now = new Date();
	const diff = now.getTime() - date.getTime();
	if (diff < 86400000) {
		return "今天";
	}
	if (diff < 172800000) {
		return "昨天";
	}
	return date.toLocaleDateString("zh-CN");
}

export default function MessageList() {
	const pollIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
	const {
		messages,
		processingResults,
		isPolling,
		pollInterval,
		feishuConnected,
		addMessages,
		setPolling,
		clearMessages,
	} = useMessageStore();

	const pollMessages = useCallback(async () => {
		if (!feishuConnected) return;
		try {
			setPolling(true);
			const newMessages = await invoke<ChannelMessage[]>(
				"poll_feishu_messages",
				{
					containerIdType: "chat",
					pageToken: null,
				},
			);
			if (newMessages.length > 0) {
				addMessages(newMessages);
			}
		} catch (err) {
			console.error("轮询消息失败:", err);
		} finally {
			setPolling(false);
		}
	}, [feishuConnected, addMessages, setPolling]);

	useEffect(() => {
		if (feishuConnected) {
			pollMessages();
			pollIntervalRef.current = setInterval(pollMessages, pollInterval * 1000);
		}
		return () => {
			if (pollIntervalRef.current) {
				clearInterval(pollIntervalRef.current);
			}
		};
	}, [feishuConnected, pollInterval, pollMessages]);

	return (
		<div className="flex flex-col h-full">
			{/* 工具栏 */}
			<div className="flex items-center justify-between px-4 py-3 border-b border-border">
				<div className="flex items-center gap-2">
					<MessageSquare className="w-4 h-4" />
					<span className="text-sm font-medium">消息流</span>
					{isPolling && (
						<span className="text-xs text-muted-foreground animate-pulse">
							轮询中...
						</span>
					)}
				</div>
				<div className="flex items-center gap-1">
					<Button
						variant="ghost"
						size="icon"
						onClick={pollMessages}
						disabled={!feishuConnected || isPolling}
						title="刷新消息"
					>
						<RefreshCw
							className={`w-4 h-4 ${isPolling ? "animate-spin" : ""}`}
						/>
					</Button>
					<Button
						variant="ghost"
						size="icon"
						onClick={clearMessages}
						disabled={messages.length === 0}
						title="清空消息"
					>
						<Trash2 className="w-4 h-4" />
					</Button>
				</div>
			</div>

			{/* 消息列表 */}
			<ScrollArea className="flex-1">
				{messages.length === 0 ? (
					<div className="flex flex-col items-center justify-center h-full text-muted-foreground gap-2 p-8">
						<MessageSquare className="w-12 h-12 opacity-20" />
						<p className="text-sm">
							{feishuConnected
								? "暂无消息，等待飞书新消息..."
								: "请先在设置中配置飞书连接"}
						</p>
					</div>
				) : (
					<div className="p-4 space-y-3">
						{messages.map((msg) => {
							const result = processingResults[msg.message_id];
							return (
								<MessageCard
									key={msg.message_id}
									message={msg}
									agentResult={result}
								/>
							);
						})}
					</div>
				)}
			</ScrollArea>
		</div>
	);
}

function MessageCard({
	message,
	agentResult,
}: {
	message: ChannelMessage;
	agentResult?: import("../lib/types").AgentResponse;
}) {
	return (
		<Card className="p-3 space-y-2">
			{/* 消息头 */}
			<div className="flex items-center justify-between">
				<div className="flex items-center gap-2">
					<span className="text-sm font-medium">{message.sender_name}</span>
					<span className="text-xs text-muted-foreground">
						{message.is_reply ? "🤖 Agent 回复" : `来自 ${message.channel}`}
					</span>
				</div>
				<div className="flex items-center gap-2">
					<span className="text-xs text-muted-foreground">
						{formatDate(message.timestamp)} {formatTime(message.timestamp)}
					</span>
					{agentResult && <StatusBadge status={agentResult.status} />}
				</div>
			</div>

			{/* 消息内容 */}
			<p className="text-sm whitespace-pre-wrap">{message.content}</p>

			{/* Agent 处理结果 */}
			{agentResult && (
				<div className="mt-2 p-2 rounded-md bg-muted/50 border border-border">
					<div className="flex items-center gap-2 mb-1">
						<span className="text-xs font-medium">Agent 处理结果</span>
						<StatusBadge status={agentResult.status} />
					</div>
					{agentResult.status === "success" && agentResult.output && (
						<p className="text-xs whitespace-pre-wrap text-muted-foreground">
							{agentResult.output}
						</p>
					)}
					{agentResult.error_message && (
						<p className="text-xs text-red-500">{agentResult.error_message}</p>
					)}
				</div>
			)}
		</Card>
	);
}

function StatusBadge({ status }: { status: "success" | "error" | "timeout" }) {
	const colors: Record<string, string> = {
		success:
			"bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400",
		error: "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400",
		timeout:
			"bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400",
	};
	const labels: Record<string, string> = {
		success: "完成",
		error: "失败",
		timeout: "超时",
	};
	return (
		<span
			className={`text-xs px-1.5 py-0.5 rounded-full ${colors[status] || ""}`}
		>
			{labels[status] || status}
		</span>
	);
}
