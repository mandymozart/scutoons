// Unit tests of the character image fallback rule (resolveExpression, owner 2026-09-29), the resolved cast and the
// validation.   npm test -w @scutoons/content   (node --test, TypeScript run directly by Node 22)
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	castIndex,
	chapterSceneCharacters,
	ensureStar,
	featuredCharacters,
	featuringSuggestions,
	suggestFeaturing,
	toonStar,
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
			{ id: 'swim', label: { en: 'swim pants' }, default: true, path: 'catpope/avatar/mannequin.webp', slots: [{ id: 'head', slot: 'head', x: 370, y: 70, size: 170 }] },
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
	// a body without slots gets one unplaced instance per slot type
	assert.deepEqual(bodies[0].slots.map((s) => s.id), ['head']);
	assert.equal(bodies[1].slots.length, 9);
	assert.deepEqual(bodies[1].slots[3], { id: 'torso', slot: 'torso', x: 0, y: 0, size: 0 });
});

test('sensitive content: A-pose looks, the reader setting, validation', async () => {
	const { avatarBodies, hidesSensitive, withoutSensitive, isSensitive, APOSE_LOOKS } = await import('../src/index.ts');
	const c: RegistryCharacter = structuredClone(catpope);
	c.apose = {
		looks: [
			{ id: 'swim', default: true, path: 'characters/catpope/catpope_apose-swim.webp' },
			{ id: 'naked', sensitive: true, path: 'characters/catpope/catpope_apose-naked.webp' }
		]
	};
	const bodies = avatarBodies({ characters: [c] }, 'en');
	assert.deepEqual(bodies.map((b) => `${b.look}:${b.sensitive}`), ['swim:false', 'naked:true']);
	// guests always hide, members by their setting (default on)
	assert.equal(hidesSensitive(null), true);
	assert.equal(hidesSensitive({}), true);
	assert.equal(hidesSensitive({ hide_sensitive: false }), false);
	assert.deepEqual(withoutSensitive(bodies, true).map((b) => b.look), ['swim']);
	assert.deepEqual(withoutSensitive(bodies, false).map((b) => b.look), ['swim', 'naked']);
	assert.equal(isSensitive(APOSE_LOOKS.find((l) => l.id === 'naked')), true);
	assert.ok(!validateCharacters({ characters: [c] }, { languages: ['en'] }).some((p) => p.level === 'error'));
	(c.apose!.looks[1] as any).sensitive = 'yes';
	assert.ok(validateCharacters({ characters: [c] }, { languages: ['en'] }).some((p) => p.level === 'error' && p.path.join('.').endsWith('looks.1.sensitive')));
});

