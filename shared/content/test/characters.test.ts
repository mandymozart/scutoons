// Unit tests of the character image fallback rule (resolveExpression, owner 2026-09-29), the resolved cast and the
// validation.   npm test -w @scutoons/content   (node --test, TypeScript run directly by Node 22)
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	castIndex,
	characterImage,
	expressionIndex,
	legacyCharacterImage,
	legacyCharacterLabel,
	legacyTransition,
	isMissingImagePath,
	missingImagePath,
	resolveCast,
	validateCast,
	validateCharacters,
	validateFile,
	type Character,
	type CharacterLook,
	type CharacterRegistry,
	type RegistryCharacter,
	type ToonCast
} from '../src/index.ts';

/** convention path of the test fixtures */
const png = (name: string, look: string, view: string, mood: string) => `characters/${name}/${name}${look === 'default' ? '' : `_${look}`}_${view}-${mood}.png`;
/** a look with portrait (+ derived avatar unless `noAvatar`) images for the moods */
const look = (name: string, id: string, moods: string[], extra: Partial<CharacterLook> = {}, noAvatar: string[] = []): CharacterLook => ({
	id,
	...extra,
	images: Object.fromEntries(
		moods.map((m) => [m, { portrait: png(name, id, 'portrait', m), ...(noAvatar.includes(m) ? {} : { avatar: png(name, id, 'avatar', m) }) }])
	)
});

const catpope: RegistryCharacter = {
	id: 'catpope',
	name: { de: 'Katzenpapst', en: 'Cat Pope' },
	color: 'blue',
	image: 'catpope/images/characters/catpope/catpope.svg',
	personas: [
		{ id: 'regular', looks: [look('catpope', 'default', ['neutral', 'happy', 'sad'], { default: true })] },
		{ id: 'human', label: { de: 'Mensch', en: 'Human' }, looks: [{ id: 'default', default: true, images: {} }] },
		{
			id: 'biker',
			label: { de: 'Biker', en: 'Biker' },
			looks: [
				look('catpope-biker', 'default', ['neutral', 'happy'], { default: true, label: { en: 'without helmet' } }),
				look('catpope-biker', 'helmet', ['neutral', 'happy', 'dizzy'], { label: { en: 'with helmet' } })
			]
		}
	]
};
const mouse: RegistryCharacter = {
	id: 'yellowmouse',
	name: { de: 'Mausboss', en: 'Yellow Mouse' },
	color: 'yellow',
	image: 'yellowmouse/avatar.webp',
	personas: [
		{
			id: 'regular',
			looks: [
				look('yellowmouse', 'default', ['neutral', 'happy', 'tripping'], { default: true }),
				look('yellowmouse', 'helmet', ['neutral', 'happy'], {}, ['happy'])
			],
			transitions: [{ from: 'helmet', to: 'default', image: 'characters/yellowmouse/yellowmouse_transition-helmet-to-default.png' }]
		}
	]
};
const empty: RegistryCharacter = {
	id: 'toptoy',
	name: { en: 'Toptoy' },
	color: 'black',
	image: 'toptoy/avatar.webp',
	personas: [{ id: 'regular', looks: [{ id: 'default', default: true, images: {} }] }]
};
const B = (look: string, view: string, mood: string) => png('catpope-biker', look, view, mood);

test('1. persona + look + mood', () => {
	assert.equal(characterImage(catpope, { persona: 'biker', look: 'helmet', mood: 'dizzy' }, 'portrait'), B('helmet', 'portrait', 'dizzy'));
});

test('2. persona + look + neutral (a missing mood never changes the look)', () => {
	// biker helmet + tripping → helmet neutral
	assert.equal(characterImage(catpope, { persona: 'biker', look: 'helmet', mood: 'tripping' }, 'portrait'), B('helmet', 'portrait', 'neutral'));
	// mouse helmet + tripping → helmet neutral, NOT default tripping
	assert.equal(characterImage(mouse, { look: 'helmet', mood: 'tripping' }, 'portrait'), png('yellowmouse', 'helmet', 'portrait', 'neutral'));
});

