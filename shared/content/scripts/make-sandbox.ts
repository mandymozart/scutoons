// Builds the sandbox toon "Cat Pope's walk" (dev / creator only, hidden in production) from the character registry:
// Cat Pope (regular) leaves the house and meets every character and every one of his own personas. Each introduces
// themselves, then shows every mood of every look, one by one; Cat Pope mimics each mood (or, when his regular look has
// no such mood, tries, fails and is sad). Look changes use the registry's transition images; Cat Pope tries each new look
// too (no such look → sad). One chapter per encounter, one VN group per persona × look, a goodbye at the end.
//
//   node --experimental-transform-types --no-warnings shared/content/scripts/make-sandbox.ts [--from=<api base>] [--out=<dir>] [--dry-run]
//
//   --from   read the published registry + moods from a content store (e.g. http://localhost:9000/api) instead of the
//            seed files shared/content/seed/scutoons/_/{characters,moods}.json
//   --out    output folder (default viewer/static/api/sandbox; the script owns it and rewrites it completely)
//
// Then put it into the local content store (published):
//   php server/bin/console import-content viewer/static/api --toon=sandbox --update
// The sandbox never goes into the production seed (`import-content --sql` skips it, ImportContentCommand::DEV_ONLY_TOONS).
// Re-run both after adding characters, personas, looks, moods or transitions. See agents/MEMORY.md "Sandbox".
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join as pathJoin, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
	DEFAULT_MOOD,
	REGULAR_PERSONA,
	characterName,
	defaultLookId,
	findLook,
	findPersona,
	lookLabel,
	lookPortraitMoods,
	moodLabel,
	personaIds,
	sortedMoods,
	type CharacterPersona,
	type CharacterRegistry,
	type MoodVocabulary,
	type RegistryCharacter,
	type ToonCast
} from '../src/index.ts';

export const SANDBOX_TOON = 'sandbox';
/** the walker: every encounter is mirrored by this character's regular persona in its default look */
const HOST = 'catpope';
const LOCALES = ['de', 'en'] as const;
type Locale = (typeof LOCALES)[number];
type Text = Record<Locale, string>;
const RELEASE = '2026-09-30';
/** preferred encounter order (owner); characters not listed follow in registry order, then the host's own personas */
const ORDER = ['yellowmouse', 'toptoy', 'boy', 'crows'];
const VIEW = 'portrait';
/** Backgrounds: existing chapter-1 slices. Slice paths are relative to `<cdn>/<toon>/`, so `../catpope/…` reuses catpope's art. */
const CATPOPE = '../catpope/';
const BACKGROUNDS = [12, 20, 27, 33, 41, 48, 56, 63, 70, 77, 85].map((n) => `${CATPOPE}images/3x/de/S1C1 - Slice ${n}.jpg`);

// ── Lines (de = source, en) ─────────────────────────────────────────────────────────────────────────────────────

/** In-character introductions; everyone else gets the generic one. Key: character or character:persona. */
const INTROS: Record<string, Text> = {
	yellowmouse: { de: 'Mausboss. Ich führe die Biker-Gang. Platz da, Papst!', en: 'Yellow Mouse. I run the biker gang. Make way, Pope!' },
	toptoy: { de: 'Wir sind Toptoy – zu zweit und immer im Duett!', en: "We're Toptoy – two of us, always a duet!" },
	boy: { de: 'Ich bin nur ein Junge. Aber ich hab dich gesehen!', en: "I'm just a boy. But I saw you!" },
	crows: { de: 'Krah! Wir sind die Krähen. Wir sehen alles.', en: 'Caw! We are the crows. We see everything.' },
	'catpope:human': { de: 'Ich bin du – als Mensch. Gruselig, oder?', en: "I'm you – as a human. Creepy, right?" },
	'catpope:cat': { de: 'Miau. Ich bin du, nur ehrlicher.', en: "Meow. I'm you, just more honest." },
	'catpope:mountaintiger': { de: 'ROAAAR! Ich bin du – die wilde Ausgabe.', en: "ROAAAR! I'm you – the wild edition." },
	'catpope:biker': { de: 'Ich bin du, auf zwei Rädern. Brumm brumm!', en: "I'm you, on two wheels. Vroom vroom!" }
};

const cap = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);
const q = (locale: Locale, s: string) => (locale === 'de' ? `„${s}“` : `“${s}”`);
const pick = <T>(list: T[], i: number) => list[i % list.length];

