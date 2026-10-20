// The variants board's state (#125; the port of #62's board.svelte.js + compare.svelte.js – agents/research/
// VARIANTS-CANVAS.md §3.1–§3.5). One per Storyboard page (StoryboardPage.board), so it lives on while the main view
// switches between the Viewport and the board. It holds: the main view's editor per step (Viewport · Variants – V;
// Generate opens Variants, every other step the Viewport; remembered per step in this browser), the current card (the
// last clicked – its actions show under it, its shot is the current shot), the selection (Ctrl / ⌘ + click, at most 4,
// the letters A–D = Compare's sides; while Compare is open a side taken out leaves its place empty), a look into a
// strip, Show rejected, the folded bands, the zoom, the pinned moment of a video card, Compare open, and the cards'
// files (object URLs, loaded once – the member's folder, the work storage, a provider's result). The statuses (§3.5,
// one per card – the record's `approved` / `mark`, PUT …/{id}): chosen = approve (one per shot and kind: the chosen
// before goes to none) · review ↔ none (R) · reject ↔ unreject (X: the card and what hangs under it – X again brings
// back what that X folded, each to its mark before as far as this session knows it). The pure rules:
// storyboard-board-layout.ts, storyboard-compare.ts, storyboard-audition.ts.
import { m } from '$lib/paraglide/messages.js';
import { api } from '$lib/api.ts';
import { toast } from '$lib/toasts.ts';
import type { GenerationMark } from '@scutoons/content/storyboard';
import { browserStorage } from '$lib/editor/storyboard-steps.ts';
import { generationResult, placeOf, workFile, type Generation } from '$lib/editor/storyboard-generations.ts';
import { rejectIds, shortName, statusOf, unrejectIds, type BandLayout, type BoardNode, type CardAt } from '$lib/editor/storyboard-board-layout.ts';
import { addSide, clickSide, filled, resolveSides, sidesOnOpen, takeOut, type Side } from '$lib/editor/storyboard-compare.ts';
import { auditionPick, currentCard } from '$lib/editor/storyboard-audition.ts';
import { ImproveState } from '$lib/editor/storyboard-improve.svelte.ts';
import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

export type Editor = 'viewport' | 'variants';
const storage = browserStorage();
const EDITOR_KEY = 'scutoons_creator:storyboard:editor';
const recall = (step: string): Editor | null => {
	try {
		const v = storage?.getItem(`${EDITOR_KEY}:${step}`);
		return v === 'viewport' || v === 'variants' ? v : null;
	} catch {
		return null;
	}
};

export class BoardState {
	/** the editor per step (remembered; Generate opens Variants) */
	editors = $state<Record<string, Editor>>({});
	current = $state<CardAt | null>(null);
	/** the selection in the order picked – A, B, C, D; null = an empty side (only while Compare is open) */
	selected = $state<(Side | null)[]>([]);
	look = $state<{ shot: string; keys: string[] } | null>(null);
	showRejected = $state(false);
	folded = $state<Record<string, boolean>>({});
	zoom = $state(1);
	/** a video card's moment clicked while it scrubs (More ▾ "Still: the frame at …") */
	pin = $state<{ key: string; ms: number } | null>(null);
	compare = $state(false);
	busy = $state(false);
	panning = $state(false);
	/** whether a selected card is still on the board (the canvas says – its layouts know the cards) */
	alive: (s: Side) => boolean = () => true;
	/** the marks before an X, by id – what X again brings back (this session) */
	private readonly before = new Map<number, GenerationMark | null>();
	private readonly files = new Map<string, Promise<string | null>>();
	readonly improve: ImproveState;

	constructor(private readonly sb: StoryboardPage) {
		this.improve = new ImproveState(sb);
	}

	// ── The editor ──────────────────────────────────────────────────────────────────────────────────────────

	editorOf(step: string): Editor {
		return this.editors[step] ?? recall(step) ?? (step === 'generate' ? 'variants' : 'viewport');
	}
	get editor(): Editor {
		return this.editorOf(this.sb.step);
	}
	get open(): boolean {
		return this.editor === 'variants';
	}
	setEditor(e: Editor): void {
		this.editors = { ...this.editors, [this.sb.step]: e };
		try {
			storage?.setItem(`${EDITOR_KEY}:${this.sb.step}`, e);
		} catch {
			// remembered for this visit only
		}
	}
	toggleEditor(): void {
		this.setEditor(this.open ? 'viewport' : 'variants');
	}

