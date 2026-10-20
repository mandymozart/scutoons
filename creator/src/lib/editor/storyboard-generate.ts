// The Generate panel's logic (#121, the studio's GeneratePanel / ManualGenerate / generate.svelte.js – workflows/
// storyboard-studio README "Generators"): which generator and model, "Build from the shot" (@scutoons/content lumaPrompt),
// the references (the shot's generations – a local file is shared to the work storage first – and the figures' character
// images on the CDN), the job body (estimated, confirmed and sent as one; the euro figures: storyboard-money.ts), the
// running jobs (polled while the page is visible), a
// finished job's result into the member's workspace folder (+ file, sha256, machine on the record) and "Luma – by hand"
// (no API: the prompt to copy, the result dropped onto the panel → the folder + a record). Paid calls: the server starts
// each exactly once; nothing here retries one. Pure – the API, the folder and the hash come in, so it is tested with
// fakes (test/storyboard-generate.test.ts); the panel's state: storyboard-generate.svelte.ts.
import { characterImage, characterIndex, resolveCharacterId, type CharacterRegistry } from '@scutoons/content';
import { lumaPrompt, moodOf, type Shot, type StoryboardDoc } from '@scutoons/content/storyboard';
import type { KeyValue } from './storyboard-steps.ts';
import { nextGenerationFile, type Workspace } from './storyboard-workspace.ts';
import {
	generationResult,
	recordGeneration,
	updateGeneration,
	type GenerateApi,
	type Generation,
	type GeneratorInfo,
	type GeneratorModel,
	type JobBody,
	type JobReference
} from './storyboard-generations.ts';

// ── Generators ───────────────────────────────────────────────────────────────────────────────────────────────

/** Luma – by hand: no API, free – the prompt to copy, the result dropped onto the panel (README "Generators") */
export const MANUAL = 'luma-manual';
export const MANUAL_MODEL = 'manual';

export type GeneratorChoice = GeneratorInfo & { manual?: boolean };

/** the server's generators + Luma – by hand (right after Luma's API generator, else last) */
export function withManual(list: readonly GeneratorInfo[], label = 'Luma – by hand'): GeneratorChoice[] {
	const manual: GeneratorChoice = {
		id: MANUAL,
		label,
		configured: true,
		setting: '',
		paid: false,
		unavailable: null,
		kinds: ['image'],
		manual: true,
		models: [{ id: MANUAL_MODEL, label: 'lumalabs.ai', kind: 'image', mode: 'manual', refs: { roles: ['still', 'reference', 'character', 'style'], max: 9 }, price: { currency: 'USD' } }]
	};
	const out: GeneratorChoice[] = list.filter((g) => g.id !== MANUAL);
	const luma = out.findIndex((g) => g.id.startsWith('luma'));
	out.splice(luma < 0 ? out.length : luma + 1, 0, manual);
	return out;
}

/** ready = it can start jobs; else why not (the panel shows the generator disabled with the reason) */
export type GeneratorState = 'ready' | 'not_configured' | 'unavailable';
export const generatorState = (g: GeneratorInfo): GeneratorState => (g.unavailable ? 'unavailable' : g.configured ? 'ready' : 'not_configured');

/** the generator to show first: the one picked last time (when it can run), else the first that can, else by hand */
export function pickGenerator(list: readonly GeneratorChoice[], remembered: string | null): GeneratorChoice | null {
	const ok = (g: GeneratorChoice) => generatorState(g) === 'ready';
	return list.find((g) => g.id === remembered && ok(g)) ?? list.find((g) => ok(g) && g.paid) ?? list.find(ok) ?? list[0] ?? null;
}

/** the model of the generator (the remembered one when it is there) */
export const pickModel = (g: GeneratorInfo | null, remembered: string | null): GeneratorModel | null => g?.models.find((m) => m.id === remembered) ?? g?.models[0] ?? null;

/** only Runway can stop a started task (Luma: 409 not_cancellable – API.md) */
export const CANCELLABLE: ReadonlySet<string> = new Set(['runway']);
export const isOpen = (g: Pick<Generation, 'status'>) => g.status === 'queued' || g.status === 'running';
export const canCancel = (g: Pick<Generation, 'status' | 'generator'>) => isOpen(g) && CANCELLABLE.has(g.generator);

/** the still's role for a model: Luma edits a `still`; Runway's image → video starts from it; Aleph takes a `control` video (+ an optional still) */
export function inputsOf(m: GeneratorModel | null): { still: boolean; stillRequired: boolean; control: boolean; duration: boolean } {
	const roles = m?.refs.roles ?? [];
	const required = m?.refs.required ?? [];
	return { still: roles.includes('still'), stillRequired: required.includes('still'), control: roles.includes('control'), duration: !!m?.durations };
}

