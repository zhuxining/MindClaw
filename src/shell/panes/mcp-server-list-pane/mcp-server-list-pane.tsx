import { Plug, Settings2 } from "lucide-react";

export function McpServerListPane() {
	return (
		<div className="flex h-full flex-col">
			<div
				className="border-b p-4"
				style={{ borderColor: "var(--flexoki-bg-2)" }}
			>
				<p className="text-sm font-semibold text-foreground">MCP Server</p>
				<p className="text-xs text-muted-foreground">
					Model Context Protocol 服务配置
				</p>
			</div>

			<div className="min-h-0 flex-1 overflow-y-auto p-3">
				<div className="space-y-4">
					<div className="rounded-lg border border-border/50 p-4 text-center">
						<Plug className="mx-auto h-8 w-8 text-muted-foreground mb-3" />
						<p className="text-sm font-medium mb-1">MCP 支持开发中</p>
						<p className="text-xs text-muted-foreground">
							Model Context Protocol 将支持：
						</p>
						<ul className="mt-2 text-xs text-muted-foreground space-y-1">
							<li>• 连接外部工具服务器</li>
							<li>• 动态加载 MCP tools</li>
							<li>• 资源访问与上下文共享</li>
						</ul>
					</div>

					<div className="rounded-lg border border-primary/30 bg-primary/5 p-3">
						<div className="flex items-center gap-2">
							<Settings2 className="h-4 w-4 text-primary" />
							<p className="text-sm text-primary">
								MCP 配置将在 Settings 工作域提供
							</p>
						</div>
					</div>
				</div>
			</div>
		</div>
	);
}