test('3. persona + default look + mood', () => {
	// an unknown look `leather` + happy → the persona's default look, happy
	assert.equal(characterImage(catpope, { persona: 'biker', look: 'leather', mood: 'happy' }, 'portrait'), B('default', 'portrait', 'happy'));
	assert.equal(characterImage(mouse, { look: 'leather', mood: 'tripping' }, 'portrait'), png('yellowmouse', 'default', 'portrait', 'tripping'));
});

test('4. persona + default look + neutral', () => {
	assert.equal(characterImage(catpope, { persona: 'biker', look: 'leather', mood: 'sad' }, 'portrait'), B('default', 'portrait', 'neutral'));
});

test('5. regular persona + default look + mood', () => {
	// the human persona has no images yet
	assert.equal(characterImage(catpope, { persona: 'human', mood: 'sad' }, 'portrait'), png('catpope', 'default', 'portrait', 'sad'));
	// an unknown persona behaves the same
	assert.equal(characterImage(catpope, { persona: 'ghost', mood: 'happy' }, 'portrait'), png('catpope', 'default', 'portrait', 'happy'));
});

test('6. regular persona + default look + neutral', () => {
	assert.equal(characterImage(catpope, { persona: 'human', mood: 'dizzy' }, 'portrait'), png('catpope', 'default', 'portrait', 'neutral'));
});

test('7. the legacy image', () => {
	assert.equal(characterImage(empty, { mood: 'happy' }, 'portrait'), 'toptoy/avatar.webp');
	assert.equal(characterImage({ ...empty, image: undefined }, { mood: 'happy' }, 'portrait', { substitute: '' }), '');
});

test('8. the missing-image substitute (after the legacy image, or when the resolved file is known to be missing)', () => {
	assert.equal(characterImage({ ...empty, image: undefined }, { mood: 'happy' }, 'portrait'), 'characters/_missing/missing_portrait.png');
	assert.equal(characterImage({ ...empty, image: undefined }, {}, 'avatar'), 'characters/_missing/missing_avatar.png');
	// a known-missing file (e.g. crows_portrait.svg) → the substitute of the requested view
	const missing = new Set([png('catpope', 'default', 'portrait', 'happy')]);
	assert.equal(characterImage(catpope, { mood: 'happy' }, 'portrait', { isMissing: (p) => missing.has(p) }), 'characters/_missing/missing_portrait.png');
	assert.equal(characterImage(catpope, { mood: 'happy' }, 'full', { isMissing: (p) => missing.has(p) }), 'characters/_missing/missing_full.png');
	assert.equal(missingImagePath('apose'), 'characters/_missing/missing_apose.png');
	assert.equal(missingImagePath('whatever'), 'characters/_missing/missing_portrait.png');
	assert.ok(isMissingImagePath('/characters/_missing/missing_full.png'));
	// the legacy (characters.json) form: from the CDN root with a leading /
	assert.equal(legacyCharacterImage({ name: 'x', label: 'X', image: '', color: '' } as Character, {}, 'full'), '/characters/_missing/missing_full.png');
});

test('views: a missing view shows the same mood’s portrait before any mood step', () => {
	// the helmet look has no avatar for happy (derived images not generated yet) → helmet happy portrait, not avatar neutral
	assert.equal(characterImage(mouse, { look: 'helmet', mood: 'happy' }, 'avatar'), png('yellowmouse', 'helmet', 'portrait', 'happy'));
	// full is never authored → portrait
	assert.equal(characterImage(catpope, { persona: 'biker', look: 'helmet', mood: 'dizzy' }, 'full'), B('helmet', 'portrait', 'dizzy'));
	// the derived avatar is used when present
	assert.equal(characterImage(mouse, { mood: 'happy' }, 'avatar'), png('yellowmouse', 'default', 'avatar', 'happy'));
});