const LINES = {
	prelude: { de: 'Ein sonniger Morgen. Katzenpapst tritt vor die Tür.', en: 'A sunny morning. Cat Pope steps out of his door.' } as Text,
	preludeHost: { de: 'Heute gehe ich spazieren. Mal sehen, wen ich treffe!', en: "Today I'm going for a walk. Let's see who I meet!" } as Text,
	walk: (name: Text): Text => ({ de: `Unterwegs trifft Katzenpapst: ${name.de}.`, en: `On his way, Cat Pope meets: ${name.en}.` }),
	walkSelf: (name: Text): Text => ({ de: `Plötzlich steht da … ${name.de}!`, en: `Suddenly, there stands … ${name.en}!` }),
	intro: (name: Text): Text => ({ de: `Hallo! Ich bin ${name.de}.`, en: `Hi! I'm ${name.en}.` }),
	reply: [
		{ de: 'Freut mich! Zeig mal, was du draufhast.', en: 'Nice to meet you! Show me what you’ve got.' },
		{ de: 'Oh, hallo! Was für ein Gesicht machst du so?', en: 'Oh, hello! What faces can you pull?' },
		{ de: 'Na sieh mal einer an. Zeig her!', en: 'Well, look at that. Show me!' }
	] as Text[],
	show: [
		(m: Text): Text => ({ de: `Pass auf – ${m.de}!`, en: `Watch this – ${m.en}!` }),
		(m: Text): Text => ({ de: `Und jetzt: ${m.de}.`, en: `And now: ${m.en}.` }),
		(m: Text): Text => ({ de: `So geht ${q('de', m.de)}:`, en: `This is how ${q('en', m.en)} goes:` })
	],
	mimic: [
		(m: Text): Text => ({ de: `${cap(m.de)}? Kann ich auch!`, en: `${cap(m.en)}? I can do that!` }),
		(m: Text): Text => ({ de: `Nachgemacht: ${m.de}!`, en: `Copied: ${m.en}!` }),
		(m: Text): Text => ({ de: `Ha, ${q('de', m.de)} – wie ich!`, en: `Ha, ${q('en', m.en)} – just like me!` })
	],
	fail: [
		(m: Text): Text => ({ de: `${q('de', m.de)}? … Nein. Das krieg ich nicht hin.`, en: `${q('en', m.en)}? … Nope. I can't do that one.` }),
		(m: Text): Text => ({ de: `Moment … ${m.de} … hmpf. Geht nicht.`, en: `Wait … ${m.en} … hmph. Doesn't work.` })
	],
	noMoods: { de: 'Mehr Gesichter hab ich noch nicht – der Zeichenstift ist unterwegs.', en: "That's all my faces so far – the pencil is on its way." } as Text,
	lookOn: (name: Text, look: Text): Text => ({ de: `${name.de} wechselt den Look: ${look.de}.`, en: `${name.en} changes the look: ${look.en}.` }),
	lookShow: (look: Text): Text => ({ de: `Neuer Look: ${look.de}!`, en: `New look: ${look.en}!` }),
	lookBack: (name: Text, look: Text): Text => ({ de: `${name.de} ist wieder ${look.de}.`, en: `${name.en} is back to ${look.en}.` }),
	styleOk: (look: Text): Text => ({ de: `${cap(look.de)}? Hab ich auch!`, en: `${cap(look.en)}? Got that too!` }),
	styleFail: (look: Text): Text => ({ de: `${q('de', look.de)}? So was hab ich nicht im Schrank …`, en: `${q('en', look.en)}? I don't have that in my closet …` }),
	bye: { de: 'Tschüss, Katzenpapst!', en: 'Bye, Cat Pope!' } as Text,
	onward: [
		{ de: 'Weiter geht’s.', en: 'On we go.' },
		{ de: 'Schöne Begegnung. Weiter!', en: 'Nice meeting. Onward!' }
	] as Text[],
	goodbyeHost: { de: 'Was für ein Spaziergang! Danke euch allen – tschüss!', en: 'What a walk! Thanks, everyone – bye!' } as Text,
	goodbyeAll: { de: 'Tschüss!', en: 'Bye!' } as Text,
	end: { de: 'Ende des Spaziergangs.', en: 'The end of the walk.' } as Text
};

