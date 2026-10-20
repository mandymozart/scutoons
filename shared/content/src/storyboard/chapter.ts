// The per-chapter document `storyboard:<toon>:<chapter>` (STUDIO-IN-CREATOR §2.2): the chapter's scenes inline, in
// scene order – what the import migration (#120) builds from the scene files and what the creator page edits.

import type { ChapterStoryboard, Shot, StoryboardDoc } from './types.ts';

/** the entity key of a chapter's storyboard */
export const storyboardKey = (toon: string, chapter: number): string => `storyboard:${toon}:${chapter}`;

/** scene documents → the chapter document, sorted by scene number (stable, as the old loader sorted the files) */
export function chapterStoryboard(toon: string, chapter: number, scenes: StoryboardDoc[]): ChapterStoryboard {
	return { storyboard: 2, toon, chapter, scenes: [...scenes].sort((a, b) => (a.source?.scene ?? 0) - (b.source?.scene ?? 0)) };
}

/** a shot by id, with its scene */
export function findShot(doc: ChapterStoryboard, id: string): { scene: StoryboardDoc; shot: Shot } | null {
	for (const scene of doc.scenes) for (const g of scene.groups ?? []) for (const shot of g.shots ?? []) if (shot.id === id) return { scene, shot };
	return null;
}
