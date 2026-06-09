import { invoke } from "@tauri-apps/api/core";
import { CheckCircle, Link, Settings, Unlink, XCircle } from "lucide-react";
import { useState } from "react";
import { useMessageStore } from "../stores/message-store";
import { Button } from "./ui/button";
import { Card } from "./ui/card";
import { Input } from "./ui/input";

interface ChannelSettingsProps {
	channelName: string;
	displayName: string;
	credentialType: "feishu" | "telegram";
}

export default function ChannelSettings({
	channelName,
	displayName,
	credentialType,
}: ChannelSettingsProps) {
	const [appId, setAppId] = useState("");
	const [appSecret, setAppSecret] = useState("");
	const [botToken, setBotToken] = useState("");
	const [testing, setTesting] = useState(false);
	const [saving, setSaving] = useState(false);
	const {
		feishuConnected,
		channelStatuses,
		pollInterval,
		setFeishuConnected,
		setChannelConnected,
		setPollInterval,
	} = useMessageStore();

	const connected = channelStatuses[channelName] ?? feishuConnected;

	const handleSave = async () => {
		setSaving(true);
		try {
			const credentials =
				credentialType === "feishu"
					? { app_id: appId, app_secret: appSecret }
					: { bot_token: botToken };

			await invoke("set_channel_credentials", {
				channel: channelName,
				credentials,
			});
			setFeishuConnected(true);
			setChannelConnected(channelName, true);
			if (credentialType === "feishu") setAppSecret("");
			else setBotToken("");
		} catch (err) {
			console.error("保存凭证失败:", err);
		} finally {
			setSaving(false);
		}
	};

	const canSave =
		credentialType === "feishu" ? !!(appId && appSecret) : !!botToken;

	const handleTest = async () => {
		setTesting(true);
		try {
			await invoke("test_channel_connection", { channel: channelName });
			setFeishuConnected(true);
			setChannelConnected(channelName, true);
		} catch (err) {
			console.error("测试连接失败:", err);
			setFeishuConnected(false);
			setChannelConnected(channelName, false);
		} finally {
			setTesting(false);
		}
	};

	const handleDisconnect = async () => {
		try {
			const credentials =
				credentialType === "feishu"
					? { app_id: "", app_secret: "" }
					: { bot_token: "" };

			await invoke("set_channel_credentials", {
				channel: channelName,
				credentials,
			});
			setFeishuConnected(false);
			setChannelConnected(channelName, false);
			setAppId("");
			setAppSecret("");
			setBotToken("");
		} catch (err) {
			console.error("断开连接失败:", err);
		}
	};

	return (
		<div className="flex flex-col h-full">
			{/* 标题栏 */}
			<div className="flex items-center gap-2 px-4 py-3 border-b border-border">
				<Settings className="w-4 h-4" />
				<span className="text-sm font-medium">{displayName}设置</span>
			</div>

			<div className="flex-1 p-4 space-y-6 overflow-y-auto">
				{/* 连接状态 */}
				<Card className="p-4">
					<div className="flex items-center justify-between">
						<div className="flex items-center gap-2">
							<span className="text-sm font-medium">连接状态</span>
							{connected ? (
								<CheckCircle className="w-4 h-4 text-green-500" />
							) : (
								<XCircle className="w-4 h-4 text-muted-foreground" />
							)}
						</div>
						<span
							className={`text-sm ${connected ? "text-green-600" : "text-muted-foreground"}`}
						>
							{connected ? "已连接" : "未连接"}
						</span>
					</div>
				</Card>

				{/* 凭证配置 */}
				<Card className="p-4 space-y-4">
					<h3 className="text-sm font-medium">{displayName}应用凭证</h3>
					{credentialType === "feishu" ? (
						<>
							<p className="text-xs text-muted-foreground">
								在{displayName}开放平台创建应用后，获取 App ID 和 App Secret。
								凭证将安全存储在本地。
							</p>
							<div className="space-y-3">
								<div className="space-y-1.5">
									<label htmlFor={`${channelName}-app-id`} className="text-sm">
										App ID
									</label>
									<Input
										id={`${channelName}-app-id`}
										type="text"
										placeholder={`输入${displayName} App ID`}
										value={appId}
										onChange={(e) => setAppId(e.target.value)}
									/>
								</div>
								<div className="space-y-1.5">
									<label
										htmlFor={`${channelName}-app-secret`}
										className="text-sm"
									>
										App Secret
									</label>
									<Input
										id={`${channelName}-app-secret`}
										type="password"
										placeholder={`输入${displayName} App Secret`}
										value={appSecret}
										onChange={(e) => setAppSecret(e.target.value)}
									/>
								</div>
							</div>
						</>
					) : (
						<>
							<p className="text-xs text-muted-foreground">
								在 @BotFather 创建 Bot 后获取 Token。凭证将安全存储在本地。
							</p>
							<div className="space-y-3">
								<div className="space-y-1.5">
									<label
										htmlFor={`${channelName}-bot-token`}
										className="text-sm"
									>
										Bot Token
									</label>
									<Input
										id={`${channelName}-bot-token`}
										type="password"
										placeholder="输入 Telegram Bot Token"
										value={botToken}
										onChange={(e) => setBotToken(e.target.value)}
									/>
								</div>
							</div>
						</>
					)}

					<div className="flex gap-2">
						<Button
							onClick={handleSave}
							disabled={!canSave || saving}
							className="flex-1"
						>
							<Link className="w-4 h-4 mr-1" />
							{saving ? "保存中..." : "保存并连接"}
						</Button>
						<Button
							variant="outline"
							onClick={handleTest}
							disabled={!connected || testing}
						>
							{testing ? "测试中..." : "测试连接"}
						</Button>
						{connected && (
							<Button variant="outline" onClick={handleDisconnect}>
								<Unlink className="w-4 h-4" />
							</Button>
						)}
					</div>
				</Card>

				{/* 轮询设置 */}
				<Card className="p-4 space-y-3">
					<h3 className="text-sm font-medium">消息轮询</h3>
					<div className="flex items-center justify-between">
						<span className="text-sm text-muted-foreground">轮询间隔</span>
						<div className="flex items-center gap-2">
							<input
								type="range"
								min="10"
								max="300"
								step="10"
								value={pollInterval}
								onChange={(e) => setPollInterval(Number(e.target.value))}
								className="w-32"
							/>
							<span className="text-sm w-16 text-right">{pollInterval} 秒</span>
						</div>
					</div>
				</Card>
			</div>
		</div>
	);
}
