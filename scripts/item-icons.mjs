// Item icons with Luma (owner 2026-10-14: "can you use luma api to create the missing assets? this is an interesting
// test"): one image per item in the SCUTOONS look – model, style, the production references, the cut-out:
// scripts/lib/luma-style.mjs (shared with the cave's furniture). Raw images: logs/item-icons/<id>.raw.png; the
// cut-outs: media/scutoons/items/<id>.png, 512 × 512 transparent (the local dev copy – review there, then upload to
// production with the asset manager or the MCP's upload_asset). An icon already made is skipped (delete it to redo).
// Costs money: pricePerImage() – uni-1 ≈ $0.052 an icon (the default).
// Key: LUMA_AGENT(S)_API_KEY in .env.   node scripts/item-icons.mjs [--dry] [--only drill] [--model uni-1 | uni-1-max]
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { DEFAULT_MODEL, ROOT, STYLE_REFS, cutOut, lumaImage, objectPrompt, pricePerImage, styleRefs } from './lib/luma-style.mjs';

const arg = (name, fallback) => (process.argv.includes(`--${name}`) ? process.argv[process.argv.indexOf(`--${name}`) + 1] : fallback);
const dry = process.argv.includes('--dry');
const only = arg('only', null);
const model = arg('model', DEFAULT_MODEL);
const RAW = join(ROOT, 'logs', 'item-icons');
const OUT = join(ROOT, 'media', 'scutoons', 'items');
mkdirSync(RAW, { recursive: true });
mkdirSync(OUT, { recursive: true });

/** the items of the score / container feature (agents/research/SCORES.md) – what each icon shows; `{ what, scheme }` fixes
 *  the surface instead of a random one */
const ICONS = {
	// owner 2026-10-14: "like an A3 folded into A4, like a regular score usually comes" – it must read as paper first
	score: {
		what: 'a printed music score as it usually comes: ONE A3 sheet of thin matte paper folded once in the middle into an A4 folio (four pages), standing slightly open, seen three-quarter, a soft paper fold; printed five-line staves with notes on the visible inner page; the upper third of the FRONT page completely empty and plain (a title is lettered there later) – no letters, no numbers',
		scheme: 'plain off-white matte paper – real paper first; at most a subtle coloured tint along the edges of the paper'
	},
	songbook_small: 'a slim closed song book with a ring binding on the left and a plain cover with a small embossed treble clef, no text',
	songbook_large: 'a thick, important conductor\'s score binder, closed, with a broad spine, metal corner caps and a bookmark ribbon, no text',
	songbook_diy: 'a home-made song book: a small stack of sheet music pages with four drilled holes along the left edge, held together by four thin zip ties, a plastic sheet as its cover',
	songbook_diy_large: 'a thick home-made song book: a big stack of sheet music pages bound along the left edge by four chunky, wide zip ties, a plastic sheet as its cover',
	songbook_diy_broken: 'a home-made song book fallen apart: snapped zip ties, loose sheet music pages fanning out and sliding away',
	drill: 'a cordless power drill with a thin drill bit, side view',
	zip_tie: 'a small bundle of four thin zip ties (cable ties), loosely crossed, floating – no shadow under them',
	zip_tie_large: 'a bundle of four long, thick, heavy-duty zip ties (cable ties), loosely crossed',
	plastic_sheet: 'one single thin, flexible, see-through coloured plastic binder cover sheet (like a report cover), its corner curling up, the light shining through it – clearly a sheet of plastic, not a device or a screen',
	dj_trolley: 'a small DJ record trolley case on two wheels with a pull handle, lid open, vinyl record sleeves standing inside',
	cd_case: 'a 90s zip-up CD wallet, half open, shiny CDs in their sleeves inside',
	cassette_suitcase: 'a little toy suitcase, lid open, audio cassettes lined up inside in a row'
};

const todo = Object.keys(ICONS).filter((id) => (only ? id === only : !existsSync(join(OUT, `${id}.png`))));
console.log(`${todo.length} icons to make (${model} ≈ $${(todo.length * pricePerImage(model, STYLE_REFS.length)).toFixed(2)})`);
const refs = dry ? [] : await styleRefs();
for (const id of todo) {
	if (dry) {
		const icon = ICONS[id];
		console.log(`\n── ${id} ──\n${typeof icon === 'string' ? objectPrompt(icon) : objectPrompt(icon.what, 'ITEM ICON', icon.scheme)}`);
		continue;
	}
	const t0 = Date.now();
	try {
		const raw = join(RAW, `${id}.raw.png`);
		const icon = ICONS[id];
		writeFileSync(raw, await lumaImage(typeof icon === 'string' ? objectPrompt(icon) : objectPrompt(icon.what, 'ITEM ICON', icon.scheme), { model, refs }));
		cutOut(raw, join(OUT, `${id}.png`));
		console.log(`${id} ✓ ${Math.round((Date.now() - t0) / 1000)} s`);
	} catch (e) {
		console.log(`${id} ✗ ${e.message}`);
	}
}
