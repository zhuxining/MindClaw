import { Send } from "lucide-react";
import { useRef, useState } from "react";
import { ipc } from "@/lib/ipc";
import { useChatStore } from "@/stores/chat";
import { useWorkspaceStore } from "@/stores/workspace";

export function ChatInput() {
	const [content, setContent] = useState("");
	const textareaRef = useRef<HTMLTextAreaElement>(null);
	const isSendingRef = useRef(false);

	const streamingRequestId = useChatStore((s) => s.streamingRequestId);
	const currentSessionId = useChatStore((s) => s.currentSessionId);
	const mode = useChatStore((s) => s.mode);
	const addUserMessage = useChatStore((s) => s.addUserMessage);
	const startStreaming = useChatStore((s) => s.startStreaming);
	const setError = useChatStore((s) => s.setError);
	const openChat = useWorkspaceStore((s) => s.openChat);

	const isStreaming = streamingRequestId !== null;

	async function handleSend() {
		const text = content.trim();
		if (!text || isStreaming || isSendingRef.current) return;

		isSendingRef.current = true;
		setContent("");
		// 重置 textarea 高度
		if (textareaRef.current) textareaRef.current.style.height = "auto";

		const msgId = crypto.randomUUID();
		addUserMessage(text, msgId);
		openChat();

		try {
			// 传递当前 mode 给后端
			const requestId = await ipc.sendMessage(
				text,
				currentSessionId ?? undefined,
				mode,
			);
			startStreaming(requestId);
		} catch (err) {
			const message = err instanceof Error ? err.message : String(err);
			// 用一个占位 requestId 展示错误 bubble，而不是修改用户消息
			const errId = crypto.randomUUID();
			startStreaming(errId);
			setError(errId, `发送失败：${message}`);
			// 把用户输入还回输入框
			setContent(text);
		} finally {
			isSendingRef.current = false;
		}
	}

	function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
		if (e.key === "Enter" && !e.shiftKey) {
			e.preventDefault();
			handleSend();
		}
	}

	function handleChange(e: React.ChangeEvent<HTMLTextAreaElement>) {
		setContent(e.target.value);
		const el = e.target;
		el.style.height = "auto";
		el.style.height = `${Math.min(el.scrollHeight, 120)}px`;
	}

	return (
		<div className="border-t border-border px-3 py-2">
			<div className="flex items-end gap-2 rounded-lg border border-border bg-background px-3 py-2 focus-within:ring-1 focus-within:ring-ring">
				<textarea
					ref={textareaRef}
					value={content}
					onChange={handleChange}
					onKeyDown={handleKeyDown}
					placeholder="输入消息（Enter 发送，Shift+Enter 换行）"
					rows={1}
					disabled={isStreaming}
					className="max-h-[120px] flex-1 resize-none bg-transparent text-sm outline-none placeholder:text-muted-foreground disabled:opacity-50"
				/>
				<button
					type="button"
					onClick={handleSend}
					disabled={!content.trim() || isStreaming}
					className="shrink-0 rounded-md p-1.5 text-primary disabled:opacity-40 hover:bg-accent transition-colors"
					title="发送"
				>
					<Send className="h-4 w-4" />
				</button>
			</div>
		</div>
	);
}
