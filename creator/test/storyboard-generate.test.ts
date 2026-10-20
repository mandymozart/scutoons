// The Generate panel's logic (src/lib/editor/storyboard-generate.ts, storyboard-money.ts, the provider-job client in
// storyboard-generations.ts – #121): the generators (+ Luma – by hand), the inputs, the job body, the references (a
// local file shared first), euro, the budget gate's notice, polling, a result into the folder + the record, by hand.
// A fake API and a fake folder.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { CharacterRegistry } from '@scutoons/content';
import type { Shot, StoryboardDoc } from '@scutoons/content/storyboard';
import {
	MANUAL,
	UNSHARED_HOST,
	buildPrompt,
	canCancel,
	characterRefs,
	cleanMachine,
	defaultStill,
	generatorState,
	inputOptions,
	inputsOf,
	jobBody,
	loadMachine,
	mediaOf,
	newlyDone,
	pickGenerator,
	pickedRefs,
	previewRefs,
	recordManual,
	resolveRefs,
	saveMachine,
	saveResult,
	sha256Hex,
	shouldPoll,
	withManual,
	type GenerateForm
} from '../src/lib/editor/storyboard-generate.ts';
import { budgetNotice, estimateLine, euro, figuresTitle, missingSetting, money } from '../src/lib/editor/storyboard-money.ts';
import { cancelGeneration, estimateJob, generationResult, getGeneration, listGenerators, placeOf, startJob, type Estimate, type EuroRate, type GenerateApi, type Generation, type GeneratorInfo, type GeneratorModel } from '../src/lib/editor/storyboard-generations.ts';

const gen = (over: Partial<Generation>): Generation => ({
	id: 1, toon: 'catpope', chapter: 3, shot: 'sb-25-1', kind: 'image', take: null, parents: [], generator: 'luma-image', model: 'uni-1', status: 'succeeded',
	prompt: null, seed: null, ratio: null, duration: null, estimate: null, cost: null, currency: null, file: 'sb-25-1--g1.png', machine: 'this browser',
	work_path: null, work_url: null, mark: null, approved: false, dataset: false, note: null, error: null, author: null, via: null, created_at: null, ...over
});

const LUMA_MODEL: GeneratorModel = { id: 'uni-1', label: 'uni-1 – image', kind: 'image', mode: 'image', refs: { roles: ['still', 'reference', 'character', 'style'], max: 9 }, ratios: ['9:16'], price: { currency: 'USD', text: 0.0404 } };
const GEN4: GeneratorModel = { id: 'gen4_turbo', label: 'Gen-4 Turbo', kind: 'video', mode: 'image_to_video', refs: { roles: ['still'], max: 1, required: ['still'] }, durations: { min: 2, max: 10 }, price: { currency: 'USD', credits_per_second: 5, credit: 0.01 } };
const ALEPH: GeneratorModel = { id: 'aleph2', label: 'Aleph 2', kind: 'video', mode: 'video_to_video', refs: { roles: ['control', 'still'], max: 2, required: ['control'] }, price: { currency: 'USD', credits_per_second: 28, credit: 0.01 } };
const generator = (id: string, over: Partial<GeneratorInfo> = {}): GeneratorInfo => ({ id, label: id, configured: false, setting: `${id.toUpperCase()}_KEY`, paid: true, unavailable: null, kinds: ['image'], models: [LUMA_MODEL], ...over });
const RATE: EuroRate = { eur_per_usd: 0.890869, usd_per_eur: 1.1225, date: '2026-10-19', source: 'ecb' };

function fakeApi(answers: Record<string, unknown> = {}) {
	const calls: { method: string; path: string; body?: unknown }[] = [];
	const answer = (method: string, path: string) => answers[`${method} ${path}`] ?? answers[method] ?? {};
	const api: GenerateApi = {
		get: async <T>(path: string) => (calls.push({ method: 'GET', path }), answer('GET', path) as T),
		post: async <T>(path: string, body?: unknown) => (calls.push({ method: 'POST', path, body }), answer('POST', path) as T),
		put: async <T>(path: string, body?: unknown) => (calls.push({ method: 'PUT', path, body }), { item: gen({ id: Number(path.split('/').pop()), ...(body as object) }) } as T),
		fetchRaw: async (path: string) => (calls.push({ method: 'RAW', path }), new Response(new Blob(['png-bytes'], { type: 'image/png' })))
	};
	return { api, calls };
}

