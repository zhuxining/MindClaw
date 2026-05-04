import { DirectoryPanel } from "./directory-panel";

interface FileExplorerPaneProps {
	scope: string;
}

export function FileExplorerPane(props: FileExplorerPaneProps) {
	void props.scope;
	return <DirectoryPanel />;
}