test('the resolved cast (viewer shape) follows the same rule', () => {
	const registry: CharacterRegistry = { characters: [catpope, mouse] };
	const cast: ToonCast = { seasons: { '1': [{ character: 'catpope', personas: ['regular', 'biker'] }, { character: 'yellowmouse', personas: ['regular'] }] } };
	const [pope, m] = resolveCast(registry, cast, 'yellowmouse', 'de');
	assert.equal(pope.label, 'Katzenpapst');
	assert.equal(legacyCharacterLabel(pope, 'biker'), 'Katzenpapst · Biker');
	assert.deepEqual(Object.keys(pope.personas ?? {}), ['regular', 'human', 'biker']);
	// the regular default look for readers without persona support; paths outside the toon folder → from the CDN root
	assert.equal(pope.images?.happy?.portrait, '/' + png('catpope', 'default', 'portrait', 'happy'));
	assert.equal(legacyCharacterImage(pope, { persona: 'biker', look: 'helmet', mood: 'tripping' }, 'portrait'), '/' + B('helmet', 'portrait', 'neutral'));
	assert.equal(legacyCharacterImage(pope, { persona: 'human', mood: 'sad' }, 'avatar'), '/' + png('catpope', 'default', 'avatar', 'sad'));
	assert.equal(legacyCharacterImage(m, { look: 'helmet', mood: 'tripping' }, 'avatar'), '/' + png('yellowmouse', 'helmet', 'avatar', 'neutral'));
	assert.equal(legacyCharacterImage(pope, { mood: 'happy' }, 'full'), '/' + png('catpope', 'default', 'portrait', 'happy'));
	assert.equal(legacyTransition(m, 'regular', 'helmet', undefined), '/characters/yellowmouse/yellowmouse_transition-helmet-to-default.png');
	assert.equal(legacyTransition(m, 'regular', 'default', 'helmet'), '');
	// only one season
	assert.equal(resolveCast(registry, { seasons: { '1': [{ character: 'yellowmouse', personas: ['regular'] }], '2': [{ character: 'catpope', personas: ['regular'] }] } }, 'x', 'en', { season: 1 }).length, 1);
});

test('an old characters.json without personas still resolves (static fallback)', () => {
	const old: Character = {
		name: 'plf',
		label: 'PLF',
		color: 'lightblue',
		image: 'images/characters/plf/plf.svg',
		images: { neutral: { avatar: 'images/characters/plf/plf_avatar-neutral.png', portrait: 'images/characters/plf/plf_portrait-neutral.png' } }
	};
	assert.equal(legacyCharacterImage(old, { mood: 'happy' }, 'avatar'), 'images/characters/plf/plf_avatar-neutral.png');
	assert.equal(legacyCharacterImage(old, { mood: 'happy' }, 'full'), 'images/characters/plf/plf_portrait-neutral.png');
	assert.equal(legacyCharacterImage(old, { persona: 'biker', look: 'helmet' }, 'portrait'), 'images/characters/plf/plf_portrait-neutral.png');
});

