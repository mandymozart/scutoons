// Builds the sandbox toon "Cat Pope's walk" (dev / creator only, hidden in production) from the character registry:
// Cat Pope (regular) leaves the house and meets every character and every one of his own personas. Each introduces
// themselves, then shows every mood of every look, one by one; Cat Pope mimics each mood (or, when his regular look has
// no such mood, tries, fails and is sad). Look changes use the registry's transition images; Cat Pope tries each new look
// too (no such look → sad). One chapter per encounter, one VN group per persona × look, a goodbye at the end.
//
//   node --experimental-transform-types --no-warnings shared/content/scripts/make-sandbox.ts [--out=<dir>] [--no-import] [--dry-run]
//   (npm run dev:db does it after loading the production seed; npm run dev:db sandbox runs it alone)
//
//   reads    the newest registry + moods (`characters`, `moods`, drafts too) from the local dev database (npm run dev:db)
//   --out        output folder in the interchange layout (default logs/dev/sandbox, gitignored: toons.json with the
//                sandbox's index entry + sandbox/…; the script owns it and rewrites it completely)
//   --no-import  only write the files. By default the script then imports them into the local content store (published):
//                php server/bin/console import-content logs/dev/sandbox --toon=sandbox --update
//                Chapters, scenes and the season are language-neutral documents (`_/chapter<N>.json`, agents/NEUTRAL.md),
//                the toon settings are the index entry's (texts per language, step 4); per-locale chapters / scenes /
//                seasons / toon settings of earlier runs are removed from the database first.
// The sandbox is dev / creator only: production never has it (Library::DEV_ONLY_TOONS).
// Re-run after adding characters, personas, looks, moods or transitions. See agents/MEMORY.md "Sandbox", agents/DATA.md.
import { spawnSync } from 'node:child_process';
// @ts-expect-error – a plain .mjs module of the dev scripts (no types)
import { localSql, phpDbEnv, phpMysqlArgs } from '../../../scripts/dev/db-env.mjs';
import { existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join as pathJoin, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
	CHARACTERS_COLLECTION,
	collectionDocsFromEntities,
	composeCharacterRegistry,
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
/**
 * A partial translation (owner 2026-10-02, a third language to test the creator's text switch with): the toon, season
 * and chapter titles and descriptions are translated, the dialogue is not (a translation in progress – the language-
 * neutral scenes simply have no `ko` texts: readers get the German ones, the creator warns about the missing texts).
 */
const PARTIAL = 'ko' as const;
type Partial = { title: string; description: string };
const RELEASE = '2026-09-30';
/** The sandbox's texts per language (toon, season 1); ko is the partial language */
const META = {
	de: { toon: 'Sandkasten', season: 'Katzenpapsts Spaziergang', description: 'Testfeld: alle Figuren, Personas, Looks und Stimmungen als Visual Novel. Nur in der Entwicklung und im Creator sichtbar (Entwurf).' },
	en: { toon: 'Sandbox', season: "Cat Pope's walk", description: 'Test bed: every character, persona, look and mood as a visual novel. Visible in development and in the creator only (draft).' },
	[PARTIAL]: { toon: '샌드박스', season: '고양이 교황의 산책', description: '테스트용: 모든 캐릭터, 페르소나, 룩, 기분을 비주얼 노벨로. 개발 환경과 크리에이터에서만 보입니다(초안).' }
};
const ALL_LOCALES = [...LOCALES, PARTIAL] as const;
const perLocale = <T>(fn: (l: (typeof ALL_LOCALES)[number]) => T) => Object.fromEntries(ALL_LOCALES.map((l) => [l, fn(l)]));
/** preferred encounter order (owner); characters not listed follow in registry order, then the host's own personas */
const ORDER = ['yellowmouse', 'toptoy', 'boy', 'crows'];
/** the view = the side (DESIGN.md → "VN layout"): Cat Pope stands on the left (avatar view), the others on the right */
const HOST_VIEW = 'avatar';
const VIEW = 'portrait';
/** the goodbye brings the characters met onto the stage in waves of up to 4 (the stage layout test: 1, 2, 3, 4 portraits) */
const STAGE_WAVE = 4;
/** Slice paths are relative to `<cdn>/<toon>/`, so `../catpope/…` reuses catpope's art (no new files). */
const CATPOPE = '../catpope/';
/**
 * Backgrounds (owner 2026-09-30): the chapter-2 slices as vertical scroll, in 6 scenes – encounter 1–5 → scene 1–5 (all
 * groups of an encounter share it), the goodbye → scene 6. More encounters wrap around scenes 1–5.
 */
const SCENES: [number, number][] = [[1, 4], [5, 8], [9, 11], [12, 14], [15, 19], [20, 22]];
/** the "Sonntagnachmittag" street: the alley with the lanterns and the cobblestones (scene 5) */
const STREET: [number, number] = [15, 19];
/**
 * The endless band (FEATURES 10.100 §8) before the street: chapter 2's pinned-strip art (slices 109–114, 390 × 693, one
 * picture side by side) as a horizontal group with `endless`, so the dev sandbox shows the band
 */
const BAND: [number, number] = [109, 114];
const slicePath = (n: number) => `${CATPOPE}s1/c2/s1c2-slice-${n}.webp`;
/**
 * Cucumbers in the art (slice number → the click area, % of the 390 × 625 slice, a tight box around it); the first group
 * of the encounter that shows the slice offers it (a slice click trigger, once).
 */
const CUCUMBERS: Record<number, { x: number; y: number; width: number; height: number }> = {
	4: { x: 7.69, y: 37.76, width: 22.56, height: 10.08 },
	8: { x: 78.46, y: 7.52, width: 8.72, height: 8.64 },
	19: { x: 52.05, y: 15.68, width: 45.38, height: 16 }
};
/**
 * A tube of testo to pick up (ITEMS.md "Substances"; owner 2026-10-01): the same click mechanism as the cucumbers – the
 * first group showing the slice offers it once (`add testo 1` grants the item, source story, once per place). The slice
 * art has no tube drawn in: the click area sits on the stone ledge right of the figure (sandbox only).
 */
const TESTOS: Record<number, { x: number; y: number; width: number; height: number }> = {
	6: { x: 80, y: 83, width: 16, height: 9 }
};

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
	end: { de: 'Ende des Spaziergangs.', en: 'The end of the walk.' } as Text,
	cucumber: { de: 'Eine Gurke!', en: 'A cucumber!' } as Text,
	testo: { de: 'Eine Tube Testo!', en: 'A tube of testo!' } as Text,
	collect: { de: 'Einsammeln', en: 'Collect' } as Text,
	leave: { de: 'Liegen lassen', en: 'Leave it' } as Text
};

