// The Generate panel's state (#121): the generators (+ Luma – by hand), the form of the current shot, the live estimate,
// sending (share local references → estimate the real body → the panel's confirmation → start, once), the running jobs
// (polled while the page is visible), a finished job's result into the workspace folder, a dropped by-hand result. The
// logic is storyboard-generate.ts (tested); this class only holds its state for generate-*.svelte. The UI words and the
// confirmation are the panel's.
import { api } from '$lib/api.ts';
import { asset } from '$lib/utils/cdn.ts';
import { registryDoc, registryOf } from '$lib/editor/characters.ts';
import { browserStorage } from '$lib/editor/storyboard-steps.ts';
import { cancelGeneration, estimateJob, getGeneration, listGenerators, startJob, type Budget, type Estimate, type Generation, type JobBody } from '$lib/editor/storyboard-generations.ts';
import {
	MANUAL,
	buildPrompt,
	characterRefs,
	defaultStill,
	generatorState,
	inputOptions,
	isOpen,
	jobBody,
	newlyDone,
	pickGenerator,
	pickModel,
	pickedRefs,
	previewRefs,
	recordManual,
	resolveRefs,
	saveResult,
	withManual,
	type GenerateForm,
	type GeneratorChoice
} from '$lib/editor/storyboard-generate.ts';
import { budgetNotice, missingSetting, type BudgetNotice } from '$lib/editor/storyboard-money.ts';
import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

const storage = browserStorage();
const KEY = 'scutoons_creator:storyboard:generator';
const recall = (k: string) => {
	try {
		return storage?.getItem(`${KEY}:${k}`) ?? null;
	} catch {
		return null;
	}
};
const remember = (k: string, v: string) => {
	try {
		storage?.setItem(`${KEY}:${k}`, v);
	} catch {
		// blocked storage: remembered for this visit only
	}
};

export class GenerateState {
	generators = $state<GeneratorChoice[]>([]);
	budget = $state<Budget | null>(null);
	loadError = $state<string | null>(null);
	form = $state<GenerateForm>({ generator: '', model: '', prompt: '', still: null, control: null, duration: 5, characters: [] });
	/** the shot the form belongs to (another shot resets it) */
	shot = $state<string | null>(null);
	estimate = $state<Estimate | null>(null);
	estimateError = $state<string | null>(null);
	/** the gate's last refusal (one notice in the panel) */
	notice = $state<BudgetNotice | null>(null);
	sending = $state(false);
	/** a finished job being saved into the folder / a dropped result being recorded */
	saving = $state<number | null>(null);
	adding = $state(false);
	private readonly registry = registryDoc();
	private registryLoaded = false;
	private timer: ReturnType<typeof setTimeout> | null = null;
	private asked = 0;

	constructor(private readonly sb: StoryboardPage) {}

	readonly current = $derived(this.generators.find((g) => g.id === this.form.generator) ?? null);
	readonly model = $derived(this.current?.models.find((m) => m.id === this.form.model) ?? null);
	readonly manual = $derived(this.current?.id === MANUAL);
	readonly ready = $derived(!!this.current && generatorState(this.current) === 'ready');
	readonly inputs = $derived.by(() => (this.sb.current ? inputOptions(this.sb.generations, this.sb.current) : { stills: [], controls: [] }));
	readonly characters = $derived.by(() => (this.sb.sel ? characterRefs(this.sb.sel.scene, this.sb.sel.shot, registryOf(this.registry), asset) : []));
	readonly picked = $derived.by(() => pickedRefs(this.form, this.model, this.sb.generations, this.characters));
	/** the shot's provider jobs still running */
	readonly running = $derived.by(() => this.sb.generations.filter((g) => g.shot === this.sb.current && isOpen(g)));

	async load(): Promise<void> {
		if (!this.registryLoaded) {
			this.registryLoaded = true;
			void this.registry.load().catch(() => {});
		}
		try {
			const r = await listGenerators(api);
			this.generators = withManual(r.generators);
			this.budget = r.budget;
			this.loadError = null;
		} catch (e) {
			// the server's generators can't be read (503 before the migration) – by hand still works
			this.generators = withManual([]);
			this.loadError = e instanceof Error ? e.message : String(e);
		}
		if (!this.form.generator) this.setGenerator(pickGenerator(this.generators, recall('last'))?.id ?? MANUAL, false);
	}

	setGenerator(id: string, keep = true): void {
		const g = this.generators.find((x) => x.id === id);
		if (!g) return;
		this.form.generator = id;
		if (keep) remember('last', id);
		this.form.model = pickModel(g, recall(`model:${id}`))?.id ?? '';
		this.estimateSoon();
	}
	setModel(id: string): void {
		this.form.model = id;
		remember(`model:${this.form.generator}`, id);
		this.estimateSoon();
	}

	/** the form follows the current shot: another shot clears the prompt and pre-fills its inputs */
	follow(): void {
		const shot = this.sb.current;
		if (shot === this.shot) return;
		this.shot = shot;
		const { stills, controls } = shot ? inputOptions(this.sb.generations, shot) : { stills: [], controls: [] };
		Object.assign(this.form, { prompt: '', still: defaultStill(stills), control: controls[0]?.id ?? null, characters: [] });
		this.estimate = null;
		this.estimateError = null;
		this.notice = null;
		this.estimateSoon();
	}

