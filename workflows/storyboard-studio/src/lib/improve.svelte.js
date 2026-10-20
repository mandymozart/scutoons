// The board's "Improve on this" · Vary · More ▾ and the manual result slot (agents/research/VARIANTS-CANVAS.md §3.3,
// §3.6, §3.7; issue #62 stage 3). A card FILLS the Generate panel – nothing is ever sent from here: Generate asks with
// the cost as always (generate.svelte.js start). Each makes the card's shot current and opens the Generate panel (App,
// gen.reveal):
//   - IMPROVE ON THIS (I / ⚡): the line "Improve on g2", the card where its kind fits the model (improve.js – an image
//     the still or ComfyUI's image 1, a video Aleph's video + the chosen image at 0, the render the shot's own inputs, a
//     layout variant its frame, a take its render as the video), its prompt, the seed New or the card's; the pickers
//     keep only the models that take it (the generator switches when its own has none – the line says so);
//   - VARY (↻): the card's own inputs (the chosen image pinned to the file it was then – improve.js pinChosen), prompt,
//     generator, model and length with a new seed – a sibling in its row;
//   - USE AS START FRAME / KEYFRAME, STILL: FIRST | <time> | LAST (More ▾): the card – or a frame of the video – becomes
//     the form's still, the rest of the form stays (it adds, it doesn't replace; during an Improve it stays over the
//     card's still and the line says so – generate.svelte.js setStill);
//   - the MANUAL RESULT SLOT (ManualGenerate – Luma by hand): a file → that generator's candidate with the form's
//     prompt, still and parent (the improved card; a Vary's sibling: the card's parent – none once the still was set by
//     hand instead) – POST /api/generated with the job, never 'upload' (a drop on the board stays a source –
//     board-sources.svelte.js). Only for the shot whose inputs the form holds (`loaded`).
// Rejected cards are refused (§3.5: "no Approve / Improve / Vary"), a missing file too.
import { api, ch, notify, pickShot, studio, toon } from './studio.svelte.js';
import { current, gen, loadInputs, loaded, manualPrompt, model, setStill, wants } from './generate.svelte.js';
import { cardOf, frameStillOf, pinChosen, useAsOf } from './improve.js';
import { fieldsOf, frameKey } from '../../generators/job-fields.mjs';
import { shortId, statusOf } from './board-layout.js';
import { board } from './board.svelte.js';
import { dataUrl } from './board-sources.svelte.js';

/** why a card can't be improved / varied / used ('improve' | 'vary' | 'use'), in words – '' = it can */
export function refusal(node, what = 'improve') {
	const r = node?.record;
	const name = r ? shortId(r.id) : '';
	if (statusOf(r) === 'rejected') return `${name} is rejected – X brings it back first`;
	if (what !== 'vary' && r?.missing) return `${name}: its file is gone`;
	if (what !== 'vary') return '';
	if (!r) return 'A source has no record to vary – Improve on it instead';
	if (r.earlier) return `${name} has no record (made before records were kept) – nothing to vary`;
	if (!r.generator || r.generator === 'upload') return `${name} was dropped – no generator to vary it with (Improve on it instead)`;
	const g = gen.generators.find((x) => x.id === r.generator);
	if (gen.generators.length && !g) return `${name} came from ${r.generator} – not one of the studio's generators`;
	if (g && !g.models.some((m) => m.id === r.model)) return `${name}'s model ${r.model} is not offered by ${g.label} any more`;
	return '';
}

/** the card's shot current, then the form filled for it (loadInputs puts the card in – applyFrom) */
async function fill(shot, from) {
	if (!(await pickShot(shot))) return;
	// the generator before the FIRST fill: ✕ goes back to it
	gen.from = { ...from, ch: ch(), prev: gen.from?.prev ?? null, applied: false, switched: null, still: null };
	await loadInputs(shot);
}

/** I / ⚡: Improve on this */
export async function improve(shot, node, take) {
	const why = refusal(node, 'improve');
	if (why) return notify(`${why}.`);
	const card = cardOf(node, { ch: ch(), shot, take });
	if (!card) return notify('Nothing to improve on here – the card has no file.');
	await fill(shot, { mode: 'improve', shot, ...card, useSeed: false });
}

