// A chapter opened for a storyboard shot (`?shot=<id>`, storyboard-handover.ts): the slice cut from it (`Slice.shot`), or
// – on first use – a new one, placed by the storyboard's order (@scutoons/content/storyboard sliceSpotFor) and edited
// into the chapter like any added slice (undo, autosave). Its picture is the shot's asset when that is a picture, else the
// next slice name. Pure but for the callbacks: test/chapter-shot.test.ts.
import { DEFAULT_SLICE, toonAssetRef } from '@scutoons/content';
import { readShotAsset, sceneName, sliceOfShot, sliceSpotFor, type ChapterStoryboard, type SliceGroups, type SliceSpot } from '@scutoons/content/storyboard';
import { nextSliceImage } from './defaults.ts';
import { shotEntries } from './storyboard-shots.ts';

export interface ShotSliceDoc {
	/** the chapter as it is now */
	data(): SliceGroups | null;
	/** one undoable edit of the chapter */
	edit(change: (file: any) => void): void;
}

/** why no slice: the storyboard does not have the shot, or the chapter has no group to put it in */
export type ShotSliceAnswer = { ok: true; spot: SliceSpot; made: boolean } | { ok: false; why: 'no-shot' | 'no-group' };

/** the slice of `shot` – found, or made from the storyboard's shot */
export async function shotSlice(
	doc: ShotSliceDoc,
	at: { toon: string; chapter: number; shot: string },
	storyboard: () => Promise<ChapterStoryboard | null>
): Promise<ShotSliceAnswer> {
	const found = doc.data() ? sliceOfShot(doc.data()!, at.shot) : null;
	if (found) return { ok: true, spot: found, made: false };
	const entries = shotEntries(await storyboard());
	const entry = entries.find((e) => e.id === at.shot);
	if (!entry) return { ok: false, why: 'no-shot' };
	const chapter = doc.data();
	const spot = chapter ? sliceSpotFor(chapter, entries.map((e) => e.id), at.shot, sceneName(entry.scene)) : null;
	if (!spot) return { ok: false, why: 'no-group' };
	const asset = readShotAsset(entry.shot.asset);
	doc.edit((file) => {
		const group = file.groups[spot.group];
		group.slices ??= [];
		const image = asset?.at === 'file' && asset.kind === 'image' ? toonAssetRef(at.toon, asset.path) : nextSliceImage(group.slices, spot.slice, at.chapter);
		group.slices.splice(spot.slice, 0, { image, width: DEFAULT_SLICE.width, height: DEFAULT_SLICE.height, shot: at.shot });
	});
	return { ok: true, spot, made: true };
}
