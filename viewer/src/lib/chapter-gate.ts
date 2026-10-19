import { isItemPublic, Status } from '@scutoons/content';

/**
 * Why the chapter page shows a gate instead of the chapter, from the season's status of the chapter (null = no season
 * lists it): `coming-soon` (listed, not released yet), `draft` (not public), null = readable. `preview` = a creator's
 * draft preview (it opens drafts, coming-soon chapters and chapters no season lists – the newest season may not list a
 * new chapter yet); `drafts` = drafts are readable (the preview or a dev build; on whenever `preview` is). The API applies the same rules (403).
 */
export function chapterGate(status: string | null, preview: boolean, drafts: boolean): 'draft' | 'coming-soon' | null {
	if (status === Status.COMING_SOON) return preview ? null : 'coming-soon';
	if (status === null) return drafts ? null : 'draft';
	return isItemPublic({ status }, drafts) ? null : 'draft';
}
