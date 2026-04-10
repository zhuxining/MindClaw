import type { ConversationMode } from "@/lib/types";
import { useChatStore } from "@/stores/chat";
import { ChatInput } from "./ChatInput";
import { MessageList } from "./MessageList";
import { ModeSelector } from "./ModeSelector";

interface ChatWindowProps {
	onClose: () => void;
}

export function ChatWindow({ onClose }: ChatWindowProps) {
	const mode = useChatStore((s) => s.mode);
	const setMode = useChatStore((s) => s.setMode);

	return (
		<div className="flex h-full flex-col">
			{/* 标题栏 */}
			<div className="flex items-center justify-between border-b border-border px-4 py-2">
				<span className="text-sm font-semibold">对话</span>
				<button
					type="button"
					onClick={onClose}
					className="text-xs text-muted-foreground hover:text-foreground transition-colors"
				>
					收起
				</button>
			</div>

			{/* 模式选择 */}
			<div className="px-3 py-2 border-b border-border">
				<ModeSelector
					mode={mode}
					onChange={(m: ConversationMode) => setMode(m)}
				/>
			</div>

			{/* 消息列表 */}
			<MessageList />

			{/* 输入框 */}
			<ChatInput />
		</div>
	);
}
