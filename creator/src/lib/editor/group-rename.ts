// Renaming a chapter group's id along (@scutoons/content renameGroupId): the open chapter changes in the editor (one undo
// step), the toon's other chapters that refer to the group are saved as new drafts in one batch.
import { describeGroupIdChange, groupIdProblem, renameGroupId, type GroupIdChange } from '@scutoons/content';

/** another chapter of the toon as loaded: its key, number, newest document and that version (the save's base) */
export interface OtherChapter {
	key: string;
	number: number;
	content: { groups?: unknown; [k: string]: unknown };
	base: number | null;
}

export interface GroupRenamePlan {
	/** the references rewritten, as a person reads them (the open chapter's first) */
	lines: string[];
	/** the other chapters to save: changed copies of `others` */
	saves: { key: string; content: unknown; base: number | null }[];
}

export interface GroupRenameInput {
	toon: string;
	chapter: number;
	from: string;
	to: string;
	/** the open chapter's document (not changed: the plan works on a copy) */
	current: { groups?: unknown; [k: string]: unknown };
	others: readonly OtherChapter[];
}

const copy = <T>(v: T): T => structuredClone(v);

/** why the open chapter can't take `to` as the id of its group `from`, or null */
export function groupRenameProblem(groups: readonly ({ id?: unknown } | null | undefined)[], from: string, to: string): string | null {
	const index = groups.findIndex((g) => g?.id === from);
	if (index < 0) return `no group "${from}" in this chapter`;
	return groupIdProblem(groups, index, to);
}

/**
 * A dry run of the rename on copies: which references change and which other chapters need a save. Throws when `to` is
 * invalid or taken (see `groupRenameProblem`).
 */
export function planGroupRename({ toon, chapter, from, to, current, others }: GroupRenameInput): GroupRenamePlan {
	const chapters = [{ number: chapter, doc: copy(current) }, ...others.map((o) => ({ number: o.number, doc: copy(o.content) }))];
	const changes = renameGroupId({ chapters, chapter, from, to, toon });
	const touched = new Set(changes.filter((c) => c.chapter !== chapter).map((c) => c.chapter));
	return {
		lines: changes.map((c: GroupIdChange) => describeGroupIdChange(c, chapters)),
		saves: others.flatMap((o) => (touched.has(o.number) ? [{ key: o.key, content: chapters.find((c) => c.number === o.number)!.doc, base: o.base }] : []))
	};
}

/** the rename on the open chapter's document itself (inside the editor's edit step); its own references go along */
export function applyGroupRename(file: { groups?: unknown; [k: string]: unknown }, input: { toon: string; chapter: number; from: string; to: string }): void {
	renameGroupId({ chapters: [{ number: input.chapter, doc: file }], chapter: input.chapter, from: input.from, to: input.to, toon: input.toon });
}
