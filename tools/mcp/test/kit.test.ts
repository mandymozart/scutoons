// The local kit (#127): generate_local's ComfyUI graph and settings (comfy-local.ts – the graph scripts/storyboard/
// comfy.mjs posts, without the owner's paths), its HTTP steps against a fake ComfyUI, whoami's kit summary, and the
// extension's empty settings.
import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import {
	comfyFetchImage,
	comfyGraph,
	comfyQueue,
	comfyReachable,
	comfySettings,
	comfyUpload,
	comfyWait,
	historyState,
	notReachable,
	pickInputs,
	recordParams
} from '../src/comfy-local.ts';
import { dropUnsetSettings } from '../src/config.ts';
import { kitSummary, type KitProbe } from '../src/kit.ts';

test('comfySettings: comfy.mjs\'s defaults, COMFY_* override, the 8-step LoRA past 4 steps', () => {
	const d = comfySettings({});
	assert.deepEqual(d, {
		url: 'http://127.0.0.1:8188',
		unet: 'qwen_image_edit_2509_fp8_e4m3fn.safetensors',
		clip: 'qwen_2.5_vl_7b_fp8_scaled.safetensors',
		vae: 'qwen_image_vae.safetensors',
		lora: 'Qwen-Image-Edit-2509-Lightning-4steps-V1.0-bf16.safetensors',
		steps: 4,
		cfg: 1,
		mp: 1,
		charLora: null,
		refMethod: null
	});
	assert.equal(comfySettings({}, { steps: 8 }).lora, 'Qwen-Image-Edit-2509-Lightning-8steps-V1.0-bf16.safetensors');
	const e = comfySettings({ COMFY_URL: 'http://gpu.local:8190/', COMFY_UNET: 'u.safetensors', COMFY_LORA: 'none', COMFY_STEPS: '40', COMFY_CFG: '4', COMFY_CHAR_LORA: 'plf.safetensors:0.6', COMFY_REF_METHOD: 'index_timestep_zero' });
	assert.deepEqual([e.url, e.unet, e.lora, e.steps, e.cfg, e.charLora, e.refMethod], ['http://gpu.local:8190', 'u.safetensors', 'none', 40, 4, { name: 'plf.safetensors', strength: 0.6 }, 'index_timestep_zero']);
	assert.equal(comfySettings({ COMFY_CHAR_LORA: 'x.safetensors' }).charLora?.strength, 0.8);
});

test('comfyGraph: the Qwen-Edit graph, image 1 = the latent, images 2–3 as guides', () => {
	const m = comfySettings({});
	const g = comfyGraph('draw it', ['a.png', 'b.png', 'c.png'], 42, m);
	assert.deepEqual(Object.keys(g).sort((x, y) => Number(x) - Number(y)), ['1', '2', '3', '4', '5', '6', '10', '11', '12', '20', '21', '30', '31', '40', '41', '42']);
	assert.deepEqual(g[10].inputs, { image: 'a.png' });
	assert.deepEqual(g[20].inputs.image, ['10', 0], 'image 1 is scaled and encoded');
	assert.deepEqual([g[30].inputs.prompt, g[30].inputs.image2, g[30].inputs.image3, g[31].inputs.prompt, g[31].inputs.image3], ['draw it', ['11', 0], ['12', 0], '', ['12', 0]]);
	assert.deepEqual([g[40].inputs.seed, g[40].inputs.steps, g[40].inputs.cfg, g[40].inputs.latent_image], [42, 4, 1, ['21', 0]]);
	assert.deepEqual(g[5].inputs.model, ['4', 0], 'the Lightning LoRA in the chain');
	assert.equal(g[4].inputs.lora_name, m.lora);

	const one = comfyGraph('p', ['a.png'], 1, m);
	assert.equal(one[11], undefined);
	assert.equal(one[30].inputs.image2, undefined);

	const big = comfyGraph('p', ['a.png'], 1, { ...m, lora: 'none', charLora: { name: 'c.safetensors', strength: 0.7 }, refMethod: 'index_timestep_zero' });
	assert.equal(big[4], undefined, 'no Lightning LoRA');
	assert.deepEqual([big[7].inputs.model, big[7].inputs.lora_name, big[7].inputs.strength_model, big[5].inputs.model], [['1', 0], 'c.safetensors', 0.7, ['7', 0]]);
	assert.deepEqual([big[40].inputs.positive, big[40].inputs.negative, big[32].inputs.reference_latents_method], [['32', 0], ['33', 0], 'index_timestep_zero']);

	assert.throws(() => comfyGraph('p', [], 1, m), /image 1/);
	assert.throws(() => comfyGraph('p', ['1', '2', '3', '4'], 1, m), /at most 3/);
});