test('A-pose slot instances: twins, validation, remapping between bodies', async () => {
	const { avatarBodies, remapAvatarSlots } = await import('../src/index.ts');
	const duo: RegistryCharacter = structuredClone(mouse);
	const twin = (side: string, x: number) => [
		{ id: `head-${side}`, slot: 'head' as const, x, y: 460, size: 330, label: { en: `head ${side}` } },
		{ id: `torso-${side}`, slot: 'torso' as const, x, y: 710, size: 200, label: { en: `torso ${side}` } }
	];
	duo.apose = { looks: [{ id: 'swim', default: true, path: 'characters/duo/duo_apose-swim.webp', slots: [...twin('left', 200), ...twin('right', 570)] }] };
	assert.ok(!validateCharacters({ characters: [duo] }, { languages: ['en'] }).some((p) => p.level === 'error'));
	const [body] = avatarBodies({ characters: [duo] }, 'en');
	assert.deepEqual(body.slots.map((s) => s.id), ['head-left', 'torso-left', 'head-right', 'torso-right']);
	// validation: unique ids, known types, numbers, no old anchors
	const bad: RegistryCharacter = structuredClone(duo);
	bad.apose!.looks[0].slots = [
		{ id: 'torso-left', slot: 'torso', x: 1, y: 1, size: 1 },
		{ id: 'torso-left', slot: 'torso', x: 1, y: 1, size: 1 },
		{ id: 'tail', slot: 'tail' as any, x: 1, y: 1, size: 1 },
		{ id: 'Neck', slot: 'neck', x: 1, y: 1, size: 1 },
		{ id: 'neck', slot: 'neck', x: -1, y: 1, size: 1 }
	];
	(bad.apose!.looks[0] as any).anchors = { head: { x: 1, y: 1, size: 1 } };
	const errors = validateCharacters({ characters: [bad] }, { languages: ['en'] }).filter((p) => p.level === 'error').map((p) => p.message);
	assert.equal(errors.length, 5, errors.join(' | '));
	assert.ok(errors.some((e) => e.includes('"torso-left" twice')));
	assert.ok(errors.some((e) => e.includes('unknown slot type "tail"')));
	assert.ok(errors.some((e) => e.includes('old "anchors"')));
	// remapping: same id stays, otherwise by type to the first free instance, else off; an entry is worn once
	const types: Record<string, 'head' | 'torso' | 'neck'> = { hoodie: 'torso', tee: 'torso', cap: 'head', chain: 'neck' };
	const single = ['head', 'torso', 'neck'].map((t) => ({ id: t, slot: t as 'head' }));
	const twins = body.slots;
	assert.deepEqual(remapAvatarSlots({ torso: 'hoodie', head: 'cap', neck: 'chain' }, twins, (e) => types[e]), {
		'head-left': 'cap',
		'torso-left': 'hoodie',
		'head-right': null,
		'torso-right': null
	});
	assert.deepEqual(remapAvatarSlots({ 'torso-left': 'hoodie', 'torso-right': 'tee' }, single, (e) => types[e]), { head: null, torso: 'hoodie', neck: null });
	assert.deepEqual(remapAvatarSlots({ 'torso-right': 'tee', torso: 'hoodie' }, twins, (e) => types[e]), {
		'head-left': null,
		'torso-left': 'hoodie',
		'head-right': null,
		'torso-right': 'tee'
	});
	assert.deepEqual(remapAvatarSlots({ torso: 'hoodie', neck: 'hoodie' }, single, (e) => types[e]), { head: null, torso: 'hoodie', neck: null });
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

test('the star is in every season of the cast (added in front, never twice)', () => {
	const registry = { characters: [{ id: 'catpope', own_toon: 'catpope', name: { de: 'Katzenpapst' }, color: '', personas: [] }, { id: 'boy', name: { de: 'Boy' }, color: '', personas: [] }] } as unknown as CharacterRegistry;
	assert.equal(toonStar(registry, 'catpope'), 'catpope');
	assert.equal(toonStar(registry, 'toptoy'), null);
	const cast: ToonCast = { seasons: { '1': [{ character: 'boy', personas: ['regular'] }], '2': [{ character: 'catpope', personas: ['regular', 'human'] }] } };
	assert.equal(ensureStar(cast, 'catpope', [3]), true);
	assert.deepEqual(cast.seasons['1'].map((e) => e.character), ['catpope', 'boy']);
	assert.deepEqual(cast.seasons['2'], [{ character: 'catpope', personas: ['regular', 'human'] }]);
	assert.deepEqual(cast.seasons['3'], [{ character: 'catpope', personas: ['regular'] }]);
	assert.equal(ensureStar(cast, 'catpope'), false);
	assert.equal(ensureStar(cast, null), false);
});

test('featuring: suggested from the VN scenes, validated against the cast', () => {
	const groups = [
		{ slices: [{ events: [{ type: 'showMessage', params: { character: 'crows', text: 'kra' }, trigger: {} }] }] },
		{ events: [{ type: 'joinScene', params: { characters: [{ character: 'boy', variant: 'portrait' }, { name: 'plf' }] } }, { type: 'showMessage', params: { character: 'boy', text: 'hi' } }] },
		{ slices: [] }
	];
	assert.deepEqual(chapterSceneCharacters(groups), ['crows', 'boy', 'catpope']);
	assert.deepEqual(suggestFeaturing(groups, 'catpope'), ['catpope', 'crows', 'boy']);
	assert.deepEqual(suggestFeaturing([{ slices: [] }], 'catpope'), ['catpope']);
	const cast = { catpope: ['regular'], boy: ['regular'] };
	const info = featuringSuggestions(['catpope'], ['crows', 'boy', 'catpope'], cast, 'chapter:catpope:2');
	assert.deepEqual(info.map((p) => [p.level, p.message.split(' ')[0]]), [['info', '"crows"'], ['info', '"boy"']]);
	assert.match(featuringSuggestions(undefined, ['boy'], cast, 'x')[0].message, /no "featuring" yet/);
	const chapter = { title: 't', author: 'a', description: '', thumbnail: '', number: 2, language: 'de', release: '2026-10-04', groups: [{ slices: [] }], featuring: ['catpope', 'crows', 'catpope'] };
	const problems = validateFile('chapter', chapter, 'de', { file: 'chapter:catpope:2', cast }).filter((p) => p.path[0] === 'featuring');
	assert.deepEqual(problems.map((p) => p.message), [`"crows" is featured but not in this season's cast`, '"catpope" is featured twice']);
	assert.equal(validateFile('chapter', { ...chapter, featuring: 'catpope' }, 'de', { file: 'x' }).some((p) => p.level === 'error' && p.path[0] === 'featuring'), true);
	assert.deepEqual(featuredCharacters(['boy', 'nobody', 'catpope'], [{ name: 'catpope' }, { name: 'boy' }]).map((c) => c.name), ['boy', 'catpope']);
});
