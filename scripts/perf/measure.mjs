// Headless Chrome page-load measurement via CDP (no dependencies; Node 22 WebSocket). agents/PERFORMANCE.md.
// Chrome: CHROME env or the usual install paths.
// usage: node measure.mjs <label> <url> [--width=390] [--height=844] [--runs=3] [--warm] [--idle=1500] [--scroll]
//        [--init=<js file run before navigation via addScriptToEvaluateOnNewDocument>] [--json=out.json]
import { spawn } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const args = process.argv.slice(2);
const opt = Object.fromEntries(args.filter((a) => a.startsWith('--')).map((a) => { const [k, v] = a.slice(2).split('='); return [k, v ?? true]; }));
const [label, url] = args.filter((a) => !a.startsWith('--'));
const width = Number(opt.width || 390), height = Number(opt.height || 844), runs = Number(opt.runs || 3), idleMs = Number(opt.idle || 1500);
import { existsSync } from 'node:fs';
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
const port = 9333 + Math.floor(Math.random() * 500);
const profile = opt.profile || mkdtempSync(join(tmpdir(), 'perf-chrome-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', `--window-size=${width},${height}`, 'about:blank'], { stdio: 'ignore' });

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function wsUrl() {
	for (let i = 0; i < 50; i++) {
		try { const list = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json(); const page = list.find((t) => t.type === 'page'); if (page) return page.webSocketDebuggerUrl; } catch {}
		await sleep(200);
	}
	throw new Error('chrome did not start');
}
const ws = new WebSocket(await wsUrl());
await new Promise((r) => ws.addEventListener('open', r));
let id = 0; const pending = new Map(); const listeners = [];
ws.addEventListener('message', (e) => { const m = JSON.parse(e.data); if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); } else listeners.forEach((l) => l(m)); });
const send = (method, params = {}) => new Promise((r) => { const i = ++id; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });

await send('Network.enable'); await send('Performance.enable'); await send('Page.enable'); await send('Runtime.enable');
await send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: Number(opt.dpr || 3), mobile: width < 768 });
if (opt.throttle) await send('Network.emulateNetworkConditions', { offline: false, latency: 60, downloadThroughput: 20e6 / 8, uploadThroughput: 5e6 / 8 });
if (opt.block) await send('Network.setBlockedURLs', { urls: String(opt.block).split(',') });
if (opt.init) await send('Page.addScriptToEvaluateOnNewDocument', { source: readFileSync(opt.init, 'utf8') });

