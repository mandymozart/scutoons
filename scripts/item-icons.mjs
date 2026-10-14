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
	// owner 2026-10-14: "like an A3 folded into A4, like a regular score usually comes"; "single fold"; then old
	// publisher editions as the example (pastel card covers, a printed band at the top, a small emblem at the bottom)
	score: {
		what: 'a classic sheet-music edition as publishers sold them: ONE thin folded score (one sheet folded once, no stack, no booklet spine), lying slightly angled, its front cover in softly coloured card with a printed colour band across the top edge and a tiny music emblem near the bottom; the middle of the front cover completely empty and plain (a title is lettered there later) – no letters, no numbers; the corner of the cover lifted a little so a page of printed five-line staves peeks out',
		scheme: () => ['pale mint-green card with a deep teal printed band', 'warm apricot card with a brick-red printed band', 'ivory card with a dark green printed band'][Math.floor(Math.random() * 3)]
	},
	songbook_small: 'a slim closed song book with a ring binding on the left and a plain cover with a small embossed treble clef, no text',
	songbook_large: 'a thick, important conductor\'s score binder, closed, with a broad spine, metal corner caps and a bookmark ribbon, no text',
	songbook_diy: 'a home-made song book: a small stack of sheet music pages with four drilled holes along the left edge, held together by four thin zip ties, a plastic sheet as its cover',
	songbook_diy_large: 'a thick home-made song book: a big stack of sheet music pages bound along the left edge by four chunky, wide zip ties, a plastic sheet as its cover',
	songbook_diy_broken: 'a home-made song book fallen apart: snapped zip ties, loose sheet music pages fanning out and sliding away',
	drill: 'a cordless power drill with a thin drill bit, side view',
	// owner 2026-10-14: "the zip looks black" – a dark scheme (chocolate) drowns on the dark inventory: bright plastic
	// owner 2026-10-14: "bundle 4 with a rubber band"
	zip_tie: {
		what: 'a SHORT, small bundle of only FOUR thin zip ties (cable ties – four heads, no more) lying parallel, the whole bundle fully inside the frame, held together around the middle by one red rubber band, their square heads at one end, the tails fanning out a little at the other – no shadow under them',
		scheme: 'bright glossy translucent plastic in lemon yellow, lime and bubblegum pink – light and luminous, never dark'
	},
	zip_tie_large: {
		what: 'a bundle of only FOUR thick, heavy-duty zip ties (cable ties – four heads, no more) lying parallel, the whole bundle with both ends fully inside the frame, held together around the middle by one red rubber band, their chunky square heads at one end, the tails fanning out a little at the other',
		scheme: 'bright glossy plastic in tangerine orange and sky blue – light and luminous, never dark'
	},
	plastic_sheet: 'one single thin, flexible, see-through coloured plastic binder cover sheet (like a report cover), its corner curling up, the light shining through it – clearly a sheet of plastic, not a device or a screen',
	dj_trolley: 'a small DJ record trolley case on two wheels with a pull handle, lid open, vinyl record sleeves standing inside',
	cd_case: 'a 90s zip-up CD wallet, half open, shiny CDs in their sleeves inside',
	cassette_suitcase: 'a little toy suitcase, lid open, audio cassettes lined up inside in a row'
};

/** an icon's prompt – its own surface when it fixes one (a text, or a function drawing among a few) */
const promptOf = (icon) =>
	typeof icon === 'string' ? objectPrompt(icon) : objectPrompt(icon.what, 'ITEM ICON', typeof icon.scheme === 'function' ? icon.scheme() : icon.scheme);

const todo = Object.keys(ICONS).filter((id) => (only ? id === only : !existsSync(join(OUT, `${id}.png`))));
console.log(`${todo.length} icons to make (${model} ≈ $${(todo.length * pricePerImage(model, STYLE_REFS.length)).toFixed(2)})`);
const refs = dry ? [] : await styleRefs();
for (const id of todo) {
	if (dry) {
		const icon = ICONS[id];
		console.log(`\n── ${id} ──\n${promptOf(icon)}`);
		continue;
	}
	const t0 = Date.now();
	try {
		const raw = join(RAW, `${id}.raw.png`);
		const icon = ICONS[id];
		writeFileSync(raw, await lumaImage(promptOf(icon), { model, refs }));
		cutOut(raw, join(OUT, `${id}.png`));
		console.log(`${id} ✓ ${Math.round((Date.now() - t0) / 1000)} s`);
	} catch (e) {
		console.log(`${id} ✗ ${e.message}`);
	}
}