	// ── The cards ───────────────────────────────────────────────────────────────────────────────────────────

	/** the shot's generations as the board reads them */
	recordsOf = (shot: string) => this.sb.generations.filter((g) => g.shot === shot);
	/** the audition (§3.1 Q3): while the board is open, the current shot's current card – the clip timeline plays it */
	readonly audition = $derived.by(() => {
		const shot = this.sb.current;
		if (!this.open || !shot) return null;
		const records = this.recordsOf(shot);
		return auditionPick({ shot, current: currentCard({ shot, current: this.current, records }), records });
	});

	/** the current card – a look closes when it leaves the look's branch (§3.5) */
	setCurrent(at: CardAt | null): void {
		this.current = at;
		if (this.look && !(at?.shot === this.look.shot && this.look.keys.includes(at.key))) this.look = null;
	}
	toggleFold(shot: string): void {
		const { [shot]: was, ...rest } = this.folded;
		this.folded = was ? rest : { ...rest, [shot]: true };
	}

	/** a card's file as an object URL (loaded once): the member's folder, the work storage, a provider's result */
	fileOf(g: Generation): Promise<string | null> {
		const place = placeOf(g, this.sb.localFiles);
		const key = `${g.id}|${place.at}|${place.at === 'work' ? place.url : ''}`;
		let p = this.files.get(key);
		if (!p) {
			const load = place.at === 'local' ? this.sb.readLocal(g.file!) : place.at === 'work' ? workFile(api, place.url) : place.at === 'result' ? generationResult(api, place.id) : Promise.resolve(null);
			p = load.then((b) => (b ? URL.createObjectURL(b) : null)).catch(() => null);
			this.files.set(key, p);
		}
		return p;
	}
	/** the page goes: the files' URLs freed */
	dispose(): void {
		for (const p of this.files.values()) void p.then((u) => u && URL.revokeObjectURL(u));
		this.files.clear();
	}

	// ── The selection = Compare's sides ─────────────────────────────────────────────────────────────────────

	/** the selection as it stands: holes and cards gone as null – Compare open, by place; closed, closed up */
	get selection(): (Side | null)[] {
		const sides = resolveSides(this.selected, this.alive);
		return this.compare ? sides : filled(sides);
	}
	/** Ctrl / ⌘ + click, More ▾: in or out – a rejected card stays out */
	toggleSelected(card: Side, node: BoardNode | null): void {
		const now = this.selection;
		const has = now.some((s) => s?.key === card.key);
		if (!has && statusOf(node?.record) === 'rejected') return void toast(m['storyboard.board_rejected_out']({ name: name(node) }));
		this.selected = has ? takeOut(now, [card.key], this.compare) : addSide(now, card);
	}
	/** C / "Compare · n": open with the selection (completed: the current shot's chosen as A) – or close */
	toggleCompare(nodeOf: (s: Side) => BoardNode | null, cur: CardAt | null): void {
		if (this.compare) return this.closeCompare();
		const shot = this.sb.current;
		const node = cur ? nodeOf(cur) : null;
		const kind = (n: BoardNode | null) => n?.record?.kind ?? (n ? 'image' : null);
		const current = cur && node ? { ...cur, kind: kind(node), rejected: statusOf(node.record) === 'rejected' } : null;
		const { sides } = sidesOnOpen({ selected: this.selection, shot, records: shot ? this.recordsOf(shot) : [], current, kindOf: (s) => kind(nodeOf(s)) });
		if (!sides.length) return void toast(m['storyboard.board_compare_select']());
		this.selected = sides;
		this.compare = true;
	}
	closeCompare(): void {
		const now = filled(this.selection);
		this.compare = false;
		this.selected = now;
	}
	/** a plain click on a card while Compare is open: it fills the first empty side, else it comes in as the last */
	compareClick(card: Side, node: BoardNode | null): void {
		const { selected, why } = clickSide(this.selection, card, statusOf(node?.record) === 'rejected');
		if (why === 'rejected') return void toast(m['storyboard.board_rejected_out']({ name: name(node) }));
		this.selected = selected;
	}