async function run(n) {
	const ignored = new Set(); let faviconCount = 0; const reqs = new Map(); let lastActivity = Date.now(); let t0 = null; let inflight = 0;
	const l = (m) => {
		const p = m.params;
		if (m.method === 'Network.requestWillBeSent') {
			if (/favicon|^data:/.test(p.request.url)) { ignored.add(p.requestId); faviconCount++; return; }
			if (t0 === null) t0 = p.timestamp;
			const prev = reqs.get(p.requestId);
			reqs.set(p.requestId, { url: p.request.url, method: p.request.method, type: p.type, start: p.timestamp, end: null, bytes: 0, status: prev?.status, fromCache: false, redirects: (prev?.redirects || 0) + (p.redirectResponse ? 1 : 0) });
			if (!prev) inflight++;
			lastActivity = Date.now();
		} else if (ignored.has(p?.requestId)) { return; } else if (m.method === 'Network.responseReceived') {
			const r = reqs.get(p.requestId); if (r) { r.status = p.response.status; r.fromCache = p.response.fromDiskCache || p.response.fromServiceWorker || false; r.timing = p.response.timing; r.mime = p.response.mimeType; }
		} else if (m.method === 'Network.requestServedFromCache') {
			const r = reqs.get(p.requestId); if (r) r.fromCache = true;
		} else if (m.method === 'Network.loadingFinished' || m.method === 'Network.loadingFailed') {
			const r = reqs.get(p.requestId); if (r && r.end === null) { r.end = p.timestamp; r.bytes = p.encodedDataLength || 0; r.failed = m.method === 'Network.loadingFailed'; r.error = p.errorText; inflight--; }
			lastActivity = Date.now();
		}
	};
	listeners.push(l);
	if (!opt.warm || n === 0) { await send('Network.clearBrowserCache'); }
	const m0 = Object.fromEntries((await send('Performance.getMetrics')).result.metrics.map((x) => [x.name, x.value]));
	const wall0 = Date.now();
	await send('Page.navigate', { url });
	// wait for network idle
	while (true) {
		await sleep(100);
		if (Date.now() - wall0 > 60000) break;
		if (inflight <= 0 && Date.now() - lastActivity > idleMs) break;
	}
	if (opt.scroll) {
		// scroll through the page to trigger lazy images, then wait for idle again
		await send('Runtime.evaluate', { expression: `(async()=>{for(let y=0;y<document.body.scrollHeight;y+=${height}){scrollTo(0,y);await new Promise(r=>setTimeout(r,60));}})()`, awaitPromise: true });
		while (true) { await sleep(100); if (inflight <= 0 && Date.now() - lastActivity > idleMs) break; if (Date.now() - wall0 > 120000) break; }
	}
	listeners.splice(listeners.indexOf(l), 1);
	const m1 = Object.fromEntries((await send('Performance.getMetrics')).result.metrics.map((x) => [x.name, x.value]));
	const cpu = { taskMs: Math.round(m1.TaskDuration * 1000), scriptMs: Math.round(m1.ScriptDuration * 1000), layoutMs: Math.round(m1.LayoutDuration * 1000) };
	const perf = JSON.parse((await send('Runtime.evaluate', { expression: `JSON.stringify({fcp: (performance.getEntriesByName('first-contentful-paint')[0]||{}).startTime, lcp: window.__lcp, dcl: performance.timing.domContentLoadedEventEnd-performance.timing.navigationStart, load: performance.timing.loadEventEnd-performance.timing.navigationStart, h: document.body.scrollHeight, imgs: document.images.length, title: document.title})`, returnByValue: true })).result.result.value);
	const list = [...reqs.values()].filter((r) => r.end !== null);
	const lastEnd = Math.max(...list.map((r) => r.end));
	const rel = (t) => Math.round((t - t0) * 1000);
	const byUrl = new Map(); for (const r of list) { const k = r.method + ' ' + r.url; byUrl.set(k, (byUrl.get(k) || 0) + 1); }
	const dups = [...byUrl].filter(([, c]) => c > 1).map(([u, c]) => `${c}x ${u}`);
	const cat = (r) => /paraglide\/messages\//.test(r.url) ? 'paraglide' : /localhost:9000|\[::1\]:9000|:9190|:9191/.test(r.url) ? 'api' : /:8001|:8011/.test(r.url) ? 'cdn' : r.type === 'Image' ? 'img' : r.type === 'Font' ? 'font' : /\.(svelte|ts|js)(\?|$)|node_modules|@fs|@vite|@id/.test(r.url) ? 'module' : 'other';
	const cats = {}; for (const r of list) { const c = cat(r); cats[c] ??= { n: 0, bytes: 0, maxMs: 0 }; cats[c].n++; cats[c].bytes += r.bytes; cats[c].maxMs = Math.max(cats[c].maxMs, rel(r.end) - rel(r.start)); }
	return { n, cpu, faviconCount, top: list.map((r) => ({ kb: Math.round(r.bytes / 1024), type: r.type, url: r.url.slice(0, 140) })).sort((a, b) => b.kb - a.kb).slice(0, 25), ms: rel(lastEnd), fcp: Math.round(perf.fcp || 0), dcl: perf.dcl, load: perf.load, requests: list.length, bytes: list.reduce((a, r) => a + r.bytes, 0), cached: list.filter((r) => r.fromCache).length, failed: list.filter((r) => r.failed).map((r) => r.url + ' ' + r.error), cats, dups, slowest: list.map((r) => ({ ms: rel(r.end) - rel(r.start), at: rel(r.start), url: r.url.slice(0, 120), status: r.status })).sort((a, b) => b.ms - a.ms).slice(0, 8), api: list.filter((r) => cat(r) === "api" || /\/api\/|\.json/.test(r.url)).map((r) => ({ at: rel(r.start), end: rel(r.end), ms: rel(r.end) - rel(r.start), status: r.status, method: r.method, url: r.url.replace(/^https?:\/\/[^/]+/, '') })).sort((a, b) => a.at - b.at), perf };
}

const results = [];
for (let n = 0; n < runs; n++) results.push(await run(n));
const med = (k) => { const v = results.map((r) => r[k]).sort((a, b) => a - b); return v[Math.floor(v.length / 2)]; };
const summary = { cats: results[results.length-1].cats, label, url, runs, warm: !!opt.warm, fullyLoadedMs: med('ms'), fcpMs: med('fcp'), requests: med('requests'), kb: Math.round(med('bytes') / 1024), all: results.map((r) => ({ ms: r.ms, fcp: r.fcp, lcp: Math.round(r.perf.lcp||0), requests: r.requests, cached: r.cached, api: r.api.length, cpu: r.cpu.taskMs, fav: r.faviconCount })) };
console.log(JSON.stringify(summary));
if (opt.verbose) { const r = results[results.length - 1]; console.log(JSON.stringify({ cats: r.cats, dups: r.dups, failed: r.failed, slowest: r.slowest, api: r.api, perf: r.perf }, null, 1)); }
if (opt.json) writeFileSync(opt.json, JSON.stringify({ summary, results }, null, 1));
ws.close(); chrome.kill();
process.exit(0);