// ── Events ──────────────────────────────────────────────────────────────────────────────────────────────────────

/**
 * An event; its texts (`Text`) are the language-neutral document's locale maps (agents/NEUTRAL.md) – showMessage
 * `params.text`, addChoice `params.label`.
 */
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
/**
 * A cucumber to collect: a click on it (once per page view) offers it – message + "Collect" (`give cucumber 1` – BACKLOG 10.38; it puts the
 * item into the inventory, with its "+1" notice) / "Leave it". Slice events, all with the same trigger.
 */
const cucumberOffer = (rect: { x: number; y: number; width: number; height: number }, item: 'cucumber' | 'testo' = 'cucumber'): Draft[] => {
	const trigger = { type: 'click', rect, repeat: { mode: 'view' } };
	return [
		{ type: 'showMessage', params: { text: LINES[item] }, trigger },
		{ type: 'addChoice', params: { label: LINES.collect, onSelect: { give: { item, qty: 1 } } }, trigger },
		{ type: 'addChoice', params: { label: LINES.leave }, trigger },
		{ type: 'showChoices', trigger }
	];
};

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
	/** where the cucumbers are offered */
	cucumbers: string[];
	/** where the testo is offered */
	testos: string[];
	/** "Sonntagnachmittag": characters in the scene at once, of them on the portrait stage, look transitions */
	sundayCast: number;
	sundayStage: number;
	/** who stands in the full view for a while */
	sundayFull: string[];
	sundayTransitions: number;
	perEncounter: { chapter: number; name: string; groups: string[]; moods: number; fails: number }[];
}
export interface SandboxFiles {
	/** path below the toon folder (`_/chapter1.json`, `_/chapter1/<scene>.json`, `de/toon.json`, `_/cast.json`, …) → JSON */
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
	const hostAs = (mood: string, look = hostLook): Ref => ({ ...HOST_REF, look, mood, variant: HOST_VIEW });

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

