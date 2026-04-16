import { useEffect, useRef, useState } from "react";
import { useChatStore } from "@/stores/chat";
import { AgentBubble, UserBubble } from "./MessageBubble";

export function MessageList() {
	const messages = useChatStore((state) => state.messages);
	const viewportRef = useRef<HTMLDivElement>(null);
	const bottomRef = useRef<HTMLDivElement>(null);
	const [stickToBottom, setStickToBottom] = useState(true);

	useEffect(() => {
		if (!stickToBottom) return;
		bottomRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
	}, [stickToBottom]);

	return (
		<div
			ref={viewportRef}
			onScroll={(event) => {
				const element = event.currentTarget;
				const distance =
					element.scrollHeight - element.scrollTop - element.clientHeight;
				setStickToBottom(distance < 32);
			}}
			className="flex-1 overflow-y-auto bg-muted/22 px-5 py-4"
		>
			{messages.length === 0 ? (
				<div className="flex h-full items-center justify-center">
					<div className="max-w-xs space-y-2 text-center">
						<p className="text-sm font-medium text-foreground">开始一段对话</p>
						<p className="text-xs leading-6 text-muted-foreground">
							它会一直悬浮在工作区上方，你可以边写边问，边问边推进任务。
						</p>
					</div>
				</div>
			) : (
				<div className="flex flex-col gap-3">
					{messages.map((message) =>
						message.type === "user" ? (
							<UserBubble key={message.id} message={message} />
						) : (
							<AgentBubble key={message.id} message={message} />
						),
					)}
					<div ref={bottomRef} />
				</div>
			)}
		</div>
	);
}
