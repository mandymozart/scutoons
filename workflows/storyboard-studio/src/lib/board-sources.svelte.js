// What comes onto the variants board besides the records (agents/research/VARIANTS-CANVAS.md §3.1–§3.3, issue #62 stage
// 2b): everything the Variants dock panel held, now on the board (the panel retired – §3.1):
//   - DROPS: image (png / jpeg / webp) or video (mp4 / webm) files dropped onto a band – or picked with the tool row's
//     + for the current shot – are one upload each for THAT band's shot (POST /api/generated: <shot>--g<n> + a record
//     { generator: 'upload' }) – a dropped file is a source card in column 0 (§3.2); what can't be added is a toast;
//   - LAYOUT VARIANTS (Q5: A, owner 2026-10-20: "A – layout variants as source cards on the board"): a kept layout's
//     group – Load (into the Viewport, unsaved – what the panel's Load did) · Take this (POST /api/variants/pick after an
//     in-page confirm: the variant becomes the shot, the shot as saved a variant – then its frame and its render without
//     texts are saved again, the render card shows the new layout);
//   - RUNNING JOBS (§3.3 "Running"): the picked shots' queued / running jobs (GET /api/generate/jobs?ch – the session's,
//     in memory), polled while one runs; a job that ended → the records again, its card replaces its placeholder (a
//     failed / cancelled one just goes – the Generate panel's Jobs list says why).
import { untrack } from 'svelte';
import { confirmDialog } from '@scutoons/ui';
import { api, ch, loadChapter, notify, pickedShots, studio, toon, touched } from './studio.svelte.js';
import { gen } from './generate.svelte.js';
import { setEditor } from './steps.svelte.js';
import { framePng } from './frames.js';
import { board, guarded, loadBoard } from './board.svelte.js';

// --- drops ----------------------------------------------------------------------------------------------------------
/** what POST /api/generated takes (generated.mjs TYPES) */
export const DROP_TYPES = ['image/png', 'image/jpeg', 'image/webp', 'video/mp4', 'video/webm'];
/** a file as a data URL (POST /api/generated's `data`) – the manual result slot's too (improve.svelte.js) */
export const dataUrl = (file) => new Promise((ok, no) => {
	const r = new FileReader();
	r.onload = () => ok(r.result);
	r.onerror = () => no(r.error ?? new Error('not readable'));
	r.readAsDataURL(file);
});
/** a drag that carries files (not a card's text or a link) */
export const carriesFiles = (e) => [...(e.dataTransfer?.types ?? [])].includes('Files');

/** files for `shot`: one upload each; a type the board can't take or a failed upload is a toast, the rest go on */
export async function upload(shot, files) {
	const c = ch();
	if (!c || !shot) return;
	let added = 0;
	for (const file of files) {
		if (!DROP_TYPES.includes(file.type)) {
			notify(`${file.name}: not added – png, jpeg, webp, mp4 or webm only.`, 'error');
			continue;
		}
		try {
			await api('POST', '/api/generated', { ch: c, id: shot, name: file.name, data: await dataUrl(file) });
			added++;
		} catch (e) {
			notify(`${file.name}: not added – ${e?.message ?? e}`, 'error');
		}
	}
	if (!added) return;
	notify(`${added === 1 ? 'One file' : `${added} files`} added to ${shot} – ${added === 1 ? 'a source card' : 'source cards'} in its band.`, 'success');
	// the board, the timeline's clips (a dropped video) and Generate's stills follow
	studio.version++;
}

// --- layout variants ------------------------------------------------------------------------------------------------
/** Load: the variant into the Viewport to change it – unsaved, the shot stays until Save + frame (Revert drops it) */
export async function loadVariant(shot, v) {
	if (studio.sel?.id !== shot || !v?.shot) return;
	if (studio.dirty && !(await confirmDialog({ title: 'Discard the changes?', message: `${shot} has unsaved changes – loading ${v.id} replaces them.`, confirmLabel: 'Discard' }))) return;
	studio.sel.shot = { ...JSON.parse(JSON.stringify(v.shot)), id: shot };
	touched();
	setEditor('viewport');
	notify(`${v.id} is in the Viewport – unsaved: Save + frame makes it the shot, Revert drops it (V: back to the board).`);
}

/** Take this: the variant becomes the shot, the shot as saved now a variant (with its saved frame) – asks first */
export async function takeVariant(shot, v) {
	if (studio.sel?.id !== shot || !v?.id || board.busy) return;
	const ok = await confirmDialog({
		title: `Take ${v.id}?`,
		message: `${v.id}${v.note ? ` (${v.note})` : ''} becomes ${shot}; the shot as it is saved now becomes a variant.${studio.dirty ? ` ${shot}'s unsaved changes are dropped.` : ''}`,
		confirmLabel: `Take ${v.id}`
	});
	if (!ok || studio.sel?.id !== shot) return;
	await guarded(async () => {
		await api('POST', '/api/variants/pick', { toon: toon(), ch: ch(), id: shot, vid: v.id });
		studio.dirty = false;
		await loadChapter(true);
		// the new shot's frame and its render without texts (the render card, Generate's "The render (no texts)")
		const png = await framePng();
		const clean = await framePng({ clean: true });
		await api('POST', '/api/frame', { toon: toon(), ch: ch(), id: shot, png, clean });
		await loadChapter(true);
		notify(`${v.id} is ${shot} now – the shot before is a variant.`, 'success');
	});
}

/** a source card's group (CardActions): a layout variant's Load / Take this */
export function sourceAction(shot, node, action) {
	if (node?.source !== 'variant') return;
	if (action === 'load') return loadVariant(shot, node.variant);
	if (action === 'take') return takeVariant(shot, node.variant);
}

// --- running jobs ---------------------------------------------------------------------------------------------------
const ACTIVE = new Set(['queued', 'running']);
const order = (id) => Number(String(id).replace(/\D/g, '')) || 0;
let timer = null;
let asked = 0;

/** the picked shots' running jobs → board.jobs; again in 1.5 s while one runs */
export async function pollJobs() {
	clearTimeout(timer);
	const c = ch();
	const ids = new Set(pickedShots());
	const n = ++asked;
	if (!c || !ids.size) {
		board.jobs = {};
		return;
	}
	const r = await api('GET', `/api/generate/jobs?ch=${c}`);
	if (n !== asked) return;
	const active = r.jobs.filter((j) => ACTIVE.has(j.status) && ids.has(j.shot)).sort((a, b) => order(a.id) - order(b.id));
	const ended = Object.values(board.jobs).flat().filter((j) => !active.some((a) => a.id === j.id));
	if (ended.length) {
		// its card arrives before its placeholder goes
		await loadBoard();
		if (n !== asked) return;
		// a new file of another shot than the Generate panel's (that one bumps it itself): the timeline and the inputs follow
		if (r.jobs.some((j) => j.status === 'succeeded' && j.shot !== gen.shot && ended.some((e) => e.id === j.id))) studio.version++;
	}
	const by = {};
	for (const j of active) by[j.shot] = [...(by[j.shot] ?? []), j];
	board.jobs = by;
	if (active.length) timer = setTimeout(() => pollJobs().catch(() => {}), 1500);
}

/** Board: the jobs follow the chapter, the picked set, studio.version and a job the Generate panel starts or ends */
export function useJobs() {
	$effect(() => {
		void [studio.chapter, studio.version, pickedShots().join(','), gen.jobs.filter((j) => ACTIVE.has(j.status)).map((j) => j.id).join(',')];
		untrack(() => pollJobs().catch(() => {}));
	});
	$effect(() => () => {
		clearTimeout(timer);
		asked++;
	});
}