// ── The shot's inputs ────────────────────────────────────────────────────────────────────────────────────────

/** the shot's finished generations that can go in: images as stills, videos as the control (Aleph) */
export function inputOptions(gens: readonly Generation[], shot: string): { stills: Generation[]; controls: Generation[] } {
	const done = gens.filter((g) => g.shot === shot && g.status === 'succeeded' && (g.file || g.work_path)).sort((a, b) => Number(b.approved) - Number(a.approved) || b.id - a.id);
	return { stills: done.filter((g) => g.kind === 'image'), controls: done.filter((g) => g.kind === 'video') };
}

/** the pre-filled still: the approved image of the shot (else none – a model that needs one waits for a pick) */
export const defaultStill = (stills: readonly Generation[]): number | null => stills.find((g) => g.approved)?.id ?? null;

/** a figure's character image on the CDN – the reference a provider fetches */
export interface CharacterRef {
	key: string;
	label: string;
	url: string;
	/** a provider fetches only public https links (the dev CDN is http) */
	usable: boolean;
}

/** the shot's figures → their character images (the registry's, by framing and mood; one per character + persona + mood) */
export function characterRefs(scene: Pick<StoryboardDoc, 'cast'>, shot: Pick<Shot, 'figures'>, registry: CharacterRegistry | null, url: (path: string) => string): CharacterRef[] {
	const index = characterIndex(registry);
	const cast = Object.fromEntries((scene.cast ?? []).map((c) => [c.ref, c]));
	const out: CharacterRef[] = [];
	for (const fig of shot.figures ?? []) {
		const c = cast[fig.cast];
		if (!c?.character) continue;
		const { character, persona } = resolveCharacterId(c.character, fig.persona ?? c.persona);
		const mood = moodOf(fig.mood);
		const view = ['cu', 'ecu', 'mcu'].includes(fig.framing ?? '') ? 'portrait' : 'full';
		const path = characterImage(index.get(character), { persona, look: c.look, mood }, view, { substitute: '' });
		const key = `${character}/${persona}/${mood}/${view}`;
		if (!path || out.some((r) => r.key === key)) continue;
		const href = url(path);
		out.push({ key, label: `${character}${persona !== 'regular' ? ` (${persona})` : ''} · ${mood}`, url: href, usable: href.startsWith('https://') });
	}
	return out;
}

/** "Build from the shot": everything set for the shot – unsaved changes too – as one prompt (+ the scene's script lines, what to attach) */
export function buildPrompt(scene: StoryboardDoc, shot: Shot, script: string[] | null, attach: { structure?: string; characters?: string[] } = {}): string {
	const references = attach.structure || attach.characters?.length ? { structure: attach.structure, characters: attach.characters } : undefined;
	return lumaPrompt(scene, shot, { script: script?.join('\n') || undefined, references });
}

/** the shot's aspect for the job: w:h only (a loop's or strip's own words stay out – the server snaps the ratio) */
export const jobAspect = (aspect: string | undefined): string | undefined => (aspect && /^\d{1,4}(\.\d+)?:\d{1,4}(\.\d+)?$/.test(aspect) ? aspect : undefined);

/** what is picked in the form */
export interface GenerateForm {
	generator: string;
	model: string;
	prompt: string;
	/** a generation id of the shot (still / control) */
	still: number | null;
	control: number | null;
	duration: number;
	/** the character images ticked (CharacterRef keys) */
	characters: string[];
	/** the seed to send again (a board card's – #125 Improve; null / missing = new: the provider picks one) */
	seed?: number | null;
}

/** a reference before it is sent: a generation (shared first when it is only local) or a CDN link */
export type PickedRef = { role: string; gen: Generation } | { role: string; url: string };

/** the references the form picks for the model (only the roles it takes) */
export function pickedRefs(form: GenerateForm, m: GeneratorModel | null, gens: readonly Generation[], chars: readonly CharacterRef[]): PickedRef[] {
	const take = inputsOf(m);
	const roles = m?.refs.roles ?? [];
	const out: PickedRef[] = [];
	const gen = (id: number | null) => (id === null ? undefined : gens.find((g) => g.id === id));
	const control = take.control ? gen(form.control) : undefined;
	if (control) out.push({ role: 'control', gen: control });
	const still = take.still ? gen(form.still) : undefined;
	if (still) out.push({ role: 'still', gen: still });
	if (roles.includes('character')) for (const c of chars) if (c.usable && form.characters.includes(c.key)) out.push({ role: 'character', url: c.url });
	return out.slice(0, m?.refs.max ?? out.length);
}

