// A still from the member's own ComfyUI (#127, `generate_local`; agents/research/STUDIO-IN-CREATOR.md §2.4 "a local
// GPU"): the graph scripts/storyboard/comfy.mjs posts (Qwen-Image-Edit 2509 + the Lightning LoRA; image 1 = the
// picture edited – the shot's 3D render –, images 2–3 = guides such as the figures' character images), without the
// owner's paths: the model / LoRA names come from COMFY_* with comfy.mjs's defaults, the server from COMFY_URL
// (default http://127.0.0.1:8188). Free – no budget, no server job. The pure parts (settings, graph, the inputs) are
// tested; the HTTP calls take a fetch so they can be tested against a fake ComfyUI.

export interface ComfySettings {
	url: string;
	unet: string;
	clip: string;
	vae: string;
	/** 'none' = without the Lightning shortcut (a big GPU: steps 40, cfg 4) */
	lora: string;
	steps: number;
	cfg: number;
	/** megapixels of image 1 after scaling (its aspect kept) */
	mp: number;
	/** an extra character LoRA on top (COMFY_CHAR_LORA "name.safetensors[:strength]") */
	charLora: { name: string; strength: number } | null;
	/** how the reference images enter the model (2511 wants 'index_timestep_zero'); null = the node's default */
	refMethod: string | null;
}

const num = (v: string | undefined, d: number): number => (v !== undefined && v.trim() !== '' && Number.isFinite(Number(v)) ? Number(v) : d);

/** the settings from the environment (the extension's comfy_url, COMFY_* for the models), comfy.mjs's defaults */
export function comfySettings(env: NodeJS.ProcessEnv = process.env, override: { steps?: number; cfg?: number; mp?: number } = {}): ComfySettings {
	const steps = override.steps ?? num(env.COMFY_STEPS, 4);
	const [charName, charStrength] = (env.COMFY_CHAR_LORA ?? '').split(':');
	return {
		url: (env.COMFY_URL?.trim() || 'http://127.0.0.1:8188').replace(/\/+$/, ''),
		unet: env.COMFY_UNET || 'qwen_image_edit_2509_fp8_e4m3fn.safetensors',
		clip: env.COMFY_CLIP || 'qwen_2.5_vl_7b_fp8_scaled.safetensors',
		vae: env.COMFY_VAE || 'qwen_image_vae.safetensors',
		lora: env.COMFY_LORA || (steps <= 4 ? 'Qwen-Image-Edit-2509-Lightning-4steps-V1.0-bf16.safetensors' : 'Qwen-Image-Edit-2509-Lightning-8steps-V1.0-bf16.safetensors'),
		steps,
		cfg: override.cfg ?? num(env.COMFY_CFG, 1),
		mp: override.mp ?? num(env.COMFY_MP, 1),
		charLora: charName ? { name: charName, strength: num(charStrength, 0.8) } : null,
		refMethod: env.COMFY_REF_METHOD || null
	};
}

type Node = { class_type: string; inputs: Record<string, unknown> };
export type ComfyGraph = Record<string, Node>;

