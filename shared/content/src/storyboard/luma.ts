// The prompt for Luma for one shot (from scripts/storyboard/luma.js, #119; owner 2026-10-09: "can it give me the prompt
// i have to pass on to luma … summarize with all the parameters we adjusted and all the context"). Everything the
// storyboard decided goes in – the story moment, the perspective, the figures (persona, mood, pose, facing, where and
// how big), the camera, the one hard light, the empty zones for text, the look (taste.md) – plus what to attach.

import type { Figure, Light, Shot, StoryboardDoc } from './types.ts';

const PERSONA: Record<string, string> = {
	'catpope/human': 'PLF – Cat Pope in human form: a figure in a long white papal robe with a short cape (mozzetta), a hairless sphynx cat head with very large upright ears',
	'catpope/mountaintiger': 'the white mountain tiger – Cat Pope himself (PLF fights himself without knowing it): white, muscular, half upright, big claws, a rune glyph on the forehead and on the body',
	'catpope/regular': 'Cat Pope, the narrator: sphynx cat head with very large ears, papal robe – speaking to the reader, outside the scene',
	'catpope/cat': 'a hairless sphynx cat',
	'catpope/biker': 'Cat Pope as a biker',
	'crows/regular': 'crow creatures: black, human-sized, heavy beaks, ragged wings',
	'boy/regular': 'the boy'
};
const VIEW: Record<string, string> = {
	real: 'what really happens, seen from outside',
	belief: 'what the character believes / perceives – subjective, it may not be what is really there (here: PLF sees a tiger; in truth it is himself)',
	narration: 'narration: the narrator addresses the reader, outside the scene',
	memory: 'a memory of the past – more distant'
};
const SIZE: Record<string, string> = { ecu: 'extreme close-up', cu: 'close-up', mcu: 'medium close-up (chest up)', medium: 'medium shot', full: 'full shot (whole figure)', wide: 'wide shot', ews: 'extreme wide establishing shot' };
const ANGLE: Record<string, string> = { eye: 'eye level', high: 'high angle, looking down', low: 'low angle, looking up', bird: "bird's-eye view, from straight above", worm: "worm's-eye view, from the ground", dutch: 'dutch angle (tilted horizon)' };
const PROJ: Record<string, string> = { 'top-down': 'straight top-down', isometric: 'isometric', polar: 'little-planet polar projection', perspective: 'normal perspective' };
const MOVE: Record<string, string> = { hold: 'a still moment', 'tilt-down': 'read top to bottom: the eye travels down', 'tilt-up': 'read bottom to top: the eye travels up', fall: 'a fall down the frame', climb: 'a climb up the frame', travel: 'travel over the ground', descend: 'a descent', reveal: 'a reveal at the bottom after empty space above', 'look-around': 'a 360° look around', pass: 'a sideways pass' };
const KEY: Record<string, string> = { left: 'from the left', right: 'from the right', top: 'from straight above', bottom: 'from below', front: 'from the front (behind the camera)', back: 'from behind the subject – backlight, silhouettes', 'top-left': 'from the upper left', 'top-right': 'from the upper right', 'bottom-left': 'from the lower left', 'bottom-right': 'from the lower right' };
const HEIGHT: Record<string, string> = { below: 'below the faces (menacing)', low: 'low and grazing (long shadows)', eye: 'at eye height', high: 'high', overhead: 'straight overhead (eyes in shadow)' };
const FACING: Record<string, string> = { front: 'facing the camera', away: 'back to the camera', left: 'in profile, facing left', right: 'in profile, facing right', 'three-quarter-left': 'three-quarter view, turned left', 'three-quarter-right': 'three-quarter view, turned right' };
const POSE: Record<string, string> = { stand: 'standing', walk: 'walking', run: 'running', jump: 'leaping', fall: 'falling', slip: 'slipping', crouch: 'crouching', kneel: 'kneeling', crawl: 'crawling', sit: 'sitting', lie: 'lying', reach: 'reaching out', point: 'pointing', throw: 'throwing', cast: 'both hands thrust forward, casting a ball of energy', 'arms-up': 'arms raised', 'hug-self': 'arms wrapped around itself against the cold', stamp: 'stamping the ground', push: 'pushing', 'look-back': 'looking back over the shoulder', pray: 'hands folded in prayer', roar: 'roaring' };

