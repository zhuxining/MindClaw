import { useQuery } from "@tanstack/react-query";
import { MessageSquare, Trash2 } from "lucide-react";
import { ipc } from "@/lib/ipc";
import type { SessionListItem } from "@/lib/types";
import { useChatStore } from "@/stores/chat";

export function SessionListPane() {
	const currentSessionId = useChatStore((s) => s.currentSessionId);
	const setSessionId = useChatStore((s) => s.setSessionId);

	const {
		data: sessions = [],
		isLoading,
		refetch,
	} = useQuery({
		queryKey: ["sessions"],
		queryFn: () => ipc.listSessions(50),
	});

	async function handleDelete(sessionId: string) {
		try {
			await ipc.deleteSession(sessionId);
			// 如果删除的是当前会话，清除当前会话 ID
			if (currentSessionId === sessionId) {
				setSessionId(null);
			}
			await refetch();
		} catch (error) {
			console.error("Failed to delete session:", error);
		}
	}

	function handleSelect(session: SessionListItem) {
		setSessionId(session.id);
	}

	if (isLoading) {
		return (
			<div className="flex h-full items-center justify-center p-4 text-sm text-muted-foreground">
				加载会话列表…
			</div>
		);
	}

	if (sessions.length === 0) {
		return (
			<div className="flex h-full items-center justify-center p-4 text-sm text-muted-foreground">
				暂无会话记录
			</div>
		);
	}

	return (
		<div className="flex h-full flex-col">
			<div
				className="border-b p-4"
				style={{ borderColor: "var(--flexoki-bg-2)" }}
			>
				<p className="text-sm font-semibold text-foreground">会话历史</p>
				<p className="text-xs text-muted-foreground">选择会话继续对话</p>
			</div>

			<div className="min-h-0 flex-1 overflow-y-auto p-3">
				<ul className="space-y-2">
					{sessions.map((session) => {
						const isActive = currentSessionId === session.id;
						const updated = new Date(session.updated);
						const timeStr = formatRelativeTime(updated);

						return (
							<li key={session.id}>
								<div
									className={`rounded-lg border p-3 transition-colors ${
										isActive
											? "border-primary bg-accent/50"
											: "border-border/50 hover:bg-muted/50"
									}`}
								>
									<button
										type="button"
										onClick={() => handleSelect(session)}
										className="w-full text-left"
									>
										<div className="flex items-center gap-2">
											<MessageSquare className="h-4 w-4 text-muted-foreground" />
											<p className="truncate text-sm font-medium">
												{session.mode} 会话
											</p>
										</div>
										<p className="mt-1 text-xs text-muted-foreground">
											{timeStr} · {session.sender}
										</p>
									</button>
									<div className="mt-2 flex justify-end">
										<button
											type="button"
											onClick={() => handleDelete(session.id)}
											className="rounded-lg p-1 text-muted-foreground transition-colors hover:bg-destructive/10 hover:text-destructive"
											title="删除会话"
										>
											<Trash2 className="h-3.5 w-3.5" />
										</button>
									</div>
								</div>
							</li>
						);
					})}
				</ul>
			</div>
		</div>
	);
}

function formatRelativeTime(date: Date): string {
	const now = new Date();
	const diffMs = now.getTime() - date.getTime();
	const diffMins = Math.floor(diffMs / 60_000);
	const diffHours = Math.floor(diffMs / 3_600_000);
	const diffDays = Math.floor(diffMs / 86_400_000);

	if (diffMins < 1) return "刚刚";
	if (diffMins < 60) return `${diffMins} 分钟前`;
	if (diffHours < 24) return `${diffHours} 小时前`;
	if (diffDays < 7) return `${diffDays} 天前`;
	return date.toLocaleDateString("zh-CN");
}