/** the job as it is estimated, confirmed and sent – one body for all three */
export function jobBody(at: { toon: string; chapter: number; shot: string; aspect?: string }, form: GenerateForm, m: GeneratorModel | null, references: JobReference[], parents: number[] = []): JobBody {
	const body: JobBody = { toon: at.toon, chapter: at.chapter, shot: at.shot, generator: form.generator, model: form.model };
	if (form.prompt.trim()) body.prompt = form.prompt;
	if (references.length) body.references = references;
	const aspect = jobAspect(at.aspect);
	if (aspect) body.aspect = aspect;
	if (m?.durations) body.params = { duration: Math.min(Math.max(Math.round(form.duration), m.durations.min), m.durations.max) };
	// a board card's seed again (#125 – only a model that takes one: Runway's video models)
	if (form.seed != null && m?.kind === 'video' && m.mode !== 'manual') body.params = { ...body.params, seed: form.seed };
	// the generated inputs, the main input first – the variants board hangs the result under its first parent (#125)
	if (parents.length) body.parents = parents;
	return body;
}

/** the generations the form's references are (the main input first: the control video before the still) – a job's
 *  `parents` (#125) */
export const pickedParents = (picked: readonly PickedRef[]): number[] => [...new Set(picked.flatMap((p) => ('gen' in p ? [p.gen.id] : [])))];

/** the stand-in host of a file not shared yet (the live estimate only – the server prices it by role and type, fetches nothing) */
export const UNSHARED_HOST = 'https://unshared.invalid/';

/**
 * The references as the live estimate sees them, without sharing anything: a file still only in the member's folder is
 * a stand-in link with its name (the price depends on the count and the type, not the bytes). Send shares it first and
 * estimates the real body again before the confirmation.
 */
export function previewRefs(picked: readonly PickedRef[]): JobReference[] {
	return picked.map((p) => ('url' in p ? { role: p.role, url: p.url } : p.gen.work_path ? { role: p.role, work: p.gen.work_path } : { role: p.role, url: `${UNSHARED_HOST}${encodeURIComponent((p.gen.file ?? `g${p.gen.id}.png`).split('/').pop()!)}` }));
}

/**
 * The references ready to send: a generation in the work storage goes as { role, work }; one only in the member's
 * folder is shared first (`share` – the existing Share: the file into the chapter's work storage, the record names it);
 * one elsewhere can't go (an error that says where it is).
 */
export async function resolveRefs(picked: readonly PickedRef[], share: (g: Generation) => Promise<Generation>, local: ReadonlySet<string>): Promise<{ references: JobReference[]; shared: Generation[] }> {
	const references: JobReference[] = [];
	const shared: Generation[] = [];
	for (const p of picked) {
		if ('url' in p) {
			references.push({ role: p.role, url: p.url });
			continue;
		}
		let g = p.gen;
		if (!g.work_path) {
			if (!g.file || !local.has(g.file)) throw new Error(`${g.file ?? `#${g.id}`} is not in your workspace folder${g.machine ? ` (it is on ${g.machine})` : ''} – share it from there first.`);
			g = await share(g);
			shared.push(g);
		}
		references.push({ role: p.role, work: g.work_path! });
	}
	return { references, shared };
}

// ── Running jobs ─────────────────────────────────────────────────────────────────────────────────────────────

/** how often the panel asks about open jobs (the server asks the provider at most every 5 s) */
export const POLL_MS = 5000;

/** poll while something is open and the page is visible; never for a hidden tab */
export const shouldPoll = (gens: readonly Pick<Generation, 'status'>[], visible: boolean): boolean => visible && gens.some(isOpen);

/** the jobs whose status changed to succeeded (their results are fetched once) */
export const newlyDone = (before: ReadonlyMap<number, Generation['status']>, after: readonly Generation[]): Generation[] =>
	after.filter((g) => g.status === 'succeeded' && before.has(g.id) && before.get(g.id) !== 'succeeded');

// ── Files: the workspace folder + the record ────────────────────────────────────────────────────────────────

/** the media a generation can be (the work storage takes these) */
export const MEDIA_TYPES: Record<string, { ext: string; kind: 'image' | 'video' }> = {
	'image/png': { ext: 'png', kind: 'image' },
	'image/jpeg': { ext: 'jpg', kind: 'image' },
	'image/webp': { ext: 'webp', kind: 'image' },
	'video/mp4': { ext: 'mp4', kind: 'video' },
	'video/webm': { ext: 'webm', kind: 'video' }
};
export const ACCEPT = Object.keys(MEDIA_TYPES).join(',');

/** a file's type: its content type, else its name's extension */
export function mediaOf(type: string, name = ''): { ext: string; kind: 'image' | 'video' } | null {
	const t = MEDIA_TYPES[type.split(';')[0].trim().toLowerCase()];
	if (t) return t;
	const ext = /\.([a-z0-9]+)$/i.exec(name)?.[1]?.toLowerCase();
	return Object.values(MEDIA_TYPES).find((x) => x.ext === ext || (ext === 'jpeg' && x.ext === 'jpg')) ?? null;
}

/** sha256 of a file as hex (crypto.subtle) */
export async function sha256Hex(blob: Blob, subtle: Pick<SubtleCrypto, 'digest'> = globalThis.crypto.subtle): Promise<string> {
	const digest = await subtle.digest('SHA-256', await blob.arrayBuffer());
	return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, '0')).join('');
}