// ── Events ──────────────────────────────────────────────────────────────────────────────────────────────────────

/** An event whose texts are still per locale (`Text`); `localize` picks one locale. Same structure in every locale. */
type Draft = Record<string, unknown>;

interface Ref {
	character: string;
	persona: string;
	look?: string;
	mood?: string;
	variant?: string;
}
const join = (...characters: Ref[]): Draft => ({ type: 'joinScene', params: { characters } });
const leave = (...characters: Ref[]): Draft => ({ type: 'leaveScene', params: { characters: characters.map(({ character, persona }) => ({ character, persona })) } });
const say = (who: { character: string; persona: string } | null, text: Text): Draft => ({
	type: 'showMessage',
	params: { ...(who ? { character: who.character, persona: who.persona } : {}), text, wait: true }
});
const jump = (target: { type: 'chapter' | 'toon'; value: string; label?: string }): Draft => ({ type: 'jumpTo', params: { target, wait: true } });

function localize(value: unknown, locale: Locale): unknown {
	if (Array.isArray(value)) return value.map((v) => localize(v, locale));
	if (!value || typeof value !== 'object') return value;
	const obj = value as Record<string, unknown>;
	const keys = Object.keys(obj);
	if (keys.length === LOCALES.length && LOCALES.every((l) => typeof obj[l] === 'string')) return obj[locale];
	return Object.fromEntries(keys.map((k) => [k, localize(obj[k], locale)]));
}

// ── Build ───────────────────────────────────────────────────────────────────────────────────────────────────────

interface Encounter {
	character: RegistryCharacter;
	personas: string[];
}
interface GroupPlan {
	persona: string;
	look: string;
}
export interface SandboxReport {
	chapters: number;
	groups: number;
	encounters: number;
	moods: number;
	mimics: number;
	fails: number;
	lookSwitches: number;
	transitions: number;
	styleFails: number;
	perEncounter: { chapter: number; name: string; groups: string[]; moods: number; fails: number }[];
}
export interface SandboxFiles {
	/** path below the toon folder (`de/chapter1.json`, `_/cast.json`, …) → JSON */
	files: Record<string, unknown>;
	report: SandboxReport;
}

const text = (fn: (l: Locale) => string): Text => ({ de: fn('de'), en: fn('en') });

/** The moods a look has a portrait for, in vocabulary order (unknown ones after, alphabetical). */
function moodsOf(persona: CharacterPersona | undefined, look: string, vocabulary: MoodVocabulary): string[] {
	const own = lookPortraitMoods(findLook(persona, look));
	const order = sortedMoods(vocabulary).map((m) => m.id);
	return [...order.filter((m) => own.includes(m)), ...own.filter((m) => !order.includes(m)).sort()];
}

/** Looks of a persona: the default first, then the others in registry order. */
function looksOf(persona: CharacterPersona | undefined): string[] {
	const def = defaultLookId(persona);
	return [def, ...(persona?.looks ?? []).map((l) => l?.id).filter((id): id is string => !!id && id !== def)];
}

