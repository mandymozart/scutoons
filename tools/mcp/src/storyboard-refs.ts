// The references for a shot's Luma prompt (storyboard_prompt, #122), found on the member's machine – the MCP runs
// there (agents/research/STUDIO-IN-CREATOR.md §2.4): the 3D render as the structure, the style pictures, each figure's
// character image. The choice is content's (shotCharacters / RefSource, @scutoons/content/storyboard refs.ts); this
// module only says which files exist. Where: `SCUTOONS_STORYBOARDS` (a storyboards folder: style/, style/refs/,
// _out/<s?c?>/frames/), else a checkout's `storyboards/` and its `media/` dev copy. A member's copy without either
// gets the prompt without paths (the render comes from the creator's 3D view).
import { existsSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { shotCharacters, type RefSource, type Shot, type StoryboardDoc } from '@scutoons/content/storyboard';
import { BUNDLED, ROOT } from './config.ts';

export interface StudioFolders {
	/** the storyboards folder (style/, style/refs/, <toon>/s<season>c<chapter>/, _out/) */
	storyboards: string | null;
	/** the media folder character images are looked up in (characters/<folder>/…) */
	media: string | null;
}

const IMAGE = /\.(png|jpe?g|webp)$/i;
const list = (dir: string): string[] | null => {
	try {
		return readdirSync(dir);
	} catch {
		return null;
	}
};

/** the folders on this machine: the env setting, else a checkout's (never in a member's one-file copy) */
export function studioFolders(env: NodeJS.ProcessEnv = process.env, root: string = ROOT, bundled: boolean = BUNDLED): StudioFolders {
	const at = (p: string) => (existsSync(p) ? p : null);
	const storyboards = env.SCUTOONS_STORYBOARDS ? at(resolve(env.SCUTOONS_STORYBOARDS)) : bundled ? null : at(resolve(root, 'storyboards'));
	const media = env.SCUTOONS_MEDIA ? at(resolve(env.SCUTOONS_MEDIA)) : bundled ? null : at(resolve(root, 'media'));
	return { storyboards, media };
}

/** content's RefSource over the folders: style/refs/<character>/<persona>/<look>/ and the media dev copy */
export function folderRefSource(f: StudioFolders): RefSource {
	return {
		refFiles: (id) => (f.storyboards ? list(resolve(f.storyboards, 'style/refs', id)) : null),
		mediaHas: f.media ? (rel) => existsSync(resolve(f.media!, rel)) : undefined
	};
}

/** the chapter's work folder name in storyboards/<toon>/ (s1c3 …) – the old tools' `_out/<name>/` */
function chapterFolder(f: StudioFolders, toon: string, chapter: number): string | null {
	if (!f.storyboards) return null;
	return (list(resolve(f.storyboards, toon)) ?? []).find((n) => new RegExp(`^s\\d+c${chapter}$`).test(n)) ?? null;
}

export interface ShotReferences {
	/** the 3D render (clean, else the frame with marks) */
	structure: string | null;
	style: string[];
	characters: string[];
	/** what is missing, in words */
	missing: string[];
}

/** the files to attach in Luma for one shot (absolute paths on this machine) */
export function shotReferences(f: StudioFolders, toon: string, chapter: number, scene: StoryboardDoc, shot: Shot): ShotReferences {
	const missing: string[] = [];
	const name = chapterFolder(f, toon, chapter);
	const frames = name && f.storyboards ? resolve(f.storyboards, '_out', name, 'frames') : null;
	const structure = frames ? [resolve(frames, '..', 'clean', `${shot.id}.png`), resolve(frames, `${shot.id}.png`)].find((p) => existsSync(p)) ?? null : null;
	if (!structure) missing.push(`no render of ${shot.id} on this machine – take the structure image from the creator's 3D view`);
	const styleDir = f.storyboards ? resolve(f.storyboards, 'style') : null;
	const style = styleDir ? (list(styleDir) ?? []).filter((n) => IMAGE.test(n)).sort().map((n) => resolve(styleDir, n)) : [];
	if (!style.length) missing.push('no style pictures (storyboards/style/*.png)');
	const characters: string[] = [];
	for (const { fig, cast, image } of shotCharacters(scene, shot, folderRefSource(f))) {
		if (!image) {
			missing.push(`no character image for ${fig.cast} (${cast.character ?? 'new'} / ${cast.persona ?? 'regular'})`);
			continue;
		}
		const base = image.rel.startsWith('refs/') ? resolve(f.storyboards!, 'style') : f.media!;
		characters.push(resolve(base, image.rel));
	}
	return { structure, style, characters, missing };
}
