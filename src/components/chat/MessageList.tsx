import { useEffect, useRef } from "react";
import { useChatStore } from "@/stores/chat";
import { AgentBubble, UserBubble } from "./MessageBubble";

export function MessageList() {
	const messages = useChatStore((s) => s.messages);
	const bottomRef = useRef<HTMLDivElement>(null);

	// 每次新消息到来时自动滚到底部
	useEffect(() => {
		bottomRef.current?.scrollIntoView({ behavior: "smooth" });
	}, []);

	return (
		<div className="flex-1 overflow-y-auto px-4 py-3">
			{messages.length === 0 ? (
				<div className="flex h-full items-center justify-center text-sm text-muted-foreground">
					<p>说点什么吧…</p>
				</div>
			) : (
				<div className="flex flex-col gap-3">
					{messages.map((msg) =>
						msg.type === "user" ? (
							<UserBubble key={msg.id} message={msg} />
						) : (
							<AgentBubble key={msg.id} message={msg} />
						),
					)}
					<div ref={bottomRef} />
				</div>
			)}
		</div>
	);
}