/** the API graph (comfy.mjs graph()): latent from image 1 (keeps its aspect), up to two more images as guides */
export function comfyGraph(prompt: string, names: string[], seed: number, m: ComfySettings): ComfyGraph {
	if (!names.length) throw new Error('image 1 (the picture edited) is required');
	if (names.length > 3) throw new Error('at most 3 images');
	const g: ComfyGraph = {
		1: { class_type: 'UNETLoader', inputs: { unet_name: m.unet, weight_dtype: 'default' } },
		2: { class_type: 'CLIPLoader', inputs: { clip_name: m.clip, type: 'qwen_image', device: 'default' } },
		3: { class_type: 'VAELoader', inputs: { vae_name: m.vae } },
		4: { class_type: 'LoraLoaderModelOnly', inputs: { model: ['1', 0], lora_name: m.lora, strength_model: 1 } },
		5: { class_type: 'ModelSamplingAuraFlow', inputs: { model: m.lora === 'none' ? ['1', 0] : ['4', 0], shift: 3 } },
		6: { class_type: 'CFGNorm', inputs: { model: ['5', 0], strength: 1 } },
		20: { class_type: 'ImageScaleToTotalPixels', inputs: { image: ['10', 0], upscale_method: 'lanczos', megapixels: m.mp, resolution_steps: 16 } },
		21: { class_type: 'VAEEncode', inputs: { pixels: ['20', 0], vae: ['3', 0] } },
		30: { class_type: 'TextEncodeQwenImageEditPlus', inputs: { clip: ['2', 0], vae: ['3', 0], prompt, image1: ['20', 0] } },
		31: { class_type: 'TextEncodeQwenImageEditPlus', inputs: { clip: ['2', 0], vae: ['3', 0], prompt: '', image1: ['20', 0] } },
		40: { class_type: 'KSampler', inputs: { model: ['6', 0], positive: ['30', 0], negative: ['31', 0], latent_image: ['21', 0], seed, steps: m.steps, cfg: m.cfg, sampler_name: 'euler', scheduler: 'simple', denoise: 1 } },
		41: { class_type: 'VAEDecode', inputs: { samples: ['40', 0], vae: ['3', 0] } },
		42: { class_type: 'SaveImage', inputs: { images: ['41', 0], filename_prefix: 'scutoons/storyboard' } }
	};
	names.forEach((n, i) => {
		g[10 + i] = { class_type: 'LoadImage', inputs: { image: n } };
		if (i > 0) {
			g[30].inputs[`image${i + 1}`] = [String(10 + i), 0];
			g[31].inputs[`image${i + 1}`] = [String(10 + i), 0];
		}
	});
	if (m.lora === 'none') delete g[4];
	if (m.charLora) {
		g[7] = { class_type: 'LoraLoaderModelOnly', inputs: { model: g[5].inputs.model, lora_name: m.charLora.name, strength_model: m.charLora.strength } };
		g[5].inputs.model = ['7', 0];
	}
	if (m.refMethod) {
		g[32] = { class_type: 'FluxKontextMultiReferenceLatentMethod', inputs: { conditioning: ['30', 0], reference_latents_method: m.refMethod } };
		g[33] = { class_type: 'FluxKontextMultiReferenceLatentMethod', inputs: { conditioning: ['31', 0], reference_latents_method: m.refMethod } };
		g[40].inputs.positive = ['32', 0];
		g[40].inputs.negative = ['33', 0];
	}
	return g;
}

export interface LocalInput {
	role: 'still' | 'character' | 'style' | 'reference';
	file: string;
}

/**
 * the images for one run, in order: image 1 = the still (given, else the shot's render on this machine), then up to two
 * guides (given, else the shot's character images); `error` says what to give when there is no image 1.
 */
export function pickInputs(given: { still?: string; guides?: string[] }, found: { structure: string | null; characters: string[] }): { inputs: LocalInput[] } | { error: string } {
	const still = given.still ?? found.structure;
	if (!still) return { error: 'no image 1: give `still` (a local PNG of the shot – e.g. the 3D view exported from the storyboard page) – this machine has no render of the shot' };
	const guides = given.guides ?? found.characters;
	return { inputs: [{ role: 'still', file: still }, ...guides.slice(0, 2).map((file) => ({ role: given.guides ? ('reference' as const) : ('character' as const), file }))] };
}

/** the record's params (what makes the run repeatable – no paths) */
export const recordParams = (m: ComfySettings, host: string): Record<string, unknown> => ({
	unet: m.unet,
	lora: m.lora,
	steps: m.steps,
	cfg: m.cfg,
	mp: m.mp,
	...(m.refMethod ? { refMethod: m.refMethod } : {}),
	...(m.charLora ? { charLora: `${m.charLora.name}:${m.charLora.strength}` } : {}),
	host
});

/** what to run when ComfyUI doesn't answer */
export function notReachable(url: string, why: string): string {
	return [
		`No ComfyUI at ${url} (${why}).`,
		'Start ComfyUI on this computer (the ComfyUI desktop app, or `python main.py --listen 127.0.0.1 --port 8188` in its folder) and try again.',
		'It needs Qwen-Image-Edit 2509 (models/diffusion_models), qwen_2.5_vl_7b (text_encoders), qwen_image_vae (vae) and the Lightning 4-step LoRA (loras) – other names: COMFY_UNET / COMFY_CLIP / COMFY_VAE / COMFY_LORA.',
		'Another address: the extension\'s setting "ComfyUI address" (COMFY_URL).'
	].join(' ');
}

