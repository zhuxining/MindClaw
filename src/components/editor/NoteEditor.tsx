import { Crepe } from "@milkdown/crepe";
import "@milkdown/crepe/theme/common/style.css";
import "@milkdown/crepe/theme/frame.css";
import { useQuery } from "@tanstack/react-query";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { useEffect, useRef } from "react";
import { ipc } from "@/lib/ipc";
import { todayDate } from "@/queries/daily";
import { useWorkspaceStore } from "@/stores/workspace";

interface NoteEditorProps {
	/** 日记模式：传 date（YYYY-MM-DD） */
	date?: string;
	/** 普通笔记模式：传 vault 相对路径 */
	path?: string;
}

interface EditorInnerProps {
	defaultValue: string;
	onSave: (markdown: string) => void;
}

// key 由父层控制，确保切换文件时完整重建
function EditorInner({ defaultValue, onSave }: EditorInnerProps) {
	const containerRef = useRef<HTMLDivElement>(null);
	const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
	const onSaveRef = useRef(onSave);
	onSaveRef.current = onSave;

	useEffect(() => {
		const container = containerRef.current;
		if (!container) return;

		const crepe = new Crepe({ root: container, defaultValue });

		crepe.on((listener) => {
			listener.markdownUpdated((_ctx, markdown) => {
				if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
				saveTimerRef.current = setTimeout(
					() => onSaveRef.current(markdown),
					1000,
				);
			});
		});

		crepe.create();

		// Cmd+S 手动保存
		function handleKeyDown(e: KeyboardEvent) {
			if ((e.metaKey || e.ctrlKey) && e.key === "s") {
				e.preventDefault();
				if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
				onSaveRef.current(crepe.getMarkdown());
			}
		}
		window.addEventListener("keydown", handleKeyDown);

		return () => {
			window.removeEventListener("keydown", handleKeyDown);
			if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
			crepe.destroy().catch(console.error);
		};
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, [defaultValue]);

	return <div ref={containerRef} className="h-full" />;
}

function offsetDate(date: string, days: number): string {
	const d = new Date(date);
	d.setDate(d.getDate() + days);
	return d.toISOString().slice(0, 10);
}

export function NoteEditor({ date, path }: NoteEditorProps) {
	const effectiveDate = date ?? todayDate();
	const openItem = useWorkspaceStore((s) => s.openItem);

	function navigateDate(delta: number) {
		const newDate = offsetDate(effectiveDate, delta);
		openItem({
			type: "daily",
			date: newDate,
			path: `daily/${newDate}.md`,
		});
	}

	// 日记模式：gcTime=0 确保离开后缓存立即清除，重新打开时拿磁盘最新内容
	const dailyQuery = useQuery({
		queryKey: ["daily", effectiveDate],
		queryFn: () => ipc.getDaily(effectiveDate),
		enabled: !!date,
		gcTime: 0,
	});

	// 普通笔记模式
	const noteQuery = useQuery({
		queryKey: ["note", path],
		queryFn: () => ipc.readNote(path ?? ""),
		enabled: !!path && !date,
		gcTime: 0,
	});

	const isLoading = date ? dailyQuery.isLoading : noteQuery.isLoading;
	const content = date ? dailyQuery.data : noteQuery.data;

	if (isLoading) {
		return (
			<div className="flex h-full items-center justify-center text-sm text-muted-foreground">
				加载中…
			</div>
		);
	}

	const editorKey = date ? effectiveDate : (path ?? "");

	function handleSave(markdown: string) {
		const save = date
			? ipc.saveDaily(effectiveDate, markdown)
			: path
				? ipc.saveNote(path, markdown)
				: Promise.resolve();

		save.catch((err) => {
			console.error("[NoteEditor] save failed:", err);
		});
	}

	return (
		<div className="flex h-full flex-col">
			{date && (
				<div className="flex items-center justify-between border-b border-border px-4 py-1.5 shrink-0">
					<button
						type="button"
						onClick={() => navigateDate(-1)}
						className="rounded p-1 text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
						title="前一天"
					>
						<ChevronLeft className="h-4 w-4" />
					</button>
					<span className="text-sm font-medium">{effectiveDate}</span>
					<button
						type="button"
						onClick={() => navigateDate(1)}
						className="rounded p-1 text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
						title="后一天"
					>
						<ChevronRight className="h-4 w-4" />
					</button>
				</div>
			)}
			<div className="flex-1 overflow-y-auto px-8 py-6">
				<div className="mx-auto min-h-full max-w-3xl">
					<EditorInner
						key={editorKey}
						defaultValue={content ?? ""}
						onSave={handleSave}
					/>
				</div>
			</div>
		</div>
	);
}