/** ↻: Vary – the card's own inputs, prompt and model, a new seed */
export async function vary(shot, node) {
	const why = refusal(node, 'vary');
	if (why) return notify(`${why}.`);
	const r = node.record;
	const name = shortId(r.id);
	const card = cardOf(node, { ch: ch(), shot });
	// its inputs as the form's fields (a frame as its frame key) – the chosen image as the file it was when the card was made
	const pinned = pinChosen(r.inputs, r.parents, board.records[shot], ch());
	const fields = fieldsOf(pinned.inputs);
	if (fields.missing.length) notify(`${name}'s record names inputs without a file (${fields.missing.join(', ')}) – they are left out.`, 'warning');
	if (pinned.gone.length) notify(`${name} was made from the chosen image of then – ${pinned.gone.join(', ')} – whose file is gone: left out.`, 'warning');
	if (pinned.unpinned) notify(`${name}'s record doesn't say which file the chosen image was when it was made – Vary uses the chosen image as it is now.`, 'warning');
	await fill(shot, { mode: 'vary', shot, ...card, fields, unpinned: pinned.unpinned, generator: r.generator, model: r.model, duration: r.duration ?? null, useSeed: false });
}

/** the form for the card's shot (the panel may not have loaded it – closed, another step) */
async function ready(shot) {
	if (!(await pickShot(shot))) return false;
	if (!loaded(shot)) await loadInputs(shot);
	return loaded(shot);
}

/** More ▾ "Use as start frame / keyframe": the image card becomes the still – the rest of the form stays */
export async function useAs(shot, node) {
	const why = refusal(node, 'use') || (!useAsOf(model()).ok && useAsOf(model(), current()?.label).why);
	if (why) return notify(`${why}.`);
	const card = cardOf(node, { ch: ch(), shot });
	if (!card || !(await ready(shot))) return;
	setStill(card.key, { reveal: true });
	notify(`${card.name} is the ${model()?.startFrame ? 'start frame' : 'keyframe at 0'} in Generate – nothing is sent before Generate.`);
}

/** More ▾ "Still: first | <time> | last frame" on a video: that frame becomes the form's still (cut by the server) */
export async function useFrame(shot, node, take, frame) {
	const ok = frameStillOf(model(), current()?.label);
	const why = refusal(node, 'use') || (!ok.ok && ok.why);
	if (why) return notify(`${why}.`);
	const card = cardOf(node, { ch: ch(), shot, take });
	if (!card?.video || !(await ready(shot))) return;
	setStill(frameKey(card.key, frame), { reveal: true });
}

/** the manual result's parent: the improved card (a Vary: the card's parent) – none once a still was set by hand over
 *  the card's (the server takes the parents of the inputs then) */
export function resultParent() {
	const f = gen.from?.applied && gen.from.shot === gen.shot ? gen.from : null;
	if (!f || f.still != null) return null;
	return f.mode === 'improve' ? f.id : f.parent;
}

/** the manual result slot: a file made by hand → the by-hand generator's candidate (the form's prompt, still, parent) */
export async function addResult(files) {
	const file = files?.[0];
	const g = current();
	if (!file || !gen.shot || g?.transport !== 'manual') return;
	if (!loaded() || gen.shot !== studio.sel?.id) return notify(`${studio.sel?.id ?? 'The shot'}'s inputs are not loaded yet – nothing is added.`, 'error');
	const parent = resultParent();
	const r = await api('POST', '/api/generated', {
		ch: ch(), id: gen.shot, name: file.name, data: await dataUrl(file),
		job: { generator: g.id, model: gen.form.model, toon: toon(), prompt: manualPrompt(), still: wants('still') ? gen.form.still || null : null },
		...(parent ? { parent } : {})
	});
	notify(`${shortId(r.id)} – ${g.label}'s result – is on the board${parent ? ` under ${shortId(parent)}` : ''}.`, 'success');
	// the board, the timeline's clips and Generate's lists follow
	studio.version++;
}
