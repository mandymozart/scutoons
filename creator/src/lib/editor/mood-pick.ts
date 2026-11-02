// The mood chip's faces (#282): which portrait a mood shows. Pure.

/** the portrait path of a mood (scene-content's `characters.<id>.portraits`): the mood's own, else neutral's, else '' (no face) */
export function moodFace(portraits: Readonly<Record<string, { path: string }>>, mood: string): string {
	return portraits[mood]?.path ?? portraits.neutral?.path ?? '';
}