/** the chapter folder as a map of names → blobs */
function fakeFolder(names: string[] = []) {
	const files = new Map<string, Blob>(names.map((n) => [n, new Blob(['x'])]));
	return {
		files,
		list: async () => [...files.keys()].sort(),
		write: async (_toon: string, _chapter: number, name: string, data: Blob) => void files.set(name, data)
	};
}

const form = (over: Partial<GenerateForm> = {}): GenerateForm => ({ generator: 'luma-image', model: 'uni-1', prompt: '', still: null, control: null, duration: 5, characters: [], ...over });

test('the provider-job client: generators, estimate, start, one job, cancel, the result – the server\'s paths', async () => {
	const { api, calls } = fakeApi({ 'GET /storyboard/generators': { generators: [generator('runway')], budget: { budget: 0 } }, POST: { estimate: { amount: 0.04 }, item: gen({ id: 9 }) }, GET: { item: gen({ id: 9 }) } });
	assert.equal((await listGenerators(api)).generators[0].id, 'runway');
	const body = { toon: 'catpope', chapter: 3, shot: 'sb-25-1', generator: 'luma-image', model: 'uni-1', prompt: 'p' };
	assert.equal((await estimateJob(api, body)).amount, 0.04);
	assert.equal((await startJob(api, body)).id, 9);
	await getGeneration(api, 9);
	await cancelGeneration(api, 9);
	assert.equal(await (await generationResult(api, 9)).text(), 'png-bytes');
	assert.deepEqual(calls.map((c) => `${c.method} ${c.path}`), [
		'GET /storyboard/generators',
		'POST /storyboard/generations/estimate',
		'POST /storyboard/generations',
		'GET /storyboard/generations/9',
		'POST /storyboard/generations/9/cancel',
		'RAW /storyboard/generations/9/result'
	]);
	// the estimated body is the started body
	assert.deepEqual(calls[1].body, calls[2].body);
});

test('the generators: Luma – by hand after Luma; one without a key is shown but not picked', () => {
	const list = withManual([generator('luma-image'), generator('runway', { configured: true }), generator('gemini-image', { configured: true, unavailable: 'inline answer' })]);
	assert.deepEqual(list.map((g) => g.id), ['luma-image', MANUAL, 'runway', 'gemini-image']);
	assert.deepEqual(list.map(generatorState), ['not_configured', 'ready', 'ready', 'unavailable']);
	assert.equal(pickGenerator(list, null)?.id, 'runway');
	assert.equal(pickGenerator(list, 'luma-image')?.id, 'runway', 'a remembered one without its key is not picked');
	assert.equal(pickGenerator(list, MANUAL)?.id, MANUAL);
	assert.equal(pickGenerator(withManual([generator('luma-image')]), null)?.id, MANUAL, 'nothing set up: by hand');
	assert.equal(withManual([]).length, 1);
});

test('what a model takes; Cancel only where the provider can stop a task', () => {
	assert.deepEqual(inputsOf(LUMA_MODEL), { still: true, stillRequired: false, control: false, duration: false });
	assert.deepEqual(inputsOf(GEN4), { still: true, stillRequired: true, control: false, duration: true });
	assert.deepEqual(inputsOf(ALEPH), { still: true, stillRequired: false, control: true, duration: false });
	assert.equal(canCancel(gen({ status: 'running', generator: 'runway' })), true);
	assert.equal(canCancel(gen({ status: 'running', generator: 'luma-image' })), false);
	assert.equal(canCancel(gen({ status: 'succeeded', generator: 'runway' })), false);
});

