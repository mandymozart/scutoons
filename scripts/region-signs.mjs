// Example welcome signs for the map's regions (agents/MAP.md §8.10, owner 2026-10-12: "spectacular – like the Wachau
// wine-region signs, 'Idaho – Famous Potatoes', the 'Welcome to Twin Peaks' sign: claim, and a fancy sight with some
// images"): one Luma image per region (uni-1-max ≈ $0.10) → logs/map-paint/signs/<id>.png. A sign already made is
// skipped (delete it to redo). Costs money – --dry prints the prompts; --only <id> makes one.
// Key: LUMA_AGENT(S)_API_KEY in .env.
//   node scripts/region-signs.mjs [--dry] [--only dunes] [--model uni-1-max]
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
try { process.loadEnvFile(resolve(ROOT, '.env')); } catch {}
const KEY = process.env.LUMA_AGENTS_API_KEY ?? process.env.LUMA_AGENT_API_KEY;
const arg = (name, fallback) => (process.argv.includes(`--${name}`) ? process.argv[process.argv.indexOf(`--${name}`) + 1] : fallback);
const dry = process.argv.includes('--dry');
const only = arg('only', null);
const model = arg('model', 'uni-1-max');
const out = join(ROOT, 'logs', 'map-paint', 'signs');
mkdirSync(out, { recursive: true });

/** the example regions (the painted map's zones, scripts/map-zones.json) – names and claims to refine */
export const REGIONS = [
	{ id: 'dunes', name: 'Dünenmeer Magenta', claim: 'Wo der Sand singt', scene: 'glowing dunes drifting from tangerine into hot magenta and turquoise, neon-pink salt pans, black rock ridges with glowing veins, a lone camel-like creature' },
	{ id: 'altiplano', name: 'Hochland Regenbogen', claim: 'Sieben Farben, ein Himmel', scene: 'golden steppe grass, rainbow-striped mountains like Vinicunca, turquoise lagoons, a llama' },
	{ id: 'alps', name: 'Ewigschnee', claim: 'Hier ist immer Dezember', scene: 'glittering snowy peaks in electric cyan and lilac, frosted pines, a cosy wooden chalet, icicles hanging from the sign' },
	{ id: 'jungle', name: 'Schuppenwald', claim: 'Leise treten, laut staunen', scene: 'dense emerald jungle canopy, iridescent lizard scales, a gecko peeking over the sign, steaming electric-blue swamp pools' },
	{ id: 'wasteland', name: 'Ödland Fracktal', claim: 'Wir sind noch da', scene: 'cracked sulphur-yellow crust, rusty drilling rigs, oily rainbow pools, radioactive green glow; the sign bent and patched, still proud' },
	{ id: 'archipelago', name: 'Juwelenarchipel', claim: 'Jede Insel ein Schatz', scene: 'little islands made of opal, malachite, rose quartz and lapis in a candy-blue sea, a shell and pearls' },
	{ id: 'highlands', name: 'Erzberge', claim: 'Geizig! Garstig! Steil!', scene: 'green lowlands rising into huge, grim, jagged snowy mountains, rust-red ore slopes, a mining pick and hammer crest, a small monastery clinging to a cliff' }
];

const prompt = (r) =>
	[
		`A SPECTACULAR roadside WELCOME SIGN for the region "${r.name}" – in the spirit of the "Welcome to Twin Peaks" sign, the US state welcome signs ("Idaho – Famous Potatoes") and the illustrated wine-region signs of the Wachau.`,
		`LETTERING: the name "${r.name}" big and proud across the top, the claim "${r.claim}" smaller underneath – spelled exactly like this, German, nothing else written.`,
		`ILLUSTRATION on the sign: ${r.scene}.`,
		'THE SIGN: a big carved and painted wooden board with a decorative frame, standing on two sturdy posts in a little patch of the region\'s own ground and plants; front view, the whole sign in the picture, centred, on a plain white background (it will be cut out).',
		'LOOK – the SCUTOONS style: a high-detail stylised 3D render like a premium collectible toy diorama – tactile materials (painted wood, glossy enamel, felt-like plants), soft studio light, saturated playful colours, crisp clean edges.',
		'No people, no other text, no logos, no watermark.'
	].join('\n');

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function luma(text) {
	const h = { authorization: `Bearer ${KEY}`, 'content-type': 'application/json' };
	const r = await fetch('https://agents.lumalabs.ai/v1/generations', { method: 'POST', headers: h, body: JSON.stringify({ type: 'image', model, prompt: text, output_format: 'png', aspect_ratio: '4:3' }) });
	let j = await r.json();
	if (!r.ok) throw new Error(`luma ${r.status}: ${JSON.stringify(j).slice(0, 300)}`);
	while (!['completed', 'failed'].includes(j.state)) {
		await sleep(3000);
		j = await (await fetch(`https://agents.lumalabs.ai/v1/generations/${j.id}`, { headers: h })).json();
	}
	if (j.state === 'failed') throw new Error(`luma: ${j.failure_code ?? ''} ${j.failure_reason ?? ''}`);
	return Buffer.from(await (await fetch(j.output.find((o) => o.type === 'image').url)).arrayBuffer());
}

const todo = REGIONS.filter((r) => (only ? r.id === only : !existsSync(join(out, `${r.id}.png`))));
console.log(`${todo.length} signs to make (${model} ≈ $${(todo.length * 0.1).toFixed(2)})`);
for (const r of todo) {
	if (dry) {
		console.log(`\n── ${r.id} ──\n${prompt(r)}`);
		continue;
	}
	if (!KEY) throw new Error('LUMA_AGENT_API_KEY is not set in .env');
	const t0 = Date.now();
	writeFileSync(join(out, `${r.id}.png`), await luma(prompt(r)));
	console.log(`${r.id} (${Math.round((Date.now() - t0) / 1000)} s)`);
}
