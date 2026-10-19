// The Runway generator (owner 2026-10-19: "Erstmal soweit bauen, dass ich die Runway API anbinden kann"): Runway's
// developer API (docs.dev.runwayml.com, API version 2024-11-06, read 2026-10-19), two of its endpoints:
//   POST /v1/image_to_video  – gen4_turbo (5 credits/s) or gen4.5 (12 credits/s): the shot's still as the first frame +
//                              the prompt, 2–10 s, the output ratio from a fixed list (the nearest to the shot's aspect)
//   POST /v1/video_to_video  – aleph2 (28 credits/s, at least 56): the shot's handheld render pass (≤ 30 s) restyled by
//                              the prompt, optionally the still as a keyframe at the start; the output follows the input
// then GET /v1/tasks/{id} (status PENDING · THROTTLED · RUNNING + progress · SUCCEEDED + output URLs · FAILED ·
// CANCELLED; not more often than every 5 s), DELETE /v1/tasks/{id} to cancel, the output URLs (they expire in 24–48 h)
// downloaded at once. 1 credit = 0.01 USD. Inputs as data URIs (≤ 5 MB encoded), bigger files as an ephemeral upload
// (POST /v1/uploads → a form POST → runway://…). act_two (character performance) needs a filmed face performance as
// the driving video, so it is not offered for camera moves.
//
// The key: RUNWAY_API_KEY from the environment or the repo's gitignored .env (the convention of comfy.mjs – LUMA keys
// live there too); read on every call, so adding it needs no restart. It is never sent to the browser.
// Never retried by itself: a create call that fails or times out is reported, not sent again (it may have been billed).
// RUNWAY_API_URL points the adapter at another server (the tests' mock: generators/test/runway-mock.mjs).