	const report: SandboxReport = { chapters: 0, groups: 0, encounters: 0, moods: 0, mimics: 0, fails: 0, lookSwitches: 0, transitions: 0, styleFails: 0, cucumbers: [], testos: [], sundayCast: 0, sundayStage: 0, sundayFull: [], sundayTransitions: 0, perEncounter: [] };
	const files: Record<string, unknown> = {};
	/** the season's sequence: every chapter a draft in every language (the status is a locale map) */
	const sequence: { number: number; status: Record<string, 'draft'> }[] = [];
	const draftEverywhere = () => Object.fromEntries([...LOCALES, PARTIAL].map((l) => [l, 'draft' as const]));
	const met: Ref[] = [];
	/** slices whose cucumber is already offered (a scene shared by several groups offers it in the first one) */
	const cucumbersPlaced = new Set<number>();
	const testosPlaced = new Set<number>();
	let lineNo = 0;

	encounters.forEach((enc, e) => {
		const number = e + 1;
		const isHostChapter = enc.character.id === HOST;
		const plans: GroupPlan[] = enc.personas.flatMap((persona) => looksOf(findPersona(enc.character, persona)).map((look) => ({ persona, look })));
		const labels = plans.map((p) => `${p.persona}-${p.look}`);
		const isLast = e === encounters.length - 1;
		if (isLast) labels.push('goodbye');
		const stats = { chapter: number, name: characterName(enc.character, 'de', isHostChapter ? undefined : enc.personas[0]), groups: labels, moods: 0, fails: 0 };
		const background = SCENES[e % (SCENES.length - 1)];
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
				met.push({ ...who, variant: VIEW });
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
			// goodbye: Cat Pope says bye, everyone met comes back onto the stage (waves of up to 4, one by one) and says bye,
			// back to the toon page
			const events: Draft[] = [join(hostAs('happy')), say(HOST_REF, LINES.goodbyeHost)];
			for (let w = 0; w < met.length; w += STAGE_WAVE) {
				const wave = met.slice(w, w + STAGE_WAVE);
				for (const m of wave) events.push(join(m), say({ character: m.character, persona: m.persona }, LINES.goodbyeAll));
				events.push(leave(...wave));
			}
			events.push(say(null, LINES.end), jump(next(labels.length - 1)));
			scene(number, 'goodbye', SCENES[SCENES.length - 1], events);
		}

		const who = name(enc.character, REGULAR_PERSONA).en;
		const ko: Partial = {
			title: isHostChapter ? '고양이 교황, 자기 자신을 만나다' : `만남: ${who}`,
			description: `샌드박스(VN 테스트): 룩 ${plans.length}개에서 기분 ${stats.moods}가지, 고양이 교황이 따라 합니다.`
		};
		const title: Text = isHostChapter
			? { de: 'Katzenpapst trifft sich selbst', en: 'Cat Pope meets himself' }
			: text((l) => (l === 'de' ? `Begegnung: ${name(enc.character, REGULAR_PERSONA)[l]}` : `Meeting ${name(enc.character, REGULAR_PERSONA)[l]}`));
		const description = text((l) =>
			l === 'de'
				? `Sandkasten (VN-Testfeld): ${stats.moods} Stimmungen in ${plans.length} ${plans.length === 1 ? 'Look' : 'Looks'}, Katzenpapst macht nach.`
				: `Sandbox (VN test bed): ${stats.moods} moods in ${plans.length} ${plans.length === 1 ? 'look' : 'looks'}, Cat Pope mimics.`
		);
		chapter(number, title, description, labels, ko);
		sequence.push({ number, status: draftEverywhere() });
		report.encounters++;
		report.perEncounter.push(stats);
	});

	sunday(encounters.length + 1);

	/**
	 * "Sonntagnachmittag" – the layout test bed (owner 2026-09-30): everyone of the registry meets on a street. Cat Pope
	 * and his personas stand on the left (avatar view), everyone else stands on the stage (right). Exercises: joining one
	 * by one until all are there, frequent speaker changes (glow, front; the order follows who spoke last),
	 * leaving + coming back, mood changes, every registered look transition, everyone leaving.
	 */
	function sunday(number: number) {
		type Who = { character: string; persona: string; view: string; name: Text };
		const cast: Who[] = [
			{ ...HOST_REF, view: HOST_VIEW, name: name(host, REGULAR_PERSONA) },
			...ordered.flatMap((c) => personaIds(c).map((persona) => ({ character: c.id, persona, view: VIEW, name: name(c, persona) }))),
			...personaIds(host).filter((p) => p !== REGULAR_PERSONA).map((persona) => ({ character: HOST, persona, view: HOST_VIEW, name: name(host, persona) }))
		];
		const personaOf = (w: Who) => findPersona(characters.find((c) => c.id === w.character), w.persona);
		const state = new Map<Who, { look: string; mood?: string }>();
		const neutralOf = (w: Who, lookId: string) => (moodsOf(personaOf(w), lookId, vocabulary).includes(DEFAULT_MOOD) ? DEFAULT_MOOD : undefined);
		/** the view each one currently stands in (the Who's slot, or `full` for a while) */
		const viewOf = new Map<Who, string>();
		const ref = (w: Who, change: { look?: string; mood?: string; view?: string } = {}): Ref => {
			const current = state.get(w) ?? { look: defaultLookId(personaOf(w)), mood: undefined };
			const lookId = change.look ?? current.look;
			const moodId = 'mood' in change ? change.mood : (current.mood ?? neutralOf(w, lookId));
			state.set(w, { look: lookId, mood: moodId });
			const view = change.view ?? viewOf.get(w) ?? w.view;
			viewOf.set(w, view);
			return { character: w.character, persona: w.persona, look: lookId, ...(moodId ? { mood: moodId } : {}), variant: view };
		};
		/**
		 * a delivered full-body image of the current look (neutral, in the convention folders `characters/…`; the legacy
		 * catpope `*_full.svg` entries are placeholder silhouettes), i.e. the character can stand in the `full` view
		 */
		const hasFull = (w: Who) => {
			const persona = personaOf(w);
			const lookId = state.get(w)?.look ?? defaultLookId(persona);
			const images = persona?.looks.find((l) => l.id === lookId)?.images as Record<string, Record<string, string>> | undefined;
			return !!images?.[DEFAULT_MOOD]?.full?.startsWith('characters/');
		};
		const who = (w: Who) => ({ character: w.character, persona: w.persona });
		const L = (de: string, en: string): Text => ({ de, en });
		const hello = [L('Schönen Sonntag!', 'Happy Sunday!'), L('Die Sonne scheint!', 'The sun is out!'), L('Da bin ich!', 'Here I am!'), L('Hallo, alle zusammen!', 'Hi, everyone!'), L('Eis, jemand?', 'Ice cream, anyone?')];
		const chatter = [L('Herrlich!', 'Lovely!'), L('Kaffee?', 'Coffee?'), L('Kuchen!', 'Cake!'), L('Ich liebe Sonntage.', 'I love Sundays.'), L('Was für ein Wetter!', 'What weather!')];
		const moodLines = [L('Hihi!', 'Hehe!'), L('Oh!', 'Oh!'), L('Hm …', 'Hm …'), L('Na so was!', 'Well, well!')];
		const MOOD_PICKS = ['happy', 'laughing', 'surprised', 'smug', 'thinking', 'confused', 'angry'];

		const stage = cast.slice(1).filter((w) => w.view === VIEW);
		const selves = cast.filter((w) => w.character === HOST && w.persona !== REGULAR_PERSONA);
		const events: Draft[] = [say(null, L('Ein sonniger Sonntagnachmittag. Alle sind auf der Straße.', 'A sunny Sunday afternoon. Everyone is out on the street.'))];
		events.push(join(ref(cast[0], { mood: 'happy' })), say(HOST_REF, L('Was für ein Sonntag!', 'What a Sunday!')));
		// one by one onto the stage, each says hello (the newcomer is the speaker: in front, glowing)
		stage.forEach((w, i) => events.push(join(ref(w)), say(who(w), pick(hello, i))));
		// Cat Pope's personas join him on the left
		for (const w of selves) events.push(join(ref(w)), say(who(w), { de: `Ich bin auch da – als ${w.name.de}!`, en: `Me too – as ${w.name.en}!` }));
		events.push(say(null, L('Jetzt sind alle da.', 'Now everyone is here.')));
		// the speaker changes often: everyone on the stage says something, back to front
		[...stage].reverse().forEach((w, i) => events.push(say(who(w), pick(chatter, i))));
		if (stage.length > 1) {
			// turns: three speak in a row → the last one in front, then the one before, then the first (who just stopped
			// speaking stands right behind the speaker)
			const turns = [L('Ich zuerst!', 'Me first!'), L('Nein, ich!', 'No, me!'), L('Ruhe jetzt!', 'Quiet now!')];
			events.push(say(null, L('Alle reden durcheinander.', 'Everyone talks at once.')));
			stage.slice(0, 3).forEach((w, i) => events.push(say(who(w), turns[i])));
			// someone leaves and comes back
			const away = stage[stage.length - 1];
			events.push(say(who(away), L('Ich hol mal Eis.', 'Getting some ice cream.')), leave(away), say(null, { de: `${away.name.de} ist kurz weg.`, en: `${away.name.en} is gone for a moment.` }));
			const awayMoods = moodsOf(personaOf(away), state.get(away)!.look, vocabulary);
			events.push(join(ref(away, awayMoods.includes('happy') ? { mood: 'happy' } : {})), say(who(away), L('Eis für alle!', 'Ice cream for everyone!')));
		}
		// full view (owner 2026-09-30): everyone with a full-body image steps forward in `full` for a while – next to the
		// avatar group and the portrait stage, with speaker changes between all three (in front of / behind the box,
		// recency order), then back to their slot
		const fulls = cast.filter(hasFull);
		if (fulls.length) {
			events.push(say(null, L('Ganzkörper-Auftritt!', 'Full-body entrance!')));
			fulls.forEach((w, i) => events.push(join(ref(w, { view: 'full' })), say(who(w), pick(hello, i + 1))));
			const others = [cast[0], ...stage.filter((w) => !fulls.includes(w))].slice(0, 3);
			const round = [...fulls, ...others];
			round.forEach((w, i) => events.push(say(who(w), pick(chatter, i + 2))));
			// the full ones again, last one first (recency among them)
			[...fulls].reverse().forEach((w, i) => events.push(say(who(w), pick(moodLines, i))));
			for (const w of fulls) events.push(join(ref(w, { view: w.view })), say(who(w), L('Zurück auf meinen Platz.', 'Back to my spot.')));
			report.sundayFull = fulls.map((w) => `${w.character}·${w.persona}`);
		}
		// mood changes: two moods each (what the look has), Cat Pope too
		for (const w of [...stage, cast[0]]) {
			const moods = moodsOf(personaOf(w), state.get(w)!.look, vocabulary);
			MOOD_PICKS.filter((m) => moods.includes(m)).slice(0, 2).forEach((m, k) => events.push(join(ref(w, { mood: m })), say(who(w), pick(moodLines, k))));
		}
		// every registered look transition (e.g. helmet → default): first into the `from` look, then the switch
		for (const w of cast) {
			const persona = personaOf(w);
			for (const t of persona?.transitions ?? []) {
				if (!looksOf(persona).includes(t.from) || !looksOf(persona).includes(t.to)) continue;
				const lookName = (id: string) => look(persona, id);
				if (state.get(w)?.look !== t.from) events.push(join(ref(w, { look: t.from, mood: neutralOf(w, t.from) })), say(who(w), { de: `${cap(lookName(t.from).de)}!`, en: `${cap(lookName(t.from).en)}!` }));
				events.push(join(ref(w, { look: t.to, mood: neutralOf(w, t.to) })), say(who(w), { de: `Und jetzt: ${lookName(t.to).de}.`, en: `And now: ${lookName(t.to).en}.` }));
				report.sundayTransitions++;
			}
		}
		// everyone leaves, the last one in first; Cat Pope last
		for (const w of [...cast.slice(1)].reverse()) events.push(say(who(w), L('Tschüss!', 'Bye!')), leave(w));
		events.push(say(HOST_REF, L('Bis nächsten Sonntag!', 'See you next Sunday!')), leave(cast[0]), say(null, L('Die Sonne geht unter.', 'The sun goes down.')), jump({ type: 'toon', value: SANDBOX_TOON }));
		report.sundayCast = cast.length;
		report.sundayStage = stage.length;

		band(number, L('Zieh das Band zur Seite – es hört nie auf.', 'Drag the band sideways – it never ends.'));
		scene(number, 'sunday', STREET, events);
		chapter(number, L('Sonntagnachmittag', 'Sunday afternoon'), L('Sandkasten (Layout-Test): alle Figuren auf einer Straße – Plätze, Reihenfolge, Stimmungen, Looks.', 'Sandbox (layout test): every character on one street – slots, order, moods, looks.'), ['band', 'sunday'], {
			title: '일요일 오후',
			description: '샌드박스(레이아웃 테스트): 모든 캐릭터가 한 거리에 – 자리, 순서, 기분, 룩.'
		});
		sequence.push({ number, status: draftEverywhere() });
	}

	function scene(number: number, label: string, [from, to]: [number, number], events: Draft[]) {
		const slices = [];
		for (let n = from; n <= to; n++) {
			const cucumber = CUCUMBERS[n] && !cucumbersPlaced.has(n) ? CUCUMBERS[n] : null;
			if (cucumber) {
				cucumbersPlaced.add(n);
				report.cucumbers.push(`chapter ${number} · ${label}: slice ${n}`);
			}
			const testo = TESTOS[n] && !testosPlaced.has(n) ? TESTOS[n] : null;
			if (testo) {
				testosPlaced.add(n);
				report.testos.push(`chapter ${number} · ${label}: slice ${n}`);
			}
			const events = [...(cucumber ? cucumberOffer(cucumber) : []), ...(testo ? cucumberOffer(testo, 'testo') : [])];
			// every event has its hidden id (EXPRESSIONS.md §5: the reader's save game), unique in the chapter
			const withIds = events.map((e, k) => ({ id: `sb${number}-${label}-s${n}-${k + 1}`, ...e }));
			slices.push({ image: slicePath(n), width: 390, height: 625, ...(withIds.length ? { events: withIds } : {}) });
		}
		// one language-neutral scene (scene:sandbox:<n>:<label>): the texts are locale maps, de + en (ko not translated)
		const script = events.map((e, k) => ({ id: `sb${number}-${label}-${k + 1}`, ...e }));
		files[`_/chapter${number}/${label}.json`] = { label, renderer: { type: 'vertical' }, slices, events: script };
		report.groups++;
	}

	/** the endless band (scene:sandbox:<n>:band): the BAND slices, one box on the first; it runs on into the next group */
	function band(number: number, hint: Text) {
		const slices = Array.from({ length: BAND[1] - BAND[0] + 1 }, (_, k) => ({
			image: slicePath(BAND[0] + k),
			width: 390,
			height: 693,
			...(k === 0 ? { dialog: [{ id: `sb${number}-band-1`, rect: { x: 6, y: 6, width: 70 }, text: hint }] } : {})
		}));
		files[`_/chapter${number}/band.json`] = { label: 'band', renderer: { type: 'horizontal', params: { endless: true } }, continues: true, slices };
	}

	/** one language-neutral chapter (chapter:sandbox:<n>): title / description in de, en, ko; no status (the season's) */
	function chapter(number: number, title: Text, description: Text, labels: string[], partial: Partial) {
		files[`_/chapter${number}.json`] = {
			title: { ...title, [PARTIAL]: partial.title },
			author: 'SCUTOONS',
			number,
			release: Object.fromEntries([...LOCALES, PARTIAL].map((l) => [l, RELEASE])),
			description: { ...description, [PARTIAL]: partial.description },
			thumbnail: `${CATPOPE}s1/c1/s1c1-thumbnail.jpg`,
			isPreview: true,
			groups: labels.map((label) => ({ ref: { src: `chapter${number}/${label}.json` } }))
		};
		report.chapters++;
	}

	const meta = META;
	// the season: one language-neutral document (season:sandbox:1) with its cover (owner 2026-10-02: covers are per season)
	files['_/season1.json'] = {
		title: perLocale((l) => meta[l].season),
		number: 1,
		release: perLocale(() => RELEASE),
		description: perLocale((l) => meta[l].description),
		cover: { square: `${CATPOPE}s1/s1-cover-square.webp` },
		sequence
	};
	for (const l of ALL_LOCALES) {
		// the toon settings (texts per language + the language-neutral ones) live in the toons.json entry (`sandboxIndex`)
		files[`${l}/locations.json`] = [];
		files[`${l}/routes.json`] = [];
	}
	// casting: the host, then everyone met, with every persona
	const cast: ToonCast = {
		seasons: { '1': [{ character: HOST, personas: personaIds(host) }, ...ordered.map((c) => ({ character: c.id, personas: personaIds(c) }))] }
	};
	files['_/cast.json'] = cast;
	return { files, report };
}

