// Files that come onto the variants board (#125; the port of #62's drops + frame → still – agents/research/
// VARIANTS-CANVAS.md §3.2 "drops", §3.7 "Video → still"; files: STUDIO-IN-CREATOR §2.2 – the member's workspace folder,
// the shared work storage, never the readers' CDN):
//   - a DROP onto a band (or the tool row's +): one record per file for THAT band's shot – `generator: 'upload'`, no
//     parents: a source card. Into the member's workspace folder when one is chosen (the record names the machine), else
//     straight into the chapter's work storage (machine `work`) – nothing is lost without a folder;
//   - a FRAME of a video (More ▾ "Still: …"): cut in the browser (storyboard-posters.ts cutFrame), recorded as an image
//     with the video as its parent (`generator: 'frame'`) – it then goes into the Generate form as the still.
// Pure: the API, the folder and the hash come in (test/storyboard-board-files.test.ts).
import { mediaOf, sha256Hex, type SaveDeps } from './storyboard-generate.ts';
import { nextGenerationFile } from './storyboard-workspace.ts';
import { recordGeneration, type Generation } from './storyboard-generations.ts';

export interface At {
	toon: string;
	chapter: number;
	shot: string;
}
/** where a new file goes: the folder (SaveDeps) or – no folder – the work storage (names taken there + the records') */
export type FileDeps = Omit<SaveDeps, 'workspace'> & {
	workspace: SaveDeps['workspace'] | null;
	/** the names already taken in the chapter's work storage and by its records (without a folder) */
	taken?: () => Promise<string[]>;
};

const q = (o: Record<string, string | number>) => Object.entries(o).map(([k, v]) => `${k}=${encodeURIComponent(String(v))}`).join('&');

/** a blob for `at.shot` → its name, and where it went ({ file, machine, work_path }) */
async function place(deps: FileDeps, at: At, blob: Blob, ext: string): Promise<{ file: string; machine: string; work_path?: string }> {
	if (deps.workspace) {
		const file = nextGenerationFile(at.shot, await deps.workspace.list(at.toon, at.chapter), ext);
		await deps.workspace.write(at.toon, at.chapter, file, blob);
		return { file, machine: deps.machine };
	}
	const file = nextGenerationFile(at.shot, (await deps.taken?.()) ?? [], ext);
	await deps.api.put(`/storyboard/work?${q({ toon: at.toon, chapter: at.chapter, name: file })}`, blob);
	return { file, machine: 'work', work_path: file };
}

/** a dropped file → a source card of its shot (png, jpeg, webp, mp4, webm) */
export async function recordUpload(deps: FileDeps, at: At, file: Blob & { name?: string }, ratio?: string): Promise<Generation> {
	const media = mediaOf(file.type, file.name);
	if (!media) throw new Error(`${file.name ?? 'The file'}: png, jpeg, webp, mp4 or webm only.`);
	const where = await place(deps, at, file, media.ext);
	const file_hash = await (deps.hash ?? sha256Hex)(file);
	return recordGeneration(deps.api, { ...at, kind: media.kind, generator: 'upload', status: 'succeeded', ...where, file_hash, ...(ratio && /^\d{1,4}:\d{1,4}$/.test(ratio) ? { ratio } : {}) });
}

/** a video's frame (a PNG) → an image of its shot, the video its parent */
export async function recordFrame(deps: FileDeps, at: At, png: Blob, video: Pick<Generation, 'id'>, note: string): Promise<Generation> {
	const where = await place(deps, at, png, 'png');
	const file_hash = await (deps.hash ?? sha256Hex)(png);
	return recordGeneration(deps.api, { ...at, kind: 'image', generator: 'frame', status: 'succeeded', parents: [video.id], ...where, file_hash, note });
}

/** the names taken in the chapter's work storage + by the records of the chapter (a file's next free g<n>) */
export async function takenNames(api: Pick<SaveDeps['api'], 'get'>, at: Omit<At, 'shot'>, records: readonly Pick<Generation, 'file' | 'work_path'>[]): Promise<string[]> {
	const r = await api.get<{ items?: { name: string }[] }>(`/storyboard/work?${q({ toon: at.toon, chapter: at.chapter })}`).catch(() => ({ items: [] as { name: string }[] }));
	return [...(r.items ?? []).map((x) => x.name), ...records.flatMap((g) => [g.file ?? '', g.work_path ?? ''])].filter(Boolean);
}