test('the shot\'s inputs: finished images as stills (the approved first, pre-filled), videos as the control', () => {
	const all = [gen({ id: 1 }), gen({ id: 2, approved: true }), gen({ id: 3, kind: 'video', file: 'sb-25-1--g3.mp4' }), gen({ id: 4, status: 'running', file: null }), gen({ id: 5, shot: 'sb-25-2' })];
	const { stills, controls } = inputOptions(all, 'sb-25-1');
	assert.deepEqual(stills.map((g) => g.id), [2, 1]);
	assert.deepEqual(controls.map((g) => g.id), [3]);
	assert.equal(defaultStill(stills), 2);
	assert.equal(defaultStill([gen({})]), null);
});

test('the figures\' character images: by framing and mood, https only reaches the provider', () => {
	const registry = { characters: [{ id: 'catpope', name: {}, color: '#000', personas: [{ id: 'human', looks: [{ id: 'default', images: { scared: { portrait: 'characters/catpope-human/p-scared.webp', full: 'characters/catpope-human/f-scared.webp' } } }] }] }] } as unknown as CharacterRegistry;
	const scene = { cast: [{ ref: 'plf', character: 'catpope', persona: 'human' }, { ref: 'x', character: null, new: 'someone' }] } as unknown as StoryboardDoc;
	const shot = { figures: [{ cast: 'plf', mood: 'terrified', framing: 'cu' }, { cast: 'plf', mood: 'panicked', framing: 'cu' }, { cast: 'x' }] } as unknown as Shot;
	const refs = characterRefs(scene, shot, registry, (p) => `https://cdn.scutoons.com/${p}`);
	assert.deepEqual(refs, [{ key: 'catpope/human/scared/portrait', label: 'catpope (human) · scared', url: 'https://cdn.scutoons.com/characters/catpope-human/p-scared.webp', usable: true }]);
	assert.equal(characterRefs(scene, shot, registry, (p) => `http://localhost:8001/${p}`)[0].usable, false);
	assert.deepEqual(characterRefs(scene, shot, null, (p) => p), []);
});

test('the job: only what the model takes; the shot\'s aspect; the length clamped; one body for estimate and start', () => {
	const at = { toon: 'catpope', chapter: 3, shot: 'sb-25-1', aspect: '1:3.2' };
	const shared = gen({ id: 2, work_path: 'sb-25-1--g2.png' });
	const chars = [{ key: 'c', label: 'c', url: 'https://cdn/c.webp', usable: true }, { key: 'd', label: 'd', url: 'http://cdn/d.webp', usable: false }];
	const picked = pickedRefs(form({ still: 2, characters: ['c', 'd'] }), LUMA_MODEL, [shared], chars);
	assert.deepEqual(picked.map((p) => p.role), ['still', 'character']);
	const body = jobBody(at, form({ prompt: 'The image', still: 2 }), LUMA_MODEL, previewRefs(picked));
	assert.deepEqual(body, { toon: 'catpope', chapter: 3, shot: 'sb-25-1', generator: 'luma-image', model: 'uni-1', prompt: 'The image', references: [{ role: 'still', work: 'sb-25-1--g2.png' }, { role: 'character', url: 'https://cdn/c.webp' }], aspect: '1:3.2' });
	// Runway: no characters, a length, no prompt when empty; a loop's aspect words stay out
	const video = jobBody({ ...at, aspect: 'loop' }, form({ generator: 'runway', model: 'gen4_turbo', duration: 30 }), GEN4, []);
	assert.deepEqual(video, { toon: 'catpope', chapter: 3, shot: 'sb-25-1', generator: 'runway', model: 'gen4_turbo', params: { duration: 10 } });
	assert.deepEqual(pickedRefs(form({ still: 2, characters: ['c'] }), GEN4, [shared], chars).map((p) => p.role), ['still']);
});

