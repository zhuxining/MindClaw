import { Clock, Plus, Settings2 } from "lucide-react";

export function CronJobListPane() {
	return (
		<div className="flex h-full flex-col">
			<div
				className="border-b p-4"
				style={{ borderColor: "var(--flexoki-bg-2)" }}
			>
				<p className="text-sm font-semibold text-foreground">定时任务</p>
				<p className="text-xs text-muted-foreground">周期性执行的后台任务</p>
			</div>

			<div className="min-h-0 flex-1 overflow-y-auto p-3">
				<div className="space-y-4">
					<div className="rounded-lg border border-border/50 p-4 text-center">
						<Clock className="mx-auto h-8 w-8 text-muted-foreground mb-3" />
						<p className="text-sm font-medium mb-1">定时任务系统开发中</p>
						<p className="text-xs text-muted-foreground">将支持以下功能：</p>
						<ul className="mt-2 text-xs text-muted-foreground space-y-1">
							<li>• Vault 索引定时刷新</li>
							<li>• Daily Note 自动创建</li>
							<li>• 记忆摘要定期生成</li>
							<li>• 自定义周期任务</li>
						</ul>
					</div>

					{/* Placeholder for future cron jobs */}
					<div className="rounded-lg border border-dashed border-border/50 p-3 text-center">
						<Plus className="mx-auto h-4 w-4 text-muted-foreground mb-1" />
						<p className="text-xs text-muted-foreground">
							功能上线后可在此添加任务
						</p>
					</div>

					<div className="rounded-lg border border-primary/30 bg-primary/5 p-3">
						<div className="flex items-center gap-2">
							<Settings2 className="h-4 w-4 text-primary" />
							<p className="text-sm text-primary">
								任务配置将在 Settings 工作域提供
							</p>
						</div>
					</div>
				</div>
			</div>
		</div>
	);
}
