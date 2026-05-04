import { create } from "zustand";
import { persist } from "zustand/middleware";

export type ContentItemType = "note" | "daily" | "task" | "resource";

export interface DateRange {
	from: string; // YYYY-MM-DD
	to: string; // YYYY-MM-DD
}

export interface FilterQuery {
	tags?: string[];
	dateRange?: DateRange;
	types?: ContentItemType[];
}

export interface SavedFilter {
	id: string;
	name: string;
	query: FilterQuery;
	createdAt: string;
}

interface FilterStoreState {
	selectedTags: string[];
	dateRange: DateRange | null;
	selectedTypes: ContentItemType[];
	savedFilters: SavedFilter[];
	/** 当前激活的保存过滤 ID（用于高亮显示） */
	activeSavedFilterId: string | null;
}

interface FilterStoreActions {
	setTags: (tags: string[]) => void;
	addTag: (tag: string) => void;
	removeTag: (tag: string) => void;
	setDateRange: (range: DateRange | null) => void;
	setTypes: (types: ContentItemType[]) => void;
	addType: (type: ContentItemType) => void;
	removeType: (type: ContentItemType) => void;
	clearFilters: () => void;
	saveFilter: (name: string) => SavedFilter;
	loadFilter: (id: string) => void;
	deleteFilter: (id: string) => void;
	getFilterQuery: () => FilterQuery;
	hasActiveFilters: () => boolean;
}

const INITIAL_STATE: FilterStoreState = {
	selectedTags: [],
	dateRange: null,
	selectedTypes: [],
	savedFilters: [],
	activeSavedFilterId: null,
};

export const useFilterStore = create<FilterStoreState & FilterStoreActions>()(
	persist(
		(set, get) => ({
			...INITIAL_STATE,

			setTags: (tags) => set({ selectedTags: tags, activeSavedFilterId: null }),
			addTag: (tag) =>
				set((s) => ({
					selectedTags: s.selectedTags.includes(tag)
						? s.selectedTags
						: [...s.selectedTags, tag],
					activeSavedFilterId: null,
				})),
			removeTag: (tag) =>
				set((s) => ({
					selectedTags: s.selectedTags.filter((t) => t !== tag),
					activeSavedFilterId: null,
				})),

			setDateRange: (range) =>
				set({ dateRange: range, activeSavedFilterId: null }),

			setTypes: (types) =>
				set({ selectedTypes: types, activeSavedFilterId: null }),
			addType: (type) =>
				set((s) => ({
					selectedTypes: s.selectedTypes.includes(type)
						? s.selectedTypes
						: [...s.selectedTypes, type],
					activeSavedFilterId: null,
				})),
			removeType: (type) =>
				set((s) => ({
					selectedTypes: s.selectedTypes.filter((t) => t !== type),
					activeSavedFilterId: null,
				})),

			clearFilters: () =>
				set({
					selectedTags: [],
					dateRange: null,
					selectedTypes: [],
					activeSavedFilterId: null,
				}),

			saveFilter: (name) => {
				const query = get().getFilterQuery();
				const saved: SavedFilter = {
					id: crypto.randomUUID(),
					name,
					query,
					createdAt: new Date().toISOString(),
				};
				set((s) => ({
					savedFilters: [...s.savedFilters, saved],
					activeSavedFilterId: saved.id,
				}));
				return saved;
			},

			loadFilter: (id) => {
				const saved = get().savedFilters.find((f) => f.id === id);
				if (!saved) return;
				set({
					selectedTags: saved.query.tags ?? [],
					dateRange: saved.query.dateRange ?? null,
					selectedTypes: saved.query.types ?? [],
					activeSavedFilterId: id,
				});
			},

			deleteFilter: (id) =>
				set((s) => ({
					savedFilters: s.savedFilters.filter((f) => f.id !== id),
					activeSavedFilterId:
						s.activeSavedFilterId === id ? null : s.activeSavedFilterId,
				})),

			getFilterQuery: (): FilterQuery => {
				const state = get();
				const query: FilterQuery = {};
				if (state.selectedTags.length > 0) {
					query.tags = state.selectedTags;
				}
				if (state.dateRange) {
					query.dateRange = state.dateRange;
				}
				if (state.selectedTypes.length > 0) {
					query.types = state.selectedTypes;
				}
				return query;
			},

			hasActiveFilters: () =>
				get().selectedTags.length > 0 ||
				get().dateRange !== null ||
				get().selectedTypes.length > 0,
		}),
		{
			name: "mindclaw-filter-store",
			partialize: (state) => ({
				savedFilters: state.savedFilters,
			}),
		},
	),
);
