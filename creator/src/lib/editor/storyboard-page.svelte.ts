// The Storyboard page's state (#123): the document `storyboard:<toon>:<chapter>` (a language-neutral ContentDoc – load,
// autosave, drafts, the stale-save conflict and Recover are the creator's, meta-page.svelte / doc-session), the shot
// list's picking, the steps, the script, the member's workspace folder and the shots' generations. The logic lives in
// the tested storyboard-*.ts modules; this class only holds their state for the page's components.
import { entities, entityKeyFor } from '@scutoons/content';
import { storyboardKey, validateChapter, type ChapterStoryboard, type Shot } from '@scutoons/content/storyboard';
import { ApiError, api, getDraft } from '$lib/api.ts';
import { neutralDoc } from '$lib/editor/languages.ts';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import { pickShot, setField, shotEntries, type ShotEntry } from '$lib/editor/storyboard-shots.ts';
import { sceneLines } from '$lib/editor/storyboard-script.ts';
import { browserStorage, loadLayout, loadStep, saveLayout, saveStep, STEPS, togglePanel, type PanelKind, type ShotTab, type StepLayout } from '$lib/editor/storyboard-steps.ts';
import { browserWorkspace, type WorkspaceState } from '$lib/editor/storyboard-workspace.ts';
import { listGenerations, shareGeneration, updateGeneration, type Generation, type GenerationPatch } from '$lib/editor/storyboard-generations.ts';
import { loadMachine, saveMachine } from '$lib/editor/storyboard-generate.ts';
import { GenerateState } from '$lib/editor/storyboard-generate.svelte.ts';
import { BoardState } from '$lib/editor/storyboard-board.svelte.ts';
import { HandheldState } from '$lib/editor/storyboard-handheld.svelte.ts';
import { TimelineState } from '$lib/editor/storyboard-timeline.svelte.ts';

const storage = browserStorage();

export class StoryboardPage {
	readonly doc: ContentDoc;
	/** the shot list's picking: the picked ids (toon order) and the current shot */
	picked = $state<string[]>([]);
	current = $state<string | null>(null);
	private anchor: string | null = null;

	step = $state(loadStep(storage));
	layouts = $state<Record<string, StepLayout>>(Object.fromEntries(STEPS.map((s) => [s.id, loadLayout(storage, s)])));

	/** the script's text in the toon's source language ('' = none); `scriptError` when it couldn't be read */
	script = $state<string | null>(null);
	scriptError = $state<string | null>(null);

	readonly workspace = browserWorkspace();
	workspaceState = $state<WorkspaceState>('none');
	/** the chosen folder's name */
	workspaceName = $state<string | null>(null);
	/** the file names in this chapter's workspace folder */
	localFiles = $state<Set<string>>(new Set());
	generations = $state<Generation[]>([]);
	/** the generations can't be read (the server's table isn't there yet: 503) */
	generationsError = $state<string | null>(null);
	/** this browser's label on the records of files saved here (storyboard-generate.ts; the Workspace control edits it) */
	machine = $state(loadMachine(storage));
	/** the Generate panel: generators, the form, the estimate, the running jobs (#121) */
	readonly gen: GenerateState = new GenerateState(this);
	/** the main view's editor (Viewport · Variants), the variants board, Compare, Improve on this (#125) */
	readonly board: BoardState = new BoardState(this);
	/** the handheld camera: the phone's pairing, the live pose, recording and rendering takes (#126) */
	readonly hh: HandheldState = new HandheldState(this);
	/** the timeline along the bottom: the picked shots' clips, the playhead, the trim (#126) */
	readonly tl: TimelineState = new TimelineState(this);

	constructor(
		readonly toon: string,
		readonly chapter: number,
		private readonly source: string
	) {
		this.doc = neutralDoc(storyboardKey(toon, chapter));
	}

	/** the document as the model's type (null while missing) */
	get data(): ChapterStoryboard | null {
		return (this.doc.data[this.doc.primary] as ChapterStoryboard | null) ?? null;
	}
	readonly entries = $derived<ShotEntry[]>(shotEntries(this.data));
	readonly order = $derived(this.entries.map((e) => e.id));
	readonly sel = $derived(this.entries.find((e) => e.id === this.current) ?? null);
	readonly problems = $derived(this.data ? validateChapter(this.data) : { errors: [], warnings: [] });

	get layout(): StepLayout {
		return this.layouts[this.step];
	}

	async load(): Promise<void> {
		await this.doc.load();
		const want = new URLSearchParams(location.search).get('shot');
		const first = this.order.includes(want ?? '') ? want : (this.order[0] ?? null);
		if (first) this.pick(first);
		void this.loadScript();
		void this.restoreWorkspace();
		void this.loadGenerations();
	}

	/** a missing document → an empty one (unsaved until Save); the import of chapters 3–5 is #120's migration */
	create(): void {
		this.doc.create(() => ({ storyboard: 2, toon: this.toon, chapter: this.chapter, scenes: [] }) satisfies ChapterStoryboard);
	}

	// ── Shots ──────────────────────────────────────────────────────────────────────────────────────────────