test('a reference only in the member\'s folder: a stand-in for the estimate, shared before it is sent', async () => {
	const local = gen({ id: 7, file: 'sb-25-1--g7.png' });
	assert.deepEqual(previewRefs([{ role: 'still', gen: local }]), [{ role: 'still', url: `${UNSHARED_HOST}sb-25-1--g7.png` }]);
	const sharedOnes: number[] = [];
	const share = async (g: Generation) => (sharedOnes.push(g.id), { ...g, work_path: 'sb-25-1--g7.png' });
	const r = await resolveRefs([{ role: 'still', gen: local }, { role: 'character', url: 'https://cdn/c.webp' }], share, new Set(['sb-25-1--g7.png']));
	assert.deepEqual(r.references, [{ role: 'still', work: 'sb-25-1--g7.png' }, { role: 'character', url: 'https://cdn/c.webp' }]);
	assert.deepEqual(sharedOnes, [7]);
	// already shared: nothing uploaded again; elsewhere: refused, says where
	assert.deepEqual((await resolveRefs([{ role: 'still', gen: gen({ work_path: 'a.png' }) }], share, new Set())).references, [{ role: 'still', work: 'a.png' }]);
	await assert.rejects(resolveRefs([{ role: 'still', gen: gen({ machine: 'studio-pc' }) }], share, new Set()), /on studio-pc/);
});

