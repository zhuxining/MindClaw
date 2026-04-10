import { Crepe } from "@milkdown/crepe";
import "@milkdown/crepe/theme/common/style.css";
import "@milkdown/crepe/theme/frame.css";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { ipc } from "@/lib/ipc";
import { todayDate } from "@/queries/daily";

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

export function NoteEditor({ date, path }: NoteEditorProps) {
	const effectiveDate = date ?? todayDate();

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
		<div className="h-full overflow-y-auto px-8 py-6">
			<div className="mx-auto min-h-full max-w-3xl">
				<EditorInner
					key={editorKey}
					defaultValue={content ?? ""}
					onSave={handleSave}
				/>
			</div>
		</div>
	);
}