	pick(id: string, event: { shiftKey?: boolean; ctrlKey?: boolean; metaKey?: boolean } = {}): void {
		const next = pickShot(this.order, { picked: this.picked, current: this.current, anchor: this.anchor }, id, event);
		this.picked = next.picked;
		this.current = next.current;
		this.anchor = next.anchor;
	}

	/** a picked shot becomes the current one, the picked set stays (a click in a variants band, #125); not picked: a pick */
	focus(id: string): void {
		if (!this.picked.includes(id)) return this.pick(id);
		this.current = id;
		this.anchor = id;
	}

	/** ← / → in the toon's order */
	move(delta: number): void {
		const at = this.order.indexOf(this.current ?? '');
		const id = this.order[Math.min(Math.max(at + delta, 0), this.order.length - 1)];
		if (id) this.pick(id);
	}

	/** a field of the current shot (`keys` inside it); typing in one field within ~1 s is one undo step */
	setShot(keys: string[], value: unknown): void {
		const path = this.sel?.path;
		if (!path) return;
		this.doc.edit((file) => {
			const shot = path.reduce((o: any, k) => o?.[k], file) as Record<string, any> | undefined;
			if (shot) setField(shot, keys, value);
		}, `${this.current}:${keys.join('.')}`);
	}

	/** a change of the current shot's figures (add, remove) */
	editShot(change: (shot: Shot) => void): void {
		const path = this.sel?.path;
		if (!path) return;
		this.doc.edit((file) => {
			const shot = path.reduce((o: any, k) => o?.[k], file) as Shot | undefined;
			if (shot) change(shot);
		});
	}

	// ── Steps ──────────────────────────────────────────────────────────────────────────────────────────────

	setStep(id: string): void {
		if (id === this.step || !STEPS.some((s) => s.id === id && s.ready)) return;
		this.step = id;
		saveStep(storage, id);
	}
	isOpen = (kind: PanelKind) => this.layout.panels.includes(kind);
	setOpen(kind: PanelKind, open: boolean): void {
		const next = togglePanel(this.layout, kind, open);
		if (next === this.layout) return;
		this.layouts[this.step] = next;
		saveLayout(storage, this.step, next);
	}
	setTab(tab: ShotTab): void {
		this.layouts[this.step] = { ...this.layout, tab };
		saveLayout(storage, this.step, this.layout);
	}

	// ── Script ─────────────────────────────────────────────────────────────────────────────────────────────

	private async loadScript(): Promise<void> {
		try {
			const r = await getDraft<{ text?: string }>(entityKeyFor(entities.script(this.toon), this.source));
			this.script = r.content?.text ?? '';
		} catch (e) {
			if (e instanceof ApiError && e.status === 404) this.script = '';
			else this.scriptError = e instanceof Error ? e.message : String(e);
		}
	}
	/** the script lines of a scene (null: the scene isn't in the script) */
	sceneScript(scene: number | undefined): string[] | null {
		return this.script && scene !== undefined ? sceneLines(this.script, scene) : null;
	}

	// ── Workspace folder ───────────────────────────────────────────────────────────────────────────────────

	private async restoreWorkspace(): Promise<void> {
		this.workspaceState = await this.workspace.restore();
		await this.listLocal();
	}
	async chooseWorkspace(): Promise<void> {
		const state = await this.workspace.choose();
		if (state) this.workspaceState = state;
		await this.listLocal();
	}
	async allowWorkspace(): Promise<void> {
		this.workspaceState = await this.workspace.allow();
		await this.listLocal();
	}
	private async listLocal(): Promise<void> {
		this.workspaceName = this.workspace.name;
		this.localFiles = this.workspaceState === 'ready' ? new Set(await this.workspace.list(this.toon, this.chapter).catch(() => [])) : new Set();
	}
	readLocal(name: string): Promise<Blob | null> {
		return this.workspace.read(this.toon, this.chapter, name);
	}

	// ── Generations ────────────────────────────────────────────────────────────────────────────────────────

	async loadGenerations(): Promise<void> {
		try {
			this.generations = await listGenerations(api, this.toon, this.chapter);
			this.generationsError = null;
		} catch (e) {
			this.generationsError = e instanceof Error ? e.message : String(e);
		}
	}
	private replace(item: Generation): void {
		this.generations = this.generations.map((g) => (g.id === item.id ? item : g));
	}
	/** a new or changed generation into the list (a started job, a record, a polled status) */
	upsert(item: Generation): void {
		if (this.generations.some((g) => g.id === item.id)) this.replace(item);
		else this.generations = [...this.generations, item];
	}
	setMachine(label: string): void {
		this.machine = saveMachine(storage, label);
	}
	/** the names in the chapter's folder again (after a file was written) */
	async refreshLocal(): Promise<void> {
		await this.listLocal();
	}
	async review(g: Generation, patch: GenerationPatch): Promise<void> {
		this.replace(await updateGeneration(api, g.id, patch));
	}
	async share(g: Generation): Promise<void> {
		this.replace(await shareGeneration(api, this.workspace, g));
	}
}