	/** "Build from the shot": the shot as edited right now (unsaved changes too) → the prompt (replaces it) */
	build(): void {
		const sel = this.sb.sel;
		if (!sel) return;
		const attach = this.characters.map((c) => c.label);
		this.form.prompt = buildPrompt(sel.scene, sel.shot, this.sb.sceneScript(sel.scene.source?.scene), { characters: attach });
		this.estimateSoon();
	}

	/** every tickable character image ticked (a new shot starts with none – the member picks) */
	toggleCharacter(key: string, on: boolean): void {
		this.form.characters = on ? [...new Set([...this.form.characters, key])] : this.form.characters.filter((k) => k !== key);
		this.estimateSoon();
	}

	private at() {
		const sel = this.sb.sel!;
		return { toon: this.sb.toon, chapter: this.sb.chapter, shot: sel.id, aspect: sel.shot.aspect };
	}

	/** the estimate follows the form (debounced); nothing for by hand or a generator that can't run */
	estimateSoon(): void {
		if (this.timer) clearTimeout(this.timer);
		this.timer = setTimeout(() => void this.estimateNow(), 300);
	}
	async estimateNow(body?: JobBody): Promise<Estimate | null> {
		const n = ++this.asked;
		if (!this.sb.sel || !this.ready || this.manual || !this.model) {
			this.estimate = null;
			this.estimateError = null;
			return null;
		}
		const job = body ?? jobBody(this.at(), this.form, this.model, previewRefs(this.picked));
		try {
			const e = await estimateJob(api, job);
			if (n === this.asked) {
				this.estimate = e;
				this.estimateError = null;
				this.budget = e.budget ?? this.budget;
			}
			return e;
		} catch (err) {
			if (n === this.asked) {
				this.estimate = null;
				const setting = missingSetting(err);
				this.estimateError = setting !== null ? null : err instanceof Error ? err.message : String(err);
			}
			return null;
		}
	}

	/**
	 * Send, step 1: the local references shared into the work storage, the real body estimated again – the panel shows
	 * this estimate in its confirmation and passes the same body to `start`
	 */
	async prepare(): Promise<{ body: JobBody; estimate: Estimate } | null> {
		if (!this.sb.sel || !this.model) return null;
		const { references } = await resolveRefs(this.picked, (g) => this.sb.share(g).then(() => this.sb.generations.find((x) => x.id === g.id) ?? g), this.sb.localFiles);
		const body = jobBody(this.at(), this.form, this.model, references);
		const estimate = await this.estimateNow(body);
		if (!estimate) throw new Error(this.estimateError ?? 'No estimate – nothing is sent.');
		return { body, estimate };
	}

	/** Send, step 2 (after the confirmation): the paid start, exactly once; a 402 becomes the panel's notice */
	async start(body: JobBody): Promise<Generation | null> {
		this.sending = true;
		this.notice = null;
		try {
			const item = await startJob(api, body);
			this.sb.upsert(item);
			return item;
		} catch (err) {
			const notice = budgetNotice(err);
			if (notice) {
				this.notice = notice;
				return null;
			}
			// a refused start comes back as the failed row (502 provider_failed { item })
			const item = (err as { body?: { item?: Generation } }).body?.item;
			if (item) this.sb.upsert(item);
			throw err;
		} finally {
			this.sending = false;
		}
	}

	async cancel(g: Generation): Promise<void> {
		this.sb.upsert(await cancelGeneration(api, g.id));
	}

	/**
	 * Asks about the open jobs once (the server asks the provider at most every 5 s); a finished one is saved into the
	 * folder when there is one → the finished ones (`saved`: their file, else null – the panel says what to do)
	 */
	async poll(): Promise<{ done: Generation[]; saved: Generation[] }> {
		const open = this.sb.generations.filter(isOpen);
		if (!open.length) return { done: [], saved: [] };
		const before = new Map(open.map((g) => [g.id, g.status]));
		const after = await Promise.all(open.map((g) => getGeneration(api, g.id).catch(() => g)));
		after.forEach((g) => this.sb.upsert(g));
		const done = newlyDone(before, after);
		const saved: Generation[] = [];
		for (const g of done) if (this.sb.workspaceState === 'ready' && !g.file) saved.push(await this.save(g));
		return { done, saved };
	}

	/** a finished job's result into the folder + the record (file, sha256, machine) */
	async save(g: Generation): Promise<Generation> {
		this.saving = g.id;
		try {
			const item = await saveResult({ api, workspace: this.sb.workspace, machine: this.sb.machine }, g);
			this.sb.upsert(item);
			await this.sb.refreshLocal();
			return item;
		} finally {
			this.saving = null;
		}
	}

	/** Luma – by hand: the dropped / picked file → the folder + a record with the prompt */
	async addManual(file: File): Promise<Generation> {
		const sel = this.sb.sel;
		if (!sel) throw new Error('No shot.');
		this.adding = true;
		try {
			const item = await recordManual({ api, workspace: this.sb.workspace, machine: this.sb.machine }, { toon: this.sb.toon, chapter: this.sb.chapter, shot: sel.id }, file, { prompt: this.form.prompt, ratio: sel.shot.aspect });
			this.sb.upsert(item);
			await this.sb.refreshLocal();
			return item;
		} finally {
			this.adding = false;
		}
	}
}


