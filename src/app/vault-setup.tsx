import { open } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
import { FolderOpen } from "lucide-react";
import { useState } from "react";
import { ipc } from "@/lib/ipc";

export function VaultSetup() {
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);

	async function handleSelect() {
		const selected = await open({
			directory: true,
			multiple: false,
			title: "选择知识库文件夹",
		});
		if (!selected || typeof selected !== "string") return;

		setLoading(true);
		setError(null);
		try {
			await ipc.setVault(selected);
			await relaunch();
		} catch (e) {
			setError(e instanceof Error ? e.message : String(e));
			setLoading(false);
		}
	}

	return (
		<div className="flex h-screen w-screen items-center justify-center bg-background text-foreground">
			<div className="flex flex-col items-center gap-6 max-w-sm text-center">
				<div className="rounded-full bg-accent p-4">
					<FolderOpen className="h-8 w-8 text-accent-foreground" />
				</div>
				<div>
					<h1 className="text-lg font-semibold">选择知识库</h1>
					<p className="mt-1 text-sm text-muted-foreground">
						选择一个文件夹作为你的 MindClaw 知识库，所有数据将存储在此处。
					</p>
				</div>
				{error && <p className="text-xs text-destructive">{error}</p>}
				<button
					type="button"
					disabled={loading}
					onClick={handleSelect}
					className="inline-flex items-center gap-2 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50 transition-colors"
				>
					<FolderOpen className="h-4 w-4" />
					{loading ? "正在初始化…" : "选择文件夹"}
				</button>
			</div>
		</div>
	);
}
