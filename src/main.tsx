import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

const queryClient = new QueryClient({
	defaultOptions: {
		queries: {
			retry: 1,
			refetchOnWindowFocus: false,
		},
	},
});

// 注意：暂时移除 StrictMode，因为 Milkdown Crepe 在 React 19 的双重渲染下会崩溃
// TODO: 当 Milkdown 修复此问题后恢复 StrictMode
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
	<QueryClientProvider client={queryClient}>
		<App />
	</QueryClientProvider>,
);
