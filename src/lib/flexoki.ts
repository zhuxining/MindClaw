// Flexoki color system — source of truth: docs/ui/desktop-ui.pen
// Light theme base: paper #FFFCF0, Dark theme base: black #100F0F

export const flexoki = {
	light: {
		paper: "#FFFCF0",
		bg: "#F2F0E5",
		"bg-2": "#E6E4D9",
		ui: "#DAD8CE",
		"ui-2": "#CECDC3",
		"ui-3": "#B7B5AC",
		tx: "#100F0F",
		"tx-2": "#6F6E69",
		"tx-3": "#B7B5AC",
		accent: {
			magenta: "#A02F6F",
			purple: "#5E409D",
			blue: "#205EA6",
			cyan: "#24837B",
			green: "#66800B",
			yellow: "#AD8301",
			orange: "#BC5215",
			red: "#AF3029",
		},
		border: "#E6E4D9",
	},
	dark: {
		black: "#100F0F",
		bg: "#1C1B1A",
		"bg-2": "#282726",
		ui: "#343331",
		"ui-2": "#403E3C",
		"ui-3": "#575653",
		tx: "#CECDC3",
		"tx-2": "#878580",
		"tx-3": "#575653",
		accent: {
			magenta: "#CE5D97",
			purple: "#8B7EC8",
			blue: "#4385BE",
			cyan: "#3AA99F",
			green: "#879A39",
			yellow: "#D0A215",
			orange: "#DA702C",
			red: "#D14D41",
		},
		border: "#282726",
	},
} as const;

export type FlexokiAccent = keyof typeof flexoki.light.accent;
