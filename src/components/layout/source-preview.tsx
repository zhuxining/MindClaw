import { convertFileSrc } from "@tauri-apps/api/core";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import { ExternalLink } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { useSettingsQuery } from "@/queries/settings";
import { EmptyState } from "./workspace-chrome";

function joinVaultPath(root: string, relativePath: string) {
	const normalizedRoot = root.replace(/[\\/]+$/, "");
	return `${normalizedRoot}/${relativePath}`;
}

export function WebPreview({ url }: { url: string }) {
	const [status, setStatus] = useState<
		"loading" | "slow" | "ready" | "timeout"
	>("loading");

	useEffect(() => {
		setStatus("loading");
		const slowTimer = setTimeout(() => setStatus("slow"), 5000);
		const timeoutTimer = setTimeout(() => setStatus("timeout"), 15000);

		return () => {
			clearTimeout(slowTimer);
			clearTimeout(timeoutTimer);
		};
	}, []);

	if (status === "timeout") {
		return (
			<EmptyState
				title="资源加载超时"
				description="网页预览超过 15 秒仍未完成。你可以直接在默认浏览器中打开它。"
				action={
					<Button size="sm" onClick={() => void openUrl(url)}>
						<ExternalLink className="h-4 w-4" />
						在浏览器中打开
					</Button>
				}
			/>
		);
	}

	return (
		<div className="relative h-full w-full">
			{status !== "ready" ? (
				<div className="absolute inset-x-4 top-4 z-10 rounded-xl border border-border/70 bg-background/95 px-3 py-2 text-xs text-muted-foreground shadow-sm backdrop-blur-sm">
					{status === "slow"
						? "网页加载较慢，可继续等待或外部打开。"
						: "网页加载中…"}
				</div>
			) : null}
			<iframe
				src={url}
				className="h-full w-full border-0"
				title="资源预览"
				onLoad={() => setStatus("ready")}
			/>
		</div>
	);
}

export function PdfViewer({ path }: { path: string }) {
	const { data: settings } = useSettingsQuery();
	const assetUrl = useMemo(() => {
		if (!settings?.vault_path) return "";
		return convertFileSrc(joinVaultPath(settings.vault_path, path));
	}, [path, settings?.vault_path]);

	if (!assetUrl) {
		return <EmptyState title="PDF 暂不可用" description="正在准备资源地址。" />;
	}

	return (
		<iframe
			src={assetUrl}
			className="h-full w-full border-0"
			title="PDF 预览"
		/>
	);
}

export function ImagePreview({ path, title }: { path: string; title: string }) {
	const { data: settings } = useSettingsQuery();
	const assetUrl = useMemo(() => {
		if (!settings?.vault_path) return "";
		return convertFileSrc(joinVaultPath(settings.vault_path, path));
	}, [path, settings?.vault_path]);

	if (!assetUrl) {
		return <EmptyState title="图片暂不可用" description="正在准备资源地址。" />;
	}

	return (
		<div className="flex h-full items-center justify-center bg-muted/30 px-6 py-6">
			<img
				src={assetUrl}
				alt={title}
				className="max-h-full max-w-full rounded-2xl border border-border/70 bg-background object-contain shadow-lg"
			/>
		</div>
	);
}

export async function openSourceExternally(
	vaultPath: string,
	path: string,
	url?: string,
) {
	if (url) {
		await openUrl(url);
		return;
	}

	await openPath(joinVaultPath(vaultPath, path));
}