test('validation: default looks, neutral anchor, casts, scenes', () => {
	const broken: RegistryCharacter = {
		...mouse,
		id: 'broken',
		personas: [
			{
				id: 'regular',
				looks: [
					{ id: 'default', images: { happy: { portrait: 'characters/broken/broken_portrait-happy.png' } } },
					{ id: 'helmet', images: {} }
				]
			}
		]
	};
	const problems = validateCharacters({ characters: [catpope, broken] }, { languages: ['en'] });
	const messages = problems.map((p) => `${p.level}: ${p.message}`);
	assert.ok(messages.some((m) => m.startsWith('error') && m.includes('exactly one look must be the default')));
	assert.ok(messages.some((m) => m.startsWith('error') && m.includes('no neutral portrait')));
	assert.ok(messages.some((m) => m.startsWith('warning') && m.includes('no images yet')));
	// the standard set is info only
	assert.ok(!problems.some((p) => p.level !== 'info' && p.message.includes('standard set')));

	const castProblems = validateCast({ seasons: { '1': [{ character: 'catpope', personas: ['regular', 'pirate'] }, { character: 'ghost', personas: [] }] } }, { registry: { characters: [catpope] }, languages: ['en'] }).map((p) => p.message);
	assert.ok(castProblems.some((m) => m.includes('unknown persona "pirate"')));
	assert.ok(castProblems.some((m) => m.includes('ghost is not in the character registry')));

	const registry: CharacterRegistry = { characters: [catpope, mouse] };
	const group = {
		label: 'scene',
		renderer: { type: 'vertical' },
		events: [
			{ type: 'joinScene', params: { characters: [{ name: 'catpope', persona: 'biker', look: 'helmet', mood: 'tripping', position: { variant: 'portrait' } }] } },
			{ type: 'joinScene', params: { characters: [{ name: 'catpope', persona: 'human' }] } },
			{ type: 'joinScene', params: { characters: [{ name: 'yellowmouse', look: 'leather' }] } },
			{ type: 'joinScene', params: { characters: [{ name: 'boy' }] } }
		],
		slices: []
	};
	const scene = validateFile('group', group, 'en', {
		file: 'scene.json',
		cast: castIndex({ seasons: { '1': [{ character: 'catpope', personas: ['regular', 'biker'] }, { character: 'yellowmouse', personas: ['regular'] }] } }, 1),
		expressions: expressionIndex(registry),
		moods: ['neutral', 'happy', 'dizzy', 'tripping']
	}).map((p) => `${p.level}: ${p.message}`);
	assert.ok(scene.some((m) => m.startsWith('warning') && m.includes('has no "tripping" image – falls back to neutral')));
	assert.ok(scene.some((m) => m.includes('catpope · human is not in this season') && m.startsWith('warning')));
	assert.ok(scene.some((m) => m.includes('has no look "leather"')));
	assert.ok(scene.some((m) => m.includes('unknown character "boy"')));
	assert.ok(!scene.some((m) => m.startsWith('error')));
});

test('wanted images and asset requests', async () => {
	const { wantedImages, wantedImagesCsv, closeFulfilledRequests, defaultMoodVocabulary, validateAssetRequests } = await import('../src/index.ts');
	const registry: CharacterRegistry = { characters: [structuredClone(catpope)] };
	const wanted = wantedImages(registry, { moods: defaultMoodVocabulary() });
	// the human persona has no images: all 17 standard moods are wanted, with the convention path
	const human = wanted.filter((w) => w.persona === 'human');
	assert.equal(human.length, 17);
	assert.equal(human[0].path, 'characters/catpope-human/catpope-human_portrait-neutral.webp');
	assert.ok(wantedImagesCsv(human).startsWith('character,persona,look,mood,view,file,path,note\ncatpope,human,default,neutral,portrait,catpope-human_portrait-neutral.webp'));
	// a request for a non-standard mood is wanted too; setting the path closes it
	const requests = {
		requests: [{ id: 'r1', character: 'catpope', persona: 'biker', look: 'helmet', mood: 'grumpy', view: 'portrait', path: 'characters/catpope-biker/catpope-biker_helmet_portrait-grumpy.png', requested_at: '2026-09-30T00:00:00Z', status: 'open' as const }]
	};
	assert.ok(wantedImages(registry, { requests }).some((w) => w.mood === 'grumpy' && w.request?.id === 'r1'));
	assert.ok(validateAssetRequests(requests, registry).every((p) => p.level === 'info'));
	assert.equal(closeFulfilledRequests(requests, registry, 'now'), 0);
	registry.characters[0].personas[2].looks[1].images.grumpy = { portrait: requests.requests[0].path };
	assert.equal(closeFulfilledRequests(requests, registry, 'now'), 1);
	assert.equal(requests.requests[0].status, 'done');
});

