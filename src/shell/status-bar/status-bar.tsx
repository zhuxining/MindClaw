import { useShellStore } from "@/stores/shell";

export function StatusBar() {
	const statusBar = useShellStore((s) => s.statusBar);

	return (
		<div
			className="flex shrink-0 items-center justify-between px-3 text-xs"
			style={{
				height: 20,
				color: "var(--flexoki-tx-2)",
				borderTop: "1px solid var(--flexoki-bg-2)",
			}}
		>
			<span>{statusBar.lineCol}</span>
			<span>{statusBar.encoding}</span>
		</div>
	);
}