/** The sandbox's language-neutral toon settings (its index entry: sandboxIndex) */
export function neutralSettings(report: SandboxReport) {
	return {
		authors: [],
		chapters: report.chapters,
		thumbnail: `${CATPOPE}s1/c1/s1c1-thumbnail.jpg`,
		map: { layers: { terrain: `${CATPOPE}map/map-layer-terrain.svg` }, width: 2048, height: 1046, center: { x: 1320, y: 440 } }
	};
}

// ── CLI ─────────────────────────────────────────────────────────────────────────────────────────────────────────

/**
 * The sandbox's toon index entry (`toon:sandbox`, language-neutral): status draft = hidden from readers; the toon
 * settings in it – title (the toon's one name) / description / release / status per language (agents/NEUTRAL.md
 * step 4, decision 4), the rest once
 */
export function sandboxIndex(report: SandboxReport) {
	const { map, ...settings } = neutralSettings(report);
	return {
		id: SANDBOX_TOON,
		uri: SANDBOX_TOON,
		status: 'draft',
		avatar: `${CATPOPE}avatar.webp`,
		sourceLanguage: 'de',
		languages: ['de', 'en', 'ko'],
		title: perLocale((l) => META[l].toon),
		description: perLocale((l) => META[l].description),
		release: perLocale(() => RELEASE),
		languageStatus: perLocale(() => 'draft'),
		...settings,
		map: { center: map.center }
	};
}

