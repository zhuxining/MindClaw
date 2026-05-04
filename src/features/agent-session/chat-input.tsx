import { Send } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { ipc } from "@/lib/ipc";
import { useChatStore } from "@/stores/chat";

export function ChatInput() {
	const [content, setContent] = useState("");
	const textareaRef = useRef<HTMLTextAreaElement>(null);
	const isSendingRef = useRef(false);

	const streamingRequestId = useChatStore((state) => state.streamingRequestId);
	const currentSessionId = useChatStore((state) => state.currentSessionId);
	const mode = useChatStore((state) => state.mode);
	const addUserMessage = useChatStore((state) => state.addUserMessage);
	const startStreaming = useChatStore((state) => state.startStreaming);
	const setError = useChatStore((state) => state.setError);

	const isStreaming = streamingRequestId !== null;

	useEffect(() => {
		textareaRef.current?.focus();
	}, []);

	async function handleSend() {
		const text = content.trim();
		if (!text || isStreaming || isSendingRef.current) return;

		isSendingRef.current = true;
		setContent("");
		if (textareaRef.current) textareaRef.current.style.height = "auto";

		const messageId = crypto.randomUUID();
		addUserMessage(text, messageId);

		try {
			const requestId = await ipc.sendMessage(
				text,
				currentSessionId ?? undefined,
				mode,
			);
			startStreaming(requestId);
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			const errorId = crypto.randomUUID();
			startStreaming(errorId);
			setError(errorId, `发送失败：${message}`);
			setContent(text);
		} finally {
			isSendingRef.current = false;
		}
	}

	function handleKeyDown(event: React.KeyboardEvent<HTMLTextAreaElement>) {
		if (event.key === "Enter" && !event.shiftKey) {
			event.preventDefault();
			void handleSend();
		}
	}

	function handleChange(event: React.ChangeEvent<HTMLTextAreaElement>) {
		setContent(event.target.value);
		const element = event.target;
		element.style.height = "auto";
		element.style.height = `${Math.min(element.scrollHeight, 140)}px`;
	}

	return (
		<div className="border-t border-border/70 bg-background px-4 py-4">
			<div
				className={`rounded-2xl border px-3 py-3 transition-colors ${
					mode === "private"
						? "border-amber-200 bg-amber-50/70"
						: "border-border/70 bg-background"
				}`}
			>
				{mode === "private" ? (
					<p className="mb-2 text-[11px] text-amber-700">
						树洞模式：内容不会进入 Vault。
					</p>
				) : null}
				<div className="flex items-end gap-3">
					<textarea
						ref={textareaRef}
						value={content}
						onChange={handleChange}
						onKeyDown={handleKeyDown}
						placeholder={
							mode === "private"
								? "把不想沉淀进 Vault 的内容放在这里…"
								: "输入消息，Enter 发送，Shift+Enter 换行"
						}
						rows={1}
						disabled={isStreaming}
						className="max-h-35 flex-1 resize-none bg-transparent text-sm leading-6 outline-none placeholder:text-muted-foreground disabled:opacity-50"
					/>
					<button
						type="button"
						onClick={() => void handleSend()}
						disabled={!content.trim() || isStreaming}
						className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-primary text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-40"
						title="发送"
						aria-label="发送"
					>
						<Send className="h-4 w-4" />
					</button>
				</div>
			</div>
		</div>
	);
}
