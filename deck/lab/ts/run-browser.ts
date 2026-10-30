// `node --experimental-transform-types deck/lab/ts/run-browser.ts <base url> <profile dir> <shots dir> [formats|demo|all]`
// – the lab pages in a headless Chromium (Edge / Chrome from LAB_BROWSER or the usual paths; its own profile dir, a
// debugging port from LAB_DEBUG_PORT, default 9531), driven over plain CDP: formats.html →
// results/formats-chromium.json + .md; demo.html played isolated, with the transfer fallback and without COOP / COEP,
// each screenshotted. The server is ts/serve.ts (started separately).
// Why plain CDP and not playwright: under playwright (its own launch and connectOverCDP, Chromium 140 and Edge 156)
// `audioWorklet.addModule` never settles – most likely its auto-attach holding the worklet's new target (2026-10-30).
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { summarise, table } from './formats.ts';

const [base, profile, shots, what = 'all'] = process.argv.slice(2);
if (!base || !profile || !shots) throw new Error('usage: run-browser.ts <base url> <profile dir> <shots dir> [formats|demo|all]');
const lab = fileURLToPath(new URL('..', import.meta.url));
mkdirSync(shots, { recursive: true });

const exe = [
	process.env.LAB_BROWSER,
	'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
	'C:/Program Files/Google/Chrome/Application/chrome.exe',
	'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
	'/usr/bin/google-chrome'
].find((p) => p && existsSync(p));
if (!exe) throw new Error('no Chromium found – set LAB_BROWSER');
const port = Number(process.env.LAB_DEBUG_PORT ?? 9531);
const proc = spawn(
	exe,
	['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--mute-audio', '--autoplay-policy=no-user-gesture-required', 'about:blank'],
	{ stdio: 'ignore' }
);

type Target = { type: string; webSocketDebuggerUrl: string };
let target: Target | undefined;
for (let i = 0; i < 100 && !target; i++) {
	try {
		target = ((await (await fetch(`http://127.0.0.1:${port}/json`)).json()) as Target[]).find((t) => t.type === 'page');
	} catch {
		await new Promise((r) => setTimeout(r, 200));
	}
}
if (!target) throw new Error(`the browser did not open its debugging port ${port}`);
const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((r) => (ws.onopen = r));
let nextId = 0;
const pending = new Map<number, (m: { result?: Record<string, unknown>; error?: { message: string } }) => void>();
ws.onmessage = (e) => {
	const m = JSON.parse(String(e.data));
	if (m.method === 'Runtime.consoleAPICalled' && m.params.type === 'error') console.error('page:', m.params.args.map((a: { value?: unknown }) => a.value).join(' '));
	if (m.method === 'Runtime.exceptionThrown') console.error('page error:', m.params.exceptionDetails.exception?.description);
	pending.get(m.id)?.(m);
	pending.delete(m.id);
};
const cdp = (method: string, params: Record<string, unknown> = {}) =>
	new Promise<Record<string, unknown>>((resolve, reject) => {
		const id = ++nextId;
		pending.set(id, (m) => (m.error ? reject(new Error(`${method}: ${m.error.message}`)) : resolve(m.result ?? {})));
		ws.send(JSON.stringify({ id, method, params }));
	});
/** an expression in the page (awaited), its value */
const evaluate = async <T>(expression: string): Promise<T> => {
	const r = (await cdp('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true, userGesture: true })) as { result: { value: T }; exceptionDetails?: { text: string } };
	if (r.exceptionDetails) throw new Error(`${expression}: ${r.exceptionDetails.text}`);
	return r.result.value;
};
const titleIs = async (ok: string[], timeoutMs: number) => {
	const until = Date.now() + timeoutMs;
	while (Date.now() < until) {
		const t = await evaluate<string>('document.title');
		if (ok.includes(t)) return t;
		await new Promise((r) => setTimeout(r, 250));
	}
	throw new Error(`timeout waiting for the title ${ok.join(' | ')}`);
};
const goto = async (url: string) => {
	await cdp('Page.navigate', { url });
	await new Promise((r) => setTimeout(r, 800));
	await evaluate(`new Promise((r) => document.readyState === 'complete' ? r() : addEventListener('load', () => r()))`);
};
const size = (width: number, height: number, mobile = false) =>
	cdp('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile });
const shot = async (name: string) => {
	const m = (await cdp('Page.getLayoutMetrics')) as { cssContentSize: { width: number; height: number } };
	const { width, height } = m.cssContentSize;
	const r = (await cdp('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true, clip: { x: 0, y: 0, width, height, scale: 1 } })) as { data: string };
	writeFileSync(`${shots}/${name}.png`, Buffer.from(r.data, 'base64'));
};
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

try {
	await cdp('Page.enable');
	await cdp('Runtime.enable');
	await size(1280, 900);
	if (what === 'formats' || what === 'all') {
		await goto(`${base}/formats.html`);
		await titleIs(['done', 'failed'], 900_000);
		const r = await evaluate<{ isolated: boolean; ua: string; crossings: { probe: string; us: number; runs: number[] }[]; rows: ReturnType<typeof summarise> }>(
			'window.__results'
		);
		writeFileSync(`${lab}/results/formats-chromium.json`, JSON.stringify(r, null, 2));
		const md = [
			`${r.ua.match(/Edg\/[\d.]+/)?.[0] ?? r.ua.match(/Chrome\/[\d.]+/)?.[0]} headless (Chromium ${r.ua.match(/Chrome\/(\d+)/)?.[1]}), Windows 10, Ryzen 5 3600X, cross-origin isolated: ${r.isolated} (timers 5 µs), ${new Date().toISOString().slice(0, 10)}.`,
			'',
			'| page → worker | µs (median of 5) | the 5 runs |',
			'|---|---|---|',
			...r.crossings.map((c) => `| ${c.probe} | **${c.us.toFixed(2)}** | ${c.runs.map((x) => x.toFixed(2)).join(' / ')} |`),
			'',
			table(r.rows)
		].join('\n');
		writeFileSync(`${lab}/results/formats-chromium.md`, md + '\n');
		await shot('formats');
		console.log(md);
	}
	if (what === 'demo' || what === 'all') {
		const play = async (name: string, query: string, w: number, h: number) => {
			await size(w, h, w < 600);
			await goto(`${base}/demo.html${query}`);
			await evaluate(`document.querySelector('#start').click()`);
			if ((await titleIs(['playing', 'failed'], 30_000)) === 'failed') console.error(`${name}: failed`);
			await sleep(800);
			// the vector live: more FM, a higher lead, forced
			await evaluate(`(() => {
				const set = (id, v) => { const el = document.querySelector(id); el.value = v; el.dispatchEvent(new Event('input')); };
				set('#index', '5'); set('#freq', '0.72');
				const f = document.querySelector('input[value=force]'); f.checked = true; f.dispatchEvent(new Event('input'));
			})()`);
			await sleep(1600);
			console.log(`${name}: ${await evaluate<string>(`document.querySelector('#status').textContent`)}`);
			await shot(name);
		};
		await play('demo-shared', '', 1280, 900);
		await play('demo-transfer', '?transport=transfer', 1280, 900);
		await play('demo-not-isolated', '?coi=0', 1280, 900);
		await play('demo-phone', '', 390, 844);
	}
} finally {
	await cdp('Browser.close').catch(() => {});
	ws.close();
	proc.kill();
}
