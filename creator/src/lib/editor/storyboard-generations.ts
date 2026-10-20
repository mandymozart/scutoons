// A shot's generations on the Storyboard page (#123; the server's ledger #120 – server/src/Http/Controllers/
// StoryboardController.php, docs/API.md "Storyboard"): the list, the review (mark / approve) and Share – the member's
// local file copied into the shared work storage (STUDIO-IN-CREATOR §2.2 place 2: the owner's own servers, signed-in
// members only, never the readers' CDN). A file is shown from the member's workspace folder when it is there, else from
// the work storage, else as "on <machine>". Starting generations is #121; the full board #125.
import type { GenerationMark } from '@scutoons/content/storyboard';
import type { Workspace } from './storyboard-workspace.ts';

/** a generation as the server answers it (Generations::view) */
export interface Generation {
	id: number;
	toon: string;
	chapter: number;
	shot: string;
	kind: 'image' | 'video';
	take: string | null;
	parents: number[];
	generator: string;
	model: string | null;
	status: 'queued' | 'running' | 'succeeded' | 'failed' | 'cancelled';
	/** a provider job's progress 0–1 (#121; null = not reported yet) */
	progress?: number | null;
	/** the provider's result link (a provider job, succeeded – fetched through `…/{id}/result`) */
	result_url?: string | null;
	prompt: string | null;
	seed: number | null;
	ratio: string | null;
	duration: number | null;
	estimate: number | null;
	cost: number | null;
	currency: string | null;
	/** the file in the member's workspace folder (relative to <toon>/<chapter>) */
	file: string | null;
	/** the machine the file is on ('work' = only in the work storage) */
	machine: string | null;
	/** its name in the chapter's work storage once shared */
	work_path: string | null;
	/** the API path that serves it (`/api/storyboard/work/…`) */
	work_url: string | null;
	mark: GenerationMark | null;
	approved: boolean;
	dataset: boolean;
	note: string | null;
	error: string | null;
	author: string | null;
	via: string | null;
	created_at: string | null;
}

/** the review fields a PUT takes */
export type GenerationPatch = Partial<Pick<Generation, 'mark' | 'approved' | 'dataset' | 'note' | 'machine' | 'work_path' | 'file'>> & { file_hash?: string };

/** the part of the creator's API client this module uses (@scutoons/api ApiClient) */
export interface StoryboardApi {
	get<T>(path: string): Promise<T>;
	put<T>(path: string, body?: unknown): Promise<T>;
	fetchRaw(path: string): Promise<Response>;
}

const q = (o: Record<string, string | number>) => Object.entries(o).map(([k, v]) => `${k}=${encodeURIComponent(String(v))}`).join('&');

/** a chapter's generations (or one shot's), oldest first */
export async function listGenerations(api: StoryboardApi, toon: string, chapter: number, shot?: string): Promise<Generation[]> {
	const r = await api.get<{ items: Generation[] }>(`/storyboard/generations?${q({ toon, chapter, ...(shot ? { shot } : {}) })}`);
	return r.items ?? [];
}

export async function updateGeneration(api: StoryboardApi, id: number, patch: GenerationPatch): Promise<Generation> {
	return (await api.put<{ item: Generation }>(`/storyboard/generations/${id}`, patch)).item;
}

/** the file name a generation gets in the work storage: its own name, without folders */
export const workName = (g: Pick<Generation, 'file'>) => (g.file ?? '').split('/').pop() ?? '';

/** the work storage's API path relative to the client's base (`/api/storyboard/…` → `/storyboard/…`) */
export const apiPath = (url: string) => url.replace(/^\/api(?=\/)/, '');

/** where a generation's file can be seen from here */
export type FilePlace = { at: 'local' } | { at: 'work'; url: string } | { at: 'result'; id: number } | { at: 'elsewhere'; machine: string | null } | { at: 'none' };

/**
 * the workspace folder's copy first (`local` = the names in this member's chapter folder), then the work storage, then
 * a provider job's result not saved anywhere yet (`result` – through the API while the provider's link lives, #121)
 */
export function placeOf(g: Pick<Generation, 'file' | 'machine' | 'work_url' | 'status'> & Partial<Pick<Generation, 'id' | 'result_url'>>, local: ReadonlySet<string>): FilePlace {
	if (g.file && local.has(g.file)) return { at: 'local' };
	if (g.work_url) return { at: 'work', url: g.work_url };
	if (!g.file && g.result_url && g.status === 'succeeded' && g.id !== undefined) return { at: 'result', id: g.id };
	if (g.file && g.status === 'succeeded') return { at: 'elsewhere', machine: g.machine };
	return { at: 'none' };
}

/** a shot's generations: the approved first, then the newest */
export function shotGenerations(all: readonly Generation[], shot: string): Generation[] {
	return all.filter((g) => g.shot === shot).sort((a, b) => Number(b.approved) - Number(a.approved) || b.id - a.id);
}

/** the work storage's file as a blob (signed in: the API serves it only to members) */
export async function workFile(api: StoryboardApi, url: string): Promise<Blob> {
	return (await api.fetchRaw(apiPath(url))).blob();
}

