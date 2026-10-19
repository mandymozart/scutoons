// A mock of Runway's developer API for the tests and the studio check – the shapes recorded from the documented
// responses (docs.dev.runwayml.com/api, version 2024-11-06, read 2026-10-19): create → { id, estimatedCost: { credits } },
// GET /v1/tasks/{id} → PENDING → RUNNING (progress) → SUCCEEDED { output: [url], cost } (or FAILED { failure,
// failureCode, cost }), DELETE → 204 (CANCELLED), POST /v1/uploads → { uploadUrl, fields, runwayUri } + the form POST.
// Never the real API. A prompt containing "FAIL" fails the task; "SLOW" keeps it running until cancelled.
//
//   node workflows/storyboard-studio/generators/test/runway-mock.mjs --port 14091 [--video some.mp4]   (standalone)

import { createServer } from 'node:http';
import { randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

const PRICE = { gen4_turbo: 5, 'gen4.5': 12, aleph2: 28 };

/** a minimal MP4 (ftyp + moov/mvhd) of `seconds` – enough for mp4Duration and a download */
export function tinyMp4(seconds = 4) {
	const box = (type, body) => { const h = Buffer.alloc(8); h.writeUInt32BE(8 + body.length); h.write(type, 4, 'latin1'); return Buffer.concat([h, body]); };
	const mvhd = Buffer.alloc(100);
	mvhd.writeUInt32BE(1000, 12); // timescale
	mvhd.writeUInt32BE(Math.round(seconds * 1000), 16); // duration
	return Buffer.concat([box('ftyp', Buffer.from('isom\0\0\x02\0isomiso2mp41', 'latin1')), box('moov', box('mvhd', mvhd))]);
}

export function startMock({ port = 0, video = tinyMp4(5), polls = 2 } = {}) {
	const tasks = new Map();
	const requests = [];
	const server = createServer(async (req, res) => {
		const chunks = [];
		for await (const c of req) chunks.push(c);
		const raw = Buffer.concat(chunks);
		const url = new URL(req.url, 'http://x');
		const send = (code, body, type = 'application/json') => { res.writeHead(code, { 'content-type': type }); res.end(body == null ? undefined : type === 'application/json' ? JSON.stringify(body) : body); };
		const body = (req.headers['content-type'] ?? '').startsWith('application/json') && raw.length ? JSON.parse(raw.toString('utf8')) : null;
		requests.push({ method: req.method, path: url.pathname, headers: req.headers, body, size: raw.length });
		const base = `http://127.0.0.1:${server.address().port}`;
		if (url.pathname.startsWith('/files/')) return send(200, video, 'video/mp4');
		if (url.pathname === '/upload' && req.method === 'POST') return send(204, null);
		if (req.headers.authorization !== 'Bearer mock-key') return send(401, { error: 'Invalid API key' });
		if (req.headers['x-runway-version'] !== '2024-11-06') return send(400, { error: 'X-Runway-Version header is required' });
		if (url.pathname === '/v1/uploads' && req.method === 'POST') return send(200, { uploadUrl: `${base}/upload`, fields: { key: 'mock/upload', policy: 'x' }, runwayUri: `runway://mock/${body.filename}` });
		const create = /^\/v1\/(image_to_video|video_to_video)$/.exec(url.pathname);
		if (create && req.method === 'POST') {
			if (!PRICE[body.model]) return send(400, { error: 'Validation of body failed', issues: [{ code: 'invalid_union', path: ['model'], message: 'Invalid model' }] });
			if (create[1] === 'image_to_video' && !['1280:720', '720:1280', '1104:832', '960:960', '832:1104', '1584:672'].includes(body.ratio)) return send(400, { error: 'Validation of body failed', issues: [{ code: 'invalid_enum_value', path: ['ratio'], message: 'Invalid ratio' }] });
			const seconds = create[1] === 'image_to_video' ? body.duration ?? 5 : 5;
			const credits = Math.max(body.model === 'aleph2' ? 56 : 0, PRICE[body.model] * seconds);
			const id = randomUUID();
			tasks.set(id, { id, createdAt: new Date().toISOString(), body, credits, polls: 0, cancelled: false });
			return send(200, { id, estimatedCost: { credits } });
		}
		const task = /^\/v1\/tasks\/([\w-]+)$/.exec(url.pathname);
		if (task) {
			const t = tasks.get(task[1]);
			if (!t) return send(404, { error: 'Task not found' });
			if (req.method === 'DELETE') { t.cancelled = true; return send(204, null); }
			const head = { id: t.id, createdAt: t.createdAt };
			if (t.cancelled) return send(200, { ...head, status: 'CANCELLED', cost: { credits: 0 } });
			t.polls++;
			if (t.polls <= 1) return send(200, { ...head, status: 'PENDING', estimatedCost: { credits: t.credits } });
			if (t.polls <= polls || /SLOW/.test(t.body.promptText ?? '')) return send(200, { ...head, status: 'RUNNING', progress: Math.min(0.9, t.polls / (polls + 1)), estimatedCost: { credits: t.credits } });
			if (/FAIL/.test(t.body.promptText ?? '')) return send(200, { ...head, status: 'FAILED', failure: 'The input did not pass moderation.', failureCode: 'SAFETY.INPUT.TEXT', cost: { credits: 0 } });
			return send(200, { ...head, status: 'SUCCEEDED', output: [`${base}/files/${t.id}.mp4?_jwt=x`], cost: { credits: t.credits } });
		}
		send(404, { error: 'Not found' });
	});
	return new Promise((ok) => server.listen(port, '127.0.0.1', () => ok({ url: `http://127.0.0.1:${server.address().port}`, requests, tasks, close: () => new Promise((d) => server.close(d)) })));
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
	const args = process.argv.slice(2);
	const opt = (f) => { const i = args.indexOf(f); return i >= 0 ? args[i + 1] : null; };
	const m = await startMock({ port: Number(opt('--port') ?? 14091), video: opt('--video') ? readFileSync(opt('--video')) : tinyMp4(5), polls: 4 });
	console.log(`runway mock on ${m.url} (key: mock-key)`);
}
