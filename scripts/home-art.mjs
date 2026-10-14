// Home art with Luma (agents/research/CAVE.md, workflows/home/furniture.md; owner 2026-10-14: "the icons for the cave
// session can also be generated through the API based on the same rules I used for the other icons"). The SCUTOONS
// Luma look is shared – scripts/lib/luma-style.mjs (model, style, production references, random colour schemes, the
// cut-out); this script only says WHAT and how a piece of furniture stands.
//   Furniture: ONE render each (three-quarter view, its back to the left wall) → the home picture (trimmed, the foot at
//   the bottom, longest side 512: <id>.home.png) and the inventory icon (the same render, a 512 square: <id>.png).
//   The VR headset: an icon only. Home scenes: --scenes with the owner's example images (--scene-ref <file> …), full
//   frame, no cut-out.
// Out: logs/home-art/ – local review; production upload only with the owner's yes. A done file is skipped (delete it to
// redo). Key: LUMA_AGENT(S)_API_KEY in .env (here, or the file named by SCUTOONS_ENV).
//   node scripts/home-art.mjs [--dry] [--only workbench] [--model uni-1] [--scenes --scene-ref <file> …]
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, extname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
// the key may live in another checkout's .env – loaded before the shared module reads it
try {
	if (process.env.SCUTOONS_ENV) process.loadEnvFile(process.env.SCUTOONS_ENV);
} catch {}
const { DEFAULT_MODEL, STYLE, cutOut, lumaImage, objectPrompt, pricePerImage, randomScheme, styleRefs } = await import('./lib/luma-style.mjs');

const argv = process.argv;
const arg = (name, fallback) => (argv.includes(`--${name}`) ? argv[argv.indexOf(`--${name}`) + 1] : fallback);
const dry = argv.includes('--dry');
const only = arg('only', null);
const model = arg('model', DEFAULT_MODEL);
const scenes = argv.includes('--scenes');
const sceneRefs = argv.flatMap((a, i) => (a === '--scene-ref' ? [argv[i + 1]] : []));
const OUT = join(ROOT, 'logs', 'home-art');
mkdirSync(OUT, { recursive: true });

/** furniture (workflows/home/furniture.md – the size is the item's, not the picture's) */
const FURNITURE = {
	workbench: 'a sturdy workbench with a vice on one end and a few tool marks, an empty top',
	cd_rack: 'a slim 90s CD tower rack with narrow slots for CD cases, empty',
	cassette_rack: 'a small 90s cassette tower with slots for cassettes, empty',
	record_shelf: 'a low cube shelf with two open compartments for vinyl records standing upright, empty',
	sofa: 'a small two-seater sofa'
};
/** items that are only an icon */
const ICONS = {
	vr_headset: 'a VR headset (virtual reality goggles) with a soft head strap'
};
/** the home scenes (research/CAVE.md D3): an empty room as a 2.5D diorama – the owner's examples as references */
const SCENES = {
	cave: 'a cosy rock cave as a home: rough stone walls and a vaulted rock ceiling, a few small stalactites, a worn sandy floor',
	tatami_room: "a Japanese monk's room: tatami mats on the floor, shoji paper walls with a wooden lattice, completely empty"
};

/** how a piece of furniture stands: turned so it can stand against the left wall of a room, on its own feet */
const FURNITURE_FRAMING =
	'Three-quarter view from above (about 30 degrees), the piece turned so its back faces the LEFT and its front faces the lower right (it will stand against the left wall of a room). The piece alone, centred, filling about 75% of the square, on a plain pure white background (#FFFFFF), no floor, no rug, no cast shadow on the background.';
const LF = String.fromCharCode(10);
const scenePrompt = (what) =>
	[
		`An EMPTY ROOM for a game home, as a small 2.5D diorama: ${what}.`,
		STYLE,
		'Three-quarter view from above (about 30 degrees) into the corner of the room: two walls meeting in the back corner, the floor in front, nothing else – no furniture, no people, no objects. The floor must stay free (furniture is placed on it later).',
		'No text, no letters, no logos, no watermark.'
	].join(LF);
const fileRef = (file) => ({ data: readFileSync(file).toString('base64'), media_type: { '.png': 'image/png', '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg', '.webp': 'image/webp' }[extname(file).toLowerCase()] ?? 'image/png' });

/** what to make */
const jobs = [];
if (scenes) {
	if (!sceneRefs.length) throw new Error("--scenes needs the owner's example images: --scene-ref <file> (one or more)");
	for (const [id, what] of Object.entries(SCENES)) jobs.push({ id, text: scenePrompt(what), scene: true, done: join(OUT, `${id}.scene.png`) });
} else {
	for (const [id, what] of Object.entries(FURNITURE)) jobs.push({ id, text: objectPrompt(what, 'IN-GAME FURNITURE ITEM', randomScheme(), FURNITURE_FRAMING), done: join(OUT, `${id}.png`), home: true });
	for (const [id, what] of Object.entries(ICONS)) jobs.push({ id, text: objectPrompt(what), done: join(OUT, `${id}.png`) });
}
const todo = jobs.filter((j) => (only ? j.id === only : !existsSync(j.done)));
const refCount = scenes ? sceneRefs.length : undefined;
console.log(`${todo.length} image(s) to make (${model} ≈ $${(todo.length * pricePerImage(model, refCount)).toFixed(2)})`);
if (dry) {
	for (const j of todo) console.log(`\n── ${j.id} ──\n${j.text}`);
	process.exit(0);
}
const refs = scenes ? sceneRefs.map(fileRef) : await styleRefs();
for (const j of todo) {
	const t0 = Date.now();
	try {
		const raw = join(OUT, `${j.id}.raw.png`);
		writeFileSync(raw, await lumaImage(j.text, { model, refs }));
		if (j.scene) writeFileSync(j.done, readFileSync(raw));
		else {
			cutOut(raw, j.done, 512);
			if (j.home) cutOut(raw, join(OUT, `${j.id}.home.png`), 512, { square: false });
		}
		console.log(`${j.id} ✓ ${Math.round((Date.now() - t0) / 1000)} s`);
	} catch (e) {
		console.log(`${j.id} ✗ ${e.message}`);
	}
}
