import type { ReactNode } from "react";
import { TabArea } from "./tab-area";

interface ContentHostProps {
	children?: ReactNode;
}

export function ContentHost({ children }: ContentHostProps) {
	return (
		<div className="flex min-h-0 flex-1 flex-col">
			<TabArea />
			<div className="min-h-0 flex-1">{children}</div>
		</div>
	);
}