/** what saving into the folder needs */
export interface SaveDeps {
	api: GenerateApi;
	workspace: Pick<Workspace, 'list' | 'write'>;
	/** this browser's machine label */
	machine: string;
	hash?: (blob: Blob) => Promise<string>;
}

/** a blob into the shot's next free file `<shot>--g<n>.<ext>` of the chapter's folder → the name */
async function intoFolder(deps: SaveDeps, toon: string, chapter: number, shot: string, blob: Blob, ext: string): Promise<string> {
	const file = nextGenerationFile(shot, await deps.workspace.list(toon, chapter), ext);
	await deps.workspace.write(toon, chapter, file, blob);
	return file;
}

/** a finished provider job: its result (through the API) into the folder, then the record names file + hash + machine */
export async function saveResult(deps: SaveDeps, g: Generation): Promise<Generation> {
	const blob = await generationResult(deps.api, g.id);
	const media = mediaOf(blob.type) ?? (g.kind === 'video' ? MEDIA_TYPES['video/mp4'] : MEDIA_TYPES['image/png']);
	const file = await intoFolder(deps, g.toon, g.chapter, g.shot, blob, media.ext);
	const file_hash = await (deps.hash ?? sha256Hex)(blob);
	return updateGeneration(deps.api, g.id, { file, file_hash, machine: deps.machine });
}

/** Luma – by hand: the file made on lumalabs.ai into the folder + a record (generator luma-manual, the prompt with it) */
export async function recordManual(deps: SaveDeps, at: { toon: string; chapter: number; shot: string }, file: Blob & { name?: string }, extra: { prompt?: string; ratio?: string; parents?: number[] } = {}): Promise<Generation> {
	const media = mediaOf(file.type, file.name);
	if (!media) throw new Error(`${file.name ?? 'The file'}: png, jpeg, webp, mp4 or webm only.`);
	const name = await intoFolder(deps, at.toon, at.chapter, at.shot, file, media.ext);
	const file_hash = await (deps.hash ?? sha256Hex)(file);
	const ratio = jobAspect(extra.ratio);
	return recordGeneration(deps.api, {
		toon: at.toon,
		chapter: at.chapter,
		shot: at.shot,
		kind: media.kind,
		generator: MANUAL,
		model: MANUAL_MODEL,
		status: 'succeeded',
		file: name,
		file_hash,
		machine: deps.machine,
		...(extra.prompt?.trim() ? { prompt: extra.prompt } : {}),
		...(ratio ? { ratio } : {}),
		...(extra.parents?.length ? { parents: extra.parents } : {})
	});
}

// ── This browser's machine label ─────────────────────────────────────────────────────────────────────────────

/** where a file is: a member's label for this browser (the record's `machine`; the server: letters, digits, space . _ -) */
export const DEFAULT_MACHINE = 'this browser';
const MACHINE_KEY = 'scutoons:storyboard-machine';
const MACHINE = /^[A-Za-z0-9][A-Za-z0-9 ._-]{0,63}$/;

/** a typed label → a valid one (umlauts and other signs become -, trimmed); '' / 'work' (the work storage's) → null */
export function cleanMachine(input: string): string | null {
	const s = input.normalize('NFKD').replace(/[̀-ͯ]/g, '').replace(/[^A-Za-z0-9 ._-]+/g, '-').replace(/^[^A-Za-z0-9]+/, '').trim().slice(0, 64);
	return s && MACHINE.test(s) && s !== 'work' ? s : null;
}

export function loadMachine(storage: KeyValue | null): string {
	try {
		return cleanMachine(storage?.getItem(MACHINE_KEY) ?? '') ?? DEFAULT_MACHINE;
	} catch {
		return DEFAULT_MACHINE;
	}
}

export function saveMachine(storage: KeyValue | null, label: string): string {
	const clean = cleanMachine(label) ?? DEFAULT_MACHINE;
	try {
		storage?.setItem(MACHINE_KEY, clean);
	} catch {
		// a blocked storage: the label lasts for this visit
	}
	return clean;
}