export function buildSandbox(registry: CharacterRegistry, vocabulary: MoodVocabulary): SandboxFiles {
	const characters = (registry.characters ?? []).filter((c) => !!c?.id);
	const host = characters.find((c) => c.id === HOST);
	if (!host) throw new Error(`the registry has no "${HOST}"`);
	const hostRegular = findPersona(host, REGULAR_PERSONA);
	const hostLook = defaultLookId(hostRegular);
	const hostMoods = new Set(moodsOf(hostRegular, hostLook, vocabulary));
	const hostLooks = new Set(looksOf(hostRegular));
	const HOST_REF = { character: HOST, persona: REGULAR_PERSONA };
	const hostAs = (mood: string, look = hostLook): Ref => ({ ...HOST_REF, look, mood, variant: VIEW });

	const others = characters.filter((c) => c.id !== HOST);
	const ordered = [...ORDER.map((id) => others.find((c) => c.id === id)).filter((c): c is RegistryCharacter => !!c), ...others.filter((c) => !ORDER.includes(c.id))];
	const encounters: Encounter[] = [
		...ordered.map((c) => ({ character: c, personas: personaIds(c) })),
		...(personaIds(host).some((p) => p !== REGULAR_PERSONA) ? [{ character: host, personas: personaIds(host).filter((p) => p !== REGULAR_PERSONA) }] : [])
	];

	const name = (c: RegistryCharacter, persona: string) => text((l) => characterName(c, l, persona, ['de', 'en']));
	const mood = (id: string) => text((l) => moodLabel(id, l, vocabulary));
	const look = (p: CharacterPersona | undefined, id: string) => text((l) => lookLabel(p, id, l));
	const transitionLabel = (p: CharacterPersona | undefined, from: string, to: string): Text | null => {
		const t = p?.transitions?.find((x) => x.from === from && x.to === to);
		if (!t) return null;
		return t.label ? text((l) => t.label?.[l] || t.label?.de || `${from} → ${to}`) : text(() => `${from} → ${to}`);
	};

	const report: SandboxReport = { chapters: 0, groups: 0, encounters: 0, moods: 0, mimics: 0, fails: 0, lookSwitches: 0, transitions: 0, styleFails: 0, perEncounter: [] };
	const files: Record<string, unknown> = {};
	const sequence: { type: 'chapter'; number: number; status: 'draft' }[] = [];
	const met: Ref[] = [];
	let lineNo = 0;

	encounters.forEach((enc, e) => {
		const number = e + 1;
		const isHostChapter = enc.character.id === HOST;
		const plans: GroupPlan[] = enc.personas.flatMap((persona) => looksOf(findPersona(enc.character, persona)).map((look) => ({ persona, look })));
		const labels = plans.map((p) => `${p.persona}-${p.look}`);
		const isLast = e === encounters.length - 1;
		if (isLast) labels.push('goodbye');
		const stats = { chapter: number, name: characterName(enc.character, 'de', isHostChapter ? undefined : enc.personas[0]), groups: labels, moods: 0, fails: 0 };
		const background = BACKGROUNDS[e % BACKGROUNDS.length];
		// the next group of this chapter, else the next chapter, after the goodbye the toon page
		const next = (i: number) =>
			i + 1 < labels.length
				? { type: 'chapter' as const, value: String(number), label: labels[i + 1] }
				: isLast
					? { type: 'toon' as const, value: SANDBOX_TOON }
					: { type: 'chapter' as const, value: String(number + 1) };

		plans.forEach((plan, i) => {
			const c = enc.character;
			const persona = findPersona(c, plan.persona);
			const def = defaultLookId(persona);
			const who = { character: c.id, persona: plan.persona };
			const as = (lookId: string, moodId?: string): Ref => ({ ...who, look: lookId, ...(moodId ? { mood: moodId } : {}), variant: VIEW });
			const neutralOf = (lookId: string) => (moodsOf(persona, lookId, vocabulary).includes(DEFAULT_MOOD) ? DEFAULT_MOOD : undefined);
			const n = name(c, plan.persona);
			const events: Draft[] = [join(hostAs(DEFAULT_MOOD))];
			const firstOfPersona = plan.look === def;

			if (firstOfPersona) {
				if (e === 0 && i === 0) events.push(say(null, LINES.prelude), say(HOST_REF, LINES.preludeHost));
				events.push(say(null, isHostChapter ? LINES.walkSelf(n) : LINES.walk(n)));
				events.push(join(as(def, neutralOf(def))));
				events.push(say(who, INTROS[`${c.id}:${plan.persona}`] ?? (plan.persona === REGULAR_PERSONA ? INTROS[c.id] : undefined) ?? LINES.intro(n)));
				events.push(say(HOST_REF, pick(LINES.reply, lineNo++)));
				met.push({ ...who, variant: 'avatar' });
			} else {
				// another look of the same persona: from the default look into this one (a transition image when there is one)
				const lk = look(persona, plan.look);
				events.push(join(as(def, neutralOf(def))));
				events.push(join(as(plan.look, neutralOf(plan.look))));
				report.lookSwitches++;
				const tl = transitionLabel(persona, def, plan.look);
				if (tl) report.transitions++;
				events.push(say(null, tl ? text((l) => `*${tl[l]}*`) : LINES.lookOn(n, lk)));
				events.push(say(who, LINES.lookShow(lk)));
				// Cat Pope tries the style
				if (hostLooks.has(plan.look)) events.push(join(hostAs(DEFAULT_MOOD, plan.look)), say(HOST_REF, LINES.styleOk(lk)));
				else {
					events.push(join(hostAs('sad')), say(HOST_REF, LINES.styleFail(lk)));
					report.styleFails++;
				}
			}

			const moods = moodsOf(persona, plan.look, vocabulary);
			if (!moods.length) events.push(say(who, LINES.noMoods));
			moods.forEach((m, k) => {
				const label = mood(m);
				events.push(join(as(plan.look, m)), say(who, pick(LINES.show, k)(label)));
				report.moods++;
				stats.moods++;
				if (hostMoods.has(m)) {
					events.push(join(hostAs(m, hostLooks.has(plan.look) && !firstOfPersona ? plan.look : hostLook)), say(HOST_REF, pick(LINES.mimic, k)(label)));
					report.mimics++;
				} else {
					events.push(join(hostAs('sad')), say(HOST_REF, pick(LINES.fail, k)(label)));
					report.fails++;
					stats.fails++;
				}
			});

			if (!firstOfPersona) {
				// back to the default look (shows e.g. the helmet → default transition image)
				events.push(join(as(def, neutralOf(def))));
				report.lookSwitches++;
				const tl = transitionLabel(persona, plan.look, def);
				if (tl) report.transitions++;
				events.push(say(null, tl ? text((l) => `*${tl[l]}*`) : LINES.lookBack(n, look(persona, def))));
			}
			const lastOfPersona = i === plans.length - 1 || plans[i + 1].persona !== plan.persona;
			if (lastOfPersona) {
				events.push(join(hostAs('happy')), say(who, LINES.bye), leave(who), say(HOST_REF, pick(LINES.onward, lineNo++)));
			}
			events.push(jump(next(i)));
			scene(number, labels[i], background, events);
		});

		if (isLast) {
			// goodbye: everyone met waves from the side, Cat Pope says bye, back to the toon page
			const events: Draft[] = [join(hostAs('happy')), join(...met), say(HOST_REF, LINES.goodbyeHost)];
			for (const m of met) events.push(say({ character: m.character, persona: m.persona }, LINES.goodbyeAll));
			events.push(say(null, LINES.end), jump(next(labels.length - 1)));
			scene(number, 'goodbye', background, events);
		}

		const title: Text = isHostChapter
			? { de: 'Katzenpapst trifft sich selbst', en: 'Cat Pope meets himself' }
			: text((l) => (l === 'de' ? `Begegnung: ${name(enc.character, REGULAR_PERSONA)[l]}` : `Meeting ${name(enc.character, REGULAR_PERSONA)[l]}`));
		const description = text((l) =>
			l === 'de'
				? `Sandkasten (VN-Testfeld): ${stats.moods} Stimmungen in ${plans.length} ${plans.length === 1 ? 'Look' : 'Looks'}, Katzenpapst macht nach.`
				: `Sandbox (VN test bed): ${stats.moods} moods in ${plans.length} ${plans.length === 1 ? 'look' : 'looks'}, Cat Pope mimics.`
		);
		chapter(number, title, description, labels);
		sequence.push({ type: 'chapter', number, status: 'draft' });
		report.encounters++;
		report.perEncounter.push(stats);
	});

	function scene(number: number, label: string, background: string, events: Draft[]) {
		const group = { label, renderer: { type: 'horizontal', params: { freeMode: true } }, slices: [{ image: background }], events };
		for (const l of LOCALES) files[`${l}/chapter${number}/${label}.json`] = localize(group, l);
		report.groups++;
	}

	function chapter(number: number, title: Text, description: Text, labels: string[]) {
		for (const l of LOCALES) {
			files[`${l}/chapter${number}.json`] = {
				title: title[l],
				author: 'SCUTOONS',
				number,
				language: l,
				release: RELEASE,
				status: 'draft',
				description: description[l],
				thumbnail: `${CATPOPE}images/s1c1-thumbnail.jpg`,
				isPreview: true,
				groups: labels.map((label) => ({ ref: { src: `chapter${number}/${label}.json` } }))
			};
		}
		report.chapters++;
	}

	const meta = {
		de: { toon: 'Sandkasten', season: 'Katzenpapsts Spaziergang', description: 'Testfeld: alle Figuren, Personas, Looks und Stimmungen als Visual Novel. Nur in der Entwicklung und im Creator sichtbar (Entwurf).' },
		en: { toon: 'Sandbox', season: "Cat Pope's walk", description: 'Test bed: every character, persona, look and mood as a visual novel. Visible in development and in the creator only (draft).' }
	};
	for (const l of LOCALES) {
		files[`${l}/toon.json`] = {
			id: SANDBOX_TOON,
			title: meta[l].toon,
			authors: [],
			chapters: report.chapters,
			language: l,
			release: RELEASE,
			status: 'draft',
			description: meta[l].description,
			thumbnail: `${CATPOPE}images/s1c1-thumbnail.jpg`,
			cover: { square: `${CATPOPE}images/1x/de/s1-cover-square.jpg` },
			map: { layers: { terrain: `${CATPOPE}images/map/map-layer-terrain.svg` }, width: 2048, height: 1046, center: { x: 1320, y: 440 } },
			download: { complete: null, music: '' }
		};
		files[`${l}/season1.json`] = { title: meta[l].season, language: l, number: 1, release: RELEASE, description: meta[l].description, sequence };
		files[`${l}/locations.json`] = [];
		files[`${l}/routes.json`] = [];
		files[`${l}/collectables.json`] = [];
	}
	// casting: the host, then everyone met, with every persona
	const cast: ToonCast = {
		seasons: { '1': [{ character: HOST, personas: personaIds(host) }, ...ordered.map((c) => ({ character: c.id, personas: personaIds(c) }))] }
	};
	files['_/cast.json'] = cast;
	return { files, report };
}