type Fetch = typeof fetch;

/** ComfyUI answers? → null, else the reason */
export async function comfyReachable(url: string, f: Fetch = fetch, ms = 3_000): Promise<string | null> {
	try {
		const r = await f(`${url}/system_stats`, { signal: AbortSignal.timeout(ms) });
		return r.ok ? null : `HTTP ${r.status}`;
	} catch (e) {
		const err = e as { name?: string; cause?: { code?: string } };
		return err.cause?.code ?? (err.name === 'TimeoutError' ? `no answer within ${ms / 1000} s` : (err.name ?? 'unreachable'));
	}
}

/** one image into ComfyUI's input folder → its name there */
export async function comfyUpload(url: string, name: string, bytes: Uint8Array, f: Fetch = fetch): Promise<string> {
	const fd = new FormData();
	fd.append('image', new Blob([new Uint8Array(bytes)]), name);
	fd.append('overwrite', 'true');
	const r = await f(`${url}/upload/image`, { method: 'POST', body: fd });
	if (!r.ok) throw new Error(`ComfyUI upload ${name}: HTTP ${r.status}`);
	return String(((await r.json()) as { name: string }).name);
}

/** queue a graph → its prompt id (ComfyUI's node errors in the message) */
export async function comfyQueue(url: string, graph: ComfyGraph, f: Fetch = fetch): Promise<string> {
	const r = await f(`${url}/prompt`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ prompt: graph, client_id: 'scutoons-mcp' }) });
	const j = (await r.json()) as { prompt_id?: string; node_errors?: unknown; error?: unknown };
	if (!r.ok || !j.prompt_id) throw new Error(`ComfyUI refused the graph: ${JSON.stringify(j.node_errors ?? j.error ?? j).slice(0, 500)}`);
	return j.prompt_id;
}

export type ComfyImage = { filename: string; subfolder?: string; type?: string };

/** a history entry → done with its first image / failed with the message / still running (null) */
export function historyState(entry: any): { image: ComfyImage } | { error: string } | null {
	if (!entry) return null;
	if (entry.status?.status_str === 'error') {
		const m = (entry.status.messages ?? []).find((x: unknown[]) => x[0] === 'execution_error')?.[1];
		return { error: String(m?.exception_message ?? 'ComfyUI: execution error') };
	}
	const img = Object.values(entry.outputs ?? {}).flatMap((o: any) => o?.images ?? [])[0] as ComfyImage | undefined;
	return img ? { image: img } : null;
}

/** poll the history up to `ms` → the image, the error, or null (still running – ask again later) */
export async function comfyWait(url: string, promptId: string, ms: number, f: Fetch = fetch, every = 2_000): Promise<{ image: ComfyImage } | { error: string } | null> {
	const t0 = Date.now();
	for (;;) {
		const h = (await (await f(`${url}/history/${encodeURIComponent(promptId)}`)).json()) as Record<string, unknown>;
		const s = historyState(h[promptId]);
		if (s) return s;
		if (Date.now() - t0 + every > ms) return null;
		await new Promise((ok) => setTimeout(ok, every));
	}
}

/** the finished image's bytes */
export async function comfyFetchImage(url: string, img: ComfyImage, f: Fetch = fetch): Promise<Buffer> {
	const q = new URLSearchParams({ filename: img.filename, subfolder: img.subfolder ?? '', type: img.type ?? 'output' });
	const r = await f(`${url}/view?${q}`);
	if (!r.ok) throw new Error(`ComfyUI /view: HTTP ${r.status}`);
	return Buffer.from(await r.arrayBuffer());
}

/** a local ComfyUI drops its models after a job (comfy.mjs: an 8 GB card's RAM grew until Windows paged) */
export async function comfyFree(url: string, f: Fetch = fetch): Promise<void> {
	if (!/\/\/(127\.0\.0\.1|localhost)[:/]/.test(`${url}/`)) return;
	await f(`${url}/free`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ unload_models: true, free_memory: true }) }).catch(() => {});
}
