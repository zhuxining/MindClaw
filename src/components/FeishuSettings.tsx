import { invoke } from "@tauri-apps/api/core";
import { CheckCircle, Link, Settings, Unlink, XCircle } from "lucide-react";
import { useState } from "react";
import { Button } from "../components/ui/button";
import { Card } from "../components/ui/card";
import { Input } from "../components/ui/input";
import { useMessageStore } from "../stores/message-store";

export default function FeishuSettings() {
	const [appId, setAppId] = useState("");
	const [appSecret, setAppSecret] = useState("");
	const [testing, setTesting] = useState(false);
	const [saving, setSaving] = useState(false);
	const {
		feishuConnected,
		pollInterval,
		autoReply,
		setFeishuConnected,
		setPollInterval,
		setAutoReply,
	} = useMessageStore();

	const handleSave = async () => {
		if (!appId || !appSecret) return;
		setSaving(true);
		try {
			await invoke("set_channel_credentials", {
				channel: "feishu",
				credentials: { app_id: appId, app_secret: appSecret },
			});
			setFeishuConnected(true);
			setAppSecret("");
		} catch (err) {
			console.error("保存凭证失败:", err);
		} finally {
			setSaving(false);
		}
	};

	const handleTest = async () => {
		setTesting(true);
		try {
			await invoke("test_channel_connection", { channel: "feishu" });
			setFeishuConnected(true);
		} catch (err) {
			console.error("测试连接失败:", err);
			setFeishuConnected(false);
		} finally {
			setTesting(false);
		}
	};

	const handleDisconnect = async () => {
		try {
			await invoke("set_channel_credentials", {
				channel: "feishu",
				credentials: { app_id: "", app_secret: "" },
			});
			setFeishuConnected(false);
			setAppId("");
			setAppSecret("");
		} catch (err) {
			console.error("断开连接失败:", err);
		}
	};

	return (
		<div className="flex flex-col h-full">
			{/* 标题栏 */}
			<div className="flex items-center gap-2 px-4 py-3 border-b border-border">
				<Settings className="w-4 h-4" />
				<span className="text-sm font-medium">飞书设置</span>
			</div>

			<div className="flex-1 p-4 space-y-6 overflow-y-auto">
				{/* 连接状态 */}
				<Card className="p-4">
					<div className="flex items-center justify-between">
						<div className="flex items-center gap-2">
							<span className="text-sm font-medium">连接状态</span>
							{feishuConnected ? (
								<CheckCircle className="w-4 h-4 text-green-500" />
							) : (
								<XCircle className="w-4 h-4 text-muted-foreground" />
							)}
						</div>
						<span
							className={`text-sm ${feishuConnected ? "text-green-600" : "text-muted-foreground"}`}
						>
							{feishuConnected ? "已连接" : "未连接"}
						</span>
					</div>
				</Card>

				{/* 凭证配置 */}
				<Card className="p-4 space-y-4">
					<h3 className="text-sm font-medium">飞书应用凭证</h3>
					<p className="text-xs text-muted-foreground">
						在飞书开放平台创建应用后，获取 App ID 和 App Secret。
						凭证将安全存储在本地。
					</p>

					<div className="space-y-3">
						<div className="space-y-1.5">
							<label htmlFor="app-id" className="text-sm">
								App ID
							</label>
							<Input
								id="app-id"
								type="text"
								placeholder="输入飞书 App ID"
								value={appId}
								onChange={(e) => setAppId(e.target.value)}
							/>
						</div>

						<div className="space-y-1.5">
							<label htmlFor="app-secret" className="text-sm">
								App Secret
							</label>
							<Input
								id="app-secret"
								type="password"
								placeholder="输入飞书 App Secret"
								value={appSecret}
								onChange={(e) => setAppSecret(e.target.value)}
							/>
						</div>
					</div>

					<div className="flex gap-2">
						<Button
							onClick={handleSave}
							disabled={!appId || !appSecret || saving}
							className="flex-1"
						>
							<Link className="w-4 h-4 mr-1" />
							{saving ? "保存中..." : "保存并连接"}
						</Button>
						<Button
							variant="outline"
							onClick={handleTest}
							disabled={!feishuConnected || testing}
						>
							{testing ? "测试中..." : "测试连接"}
						</Button>
						{feishuConnected && (
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

				{/* 自动回复设置 */}
				<Card className="p-4 space-y-3">
					<h3 className="text-sm font-medium">自动回复</h3>
					<div className="flex items-center justify-between">
						<span className="text-sm text-muted-foreground">
							Agent 处理完成后自动回复飞书
						</span>
						<button
							type="button"
							role="switch"
							aria-checked={autoReply}
							onClick={() => setAutoReply(!autoReply)}
							className={`relative inline-flex h-5 w-9 items-center rounded-full transition-colors ${
								autoReply ? "bg-primary" : "bg-muted"
							}`}
						>
							<span
								className={`inline-block h-4 w-4 rounded-full bg-white transition-transform ${
									autoReply ? "translate-x-4" : "translate-x-0.5"
								}`}
							/>
						</button>
					</div>
				</Card>
			</div>
		</div>
	);
}
