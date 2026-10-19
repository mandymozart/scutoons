// The shot's page actions (the ActionBar, DESIGN "The bottom bar is grouped"): Save + frame (the one primary), Keep as
// variant, Revert; the review (Approve / Ask for a change → approve.mjs); the Chapter menu's scripts and a taste rule
// (→ storyboard-taste.md). One job at a time: `busy.text` says what runs (the actions are disabled meanwhile), `busy.on`
// where it shows – 'shot' in the Save button, 'chapter' in the Chapter menu's place (DESIGN "Notifications": a running
// job is state where the doing happens, the result one toast).
import { askDialog } from '@scutoons/ui';
import { api, ch, fail, loadChapter, notify, save, saveVariant, studio, toon } from './studio.svelte.js';
import { framePng } from './frames.js';

export const busy = $state({ text: '', on: '' });

async function guarded(text, fn, on = 'shot') {
	if (busy.text) return;
	busy.text = text;
	busy.on = on;
	try { await fn(); } catch (e) { fail(e); } finally { busy.text = ''; busy.on = ''; }
}

export const saveShot = () => guarded('Saving …', async () => save(await framePng()));

export async function keepVariant() {
	const note = await askDialog({ title: 'Keep as a variant', label: 'What is different?', confirmLabel: 'Keep', skipLabel: 'Keep without note' });
	if (note === null) return;
	await guarded('Keeping …', async () => saveVariant(note, await framePng()));
}

export async function review(status) {
	const ok = status === 'ok';
	const note = await askDialog({ title: ok ? 'Approve the shot' : 'Ask for a change', label: ok ? 'Note (optional)' : 'What should change?', confirmLabel: ok ? 'Approve' : 'Ask', skipLabel: ok ? 'Approve without note' : undefined });
	if (note === null || (!ok && !note)) return;
	await guarded(ok ? 'Approving …' : 'Asking …', async () => {
		if (studio.dirty) await save(await framePng());
		const r = await api('POST', '/api/review', { toon: toon(), ch: ch(), id: studio.sel.id, status, note });
		notify(r.out, 'success');
		await loadChapter(true);
	});
}

/** a chapter script (Check, All frames – ~4 min, Reference packs, Chapter draft): "Running All frames …" in the Chapter
 * menu's place until it ends (a second run can't start), then its last output line as one toast */
export const runScript = ({ cmd, label }) =>
	guarded(`Running ${label} …`, async () => {
		const r = await api('POST', '/api/run', { toon: toon(), ch: ch(), cmd });
		const last = (r.out || '').split('\n').filter(Boolean).slice(-1)[0];
		notify(`${label}: ${last || (r.ok ? 'done' : 'failed')}`, r.ok ? 'success' : 'error');
		if (cmd === 'frames') await loadChapter(true);
	}, 'chapter');

export async function addRule() {
	const text = await askDialog({ title: 'A rule for the storyboards', message: 'Goes into workflows/scenes/storyboard-taste.md – the drafting agents read it.', label: 'The rule, in your words', confirmLabel: 'Add' });
	if (!text) return;
	await guarded('Adding …', async () => {
		await api('POST', '/api/taste', { rule: text, quote: '' });
		notify('Added to storyboard-taste.md.', 'success');
	}, 'chapter');
}
