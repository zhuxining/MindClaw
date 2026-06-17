import { invoke } from "@tauri-apps/api/core";
import {
	Download,
	ExternalLink,
	Package,
	Search,
	ShieldAlert,
	X,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import type { AcpServer } from "../lib/types";
import { useMessageStore } from "../stores/message-store";
import { Button } from "./ui/button";
import { Card } from "./ui/card";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "./ui/dialog";
import { ScrollArea } from "./ui/scroll-area";

interface RegistryAgent {
	id: string;
	name: string;
	version: string;
	description: string;
	repository?: string;
	website?: string;
	authors: string[];
	license: string;
	icon?: string;
	distribution: {
		npx?: {
			package: string;
			args?: string[];
			env?: Record<string, string>;
		};
		binary?: {
			[key: string]: {
				archive: string;
				cmd: string;
			};
		};
	};
}

export default function AcpRegistry() {
	const [agents, setAgents] = useState<RegistryAgent[]>([]);
	const [loading, setLoading] = useState(true);
	const [searchQuery, setSearchQuery] = useState("");
	const [installingId, setInstallingId] = useState<string | null>(null);
	const [error, setError] = useState<string | null>(null);
	const [confirmDialogOpen, setConfirmDialogOpen] = useState(false);
	const [agentToInstall, setAgentToInstall] = useState<RegistryAgent | null>(
		null,
	);
	const { acpServers, setAcpServers } = useMessageStore();

	const fetchRegistry = useCallback(async () => {
		try {
			setLoading(true);
			setError(null);
			const result = await invoke<{ version: string; agents: RegistryAgent[] }>(
				"fetch_acp_registry",
			);
			setAgents(result.agents);
		} catch (err) {
			console.error("获取 ACP 注册表失败:", err);
			setError("获取注册表失败，请检查网络连接");
		} finally {
			setLoading(false);
		}
	}, []);

	useEffect(() => {
		fetchRegistry();
	}, [fetchRegistry]);

	const handleInstallClick = (agent: RegistryAgent) => {
		setAgentToInstall(agent);
		setConfirmDialogOpen(true);
	};

	const handleConfirmInstall = async () => {
		if (!agentToInstall) return;

		try {
			setInstallingId(agentToInstall.id);
			setError(null);
			await invoke("install_acp_agent", { registryAgent: agentToInstall });
			// 刷新已安装的服务器列表
			const servers = await invoke<AcpServer[]>("list_acp_servers");
			setAcpServers(servers);
			setConfirmDialogOpen(false);
			setAgentToInstall(null);
		} catch (err) {
			console.error("安装失败:", err);
			setError(`安装 ${agentToInstall.name} 失败: ${err}`);
		} finally {
			setInstallingId(null);
		}
	};

	const isInstalled = (agentId: string) => {
		return acpServers.some((s) => s.id === agentId);
	};

	const filteredAgents = agents.filter(
		(agent) =>
			agent.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
			agent.description.toLowerCase().includes(searchQuery.toLowerCase()),
	);

	const hasNpxDistribution = (agent: RegistryAgent) => {
		return agent.distribution?.npx;
	};

	return (
		<div className="flex flex-col h-full">
			{/* 标题栏 */}
			<div className="flex items-center justify-between px-4 py-3 border-b border-border">
				<div className="flex items-center gap-2">
					<Package className="w-4 h-4" />
					<span className="text-sm font-medium">ACP 应用市场</span>
				</div>
				<span className="text-xs text-muted-foreground">
					共 {agents.length} 个 Agent
				</span>
			</div>

			{/* 搜索栏 */}
			<div className="p-4 border-b border-border">
				<div className="relative">
					<Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
					<input
						type="text"
						placeholder="搜索 Agent..."
						value={searchQuery}
						onChange={(e) => setSearchQuery(e.target.value)}
						className="w-full pl-10 pr-4 py-2 bg-background border border-input rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-ring"
					/>
				</div>
			</div>

			{/* 错误提示 */}
			{error && (
				<div className="mx-4 mt-4 p-3 bg-destructive/10 border border-destructive/20 rounded-md">
					<div className="flex items-center justify-between">
						<span className="text-sm text-destructive">{error}</span>
						<Button
							variant="ghost"
							size="sm"
							className="h-6 w-6 p-0"
							onClick={() => setError(null)}
						>
							<X className="w-3 h-3" />
						</Button>
					</div>
				</div>
			)}

			{/* Agent 列表 */}
			<ScrollArea className="flex-1">
				{loading ? (
					<div className="flex items-center justify-center p-8">
						<div className="animate-spin w-6 h-6 border-2 border-primary border-t-transparent rounded-full" />
					</div>
				) : filteredAgents.length === 0 ? (
					<div className="flex flex-col items-center justify-center p-8 text-muted-foreground">
						<Package className="w-12 h-12 mb-2 opacity-50" />
						<p className="text-sm">未找到匹配的 Agent</p>
					</div>
				) : (
					<div className="p-4 space-y-3">
						{filteredAgents.map((agent) => (
							<Card
								key={agent.id}
								className="p-4 hover:border-primary/50 transition-colors"
							>
								<div className="flex gap-4">
									{/* Icon */}
									<div className="shrink-0 w-12 h-12 bg-muted rounded-md flex items-center justify-center overflow-hidden">
										{agent.icon ? (
											// eslint-disable-next-line @next/next/no-img-element
											<img
												src={agent.icon}
												alt={agent.name}
												className="w-full h-full object-contain"
												onError={(e) => {
													(e.target as HTMLImageElement).style.display = "none";
												}}
											/>
										) : (
											<Package className="w-6 h-6 text-muted-foreground" />
										)}
									</div>

									{/* 内容 */}
									<div className="flex-1 min-w-0">
										<div className="flex items-start justify-between gap-2">
											<div>
												<div className="flex items-center gap-2">
													<h3 className="font-medium text-sm">{agent.name}</h3>
													<span className="text-xs text-muted-foreground font-mono">
														v{agent.version}
													</span>
												</div>
												<p className="text-xs text-muted-foreground mt-1">
													{agent.authors.join(", ")}
												</p>
											</div>
											<div className="flex items-center gap-1">
												{agent.website && (
													<a
														href={agent.website}
														target="_blank"
														rel="noopener noreferrer"
														onClick={(e) => e.stopPropagation()}
													>
														<Button
															variant="ghost"
															size="sm"
															className="h-7 w-7 p-0"
														>
															<ExternalLink className="w-3 h-3" />
														</Button>
													</a>
												)}
												{isInstalled(agent.id) ? (
													<Button
														variant="secondary"
														size="sm"
														className="h-7"
														disabled
													>
														已安装
													</Button>
												) : hasNpxDistribution(agent) ? (
													<Button
														variant="default"
														size="sm"
														className="h-7"
														onClick={() => handleInstallClick(agent)}
														disabled={installingId === agent.id}
													>
														{installingId === agent.id ? (
															<>
																<div className="animate-spin w-3 h-3 border-2 border-current border-t-transparent rounded-full mr-1" />
																安装中
															</>
														) : (
															<>
																<Download className="w-3 h-3 mr-1" />
																安装
															</>
														)}
													</Button>
												) : (
													<Button
														variant="secondary"
														size="sm"
														className="h-7"
														disabled
													>
														需手动安装
													</Button>
												)}
											</div>
										</div>

										<p className="text-sm text-muted-foreground mt-2 line-clamp-2">
											{agent.description}
										</p>

										<div className="flex items-center gap-2 mt-2">
											<span className="text-xs px-2 py-0.5 bg-muted rounded-full">
												{agent.license}
											</span>
											{agent.distribution.npx && (
												<span className="text-xs px-2 py-0.5 bg-green-500/10 text-green-600 rounded-full">
													npx
												</span>
											)}
											{agent.distribution.binary && !agent.distribution.npx && (
												<span className="text-xs px-2 py-0.5 bg-yellow-500/10 text-yellow-600 rounded-full">
													binary
												</span>
											)}
										</div>
									</div>
								</div>
							</Card>
						))}
					</div>
				)}
			</ScrollArea>

			{/* 安装确认对话框 */}
			<Dialog open={confirmDialogOpen} onOpenChange={setConfirmDialogOpen}>
				<DialogContent>
					<DialogHeader>
						<DialogTitle className="flex items-center gap-2">
							<ShieldAlert className="w-5 h-5 text-yellow-500" />
							确认安装
						</DialogTitle>
						<DialogDescription>
							您即将安装以下 ACP Agent。请确认这是您信任的来源。
						</DialogDescription>
					</DialogHeader>

					{agentToInstall && (
						<div className="space-y-4">
							<Card className="p-4">
								<div className="flex items-center gap-3">
									<div className="w-10 h-10 bg-muted rounded-md flex items-center justify-center">
										{agentToInstall.icon ? (
											// eslint-disable-next-line @next/next/no-img-element
											<img
												src={agentToInstall.icon}
												alt={agentToInstall.name}
												className="w-full h-full object-contain"
											/>
										) : (
											<Package className="w-5 h-5 text-muted-foreground" />
										)}
									</div>
									<div>
										<div className="font-medium">{agentToInstall.name}</div>
										<div className="text-xs text-muted-foreground">
											v{agentToInstall.version} ·{" "}
											{agentToInstall.authors.join(", ")}
										</div>
									</div>
								</div>
								<p className="text-sm text-muted-foreground mt-3">
									{agentToInstall.description}
								</p>
							</Card>

							<div className="text-sm space-y-2">
								<div className="flex items-start gap-2">
									<span className="text-muted-foreground">安装方式:</span>
									<span className="font-mono text-xs bg-muted px-2 py-0.5 rounded">
										npx {agentToInstall.distribution.npx?.package}
									</span>
								</div>
								<div className="text-yellow-600 bg-yellow-500/10 p-3 rounded-md">
									<p className="font-medium">⚠️ 安全提示</p>
									<p className="text-xs mt-1">
										ACP Agent 可以执行代码和访问系统资源。仅安装您信任的来源。
									</p>
								</div>
							</div>
						</div>
					)}

					<DialogFooter>
						<Button
							variant="outline"
							onClick={() => setConfirmDialogOpen(false)}
						>
							取消
						</Button>
						<Button
							onClick={handleConfirmInstall}
							disabled={installingId !== null}
						>
							{installingId !== null ? (
								<>
									<div className="animate-spin w-3 h-3 border-2 border-current border-t-transparent rounded-full mr-1" />
									安装中
								</>
							) : (
								"确认安装"
							)}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</div>
	);
}
