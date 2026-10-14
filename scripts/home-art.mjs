// Home art with Luma (agents/research/CAVE.md, workflows/home/furniture.md; owner 2026-10-14: "the icons for the cave
// session can also be generated through the API based on the same rules I used for the other icons"). The SCUTOONS
// Luma look is shared – scripts/lib/luma-style.mjs (model, style, production references, random colour schemes, the
// cut-out); this script only says WHAT and how a piece of furniture stands.
//   Furniture: ONE render each (three-quarter view, its back to the left wall) → the home picture (trimmed, the foot at
//   the bottom, longest side 512: <id>.home.png) and the inventory icon (the same render, a 512 square: <id>.png).
//   The VR headset: an icon only. Home scenes: --scenes (from a description, 16:9 full frame, no cut-out; optional
//   --scene-ref <file> …). The airlift's carriers: --carriers (placeholders, cut-outs).
// Out: logs/home-art/ – local review; production upload only with the owner's yes. A done file is skipped (delete it to
// redo). Key: LUMA_AGENT(S)_API_KEY in .env (here, or the file named by SCUTOONS_ENV).
//   node scripts/home-art.mjs [--dry] [--only workbench] [--model uni-1] [--scenes [--scene-ref <file> …]] [--carriers]
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
const carriers = argv.includes('--carriers');
const sceneRefs = argv.flatMap((a, i) => (a === '--scene-ref' ? [argv[i + 1]] : []));
const OUT = join(ROOT, 'logs', 'home-art');
mkdirSync(OUT, { recursive: true });

/** furniture (workflows/home/furniture.md – the size is the item's, not the picture's) */
const FURNITURE = {
	workbench: 'a sturdy workbench with a vice on one end and a few tool marks, an empty top',
	cd_rack: 'a classic 90s CD tower rack: an open, VERY tall and slim frame – two narrow upright side boards on a small foot, about five times as tall as it is wide, like a floor lamp – with dozens of CD cases stacked horizontally one above the other between the sides, from the floor up to the top',
	cassette_rack: 'a small 90s cassette tower with slots for cassettes, empty',
	record_shelf: 'a low cube shelf with two open compartments for vinyl records standing upright, empty',
	sofa: 'a small two-seater sofa'
};
/** items that are only an icon */
const ICONS = {
	vr_headset: 'a VR headset (virtual reality goggles) with a soft head strap'
};
/**
 * the home scenes (research/CAVE.md D3) – from a description (owner 2026-10-14: "any human habitable cave", "a standard
 * Japanese mat chamber with tatamis"; generated, no internet photos)
 */
const SCENES = {
	cave: 'a cosy, human-habitable cave dwelling (a troglodyte-style living cave): rough carved rock walls and a vaulted rock ceiling, warm light from a hidden source, a smooth, level, completely EMPTY floor area in the middle for furniture',
	tatami_room: 'a standard, empty Japanese washitsu: tatami mats covering the floor, sliding shoji paper screens with a wooden lattice, a tokonoma alcove in the back wall, soft daylight, the floor completely EMPTY for furniture'
};
/** the creature airlift's carriers (research/CAVE.md D8): crazy creatures and flying robot taxis – placeholders */
const CARRIERS = {
	carrier_moth: 'a giant fluffy friendly moth creature flying with wide patterned wings, a small padded passenger saddle on its back',
	carrier_jelly: 'a floating jellyfish-like sky creature, its bell glowing softly, a little woven passenger basket hanging below its tentacles',
	carrier_robot_taxi: 'a chubby flying robot taxi with four little propellers, round headlight eyes, a taxi sign on its roof and an open passenger seat',
	carrier_drone_cab: 'a quirky retro-futuristic hover cab robot with stubby wings and a jet nozzle, a bubble cockpit with one passenger seat'
};

/** how a piece of furniture stands: turned so it can stand against the left wall of a room, on its own feet */
const FURNITURE_FRAMING =
	'Three-quarter view from above (about 30 degrees), the piece turned so its back faces the LEFT and its front faces the lower right (it will stand against the left wall of a room). The piece alone, centred, filling about 75% of the square, on a plain pure white background (#FFFFFF), no floor, no rug, no cast shadow on the background.';
const LF = String.fromCharCode(10);
const scenePrompt = (what) =>
	[
		`An EMPTY ROOM for a game home: ${what}.`,
		STYLE,
		'A full-frame wide view (16:9) into the room from slightly above, like a game location: the floor in front and in the middle, the walls behind – nothing else, no furniture, no people, no objects. The floor must stay free (furniture is placed on it later).',
		'No text, no letters, no logos, no watermark.'
	].join(LF);
const fileRef = (file) => ({ data: readFileSync(file).toString('base64'), media_type: { '.png': 'image/png', '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg', '.webp': 'image/webp' }[extname(file).toLowerCase()] ?? 'image/png' });

/** what to make */
const jobs = [];
if (scenes) {
	for (const [id, what] of Object.entries(SCENES)) jobs.push({ id, text: scenePrompt(what), scene: true, done: join(OUT, `${id}.scene.png`) });
} else if (carriers) {
	for (const [id, what] of Object.entries(CARRIERS)) jobs.push({ id, text: objectPrompt(what, 'ITEM ICON'), done: join(OUT, `${id}.png`) });
} else {
	for (const [id, what] of Object.entries(FURNITURE)) jobs.push({ id, text: objectPrompt(what, 'IN-GAME FURNITURE ITEM', randomScheme(), FURNITURE_FRAMING), done: join(OUT, `${id}.png`), home: true });
	for (const [id, what] of Object.entries(ICONS)) jobs.push({ id, text: objectPrompt(what), done: join(OUT, `${id}.png`) });
}
const todo = jobs.filter((j) => (only ? j.id === only : !existsSync(j.done)));
const refCount = scenes && sceneRefs.length ? sceneRefs.length : undefined;
console.log(`${todo.length} image(s) to make (${model} ≈ $${(todo.length * pricePerImage(model, refCount)).toFixed(2)})`);
if (dry) {
	for (const j of todo) console.log(`\n── ${j.id} ──\n${j.text}`);
	process.exit(0);
}
const refs = scenes && sceneRefs.length ? sceneRefs.map(fileRef) : await styleRefs();
for (const j of todo) {
	const t0 = Date.now();
	try {
		const raw = join(OUT, `${j.id}.raw.png`);
		writeFileSync(raw, await lumaImage(j.text, { model, refs, aspect: j.scene ? '16:9' : '1:1' }));
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
