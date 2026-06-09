import { Menu, MessageSquare, Settings } from "lucide-react";
import { useState } from "react";
import ChannelSettings from "./components/ChannelSettings";
import MessageList from "./components/MessageList";
import { Button } from "./components/ui/button";
import "./App.css";

type Tab = "messages" | "settings";
type ChannelTab = "feishu" | "telegram";

export default function App() {
	const [activeTab, setActiveTab] = useState<Tab>("messages");
	const [channelTab, setChannelTab] = useState<ChannelTab>("feishu");

	return (
		<div className="flex flex-col h-screen bg-background">
			{/* 自定义标题栏（透明标题栏模式） */}
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
						variant={activeTab === "settings" ? "secondary" : "ghost"}
						size="sm"
						onClick={() => setActiveTab("settings")}
					>
						<Settings className="w-4 h-4 mr-1" />
						设置
					</Button>
				</div>
			</div>

			{/* 主内容区 */}
			<div className="flex-1 overflow-hidden">
				{activeTab === "messages" ? (
					<MessageList />
				) : (
					<div className="flex flex-col h-full">
						{/* 渠道切换标签 */}
						<div className="flex border-b border-border px-4 py-2 gap-1">
							<Button
								variant={channelTab === "feishu" ? "secondary" : "ghost"}
								size="sm"
								onClick={() => setChannelTab("feishu")}
							>
								<img
									src="/feishu.svg"
									alt="飞书"
									className="w-4 h-4 mr-1"
									onError={(e) => {
										(e.target as HTMLImageElement).style.display = "none";
									}}
								/>
								飞书
							</Button>
							<Button
								variant={channelTab === "telegram" ? "secondary" : "ghost"}
								size="sm"
								onClick={() => setChannelTab("telegram")}
							>
								电报
							</Button>
						</div>
						{channelTab === "feishu" ? (
							<ChannelSettings
								channelName="feishu"
								displayName="飞书"
								credentialType="feishu"
							/>
						) : (
							<ChannelSettings
								channelName="telegram"
								displayName="Telegram"
								credentialType="telegram"
							/>
						)}
					</div>
				)}
			</div>
		</div>
	);
}