test('pickInputs: given files first, else the render + character images found here', () => {
	assert.deepEqual(pickInputs({}, { structure: '/r.png', characters: ['/c1.png', '/c2.png', '/c3.png'] }), {
		inputs: [{ role: 'still', file: '/r.png' }, { role: 'character', file: '/c1.png' }, { role: 'character', file: '/c2.png' }]
	});
	assert.deepEqual(pickInputs({ still: '/mine.png', guides: ['/g.png'] }, { structure: '/r.png', characters: ['/c1.png'] }), {
		inputs: [{ role: 'still', file: '/mine.png' }, { role: 'reference', file: '/g.png' }]
	});
	const none = pickInputs({}, { structure: null, characters: ['/c1.png'] });
	assert.ok('error' in none && /give `still`/.test(none.error));
});

test('recordParams and the not-reachable message', () => {
	assert.deepEqual(recordParams(comfySettings({}), '127.0.0.1:8188'), { unet: 'qwen_image_edit_2509_fp8_e4m3fn.safetensors', lora: 'Qwen-Image-Edit-2509-Lightning-4steps-V1.0-bf16.safetensors', steps: 4, cfg: 1, mp: 1, host: '127.0.0.1:8188' });
	const msg = notReachable('http://127.0.0.1:8188', 'ECONNREFUSED');
	assert.match(msg, /No ComfyUI at http:\/\/127\.0\.0\.1:8188 \(ECONNREFUSED\)/);
	assert.match(msg, /Start ComfyUI/);
	assert.match(msg, /COMFY_URL/);
});

test('historyState: running, done, failed', () => {
	assert.equal(historyState(undefined), null);
	assert.equal(historyState({ outputs: {} }), null);
	assert.deepEqual(historyState({ outputs: { 42: { images: [{ filename: 'x.png', subfolder: 's', type: 'output' }] } } }), { image: { filename: 'x.png', subfolder: 's', type: 'output' } });
	assert.deepEqual(historyState({ status: { status_str: 'error', messages: [['execution_error', { exception_message: 'OOM' }]] } }), { error: 'OOM' });
});