async function main() {
	const root = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
	const arg = (key: string) => process.argv.find((a) => a.startsWith(`--${key}=`))?.slice(key.length + 3);
	const dry = process.argv.includes('--dry-run');
	const outRoot = resolve(root, arg('out') ?? 'logs/dev/sandbox');
	const out = pathJoin(outRoot, SANDBOX_TOON);
	// the newest version (drafts too) of the platform-wide registry + mood vocabulary in the local database
	const newest = (key: string) => {
		const [rows] = localSql(['SELECT json FROM entity_versions WHERE entity_key = :key ORDER BY id DESC LIMIT 1'], { key });
		if (!rows.length) throw new Error(`no "${key}" in the local database – npm run dev:db loads production's content first`);
		return JSON.parse(rows[0].json);
	};
	// the registry is many documents (agents/MAP-DOCUMENTS.md): `characters` + one `character:<id>` each, in creation order
	const [entries] = localSql([
		"SELECT v.entity_key AS `key`, v.json FROM entity_versions v JOIN (SELECT entity_key, MAX(id) AS id, MIN(id) AS first FROM entity_versions WHERE kind IN ('characters', 'character') GROUP BY entity_key) m ON m.id = v.id ORDER BY m.first"
	]);
	if (!entries.length) throw new Error('no character registry in the local database – npm run dev:db loads the content of production first');
	const registry: CharacterRegistry = composeCharacterRegistry(collectionDocsFromEntities(CHARACTERS_COLLECTION, entries.map((e: { key: string; json: string }) => ({ key: e.key, content: JSON.parse(e.json) }))));
	const vocabulary: MoodVocabulary = newest('moods');

	const { files, report } = buildSandbox(registry, vocabulary);
	if (!dry) {
		if (existsSync(outRoot)) rmSync(outRoot, { recursive: true });
		for (const [path, json] of Object.entries(files)) {
			const file = pathJoin(out, path);
			mkdirSync(dirname(file), { recursive: true });
			writeFileSync(file, JSON.stringify(json, null, '\t') + '\n');
		}
		writeFileSync(pathJoin(outRoot, 'toons.json'), JSON.stringify([sandboxIndex(report)], null, '\t') + '\n');
	}

	console.log(`${dry ? '[dry run] ' : ''}${Object.keys(files).length} files → ${out}`);
	console.log(
		`${report.chapters} chapters (encounters), ${report.groups} VN groups, ${report.moods} moods shown, ${report.mimics} mimicked, ${report.fails} failed (sad), ` +
			`${report.lookSwitches} look switches (${report.transitions} with a transition image), ${report.styleFails} styles Cat Pope lacks`
	);
	console.log(`cucumbers: ${report.cucumbers.join(', ') || 'none'}`);
	console.log(`testo: ${report.testos.join(', ') || 'none'}`);
	console.log(`Sonntagnachmittag (chapter ${report.chapters}): ${report.sundayCast} characters at once, ${report.sundayStage} on the portrait stage, full view: ${report.sundayFull.join(', ') || 'nobody'}, ${report.sundayTransitions} look transitions`);
	for (const s of report.perEncounter) console.log(`  chapter ${s.chapter}  ${s.name.padEnd(28)} ${String(s.moods).padStart(3)} moods, ${s.fails} failed  [${s.groups.join(', ')}]`);
	const importArgs = ['bin/console', 'import-content', outRoot, `--toon=${SANDBOX_TOON}`, '--update'];
	if (dry || process.argv.includes('--no-import')) {
		console.log(`next: npm run dev:db -- console ${importArgs.slice(1).join(' ')}`);
		return;
	}
	// the per-locale chapters / scenes / season / toon settings of runs before the language-neutral documents
	// (agents/NEUTRAL.md phase 2B, step 4): the sandbox is generated, nothing of it is edited by hand per locale – they
	// go, so no stale per-locale version sits behind the neutral ones (the Convert page, content:neutral-check, the history)
	const [, dropped] = localSql(
		[
			"DELETE FROM asset_usages WHERE source = 'content' AND ref IN (SELECT entity_key FROM entity_versions WHERE toon = :toon AND locale IS NOT NULL AND kind IN ('chapter', 'scene', 'season', 'toon-settings'))",
			"DELETE FROM entity_versions WHERE toon = :toon AND locale IS NOT NULL AND kind IN ('chapter', 'scene', 'season', 'toon-settings')"
		],
		{ toon: SANDBOX_TOON }
	);
	if (dropped) console.log(`removed ${dropped} per-locale chapter / scene / season / toon settings versions of earlier sandbox runs`);
	// into the content store through the import (the interchange format → entity keys), like every other content
	const result = spawnSync(process.env.PHP_BINARY || 'php', [...phpMysqlArgs(), ...importArgs], { cwd: pathJoin(root, 'server'), env: { ...process.env, ...phpDbEnv() }, encoding: 'utf8' });
	process.stdout.write((result.stdout || '').split('\n').slice(-3).join('\n') + '\n');
	if (result.status !== 0) {
		console.error(result.stderr || result.error?.message || 'import-content failed');
		process.exit(1);
	}
}

if (process.argv[1]?.replace(/\\/g, '/').endsWith('make-sandbox.ts')) {
	main().catch((e) => {
		console.error(e);
		process.exit(1);
	});
}