test('A-pose: character level, its own view without fallback, avatar bodies', async () => {
	const { avatarBodies, wantedImages } = await import('../src/index.ts');
	const c: RegistryCharacter = structuredClone(catpope);
	c.apose = {
		looks: [
			{ id: 'swim', label: { en: 'swim pants' }, default: true, path: 'catpope/avatar/mannequin.webp', anchors: { head: { x: 370, y: 70, size: 170 } } },
			{ id: 'naked', label: { en: 'naked' }, path: 'catpope/avatar/mannequin-nude.webp' }
		]
	};
	const m2: RegistryCharacter = { ...structuredClone(mouse), apose: { looks: [{ id: 'swim', default: true, path: 'yellowmouse/avatar/mannequin.webp' }, { id: 'naked' }] } };
	assert.equal(characterImage(c, {}, 'apose'), 'catpope/avatar/mannequin.webp');
	assert.equal(characterImage(c, { look: 'naked' }, 'apose'), 'catpope/avatar/mannequin-nude.webp');
	// no A-pose → the A-pose substitute (never the portrait)
	assert.equal(characterImage(catpope, {}, 'apose'), 'characters/_missing/missing_apose.png');
	assert.equal(characterImage(catpope, {}, 'apose', { substitute: '' }), '');
	const bodies = avatarBodies({ characters: [c, m2] }, 'en');
	assert.deepEqual(bodies.map((b) => `${b.character}/${b.look}${b.default ? '*' : ''}: ${b.label}`), ['catpope/swim*: Cat Pope (swim pants)', 'catpope/naked: Cat Pope (naked)', 'yellowmouse/swim*: Yellow Mouse (swim)']);
	// the missing naked A-pose of the mouse is a wanted image
	assert.ok(wantedImages({ characters: [m2] }).some((w) => w.view === 'apose' && w.look === 'naked' && w.path === 'characters/yellowmouse/yellowmouse_apose-naked.webp'));
	assert.ok(!validateCharacters({ characters: [c] }, { languages: ['en'] }).some((p) => p.level === 'error'));
});

test('legacy ids and the new ref form', async () => {
	const { normalizeSceneRef, resolveCharacterId } = await import('../src/index.ts');
	assert.deepEqual(resolveCharacterId('plf'), { character: 'catpope', persona: 'human' });
	assert.deepEqual(resolveCharacterId('mountain-tiger'), { character: 'catpope', persona: 'mountaintiger' });
	assert.deepEqual(resolveCharacterId('boy'), { character: 'boy', persona: 'regular' });
	assert.deepEqual(normalizeSceneRef({ name: 'cat', position: { variant: 'portrait' }, mood: 'happy' }), { character: 'catpope', persona: 'cat', mood: 'happy', variant: 'portrait' });
	assert.deepEqual(normalizeSceneRef({ character: 'catpope', persona: 'biker', look: 'helmet', variant: 'avatar' }), { character: 'catpope', persona: 'biker', look: 'helmet', variant: 'avatar' });
	assert.equal(normalizeSceneRef({}), null);
	// scene validation accepts both forms; an old id is info only
	const registry: CharacterRegistry = { characters: [catpope] };
	const group = {
		label: 's',
		renderer: { type: 'vertical' },
		slices: [],
		events: [
			{ type: 'joinScene', params: { characters: [{ name: 'plf', position: { variant: 'portrait' } }] } },
			{ type: 'joinScene', params: { characters: [{ character: 'catpope', persona: 'human', variant: 'portrait' }] } }
		]
	};
	const problems = validateFile('group', group, 'en', { file: 'g.json', cast: { catpope: ['regular', 'human'] }, expressions: expressionIndex(registry), moods: ['neutral'] });
	assert.ok(problems.some((p) => p.level === 'info' && p.message.includes('"plf" is an old id')));
	assert.ok(!problems.some((p) => p.level !== 'info'));
});