test('the HTTP steps against a fake ComfyUI', async () => {
	const seen: string[] = [];
	let polls = 0;
	const fake = (async (input: string | URL | Request, init?: RequestInit) => {
		const url = String(input);
		seen.push(`${init?.method ?? 'GET'} ${url.replace('http://c', '')}`);
		if (url.endsWith('/system_stats')) return new Response('{}');
		if (url.endsWith('/upload/image')) {
			assert.ok(init?.body instanceof FormData);
			return Response.json({ name: (init.body.get('image') as File).name });
		}
		if (url.endsWith('/prompt')) {
			const body = JSON.parse(String(init?.body));
			assert.equal(body.prompt[10].inputs.image, 'up.png');
			return Response.json({ prompt_id: 'p1' });
		}
		if (url.includes('/history/p1')) return Response.json(++polls < 2 ? {} : { p1: { outputs: { 42: { images: [{ filename: 'o.png', subfolder: '', type: 'output' }] } } } });
		if (url.includes('/view?')) return new Response(new Uint8Array([1, 2, 3]));
		return new Response('', { status: 404 });
	}) as typeof fetch;
	assert.equal(await comfyReachable('http://c', fake), null);
	assert.equal(await comfyUpload('http://c', 'up.png', new Uint8Array([9]), fake), 'up.png');
	assert.equal(await comfyQueue('http://c', comfyGraph('p', ['up.png'], 1, comfySettings({})), fake), 'p1');
	assert.equal(await comfyWait('http://c', 'p1', 0, fake, 5), null, 'still running → null (collect later)');
	assert.deepEqual(await comfyWait('http://c', 'p1', 1000, fake, 5), { image: { filename: 'o.png', subfolder: '', type: 'output' } });
	assert.deepEqual([...(await comfyFetchImage('http://c', { filename: 'o.png' }, fake))], [1, 2, 3]);
	assert.ok(seen.includes('GET /view?filename=o.png&subfolder=&type=output'));
	// nothing listening
	assert.equal(await comfyReachable('http://127.0.0.1:3985', fetch, 1000), 'ECONNREFUSED');
	const refusing = (async () => Response.json({ error: 'bad', node_errors: { 1: 'no such model' } }, { status: 400 })) as unknown as typeof fetch;
	await assert.rejects(comfyQueue('http://c', {}, refusing), /ComfyUI refused the graph: .*no such model/);
});

const probe = (over: Partial<KitProbe> = {}): KitProbe => ({ workspace: null, workspaceExists: false, figmaPort: 3056, relay: null, autoStart: true, ownRelay: false, comfyUrl: 'http://127.0.0.1:8188', comfyDown: 'ECONNREFUSED', ...over });

test('kitSummary: what is ready, the next step for what is not', () => {
	let k = kitSummary(probe());
	assert.equal(k.workspace.ok, false);
	assert.match(k.workspace.next!, /Workspace folder/);
	assert.deepEqual([k.figma.ok, k.figma.relay], [true, 'starts with the first figma_* call']);
	assert.deepEqual([k.comfy.ok, k.comfy.optional], [false, true]);

	k = kitSummary(probe({ workspace: '/w', workspaceExists: true, relay: { plugin: true, file: 'Cat-Pope', version: '2026-10-20.4', host: 'scutoons-mcp', pid: 1 }, ownRelay: true, comfyDown: null }));
	assert.deepEqual(k.workspace, { folder: '/w', ok: true });
	assert.deepEqual(k.figma, { relay: 'running on :3056 (this MCP)', plugin: 'connected – file "Cat-Pope", code 2026-10-20.4', ok: true });
	assert.deepEqual(k.comfy, { url: 'http://127.0.0.1:8188', ok: true });

	k = kitSummary(probe({ workspace: '/gone', relay: { plugin: false, relay: '2026-10-20.3' }, autoStart: false }));
	assert.match(k.workspace.next!, /does not exist/);
	assert.deepEqual([k.figma.relay, (k.figma as { plugin?: string }).plugin, k.figma.ok], ['running on :3056 (relay.mjs)', 'not connected', false]);
	assert.match(k.figma.next!, /figma-plugin/);

	k = kitSummary(probe({ autoStart: false }));
	assert.match(k.figma.next!, /supervise\.mjs --detach/);
});

test('dropUnsetSettings: an empty optional setting or its unreplaced placeholder counts as unset', () => {
	const env: NodeJS.ProcessEnv = { SCUTOONS_WORKSPACE: '${user_config.workspace}', COMFY_URL: '', SCUTOONS_MACHINE: 'studio', SCUTOONS_TOKEN: 'sct_x' };
	dropUnsetSettings(env);
	assert.deepEqual(env, { SCUTOONS_MACHINE: 'studio', SCUTOONS_TOKEN: 'sct_x' });
});
