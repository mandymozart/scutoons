// The rig open on a Sound page (#283, #412): the draft of `rig:<subject>`, the catalog lists it picks from and what the core
// says about it – shared by the Assignment tab (the form) and the Rust tab (the rig's policy: its `module:<id>`, New policy).
// The rules are the core's (space-web, loaded once per page): the bundle the deck hears, the face that says who owns a row
// (`rules.face`), the policy template (`rules.policyTemplate`), the coverage. This only holds the draft and asks.
import { onMount } from 'svelte';
import { emptyRig, policyOf, setPolicy, type ModuleFiles, type RigCoverage, type RigDesigner, type RigDoc, type RigFace } from '@scutoons/rig-designer';
import { api, getDraft, library, publishVersion, putDraft } from '$lib/api.ts';
import type { EditTarget } from '$lib/editor/edit-key.ts';
import { NO_RIG_LISTS, bundleRequest, coverageRequest, createRigStore, moduleIdsOf, rigLists, templateRequest, type CatalogEntryLike, type RigLists, type RigResult } from '$lib/editor/rig-assign.ts';

/** the core's rig rules (shared/live-deck `assign`): JSON in and out, a throw carries the core's words */
export interface RigRules {
	coverage(request: string): string;
	bundle(request: string): string;
	face(bundle: string): string;
	policyTemplate(request: string): string;
}

/** the deck half's engine: plays a bundle (throws the core's words), and says what the browser leaves out of it */
export interface PlayHost {
	playRig(bundle: string): { engine: { note?: string } };
}

const said = (e: unknown) => (e instanceof Error ? e.message : String(e));

export class RigSession {
	rig = $state<RigDoc | null>(null);
	lists = $state<RigLists>(NO_RIG_LISTS);
	coverage = $state<RigCoverage[]>([]);
	/** what the core says of the draft ('' = it plays): a refusal, or what the browser's deck leaves out of it */
	problem = $state('');
	/** the draft as the core describes it (null while it does not read: `problem` says why) */
	face = $state<RigFace | null>(null);
	busy = $state(false);
	/** the deck half's engine, once it runs */
	host: PlayHost | null = null;
	private saved = $state('null');
	private characters: { id: string; personas?: { id: string }[] }[] = [];
	private readonly store = createRigStore({ getDraft, putDraft, publishVersion });

	constructor(
		readonly subject: string,
		private readonly rules: Promise<RigRules>
	) {}

	get dirty(): boolean {
		return JSON.stringify(this.rig) !== this.saved;
	}
	/** the module that drives this rig (a code rig), null for a declarative one */
	get policy(): string | null {
		return policyOf(this.rig);
	}
	/** every module in the catalog (a new policy's id is none of them) */
	get moduleIds(): string[] {
		return moduleIdsOf(this.lists);
	}
	get mine(): RigCoverage | null {
		return this.coverage.find((c) => c.subject === this.subject) ?? null;
	}
	get draftWaiting(): boolean {
		return !!this.lists.rigs.find((r) => r.id === this.subject)?.draft;
	}

	async load(): Promise<void> {
		this.characters = (await library.fetchRegistry({ draft: true }).catch(() => null))?.characters ?? [];
		const loaded = await this.store.load(this.subject);
		if (loaded.ok) [this.rig, this.saved] = [loaded.rig, JSON.stringify(loaded.rig)];
		else this.problem = loaded.message;
		await this.loadLists();
		await this.describe();
	}

	async loadLists(): Promise<void> {
		const kind = (k: string) => api.get<{ entries: CatalogEntryLike[] }>(`/catalog/docs/${k}`).then((r) => r.entries).catch(() => []);
		const [i, p, r, m] = await Promise.all([kind('instruments'), kind('presets'), kind('rigs'), kind('modules')]);
		this.lists = rigLists(i, p, r, m);
		try {
			this.coverage = JSON.parse((await this.rules).coverage(coverageRequest(this.characters, this.lists)));
		} catch (e) {
			this.problem = said(e);
		}
	}

	/** the core's bundle of the draft and its face; null with `problem` when it does not read */
	private async describe(): Promise<string | null> {
		if (!this.rig) return null;
		try {
			const rules = await this.rules;
			const bundle = rules.bundle(bundleRequest($state.snapshot(this.rig), this.lists));
			this.face = JSON.parse(rules.face(bundle));
			this.problem = '';
			return bundle;
		} catch (e) {
			this.face = null;
			this.problem = said(e);
			return null;
		}
	}

	/** the draft on the deck; what the core says is shown (a code rig: what the browser cannot play of it, #290) */
	async hear(): Promise<void> {
		const bundle = await this.describe();
		if (bundle === null) return;
		try {
			this.problem = this.host?.playRig(bundle).engine.note ?? '';
		} catch (e) {
			this.problem = said(e);
		}
	}

	/** after a compile of the policy: the catalog has its new descriptor */
	async reload(): Promise<void> {
		await this.loadLists();
		await this.hear();
	}

	change(next: RigDoc): void {
		this.rig = next;
		void this.hear();
	}

	create(): void {
		this.change(emptyRig(this.subject));
	}

	async save(): Promise<RigResult> {
		if (!this.rig || this.busy) return { ok: true };
		this.busy = true;
		const r = await this.store.save(this.subject, $state.snapshot(this.rig));
		this.busy = false;
		if (!r.ok) return r;
		this.saved = JSON.stringify(this.rig);
		await this.loadLists();
		return r;
	}

	async publish(): Promise<RigResult> {
		this.busy = true;
		const r = await this.store.publish(this.subject);
		this.busy = false;
		if (r.ok) await this.loadLists();
		return r;
	}

	/** the first files of a new policy `module:<id>` for this rig, written by the core from its tracks (throws its words) */
	async policyFiles(id: string): Promise<ModuleFiles> {
		if (!this.rig) throw new Error(this.subject);
		return JSON.parse((await this.rules).policyTemplate(templateRequest(id, $state.snapshot(this.rig), this.lists)));
	}

	/** the rig driven by `module:<id>`, saved as a draft (its module is one too; the publish asks the core again) */
	async attachPolicy(id: string): Promise<RigResult> {
		if (!this.rig) return { ok: false, message: this.subject };
		this.rig = setPolicy($state.snapshot(this.rig), id);
		const saved = await this.save();
		await this.hear(); // after the save's lists: the catalog has the new module
		return saved;
	}
}

/** a rig page's session, loading with the page; a compile of its policy reaches the deck half (the catalog has the new descriptor) */
export function openRigSession(target: EditTarget, rules: Promise<RigRules>, designer: RigDesigner): RigSession | null {
	if (!('subject' in target)) return null;
	const session = new RigSession(target.subject, rules);
	onMount(() => void session.load());
	designer.onCompiled = (id) => {
		if (id === session.policy) void session.reload();
	};
	return session;
}