test('euro (de-DE): ≈ for an estimate, fine for tiny amounts, the provider\'s figures in the tooltip', () => {
	assert.equal(euro(0.04).replace(/\s/g, ' '), '0,04 €');
	assert.equal(euro(0.0036).replace(/\s/g, ' '), '0,0036 €');
	assert.equal(euro(400, { whole: true }).replace(/\s/g, ' '), '400 €');
	assert.equal(money(0.0434, 'USD', RATE, true).replace(/\s/g, ' '), '≈ 0,04 €');
	assert.equal(money(2, 'EUR', null).replace(/\s/g, ' '), '2,00 €');
	assert.equal(figuresTitle(0.25, 'USD', RATE, GEN4), '25 credits = 0.25 USD · 1 USD = 0,8909 € (ECB 2026-10-19)');
	assert.match(figuresTitle(0.0434, 'USD', { ...RATE, source: 'fallback' }), /^0\.0434 USD · 1 USD ≈ 0,8909 € \(the fixed fallback/);
	const e = { amount: 0.25, currency: 'USD', ratio: '720:1280', duration: 5, budget: { rate: RATE } } as unknown as Estimate;
	assert.equal(estimateLine(e, null).replace(/\s/g, ' '), '≈ 0,22 € · 720:1280 · 5 s');
});

test('the budget gate: no budget / used up as one notice; a missing key names its setting', () => {
	assert.deepEqual(budgetNotice({ status: 402, code: 'no_budget', body: {} }), { kind: 'none' });
	assert.deepEqual(budgetNotice({ status: 402, code: 'budget_exhausted', body: { spent: 412, reserved: 0, budget: 400, estimate: 0.04 } }), { kind: 'exhausted', spent: 412, reserved: 0, budget: 400, estimate: 0.04 });
	assert.equal(budgetNotice({ status: 502, code: 'provider_failed' }), null);
	assert.equal(budgetNotice(new Error('x')), null);
	assert.equal(missingSetting({ status: 409, code: 'generator_not_configured', body: { setting: 'LUMA_API_KEY' } }), 'LUMA_API_KEY');
	assert.equal(missingSetting({ status: 409, code: 'not_cancellable' }), null);
});

test('polling: only while something is open and the page is visible; the newly finished ones', () => {
	assert.equal(shouldPoll([gen({ status: 'running' })], true), true);
	assert.equal(shouldPoll([gen({ status: 'running' })], false), false);
	assert.equal(shouldPoll([gen({}), gen({ status: 'failed' })], true), false);
	const before = new Map<number, Generation['status']>([[1, 'running'], [2, 'running']]);
	assert.deepEqual(newlyDone(before, [gen({ id: 1 }), gen({ id: 2, status: 'running' }), gen({ id: 3 })]).map((g) => g.id), [1]);
});

test('a finished job: its result into the next free file of the shot, then file + sha256 + machine on the record', async () => {
	const { api, calls } = fakeApi();
	const folder = fakeFolder(['sb-25-1--g1.png', 'sb-25-1--g2.png']);
	const item = await saveResult({ api, workspace: folder, machine: 'studio-pc' }, gen({ id: 12, file: null, result_url: 'https://luma/x' }));
	assert.equal(item.file, 'sb-25-1--g3.png');
	assert.equal(await folder.files.get('sb-25-1--g3.png')!.text(), 'png-bytes');
	const put = calls.find((c) => c.method === 'PUT')!;
	assert.equal(put.path, '/storyboard/generations/12');
	assert.deepEqual(put.body, { file: 'sb-25-1--g3.png', file_hash: await sha256Hex(new Blob(['png-bytes'])), machine: 'studio-pc' });
});

test('Luma – by hand: the dropped file into the folder + a record (luma-manual, the prompt, the hash); other types refused', async () => {
	const { api, calls } = fakeApi({ POST: { item: gen({ id: 20, generator: MANUAL }) } });
	const folder = fakeFolder(['sb-25-1--g1.png']);
	const file = Object.assign(new Blob(['jpg'], { type: 'image/jpeg' }), { name: 'luma.jpg' });
	await recordManual({ api, workspace: folder, machine: 'this browser', hash: async () => 'a'.repeat(64) }, { toon: 'catpope', chapter: 3, shot: 'sb-25-1' }, file, { prompt: 'SHOT …', ratio: '1:1.6' });
	assert.ok(folder.files.has('sb-25-1--g2.jpg'));
	assert.deepEqual(calls[0], {
		method: 'POST',
		path: '/storyboard/generations',
		body: { toon: 'catpope', chapter: 3, shot: 'sb-25-1', kind: 'image', generator: MANUAL, model: 'manual', status: 'succeeded', file: 'sb-25-1--g2.jpg', file_hash: 'a'.repeat(64), machine: 'this browser', prompt: 'SHOT …', ratio: '1:1.6' }
	});
	await assert.rejects(recordManual({ api, workspace: folder, machine: 'm' }, { toon: 'catpope', chapter: 3, shot: 'sb-25-1' }, Object.assign(new Blob(['x'], { type: 'text/plain' }), { name: 'a.txt' })), /png, jpeg/);
	assert.deepEqual(mediaOf('', 'clip.MP4'), { ext: 'mp4', kind: 'video' });
	assert.deepEqual(mediaOf('', 'a.jpeg'), { ext: 'jpg', kind: 'image' });
});

test('the machine label: per browser, "this browser" by default, cleaned to what the server takes', () => {
	const store = new Map<string, string>();
	const storage = { getItem: (k: string) => store.get(k) ?? null, setItem: (k: string, v: string) => void store.set(k, v) };
	assert.equal(loadMachine(storage), 'this browser');
	assert.equal(saveMachine(storage, '  Tilmans Büro-PC! '), 'Tilmans Buro-PC-');
	assert.equal(loadMachine(storage), 'Tilmans Buro-PC-');
	assert.equal(cleanMachine('work'), null);
	assert.equal(cleanMachine('…'), null);
	assert.equal(loadMachine(null), 'this browser');
});

test('Build from the shot: the shot as edited + the scene\'s script lines + what to attach', () => {
	const scene = { storyboard: 2, toon: 'catpope', source: { scene: 25, heading: 'EXT. SNOW' }, groups: [] } as unknown as StoryboardDoc;
	const shot = { id: 'sb-25-1', format: 'viewport', aspect: '1:1.6', action: { de: 'PLF fällt' }, prompt: { en: 'PLF falls into the snow' } } as unknown as Shot;
	const text = buildPrompt(scene, shot, ['PLF: Ah!'], { characters: ['catpope (human) · scared'] });
	assert.match(text, /^SHOT sb-25-1 – viewport 1:1\.6/);
	assert.match(text, /The image: PLF falls into the snow/);
	assert.match(text, /Character reference: catpope \(human\) · scared/);
	assert.match(text, /THE SCRIPT \(for context\)\nPLF: Ah!/);
});

test('a finished provider job not saved anywhere yet shows through /result; once in a folder, from there', () => {
	const done = gen({ id: 30, file: null, result_url: 'https://luma/r.png' });
	assert.deepEqual(placeOf(done, new Set()), { at: 'result', id: 30 });
	assert.deepEqual(placeOf({ ...done, status: 'running' }, new Set()), { at: 'none' });
	assert.deepEqual(placeOf({ ...done, file: 'sb-25-1--g4.png' }, new Set(['sb-25-1--g4.png'])), { at: 'local' });
});
