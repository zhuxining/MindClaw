import { MessageSquare, X } from "lucide-react";
import { createPortal } from "react-dom";
import { cn } from "@/lib/utils";
import { useWorkspaceStore } from "@/stores/workspace";
import { ChatWindow } from "./ChatWindow";

export function ChatOverlay() {
	const chatOpen = useWorkspaceStore((s) => s.chatOpen);
	const toggleChat = useWorkspaceStore((s) => s.toggleChat);
	const closeChat = useWorkspaceStore((s) => s.closeChat);

	return createPortal(
		<>
			{/* Chat 触发按钮 */}
			<button
				type="button"
				onClick={toggleChat}
				className={cn(
					"fixed right-4 top-4 z-50 flex h-9 w-9 items-center justify-center rounded-full shadow-md transition-all",
					chatOpen
						? "bg-foreground text-background"
						: "bg-primary text-primary-foreground hover:bg-primary/90",
				)}
				title={chatOpen ? "关闭对话" : "打开对话"}
			>
				{chatOpen ? (
					<X className="h-4 w-4" />
				) : (
					<MessageSquare className="h-4 w-4" />
				)}
			</button>

			{/* Chat 悬浮卡片 */}
			{chatOpen && (
				<div className="animate-in fade-in fixed right-4 top-16 z-40 flex h-140 w-[480px] flex-col rounded-xl border border-border bg-background shadow-xl duration-150">
					<ChatWindow onClose={closeChat} />
				</div>
			)}
		</>,
		document.body,
	);
}
