import { useQuery } from "@tanstack/react-query";
import { Puzzle, Search, Star } from "lucide-react";
import { useState } from "react";
import { ipc } from "@/lib/ipc";
import type { SkillMetadata } from "@/lib/types";

export function SkillListPane() {
	const [searchQuery, setSearchQuery] = useState("");

	const { data: skills = [], isLoading } = useQuery({
		queryKey: ["skills"],
		queryFn: () => ipc.listSkills(),
	});

	const filteredSkills =
		searchQuery.trim().length > 0
			? skills.filter(
					(skill) =>
						skill.name
							.toLowerCase()
							.includes(searchQuery.trim().toLowerCase()) ||
						skill.description
							.toLowerCase()
							.includes(searchQuery.trim().toLowerCase()),
				)
			: skills;

	const residentSkills = filteredSkills.filter((s) => s.always_load);
	const discoverableSkills = filteredSkills.filter((s) => !s.always_load);

	if (isLoading) {
		return (
			<div className="flex h-full items-center justify-center p-4 text-sm text-muted-foreground">
				加载技能列表…
			</div>
		);
	}

	return (
		<div className="flex h-full flex-col">
			<div
				className="border-b p-4"
				style={{ borderColor: "var(--flexoki-bg-2)" }}
			>
				<div className="mb-3">
					<p className="text-sm font-semibold text-foreground">Agent 技能</p>
					<p className="text-xs text-muted-foreground">扩展 Agent 能力的模块</p>
				</div>

				<div className="relative">
					<Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
					<input
						type="text"
						value={searchQuery}
						onChange={(e) => setSearchQuery(e.target.value)}
						placeholder="搜索技能"
						className="w-full rounded-lg border border-border/50 bg-background pl-9 pr-3 py-2 text-sm outline-none focus:border-primary"
					/>
				</div>
			</div>

			<div className="min-h-0 flex-1 overflow-y-auto p-3">
				{filteredSkills.length === 0 ? (
					<div className="py-8 text-center text-sm text-muted-foreground">
						{skills.length === 0
							? "暂无技能（vault/skills/ 目录下添加）"
							: "未找到匹配技能"}
					</div>
				) : (
					<div className="space-y-4">
						{residentSkills.length > 0 && (
							<div>
								<p className="mb-2 text-xs font-medium text-muted-foreground">
									常驻加载 ({residentSkills.length})
								</p>
								<ul className="space-y-1.5">
									{residentSkills.map((skill) => (
										<SkillItem key={skill.name} skill={skill} />
									))}
								</ul>
							</div>
						)}

						{discoverableSkills.length > 0 && (
							<div>
								<p className="mb-2 text-xs font-medium text-muted-foreground">
									可发现技能 ({discoverableSkills.length})
								</p>
								<ul className="space-y-1.5">
									{discoverableSkills.map((skill) => (
										<SkillItem key={skill.name} skill={skill} />
									))}
								</ul>
							</div>
						)}
					</div>
				)}
			</div>
		</div>
	);
}

function SkillItem({ skill }: { skill: SkillMetadata }) {
	return (
		<li>
			<div className="rounded-lg border border-border/50 p-3 transition-colors hover:bg-muted/50">
				<div className="flex items-center gap-2">
					<Puzzle className="h-4 w-4 text-muted-foreground" />
					<p className="truncate text-sm font-medium">{skill.name}</p>
					{skill.always_load && (
						<Star className="h-3.5 w-3.5 text-primary fill-primary" />
					)}
				</div>
				<p className="mt-1 text-xs text-muted-foreground line-clamp-2">
					{skill.description || "无描述"}
				</p>
				<p className="mt-1.5 text-xs text-muted-foreground/70 truncate">
					{skill.path}
				</p>
			</div>
		</li>
	);
}
