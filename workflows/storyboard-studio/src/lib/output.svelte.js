// The Output panel's state and its two jobs (the Export step – README "Handheld camera"): the chapter's handheld shots
// rendered as control videos, or their cameras as one zip. In a module, not the panel: the panel is docked only while
// its step shows it, and a render runs for minutes – a step change unmounts the panel, and nothing may be lost by it
// (research/STUDIO-LAYOUT.md §4 B: "the person must trust that nothing is lost when switching"): the progress, the
// result links, the options, and Render stays off while one runs (generate.svelte.js keeps its jobs the same way).
import { api, ch, fail, find, notify, studio, toon } from './studio.svelte.js';
import { endPreview } from './handheld/handheld.svelte.js';
import { downloadAll } from './handheld/export.js';
import { renderAll } from './handheld/render.js';

export const out = $state({
	/** what runs now – its progress line ('' = nothing runs) */
	busy: '',
	/** the last render: [{ shot, files? | error? }], of `chapter` */
	results: [],
	chapter: '',
	/** the render's options: passes, 24 / 25 / 30 fps, size 1× / 2×, + WebM */
	opts: { passes: ['beauty', 'depth'], fps: '24', ss: '1', webm: false },
	/** the zip's GLBs with the blocked set + figures */
	withScene: false
});

/** one job at a time: a second click while one runs does nothing */
async function each(label, fn) {
	if (out.busy) return;
	out.busy = label;
	try { await fn(); } catch (e) { fail(e); } finally { out.busy = ''; }
}

/** the chapter's handheld shots with their active take ({ shotId: take }), or null with a warning */
async function all() {
	const takes = await api('GET', `/api/takes/all?toon=${toon()}&ch=${ch()}`);
	if (!Object.keys(takes).length) { notify('No shot has the handheld camera on with an active take yet.', 'warning'); return null; }
	return takes;
}

export const zipAll = () => each('Packing …', async () => {
	const takes = await all();
	if (takes) notify(`${await downloadAll(studio.chapter, takes, find, out.withScene, (m) => (out.busy = m))} shot(s) in the zip.`, 'success');
});

export const renderShots = () => each('Rendering …', async () => {
	const takes = await all();
	if (!takes) return;
	endPreview();
	const chapter = studio.chapter;
	const { passes, fps, ss, webm } = out.opts;
	out.results = await renderAll(chapter, takes, find, { passes: [...passes], fps: Number(fps), ss: Number(ss), webm }, (m) => (out.busy = m));
	out.chapter = chapter;
	notify(`Rendered ${out.results.filter((r) => r.files).length} of ${out.results.length} handheld shot(s).`, 'success');
});

export const togglePass = (p) => (out.opts.passes = out.opts.passes.includes(p) ? out.opts.passes.filter((x) => x !== p) : [...out.opts.passes, p]);