const where = (f: Figure) => `${(f.x ?? 0.5) < 0.38 ? 'left' : (f.x ?? 0.5) > 0.62 ? 'right' : 'centre'} ${(f.y ?? 0.5) < 0.35 ? 'top' : (f.y ?? 0.5) > 0.65 ? 'bottom' : 'middle'}`;
const lookup = (map: Record<string, string>, k: string | undefined) => (k === undefined ? undefined : map[k]);

export interface LumaContext {
	/** the scene's script lines (optional) */
	script?: string;
	/** what to attach (file names or URLs) */
	references?: { structure?: string; style?: string[]; characters?: string[] };
}

/** sb = the scene's storyboard, shot = the shot (as edited), ctx = the script + the references → the prompt text */
export function lumaPrompt(sb: StoryboardDoc, shot: Shot, ctx: LumaContext = {}): string {
	const cast = Object.fromEntries((sb.cast ?? []).map((c) => [c.ref, c]));
	const place = (sb.locations ?? []).find((l) => l.ref === shot.location);
	const light: Light = { ...(sb.light ?? {}), ...(shot.light ?? {}) };
	const lines: string[] = [];
	const add = (...l: string[]) => lines.push(...l);

	add(`SHOT ${shot.id} – ${shot.format} ${shot.aspect}${shot.format === 'vertical' ? ' (a tall image, read from top to bottom)' : shot.format === 'loop360' ? ' (a seamless 360° panorama: left and right edges must continue each other)' : ''}`, '');
	add('WHAT TO MAKE', 'One storyboard frame for a vertical-scroll webtoon. Keep the exact composition, camera, poses and light of the attached structure image (our 3D render) – it is the layout. Draw it as clean line art in black and white.', '');

	add('THE STORY MOMENT');
	add(`Scene ${sb.source?.scene ?? ''}: ${sb.source?.heading ?? ''}${place ? ` – place: ${place.place ?? place.new ?? place.ref}${place.time ? `, ${place.time}` : ''}` : ''}.`);
	if (shot.prompt?.en) add(`The image: ${shot.prompt.en}`);
	add(`In German (the storyboard's words – take the details): ${shot.action?.de ?? ''}`);
	if (shot.view) add(`Perspective: ${VIEW[shot.view] ?? shot.view}.`);
	const plan = (sb.notes ?? []).find((n) => /^Perspektive/i.test(n));
	if (plan) add(`Scene plan: ${plan}`);
	add('');

	add('FIGURES');
	if (!(shot.figures ?? []).length) add('No figures.');
	for (const f of shot.figures ?? []) {
		const c: Partial<(typeof cast)[string]> = { ...(cast[f.cast] ?? {}), ...(f.persona ? { persona: f.persona } : {}) };
		const who = c.new ?? PERSONA[`${c.character}/${c.persona}`] ?? `${c.character ?? f.cast}${c.persona ? ` (${c.persona})` : ''}`;
		const size = Math.round((f.h ?? 0.3) * 100);
		add(`- ${who}${f.mood ? `; mood: ${f.mood}` : ''}; ${lookup(POSE, f.pose) ?? f.pose ?? 'standing'}; ${lookup(FACING, f.facing) ?? f.facing ?? ''}${f.turn != null ? ` (turned ${f.turn}°)` : ''}; ${f.framing && f.framing !== 'full' ? `${SIZE[f.framing] ?? f.framing}; ` : ''}in the ${where(f)} of the frame, about ${size} % of a phone screen's height.`);
	}
	add('');

	add('CAMERA');
	const cam = shot.camera ?? {};
	add(`${lookup(SIZE, shot.shot?.size) ?? shot.shot?.size ?? ''}, ${lookup(ANGLE, shot.shot?.angle) ?? shot.shot?.angle ?? ''}, ${lookup(PROJ, shot.projection) ?? shot.projection ?? ''}. ${lookup(MOVE, shot.move) ? `The frame is ${MOVE[shot.move!]}.` : ''}`.replace(/\s+/g, ' ').trim());
	if (cam.pos) add(`Hand-placed camera: at [${cam.pos.map((n) => n.toFixed(1)).join(', ')}] looking at [${(cam.target ?? [0, 0, 0]).map((n) => n.toFixed(1)).join(', ')}] (metres; the figures stand on the ground at y = 0)${cam.fov ? `, lens ${Math.round(cam.fov)}° vertical` : ''}${cam.roll ? `, rolled ${cam.roll}°` : ''} – follow the structure image.`);
	else if (Object.keys(cam).length) add(`Hand adjustments: ${Object.entries(cam).map(([k, v]) => `${k} ${v}`).join(', ')} – follow the structure image.`);
	add('');

	add('LIGHT – the most important part');
	const keyText = light.azimuth != null || light.elevation != null
		? `from ${light.azimuth ?? -45}° around the subject (0 = from the camera's side, 90 = from the right, 180 = from behind) and ${light.elevation ?? 30}° up`
		: `${lookup(KEY, light.key) ?? 'from the upper left'}${light.height ? `, ${HEIGHT[light.height] ?? light.height}` : ''}`;
	add(light.source === 'none'
		? 'Almost no light: a black frame, only a rim of light on the edges.'
		: `ONE extremely hard ${light.source ? `${light.source} ` : ''}key light ${keyText}. ${light.shadow === 'long' ? 'Long hard cast shadows. ' : light.shadow === 'short' ? 'Short hard shadows. ' : ''}${light.rim ? 'A bright rim of light on the figures against the dark. ' : ''}${(light.soft ?? 0.5) < 0.3 ? 'No fill light, no soft or diffuse light: what faces the light is pure white, everything else solid black.' : `A little fill light (softness ${light.soft ?? 0.5} of 1): what faces the key is white, the shadow side dark but readable, solid black only in the deepest shadows. Still one clear light direction, never flat or diffuse.`}`);
	if (light.note?.de) add(`What the light does (storyboard): ${light.note.de}`);
	add(light.accent?.color ? `The only colour in the frame: ${light.accent.color}${light.accent.on ? `, on ${light.accent.on}` : ''}. Everything else black and white.` : 'No colour at all: black and white only.');
	add('');

	if ((shot.air ?? []).length) {
		add('EMPTY ZONES (space for text boxes – keep them calm and plain)');
		for (const [a, b] of shot.air ?? []) add(`- ${a === 0 ? `the top ${b} %` : b === 100 ? `the bottom ${100 - a} %` : `from ${a} % to ${b} % down`} of the frame`);
		add('');
	}

	add('LOOK');
	add('Clean line art: crisp, even contour lines on white paper; solid black shadow shapes from the one hard light so the lighting reads at a glance; intense black and white like a noir storyboard. No hatching, no grey marker shading, no sketchy or hand-drawn strokes, no gradients.');
	add('');
	add('DO NOT', 'No text, no lettering, no speech bubbles, no panel borders, no watermark, no colour except the one accent named above.', '');

	const r = ctx.references;
	if (r) {
		add('ATTACH IN LUMA');
		if (r.structure) add(`- Image reference (strong): ${r.structure} – the layout`);
		for (const s of r.style ?? []) add(`- Style reference: ${s}`);
		for (const c of r.characters ?? []) add(`- Character reference: ${c}`);
		add('');
	}
	if (ctx.script) add('THE SCRIPT (for context)', ctx.script.trim(), '');
	return lines.join('\n').replace(/\n{3,}/g, '\n\n').trim() + '\n';
}
