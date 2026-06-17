import { invoke } from "@tauri-apps/api/core";
import { Menu, MessageSquare, Package, Settings } from "lucide-react";
import { useEffect, useState } from "react";
import AcpRegistry from "./components/AcpRegistry";
import ChannelSettings from "./components/ChannelSettings";
import MessageList from "./components/MessageList";
import { Button } from "./components/ui/button";
import type { ChannelDescriptor } from "./lib/types";
import { useMessageStore } from "./stores/message-store";
import "./App.css";

type Tab = "messages" | "registry" | "settings";

export default function App() {
	const [activeTab, setActiveTab] = useState<Tab>("messages");
	const [channelTabId, setChannelTabId] = useState<string>("feishu");
	const descriptors = useMessageStore((s) => s.channelDescriptors);
	const setDescriptors = useMessageStore((s) => s.setChannelDescriptors);

	useEffect(() => {
		invoke<ChannelDescriptor[]>("list_channel_descriptors")
			.then(setDescriptors)
			.catch(console.error);
	}, [setDescriptors]);

	const currentDescriptor = descriptors.find((d) => d.id === channelTabId);

	return (
		<div className="flex flex-col h-screen bg-background">
			<div
				data-tauri-drag-region
				className="flex items-center justify-between h-10 px-4 border-b border-border select-none"
			>
				<div className="flex items-center gap-2">
					<Menu className="w-4 h-4 text-muted-foreground" />
					<span className="text-sm font-semibold">MindClaw</span>
				</div>
				<div className="flex items-center gap-1">
					<Button
						variant={activeTab === "messages" ? "secondary" : "ghost"}
						size="sm"
						onClick={() => setActiveTab("messages")}
					>
						<MessageSquare className="w-4 h-4 mr-1" />
						消息
					</Button>
					<Button
						variant={activeTab === "registry" ? "secondary" : "ghost"}
						size="sm"
						onClick={() => setActiveTab("registry")}
					>
						<Package className="w-4 h-4 mr-1" />
						应用市场
					</Button>
					<Button
						variant={activeTab === "settings" ? "secondary" : "ghost"}
						size="sm"
						onClick={() => setActiveTab("settings")}
					>
						<Settings className="w-4 h-4 mr-1" />
						设置
					</Button>
				</div>
			</div>

			<div className="flex-1 overflow-hidden">
				{activeTab === "messages" ? (
					<MessageList />
				) : activeTab === "registry" ? (
					<AcpRegistry />
				) : (
					<div className="flex flex-col h-full">
						<div className="flex border-b border-border px-4 py-2 gap-1">
							{descriptors.map((desc) => (
								<Button
									key={desc.id}
									variant={channelTabId === desc.id ? "secondary" : "ghost"}
									size="sm"
									onClick={() => setChannelTabId(desc.id)}
								>
									{desc.display_name}
								</Button>
							))}
						</div>
						{currentDescriptor ? (
							<ChannelSettings descriptor={currentDescriptor} />
						) : (
							<div className="flex items-center justify-center h-full text-muted-foreground">
								暂无可用渠道
							</div>
						)}
					</div>
				)}
			</div>
		</div>
	);
}