/**
 * Share: the local file (the workspace folder) into the chapter's work storage, then the record names it there.
 * The same bytes again are fine (the server answers 200); other bytes under the name are refused (409 exists).
 */
export async function shareGeneration(api: StoryboardApi, workspace: Pick<Workspace, 'read'>, g: Generation): Promise<Generation> {
	if (!g.file) throw new Error('This generation has no file.');
	const blob = await workspace.read(g.toon, g.chapter, g.file);
	if (!blob) throw new Error(`${g.file} is not in your workspace folder.`);
	const name = workName(g);
	await api.put(`/storyboard/work?${q({ toon: g.toon, chapter: g.chapter, name })}`, blob);
	return updateGeneration(api, g.id, { work_path: name });
}

/** "Approve" picks this one for the shot – a second click takes it back */
export const approvePatch = (g: Pick<Generation, 'approved'>): GenerationPatch => ({ approved: !g.approved });

// ── Provider jobs (#121, server/docs/API.md "Provider jobs", "The budget gate") ───────────────────────────────

/** the client with POST (starting jobs, estimates, records) */
export interface GenerateApi extends StoryboardApi {
	post<T>(path: string, body?: unknown): Promise<T>;
}

/** the euro rate the server converts with (EuroRates::describe) */
export interface EuroRate {
	eur_per_usd: number;
	usd_per_eur: number;
	date: string;
	source: 'ecb' | 'fallback' | string;
}

/** the budget gate's state (GET /storyboard/budget) – euro */
export interface Budget {
	currency: 'EUR';
	budget: number;
	spent: number;
	reserved: number;
	left: number;
	rate: EuroRate;
}

/** a generator's model as GET /storyboard/generators lists it */
export interface GeneratorModel {
	id: string;
	label: string;
	kind: 'image' | 'video';
	mode: string;
	refs: { roles: string[]; max: number; required?: string[]; note?: string };
	ratios?: string[];
	durations?: { min: number; max: number };
	prompt_required?: boolean;
	price: { currency: string; [k: string]: number | string };
}

export interface GeneratorInfo {
	id: string;
	label: string;
	configured: boolean;
	/** the setting that holds the key (LUMA_API_KEY …) – never its value */
	setting: string;
	paid: boolean;
	/** why it can't run even with a key (null = it can) */
	unavailable: string | null;
	kinds: string[];
	models: GeneratorModel[];
}

/** POST …/estimate's answer: the provider's amount + currency, the values it runs with, the budget */
export interface Estimate {
	amount: number;
	currency: string;
	kind: 'image' | 'video';
	mode: string;
	model: string;
	ratio: string | null;
	duration: number | null;
	seed: number | null;
	note: string | null;
	generator: string;
	budget: Budget;
}

/** a reference as a job takes it: a work-storage file of the chapter or a public https link */
export type JobReference = { role: string; work: string } | { role: string; url: string };

/** a paid provider job (POST /storyboard/generations without file / machine / status) */
export interface JobBody {
	toon: string;
	chapter: number;
	shot: string;
	generator: string;
	model: string;
	prompt?: string;
	references?: JobReference[];
	params?: Record<string, unknown>;
	aspect?: string;
	take?: string;
	parents?: number[];
	note?: string;
}

export async function listGenerators(api: GenerateApi): Promise<{ generators: GeneratorInfo[]; budget: Budget | null }> {
	const r = await api.get<{ generators?: GeneratorInfo[]; budget?: Budget }>('/storyboard/generators');
	return { generators: r.generators ?? [], budget: r.budget ?? null };
}

export async function getBudget(api: GenerateApi): Promise<Budget> {
	return (await api.get<{ budget: Budget }>('/storyboard/budget')).budget;
}

/** what the job would cost – nothing started or booked */
export async function estimateJob(api: GenerateApi, job: JobBody): Promise<Estimate> {
	return (await api.post<{ estimate: Estimate }>('/storyboard/generations/estimate', job)).estimate;
}

/** starts the PAID job (exactly once – never retried by the client either): the queued row */
export async function startJob(api: GenerateApi, job: JobBody): Promise<Generation> {
	return (await api.post<{ item: Generation }>('/storyboard/generations', job)).item;
}

/** records a finished generation made elsewhere (by hand, a local tool): file + machine + status succeeded */
export async function recordGeneration(api: GenerateApi, record: Record<string, unknown>): Promise<Generation> {
	return (await api.post<{ item: Generation }>('/storyboard/generations', record)).item;
}

/** one generation – an open provider job is asked about first (at most every 5 s on the server) */
export async function getGeneration(api: GenerateApi, id: number): Promise<Generation> {
	return (await api.get<{ item: Generation }>(`/storyboard/generations/${id}`)).item;
}

export async function cancelGeneration(api: GenerateApi, id: number): Promise<Generation> {
	return (await api.post<{ item: Generation }>(`/storyboard/generations/${id}/cancel`)).item;
}

/** the provider's result, passed through the API (nothing is stored there; 410 once the provider's link ran out) */
export async function generationResult(api: StoryboardApi, id: number): Promise<Blob> {
	return (await api.fetchRaw(`/storyboard/generations/${id}/result`)).blob();
}
