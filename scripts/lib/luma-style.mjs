// The SCUTOONS Luma look, shared by every script that makes images of things (owner 2026-10-14: the cave must use
// "exactly the same Luma settings" as the item icons): the model, the style text, the production style references,
// the API call, and the white background → transparent cut-out. Users: scripts/item-icons.mjs, the cave's furniture.
//
// The look ("don't bring me back this vector style"; "more creative … more colourful, experimental stuff"): pseudo-3D,
// a hyper-real but cartoony stylised render like the production items, characters and map places – each object in its
// OWN colours and materials like a real product (no house accent colour). References: the PRODUCTION icons from the CDN,
// never the local dev copy ("use only from production").
// Price (docs.agents.lumalabs.ai/guides/pricing): uni-1 $0.0434 with one reference + $0.003 per extra (≈ $0.052 with
// the 4 references) – the default ("save some money"); uni-1-max $0.103 + $0.003 per extra (≈ $0.112).
import { execFileSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
try { process.loadEnvFile(resolve(ROOT, '.env')); } catch {}
const KEY = process.env.LUMA_AGENTS_API_KEY ?? process.env.LUMA_AGENT_API_KEY;
const LF = String.fromCharCode(10);

export const DEFAULT_MODEL = 'uni-1';

/** the production item icons sent along as the style (cdn.scutoons.com) */
export const STYLE_REFS = ['scutoons/items/record_player.webp', 'catpope/items/camera.webp', 'scutoons/items/tuna_can.webp', 'scutoons/items/repair_coupon.webp'];

/** what one image costs on `model` with `refs` reference images (dollars) */
export const pricePerImage = (model = DEFAULT_MODEL, refs = STYLE_REFS.length) => (model === 'uni-1-max' ? 0.103 : 0.0434) + Math.max(0, refs - 1) * 0.003;

/** the style paragraph for an object (an item, a piece of furniture …) – put it after the "what" */
export const STYLE = [
	'STYLE – the same set as the attached reference images: pseudo-3D, a hyper-realistic but cartoony stylised 3D render like a premium collectible toy or game item – chunky, slightly rounded forms, tactile believable materials, soft studio light from the upper left, gentle contact shading, crisp clean edges.',
	'COLOURS AND MATERIALS: give this object its OWN look like a real, general product of its kind – be creative, colourful and a little experimental (unexpected colour combinations, patterned or translucent plastic, enamel, rubber, fabric, metal finishes); do NOT copy the reference images\' colours and do NOT add a cyan accent.'
].join(LF);

/**
 * Material and colour schemes – one drawn AT RANDOM per image (owner 2026-10-14: "random is necessary"; "use creative
 * colors. maybe exotic textures like from a 3d texture library. organics, experimental. or sugar coated. make it fun a
 * little"), so a set never shares one house look; the prompt names it as the object's surface.
 */
export const COLOUR_SCHEMES = [
	'sugar-coated candy glaze in strawberry pink and lemon, with sparkling sugar crystals',
	'gummy translucent jelly in lime green and cherry red, light glowing through',
	'iridescent beetle-shell finish shifting from emerald to violet',
	'soft mossy organic surface in forest greens with tiny lichen speckles',
	'marbled resin swirls of teal, gold flakes and cream',
	'colourful terrazzo with chunky chips of coral, mustard and mint on off-white',
	'chunky knitted wool in tomato red and cream stripes',
	'glossy lacquered enamel in cobalt blue with chipped white edges',
	'holographic rainbow foil with crisp reflections',
	'thick ceramic glaze in burnt orange with drips running down',
	'speckled stoneware in sand and charcoal',
	'frosted milk glass in pastel lilac and peach',
	'glossy liquorice black with neon pink piping',
	'woven rattan and natural cane with turquoise accents',
	'brushed copper with verdigris patina',
	'puffy inflated vinyl in bubblegum pink and sky blue',
	'chocolate-dipped finish with rainbow sprinkles',
	'cork and recycled-paper texture with mustard yellow paint',
	'jade-green polished stone with gold inlays',
	'glow-in-the-dark plastic in acid yellow-green with purple details'
];

/** one material / colour scheme, drawn at random */
export const randomScheme = () => COLOUR_SCHEMES[Math.floor(Math.random() * COLOUR_SCHEMES.length)];

/** how an object sits in the image (the default: an item icon) */
export const ICON_FRAMING = 'The object alone, centred, three-quarter view, filling about 75% of the square, on a plain pure white background (#FFFFFF), no floor, no cast shadow on the background.';

/**
 * an object alone on white, ready to be cut out; `scheme` = its main colours (default: drawn at random); `framing` = how
 * it sits (default ICON_FRAMING – the cave's furniture turns its back to the left wall: scripts/home-art.mjs)
 */
export const objectPrompt = (what, kind = 'ITEM ICON', scheme = randomScheme(), framing = ICON_FRAMING) =>
	[
		`An ${kind} of ${what}.`,
		STYLE,
		`THIS OBJECT'S SURFACE: ${scheme} – fun and a little experimental, while the object stays clearly recognisable (natural material where the product needs it to work, e.g. a drill bit's steel, paper pages).`,
		framing,
		'No text, no letters, no logos, no watermark, no frame, no scene.'
	].join(LF);

let refsCache = null;
/** the style references as Luma image_ref entries – fetched from production once */
export async function styleRefs(paths = STYLE_REFS) {
	if (refsCache && paths === STYLE_REFS) return refsCache;
	const refs = [];
	for (const path of paths) {
		const r = await fetch(`https://cdn.scutoons.com/${path}`);
		if (!r.ok) throw new Error(`reference ${path}: ${r.status}`);
		refs.push({ data: Buffer.from(await r.arrayBuffer()).toString('base64'), media_type: path.endsWith('.png') ? 'image/png' : 'image/webp' });
	}
	if (paths === STYLE_REFS) refsCache = refs;
	return refs;
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
/** one Luma image → its PNG bytes. `refs` = image_ref entries (styleRefs()) */
export async function lumaImage(prompt, { model = DEFAULT_MODEL, refs = [], aspect = '1:1' } = {}) {
	if (!KEY) throw new Error('LUMA_AGENT_API_KEY is not set in .env');
	const h = { authorization: `Bearer ${KEY}`, 'content-type': 'application/json' };
	const body = { type: 'image', model, prompt, output_format: 'png', aspect_ratio: aspect, ...(refs.length ? { image_ref: refs } : {}) };
	const r = await fetch('https://agents.lumalabs.ai/v1/generations', { method: 'POST', headers: h, body: JSON.stringify(body) });
	let j = await r.json();
	if (!r.ok) throw new Error(`luma ${r.status}: ${JSON.stringify(j).slice(0, 300)}`);
	while (!['completed', 'failed'].includes(j.state)) {
		await sleep(3000);
		j = await (await fetch(`https://agents.lumalabs.ai/v1/generations/${j.id}`, { headers: h })).json();
	}
	if (j.state === 'failed') throw new Error(`luma: ${j.failure_code ?? ''} ${j.failure_reason ?? ''}`);
	return Buffer.from(await (await fetch(j.output.find((o) => o.type === 'image').url)).arrayBuffer());
}

/** the white around the object → transparent (flood fill from the border; enclosed white stays), centred, square */
const CUT = `
import sys
from PIL import Image, ImageDraw
src, dst, size, square = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4] == '1'
im = Image.open(src).convert('RGB')
w, h = im.size
mark = (255, 0, 255)
for x, y in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1), (w // 2, 0), (w // 2, h - 1), (0, h // 2), (w - 1, h // 2)]:
    if sum(im.getpixel((x, y))) > 700:
        ImageDraw.floodfill(im, (x, y), mark, thresh=40)
px = im.load()
out = Image.new('RGBA', (w, h))
op = out.load()
for y in range(h):
    for x in range(w):
        p = px[x, y]
        op[x, y] = (0, 0, 0, 0) if p == mark else (*p, 255)
bbox = out.getbbox()
if bbox:
    out = out.crop(bbox)
if square:
    side = int(max(out.size) / 0.86)
    canvas = Image.new('RGBA', (side, side), (0, 0, 0, 0))
    canvas.paste(out, ((side - out.width) // 2, (side - out.height) // 2))
    canvas.resize((size, size), Image.LANCZOS).save(dst)
else:
    k = size / max(out.size)
    out.resize((max(1, round(out.width * k)), max(1, round(out.height * k))), Image.LANCZOS).save(dst)
`;

/**
 * a PNG on white → a transparent PNG (python + Pillow): a centred square of `size` px, or with `square: false` trimmed
 * to the object, its longest side `size` (a piece of furniture standing on its foot – scripts/home-art.mjs)
 */
export function cutOut(srcPng, dstPng, size = 512, { square = true } = {}) {
	execFileSync('python', ['-c', CUT, srcPng, dstPng, String(size), square ? '1' : '0']);
}
