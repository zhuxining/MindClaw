import type { ConversationMode } from "@/lib/types";
import { useChatStore } from "@/stores/chat";
import { ChatInput } from "./ChatInput";
import { MessageList } from "./MessageList";
import { ModeSelector } from "./ModeSelector";

interface ChatWindowProps {
	onClose: () => void;
}

export function ChatWindow({ onClose }: ChatWindowProps) {
	const mode = useChatStore((state) => state.mode);
	const setMode = useChatStore((state) => state.setMode);

	return (
		<div className="flex h-full flex-col overflow-hidden rounded-[28px] border border-white/80 bg-background/98 shadow-[0_24px_64px_rgba(15,23,42,0.24)] backdrop-blur-xl">
			<div className="border-b border-border/70 px-5 py-4">
				<div className="flex items-center justify-between">
					<div>
						<p className="text-[11px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">
							Companion Chat
						</p>
						<h2 className="mt-1 text-base font-semibold text-foreground">
							对话
						</h2>
					</div>
					<button
						type="button"
						onClick={onClose}
						className="rounded-lg px-2 py-1 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
					>
						收起
					</button>
				</div>

				<div className="mt-4">
					<ModeSelector
						mode={mode}
						onChange={(nextMode: ConversationMode) => setMode(nextMode)}
					/>
				</div>
			</div>

			<MessageList />
			<ChatInput />
		</div>
	);
}