import { existsSync, readFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import { parseEnv } from 'node:util';
import { EXT, dataUri, dataUriLength, mimeOf, mp4Duration, nearestRatio } from './media.mjs';

export const VERSION = '2024-11-06';
const I2V_RATIOS = ['1280:720', '720:1280', '1104:832', '960:960', '832:1104', '1584:672'];
export const MODELS = [
	{ id: 'gen4_turbo', label: 'Gen-4 Turbo – image to video', kind: 'video', mode: 'image_to_video', inputs: ['still', 'prompt', 'duration', 'ratio'], durations: { min: 2, max: 10 }, ratios: I2V_RATIOS, price: { creditsPerSecond: 5 } },
	{ id: 'gen4.5', label: 'Gen-4.5 – image to video', kind: 'video', mode: 'image_to_video', inputs: ['still', 'prompt', 'duration', 'ratio'], durations: { min: 2, max: 10 }, ratios: I2V_RATIOS, price: { creditsPerSecond: 12 }, promptRequired: true },
	{ id: 'aleph2', label: 'Aleph 2 – video to video (the handheld pass)', kind: 'video', mode: 'video_to_video', inputs: ['control', 'still', 'prompt'], maxInputSeconds: 30, price: { creditsPerSecond: 28, minCredits: 56 } }
];
const DATA_URI_MAX = 5 * 1024 * 1024;
const sleep = (ms, signal) => new Promise((ok, no) => {
	if (signal?.aborted) return no(Object.assign(new Error('cancelled'), { name: 'AbortError' }));
	const t = setTimeout(ok, ms);
	signal?.addEventListener('abort', () => { clearTimeout(t); no(Object.assign(new Error('cancelled'), { name: 'AbortError' })); }, { once: true });
});

export function createRunway({ root, env = process.env, pollMs = 5000, fetchImpl = fetch } = {}) {
	const base = () => (env.RUNWAY_API_URL || 'https://api.dev.runwayml.com').replace(/\/$/, '');
	function apiKey() {
		if (env.RUNWAY_API_KEY) return env.RUNWAY_API_KEY;
		const file = root && resolve(root, '.env');
		if (file && existsSync(file)) try { return parseEnv(readFileSync(file, 'utf8')).RUNWAY_API_KEY || null; } catch {}
		return null;
	}
	const headers = (json = true) => ({ authorization: `Bearer ${apiKey()}`, 'x-runway-version': VERSION, ...(json ? { 'content-type': 'application/json' } : {}) });
	async function call(method, path, body, signal) {
		const r = await fetchImpl(`${base()}${path}`, { method, headers: headers(!!body), body: body ? JSON.stringify(body) : undefined, signal });
		const text = await r.text();
		let j = {};
		try { j = text ? JSON.parse(text) : {}; } catch { j = { error: text.slice(0, 200) }; }
		if (!r.ok) {
			const issues = (j.issues ?? []).map((i) => `${(i.path ?? []).join('.')}: ${i.message}`).join('; ');
			const why = r.status === 401 ? 'the key was refused (RUNWAY_API_KEY)' : r.status === 429 ? 'Runway says too many requests – the daily limit of your tier is reached' : `${j.error ?? r.statusText}${issues ? ` (${issues})` : ''}${j.reason ? ` [${j.reason}]` : ''}`;
			throw Object.assign(new Error(`Runway ${r.status}: ${why}`), { status: r.status });
		}
		return j;
	}

	/** a local file as something Runway reads: a data URI when small enough, else an ephemeral upload (runway://…) */
	async function asUri(file, signal) {
		if (dataUriLength(file) <= DATA_URI_MAX) return dataUri(file);
		const up = await call('POST', '/v1/uploads', { filename: basename(file), type: 'ephemeral' }, signal);
		const fd = new FormData();
		for (const [k, v] of Object.entries(up.fields ?? {})) fd.append(k, v);
		fd.append('file', new Blob([readFileSync(file)], { type: mimeOf(file) }), basename(file));
		const r = await fetchImpl(up.uploadUrl, { method: 'POST', body: fd, signal });
		if (!r.ok) throw new Error(`Runway upload ${r.status}`);
		return up.runwayUri;
	}

	const modelOf = (id) => MODELS.find((m) => m.id === id) ?? MODELS[0];
	function estimate(job) {
		const m = modelOf(job.model);
		let seconds, ratio, note = '';
		if (m.mode === 'video_to_video') {
			seconds = job.controlFile && existsSync(job.controlFile) ? mp4Duration(job.controlFile) : null;
			if (seconds == null) return { credits: 0, usd: 0, seconds: null, error: 'pick the handheld render pass (an MP4) first' };
			if (seconds > m.maxInputSeconds) return { credits: 0, usd: 0, seconds, error: `the video is ${seconds.toFixed(1)} s – Runway takes at most ${m.maxInputSeconds} s` };
			note = 'the output follows the input video (length, aspect)';
		} else {
			seconds = Math.min(m.durations.max, Math.max(m.durations.min, Math.round(Number(job.duration) || 5)));
			ratio = nearestRatio(job.aspect, m.ratios);
		}
		const credits = Math.ceil(Math.max(m.price.minCredits ?? 0, m.price.creditsPerSecond * seconds));
		return { credits, usd: Math.round(credits) / 100, seconds, ratio, note, rate: `${m.price.creditsPerSecond} credits/s${m.price.minCredits ? `, at least ${m.price.minCredits}` : ''}` };
	}

	async function run(job, ctx) {
		const m = modelOf(job.model);
		const est = estimate(job);
		if (est.error) throw new Error(est.error);
		const prompt = String(job.prompt ?? '').trim().slice(0, 1000);
		if (m.promptRequired && !prompt) throw new Error(`${m.id} needs a prompt`);
		let body;
		if (m.mode === 'image_to_video') {
			if (!job.stillFile) throw new Error('pick a still first');
			body = { model: m.id, promptImage: await asUri(job.stillFile, ctx.signal), ratio: est.ratio, duration: est.seconds, ...(prompt ? { promptText: prompt } : {}), ...(job.seed != null ? { seed: Number(job.seed) } : {}) };
		} else {
			body = { model: m.id, videoUri: await asUri(job.controlFile, ctx.signal), ...(prompt ? { promptText: prompt } : {}), ...(job.stillFile ? { keyframes: [{ uri: await asUri(job.stillFile, ctx.signal), at: 0 }] } : {}), ...(job.seed != null ? { seed: Number(job.seed) } : {}) };
		}
		if (ctx.signal.aborted) throw Object.assign(new Error('cancelled'), { name: 'AbortError' });
		// the paid call: exactly once
		const created = await call('POST', `/v1/${m.mode}`, body);
		ctx.remote(created.id, created.estimatedCost?.credits);
		ctx.progress(0, 'Sent to Runway');
		let misses = 0;
		for (;;) {
			try {
				await sleep(pollMs, ctx.signal);
			} catch (e) {
				await call('DELETE', `/v1/tasks/${created.id}`).catch(() => {});
				throw e;
			}
			let t;
			try {
				t = await call('GET', `/v1/tasks/${created.id}`);
				misses = 0;
			} catch (e) {
				// polling is free: a network hiccup or 5xx is tried again, a few times
				if ((e.status && e.status < 500 && e.status !== 429) || ++misses > 10) throw e;
				continue;
			}
			if (t.cost?.credits != null) ctx.cost(t.cost.credits);
			if (t.status === 'RUNNING') ctx.progress(t.progress ?? null, 'Running');
			else if (t.status === 'PENDING' || t.status === 'THROTTLED') ctx.progress(0, t.status === 'THROTTLED' ? 'Waiting – the tier\'s concurrency limit' : 'Waiting');
			else if (t.status === 'FAILED') throw new Error(`Runway: ${t.failure ?? 'failed'}${t.failureCode ? ` (${t.failureCode})` : ''}`);
			else if (t.status === 'CANCELLED') throw Object.assign(new Error('cancelled'), { name: 'AbortError' });
			else if (t.status === 'SUCCEEDED') {
				const files = [];
				for (const url of t.output ?? []) {
					const r = await fetchImpl(url);
					if (!r.ok) throw new Error(`the result could not be downloaded (${r.status}) – task ${created.id}`);
					const type = (r.headers.get('content-type') ?? '').split(';')[0];
					const ext = EXT[type] ?? (/\.(mp4|webm|mov|png|jpe?g|webp)(\?|$)/i.exec(url)?.[1].toLowerCase().replace('jpeg', 'jpg')) ?? 'mp4';
					files.push({ bytes: Buffer.from(await r.arrayBuffer()), ext });
				}
				return { files };
			}
		}
	}

	return {
		id: 'runway',
		label: 'Runway',
		transport: 'api',
		kinds: ['video'],
		paid: true,
		creditUsd: 0.01,
		models: MODELS,
		note: 'Runway developer API – billed per second of video (1 credit = 0.01 USD)',
		async status() {
			return apiKey() ? { configured: true } : { configured: false, how: 'Not configured: put RUNWAY_API_KEY=… into the repo\'s .env (dev.runwayml.com → API keys) – README "Generators".' };
		},
		estimate,
		run
	};
}
