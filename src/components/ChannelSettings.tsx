import { invoke } from "@tauri-apps/api/core";
import { CheckCircle, Link, Settings, Unlink, XCircle } from "lucide-react";
import { useEffect, useState } from "react";
import type { ChannelDescriptor } from "../lib/types";
import { useMessageStore } from "../stores/message-store";
import { Button } from "./ui/button";
import { Card } from "./ui/card";
import { Input } from "./ui/input";

interface ChannelSettingsProps {
	descriptor: ChannelDescriptor;
}

/** 从 descriptor.credential_schema (JSON Schema) 提取字段定义。 */
function extractFields(
	schema: unknown,
): { key: string; title: string; isPassword: boolean }[] {
	const obj = schema as Record<string, unknown> | undefined;
	const props = obj?.properties as
		| Record<string, { title?: string; format?: string }>
		| undefined;
	if (!props) return [];
	return Object.entries(props).map(([key, meta]) => ({
		key,
		title: meta.title ?? key,
		isPassword: meta.format === "password",
	}));
}

export default function ChannelSettings({ descriptor }: ChannelSettingsProps) {
	const channelStatuses = useMessageStore((s) => s.channelStatuses);
	const setChannelConnected = useMessageStore((s) => s.setChannelConnected);
	const connected = channelStatuses[descriptor.id] ?? false;

	const fields = extractFields(descriptor.credential_schema);
	const [values, setValues] = useState<Record<string, string>>({});
	const [testing, setTesting] = useState(false);
	const [saving, setSaving] = useState(false);

	useEffect(() => {
		// 检查已配置凭证
		invoke<boolean>("get_channel_connection_status", { channel: descriptor.id })
			.then(setChannelConnected.bind(null, descriptor.id))
			.catch(() => setChannelConnected(descriptor.id, false));
	}, [descriptor.id, setChannelConnected]);

	const handleSave = async () => {
		setSaving(true);
		try {
			// 构造凭证对象（key 与 schema properties 对应）
			const credentials: Record<string, string> = {};
			for (const f of fields) {
				credentials[f.key] = values[f.key] ?? "";
			}
			await invoke("set_channel_credentials", {
				channel: descriptor.id,
				credentials,
			});
			setChannelConnected(descriptor.id, true);
		} catch (err) {
			console.error("保存凭证失败:", err);
		} finally {
			setSaving(false);
		}
	};

	const handleTest = async () => {
		setTesting(true);
		try {
			await invoke("test_channel_connection", { channel: descriptor.id });
			setChannelConnected(descriptor.id, true);
		} catch (err) {
			console.error("测试连接失败:", err);
			setChannelConnected(descriptor.id, false);
		} finally {
			setTesting(false);
		}
	};

	const handleDisconnect = async () => {
		// 清空凭证
		await invoke("clear_channel_credentials", { channel: descriptor.id });
		setChannelConnected(descriptor.id, false);
		setValues({});
	};

	const canSave = fields.every((f) => values[f.key]);

	return (
		<div className="flex flex-col h-full">
			<div className="flex items-center gap-2 px-4 py-3 border-b border-border">
				<Settings className="w-4 h-4" />
				<span className="text-sm font-medium">
					{descriptor.display_name} 设置
				</span>
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

				{/* 凭证配置（由 credential_schema 动态渲染） */}
				<Card className="p-4 space-y-4">
					<h3 className="text-sm font-medium">
						{descriptor.display_name} 凭证
					</h3>
					<p className="text-xs text-muted-foreground">
						凭证将安全存储在本地。入口模式：
						{descriptor.inbound === "long_connection"
							? "长连接"
							: descriptor.inbound === "long_polling"
								? "长轮询"
								: descriptor.inbound}
						。{descriptor.capabilities.streaming && " · 支持流式输出"}
					</p>
					<div className="space-y-3">
						{fields.map((f) => (
							<div key={f.key} className="space-y-1.5">
								<label
									htmlFor={`${descriptor.id}-${f.key}`}
									className="text-sm"
								>
									{f.title}
								</label>
								<Input
									id={`${descriptor.id}-${f.key}`}
									type={f.isPassword ? "password" : "text"}
									placeholder={`输入 ${f.title}`}
									value={values[f.key] ?? ""}
									onChange={(e) =>
										setValues((v) => ({ ...v, [f.key]: e.target.value }))
									}
								/>
							</div>
						))}
					</div>

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
			</div>
		</div>
	);
}
