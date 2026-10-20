// The board's "Improve on this" · Vary · More ▾ (#125; the port of #62's improve.svelte.js – agents/research/
// VARIANTS-CANVAS.md §3.3, §3.6, §3.7). A card FILLS the Generate form (storyboard-generate.svelte.ts) – nothing is
// ever sent from here: Generate asks with the cost as always. Each makes the card's shot current and opens the Generate
// panel:
//   - IMPROVE ON THIS (I / ⚡): the line "Improve on g2" (improve-line.svelte), the card where its kind fits the model (an
//     image the still, a video Aleph's video + the shot's chosen image as the keyframe, the render / a take the shot's own
//     inputs), its prompt, the seed New or the card's; a generator whose models take no such card is switched (said);
//   - VARY (↻): the card's own inputs (its references → the shot's generations), prompt, generator, model and length
//     with a new seed – a sibling in its row;
//   - USE AS START FRAME / KEYFRAME, STILL: FIRST | <time> | LAST (More ▾): the card – or a frame of the video, cut in the
//     browser and recorded as an image of the shot (storyboard-board-files.ts) – becomes the form's still; the rest stays.
// The job's parents are the form's generations, the main input first (storyboard-generate.ts pickedParents) – the
// result lands under the card. Rejected cards are refused (§3.5). The rules: storyboard-improve.ts.
import { m } from '$lib/paraglide/messages.js';
import { api } from '$lib/api.ts';
import { toast } from '$lib/toasts.ts';
import { defaultStill, inputOptions } from '$lib/editor/storyboard-generate.ts';
import { cardOf, durationFor, fieldsFor, frameSeconds, frameStillOf, pickImproveModel, refusal, useAsOf, varyFields, type ImproveFrom, type Refusal } from '$lib/editor/storyboard-improve.ts';
import { recordFrame, takenNames } from '$lib/editor/storyboard-board-files.ts';
import { cutFrame } from '$lib/editor/storyboard-posters.ts';
import { shortName, type BoardNode } from '$lib/editor/storyboard-board-layout.ts';
import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

/** a refusal in words */
export const refusalText = (why: Refusal, name: string): string =>
	why === 'rejected' ? m['storyboard.board_rejected_first']({ name })
		: why === 'source' ? m['storyboard.improve_no_source']()
			: why === 'dropped' ? m['storyboard.improve_no_dropped']({ name })
				: why === 'generator' ? m['storyboard.improve_no_generator']({ name })
					: why === 'model' ? m['storyboard.improve_no_model']({ name })
						: why === 'running' ? m['storyboard.improve_no_running']()
							: '';

export class ImproveState {
	from = $state<ImproveFrom | null>(null);
	cutting = $state(false);

	constructor(private readonly sb: StoryboardPage) {}

	private get gen() {
		return this.sb.gen;
	}
	/** the shot's chosen image (Aleph's keyframe at 0) */
	private chosenImage(shot: string): number | null {
		return this.sb.generations.find((g) => g.shot === shot && g.kind === 'image' && g.approved && g.status === 'succeeded')?.id ?? null;
	}

	/** the card's shot current, the Generate panel open and its form for that shot */
	private async ready(shot: string): Promise<void> {
		if (this.sb.current !== shot) this.sb.focus(shot);
		this.sb.setOpen('generate', true);
		if (!this.gen.generators.length) await this.gen.load();
		if (this.gen.shot !== shot) this.gen.follow();
	}

	/** I / ⚡: Improve on this */
	async improveOn(shot: string, node: BoardNode): Promise<void> {
		const why = refusal(node, 'improve');
		if (why) return void toast(refusalText(why, nameOf(node)));
		const card = cardOf(node);
		if (!card) return;
		await this.ready(shot);
		const f = this.gen.form;
		const from: ImproveFrom = { ...card, mode: 'improve', shot, useSeed: false, switched: null, prev: this.from?.prev ?? { generator: f.generator, model: f.model } };
		const pick = pickImproveModel(this.gen.generators, { generator: f.generator, model: f.model }, from);
		if (!pick) return void toast(m['storyboard.improve_no_model_takes']({ name: nameOf(node) }));
		if (pick.switched) from.switched = { from: f.generator, to: pick.generator };
		this.from = from;
		Object.assign(f, { generator: pick.generator, model: pick.model });
		this.fill();
	}

	/** ↻: Vary – the card's own inputs, prompt and model, a new seed */
	async vary(shot: string, node: BoardNode): Promise<void> {
		const why = refusal(node, 'vary', this.gen.generators);
		if (why) return void toast(refusalText(why, nameOf(node)));
		const card = cardOf(node);
		const r = node.record as (typeof this.sb.generations)[number] | null;
		if (!card || !r) return;
		await this.ready(shot);
		const f = this.gen.form;
		const fields = varyFields(r.inputs ?? null, this.sb.generations.filter((g) => g.shot === shot), this.gen.characters);
		this.from = { ...card, mode: 'vary', shot, fields, useSeed: false, switched: null, prev: this.from?.prev ?? { generator: f.generator, model: f.model } };
		Object.assign(f, { generator: r.generator, model: r.model ?? f.model });
		this.fill(r.duration);
	}