	// ── The statuses ────────────────────────────────────────────────────────────────────────────────────────

	private async guarded(fn: () => Promise<void>): Promise<void> {
		if (this.busy) return;
		this.busy = true;
		try {
			await fn();
		} catch (e) {
			toast(e instanceof Error ? e.message : String(e), { type: 'error' });
		} finally {
			this.busy = false;
		}
	}
	private put(ids: number[], patch: { mark?: GenerationMark | null; approved?: boolean }): Promise<unknown> {
		return Promise.all(ids.map((id) => this.sb.review({ id } as Generation, patch)));
	}

	/** ✓ / Enter: the card becomes its shot's chosen of its kind – the chosen before goes to none */
	choose(node: BoardNode | null): Promise<void> | void {
		const r = node?.record;
		if (!r) return void toast(m['storyboard.board_source_no_status']());
		if (r.approved) return;
		if (statusOf(r) === 'rejected') return void toast(m['storyboard.board_rejected_first']({ name: shortName(r) }));
		return this.guarded(async () => {
			const before = this.recordsOf(r.shot).filter((g) => g.approved && g.kind === r.kind && g.id !== r.id).map((g) => g.id);
			await this.put([r.id], { approved: true, mark: null });
			await this.put(before, { approved: false });
		});
	}
	/** ? / R: review ↔ none */
	review(node: BoardNode | null): Promise<void> | void {
		const r = node?.record;
		if (!r) return void toast(m['storyboard.board_source_no_status']());
		if (statusOf(r) === 'chosen') return void toast(m['storyboard.board_chosen_kept']({ name: shortName(r) }));
		if (statusOf(r) === 'rejected') return void toast(m['storyboard.board_rejected_first']({ name: shortName(r) }));
		return this.guarded(() => this.put([r.id], { mark: statusOf(r) === 'review' ? null : 'review' }).then(() => {}));
	}
	/** ✕ / X: reject the card and its branch – or, rejected, bring back what that X marked */
	reject(layout: BandLayout, node: BoardNode | null): Promise<void> | void {
		const r = node?.record;
		if (!node || !r) return void toast(m['storyboard.board_source_no_status']());
		if (statusOf(r) === 'chosen') return void toast(m['storyboard.board_chosen_kept']({ name: shortName(r) }));
		if (node.kind === 'source' && statusOf(r) !== 'rejected') return void toast(m['storyboard.board_drop_kept']({ name: shortName(r) }));
		return this.guarded(async () => {
			if (statusOf(r) === 'rejected') {
				const groups = unrejectIds(layout, node.key, (id) => this.before.get(id) ?? null);
				for (const g of groups) await this.put(g.ids, { mark: g.mark });
				groups.flatMap((g) => g.ids).forEach((id) => this.before.delete(id));
				const n = groups.reduce((s, g) => s + g.ids.length, 0);
				if (n > 1) toast(m['storyboard.board_back']({ name: shortName(r), n: n - 1 }));
			} else {
				const ids = rejectIds(layout, node.key);
				for (const id of ids) this.before.set(id, this.sb.generations.find((g) => g.id === id)?.mark ?? null);
				// one X = one moment: the marks go out together (the server stamps each)
				await this.put(ids, { mark: 'rejected' });
				const keys = ids.map(String);
				if (this.selected.some((s) => s && keys.includes(s.key))) this.selected = takeOut(this.selection, keys, this.compare);
				if (ids.length > 1) toast(m['storyboard.board_rejected_with']({ name: shortName(r), n: ids.length - 1 }));
			}
		});
	}
	/** the status a button / key sets: the pressed one again = none (✓ stays) */
	setStatus(layout: BandLayout, node: BoardNode | null, status: 'chosen' | 'review' | 'rejected'): Promise<void> | void {
		if (status === 'chosen') return this.choose(node);
		if (status === 'review') return this.review(node);
		return this.reject(layout, node);
	}
}

const name = (node: BoardNode | null | undefined) => (node?.record ? shortName(node.record) : (node?.key ?? ''));
