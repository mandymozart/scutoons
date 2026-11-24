// The shot's asset slot (lap 2 C3c, AI-HOOKS.md §0): a picture or a video from the Library, an upload, or a generator
// added from the Library – the generator stands in until a take is assigned. The pick is stored in the shot (`asset`, a
// Library address – @scutoons/content/storyboard handover.ts); empty, the slot shows the shot's approved generation.
// Pure: test/storyboard-slot.test.ts.
import { fileShotAsset, kindOf, readShotAsset } from '@scutoons/content/storyboard';

/** what the slot shows */
export type SlotContent =
	| { at: 'file'; path: string; video: boolean }
	| { at: 'generator'; address: string; name: string }
	| { at: 'approved'; generation: number; video: boolean }
	| { at: 'empty' };

/** the asset address of a picked file: its Library family's main file when the Library knows it, else the path itself */
export function pickedFile(path: string, family?: { kind: string; main: string } | null): string | null {
	return family ? fileShotAsset(family.kind, family.main) : fileShotAsset(kindOf(path), path);
}

/** what a shot's slot shows: its asset, else its approved generation with a file, else nothing */
export function slotContent(asset: string | undefined, approved: { id: number; file: string | null; kind?: string } | undefined): SlotContent {
	const read = readShotAsset(asset);
	if (read?.at === 'file') return { at: 'file', path: read.path, video: read.kind === 'video' };
	if (read?.at === 'generator') return { at: 'generator', address: read.address, name: read.id };
	if (approved?.file) return { at: 'approved', generation: approved.id, video: (approved.kind ?? kindOf(approved.file)) === 'video' };
	return { at: 'empty' };
}