	/** the form from `from`: the shot's own inputs, then the card's fields, its prompt, the seed */
	private fill(duration?: number | null): void {
		const from = this.from;
		const shot = this.sb.current;
		if (!from || !shot) return;
		const f = this.gen.form;
		const { stills, controls } = inputOptions(this.sb.generations, shot);
		Object.assign(f, { still: defaultStill(stills), control: controls[0]?.id ?? null, characters: [] }, fieldsFor(from, this.gen.model, this.chosenImage(shot)));
		f.prompt = from.prompt;
		f.seed = from.useSeed ? from.seed : null;
		const d = durationFor(this.gen.model, duration);
		if (d !== null) f.duration = d;
		this.gen.estimateSoon();
	}
	/** another model picked while the line is on: the card's fields for it (the prompt stays as edited) */
	refit(): void {
		const from = this.from;
		const shot = this.sb.current;
		if (!from || !shot || from.shot !== shot) return;
		Object.assign(this.gen.form, fieldsFor(from, this.gen.model, this.chosenImage(shot)));
		this.gen.estimateSoon();
	}
	/** the seed: the card's own (true) or New */
	setSeed(own: boolean): void {
		if (!this.from) return;
		this.from.useSeed = own;
		this.gen.form.seed = own ? this.from.seed : null;
		this.gen.estimateSoon();
	}
	/** ✕: back to the shot's own inputs (and the generator before, when Improve switched it) */
	clear(): void {
		const prev = this.from?.prev;
		this.from = null;
		if (prev && this.gen.generators.some((g) => g.id === prev.generator)) Object.assign(this.gen.form, prev);
		this.gen.shot = null;
		this.gen.follow();
	}

	/** More ▾ "Use as start frame / keyframe": the image card becomes the still – the rest of the form stays */
	async useAs(shot: string, node: BoardNode): Promise<void> {
		const why = refusal(node, 'use');
		if (why) return void toast(refusalText(why, nameOf(node)));
		await this.ready(shot);
		if (!useAsOf(this.gen.model)) return void toast(m['storyboard.improve_no_start']({ model: this.gen.model?.label ?? '' }));
		this.setStill(node.record!.id);
		toast(m['storyboard.improve_used']({ name: nameOf(node) }));
	}

	/** More ▾ "Still: first | <time> | last": that frame of the video, cut here, recorded, then the form's still */
	async useFrame(shot: string, node: BoardNode, frame: 'first' | 'last' | number): Promise<void> {
		const why = refusal(node, 'use');
		const g = this.sb.generations.find((x) => x.id === node.record?.id);
		if (why || !g) return void toast(refusalText(why || 'source', nameOf(node)));
		await this.ready(shot);
		if (!frameStillOf(this.gen.model)) return void toast(m['storyboard.improve_no_still']({ model: this.gen.model?.label ?? '' }));
		const url = await this.sb.board.fileOf(g);
		if (!url) return void toast(m['storyboard.improve_no_file']({ name: nameOf(node) }), { type: 'error' });
		this.cutting = true;
		try {
			const png = await cutFrame(url, (d) => frameSeconds(frame, d));
			const at = { toon: this.sb.toon, chapter: this.sb.chapter, shot };
			const ready = this.sb.workspaceState === 'ready';
			const deps = { api, machine: this.sb.machine, workspace: ready ? this.sb.workspace : null, taken: () => takenNames(api, at, this.sb.generations) };
			const note = typeof frame === 'number' ? `${shortName(g)} @ ${(frame / 1000).toFixed(1)} s` : `${shortName(g)} – ${frame}`;
			const item = await recordFrame(deps, at, png, g, note);
			this.sb.upsert(item);
			if (ready) await this.sb.refreshLocal();
			this.setStill(item.id);
			toast(m['storyboard.improve_frame_done']({ name: shortName(item) }), { type: 'success' });
		} catch (e) {
			toast(e instanceof Error ? e.message : String(e), { type: 'error' });
		} finally {
			this.cutting = false;
		}
	}

	/** a still set over the card's (the line names it) */
	private setStill(id: number): void {
		this.gen.form.still = id;
		if (this.from && this.from.shot === this.sb.current) this.from.still = id;
		this.gen.estimateSoon();
	}
}

const nameOf = (node: BoardNode) => (node.record ? shortName(node.record) : node.take ?? '');
