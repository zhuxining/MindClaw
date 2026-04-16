import { MessageSquare, X } from "lucide-react";
import { useEffect } from "react";
import { createPortal } from "react-dom";
import { cn } from "@/lib/utils";
import { useWorkspaceStore } from "@/stores/workspace";
import { ChatWindow } from "./ChatWindow";

export function ChatOverlay() {
	const chatOpen = useWorkspaceStore((state) => state.chatOpen);
	const toggleChat = useWorkspaceStore((state) => state.toggleChat);
	const closeChat = useWorkspaceStore((state) => state.closeChat);

	useEffect(() => {
		if (!chatOpen) return;

		function handleKeydown(event: KeyboardEvent) {
			if (event.key === "Escape") {
				closeChat();
			}
		}

		window.addEventListener("keydown", handleKeydown);
		return () => window.removeEventListener("keydown", handleKeydown);
	}, [chatOpen, closeChat]);

	return createPortal(
		<>
			<button
				type="button"
				onClick={toggleChat}
				className={cn(
					"fixed right-6 top-6 z-70 flex h-11 w-11 items-center justify-center rounded-full border border-white/80 shadow-[0_12px_36px_rgba(15,23,42,0.18)] transition-all",
					chatOpen
						? "bg-foreground text-background"
						: "bg-primary text-primary-foreground hover:-translate-y-0.5 hover:bg-primary/90",
				)}
				title={chatOpen ? "关闭对话" : "打开对话"}
				aria-label={chatOpen ? "关闭对话" : "打开对话"}
			>
				{chatOpen ? (
					<X className="h-4 w-4" />
				) : (
					<MessageSquare className="h-4 w-4" />
				)}
			</button>

			{chatOpen ? (
				<div className="fixed inset-0 z-60">
					<button
						type="button"
						aria-label="关闭对话遮罩"
						className="absolute inset-0 bg-[radial-gradient(circle_at_top_right,rgba(15,23,42,0.12),rgba(15,23,42,0.28))] backdrop-blur-[2px]"
						onClick={closeChat}
					/>
					<div className="pointer-events-none absolute inset-0 flex justify-end p-6 pt-20">
						<div className="pointer-events-auto h-140 w-120 animate-in fade-in slide-in-from-top-2 duration-150">
							<ChatWindow onClose={closeChat} />
						</div>
					</div>
				</div>
			) : null}
		</>,
		document.body,
	);
}
