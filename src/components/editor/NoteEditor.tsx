import { Crepe } from "@milkdown/crepe";
import "@milkdown/crepe/theme/common/style.css";
import "@milkdown/crepe/theme/frame.css";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { ipc } from "@/lib/ipc";
import type { EditorSaveState } from "@/lib/types";

interface NoteEditorProps {
	date?: string;
	path: string;
	onSaveStateChange?: (state: EditorSaveState) => void;
}

interface EditorInnerProps {
	defaultValue: string;
	onSave: (markdown: string) => Promise<void>;
	onSaveStateChange?: (state: EditorSaveState) => void;
}

function EditorInner({
	defaultValue,
	onSave,
	onSaveStateChange,
}: EditorInnerProps) {
	const containerRef = useRef<HTMLDivElement>(null);
	const crepeRef = useRef<Crepe | null>(null);
	const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
	const stateTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
	const onSaveRef = useRef(onSave);
	const onSaveStateChangeRef = useRef(onSaveStateChange);
	const lastPersistedRef = useRef(defaultValue);
	onSaveRef.current = onSave;
	onSaveStateChangeRef.current = onSaveStateChange;

	useEffect(() => {
		const container = containerRef.current;
		if (!container) return;

		lastPersistedRef.current = defaultValue;
		onSaveStateChangeRef.current?.("idle");

		const crepe = new Crepe({ root: container, defaultValue });
		crepeRef.current = crepe;

		async function persist(markdown: string) {
			if (markdown === lastPersistedRef.current) return;

			if (stateTimerRef.current) clearTimeout(stateTimerRef.current);
			onSaveStateChangeRef.current?.("saving");

			try {
				await onSaveRef.current(markdown);
				lastPersistedRef.current = markdown;
				onSaveStateChangeRef.current?.("saved");
				stateTimerRef.current = setTimeout(() => {
					onSaveStateChangeRef.current?.("idle");
				}, 2000);
			} catch (error) {
				console.error("[NoteEditor] save failed:", error);
				onSaveStateChangeRef.current?.("error");
			}
		}

		crepe.on((listener) => {
			listener.markdownUpdated((_ctx, markdown) => {
				if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
				saveTimerRef.current = setTimeout(() => {
					void persist(markdown);
				}, 1000);
			});
		});

		void crepe.create().then(() => {
			console.log("[NoteEditor] Crepe editor created");
		});

		function handleKeyDown(event: KeyboardEvent) {
			if ((event.metaKey || event.ctrlKey) && event.key === "s") {
				event.preventDefault();
				if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
				void persist(crepe.getMarkdown());
			}
		}

		window.addEventListener("keydown", handleKeyDown);

		return () => {
			window.removeEventListener("keydown", handleKeyDown);
			if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
			if (stateTimerRef.current) clearTimeout(stateTimerRef.current);

			const crepe = crepeRef.current;
			crepeRef.current = null;

			if (crepe) {
				const markdown = crepe.getMarkdown();
				if (markdown !== lastPersistedRef.current) {
					void onSaveRef.current(markdown);
				}
				void crepe.destroy().catch(console.error);
			}
		};
	}, [defaultValue]);

	return <div ref={containerRef} className="note-editor h-full" />;
}

export function NoteEditor({ date, path, onSaveStateChange }: NoteEditorProps) {
	const dailyQuery = useQuery({
		queryKey: ["daily", date],
		queryFn: () => ipc.getDaily(date ?? ""),
		enabled: Boolean(date),
		gcTime: 0,
	});

	const noteQuery = useQuery({
		queryKey: ["note", path],
		queryFn: () => ipc.readNote(path),
		enabled: !date,
		gcTime: 0,
	});

	const isLoading = date ? dailyQuery.isLoading : noteQuery.isLoading;
	const content = date ? dailyQuery.data : noteQuery.data;

	useEffect(() => {
		onSaveStateChange?.("idle");
	}, [onSaveStateChange]);

	if (isLoading) {
		return (
			<div className="flex h-full items-center justify-center text-sm text-muted-foreground">
				加载中…
			</div>
		);
	}

	async function handleSave(markdown: string) {
		if (date) {
			await ipc.saveDaily(date, markdown);
			return;
		}
		await ipc.saveNote(path, markdown);
	}

	return (
		<div className="flex h-full flex-col">
			<div className="flex-1 overflow-y-auto px-8 py-8">
				<div className="mx-auto min-h-full max-w-4xl">
					<EditorInner
						defaultValue={content ?? ""}
						onSave={handleSave}
						{...(onSaveStateChange ? { onSaveStateChange } : {})}
					/>
				</div>
			</div>
		</div>
	);
}