// ── CLI ─────────────────────────────────────────────────────────────────────────────────────────────────────────

async function main() {
	const root = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
	const arg = (key: string) => process.argv.find((a) => a.startsWith(`--${key}=`))?.slice(key.length + 3);
	const dry = process.argv.includes('--dry-run');
	const out = resolve(root, arg('out') ?? `viewer/static/api/${SANDBOX_TOON}`);
	const from = arg('from');
	let registry: CharacterRegistry;
	let vocabulary: MoodVocabulary;
	if (from) {
		const get = async (file: string) => {
			const res = await fetch(`${from.replace(/\/+$/, '')}/content/scutoons/_/${file}`);
			if (!res.ok) throw new Error(`${file}: HTTP ${res.status}`);
			return res.json();
		};
		[registry, vocabulary] = await Promise.all([get('characters.json'), get('moods.json')]);
	} else {
		const seed = (file: string) => JSON.parse(readFileSync(pathJoin(root, 'shared/content/seed/scutoons/_', file), 'utf8'));
		registry = seed('characters.json');
		vocabulary = seed('moods.json');
	}

	const { files, report } = buildSandbox(registry, vocabulary);
	if (!dry) {
		if (existsSync(out)) rmSync(out, { recursive: true });
		for (const [path, json] of Object.entries(files)) {
			const file = pathJoin(out, path);
			mkdirSync(dirname(file), { recursive: true });
			writeFileSync(file, JSON.stringify(json, null, '\t') + '\n');
		}
	}
	const toons = JSON.parse(readFileSync(pathJoin(root, 'viewer/static/api/toons.json'), 'utf8')) as { id: string; status: string }[];
	const entry = toons.find((t) => t.id === SANDBOX_TOON);

	console.log(`${dry ? '[dry run] ' : ''}${Object.keys(files).length} files → ${out}`);
	console.log(
		`${report.chapters} chapters (encounters), ${report.groups} VN groups, ${report.moods} moods shown, ${report.mimics} mimicked, ${report.fails} failed (sad), ` +
			`${report.lookSwitches} look switches (${report.transitions} with a transition image), ${report.styleFails} styles Cat Pope lacks`
	);
	for (const s of report.perEncounter) console.log(`  chapter ${s.chapter}  ${s.name.padEnd(28)} ${String(s.moods).padStart(3)} moods, ${s.fails} failed  [${s.groups.join(', ')}]`);
	if (!entry) console.warn(`! viewer/static/api/toons.json has no "${SANDBOX_TOON}" entry (status draft) – the creator lists toons from there`);
	else if (entry.status !== 'draft') console.warn(`! toons.json: "${SANDBOX_TOON}" must stay status draft (hidden in production)`);
	console.log(`next: php server/bin/console import-content viewer/static/api --toon=${SANDBOX_TOON} --update`);
}

if (process.argv[1]?.replace(/\\/g, '/').endsWith('make-sandbox.ts')) {
	main().catch((e) => {
		console.error(e);
		process.exit(1);
	});
}
