// page.ts – the #306 benchmark page (index.html): every browser runtime in its own module worker, the watchdog, then
// the table (the Rust report) and the results sent back to the PC that serves the page (serve.ts `POST /results`).
import { runAll, table, type WorkerLike } from './bench.ts';

const $ = (id: string) => document.getElementById(id) as HTMLElement;
const q = new URLSearchParams(location.search);

/** a short name of the browser for the results file */
function browser(ua: string): string {
	if (/Firefox\//.test(ua)) return 'firefox';
	if (/Edg\//.test(ua)) return 'edge';
	if (/Chrome\//.test(ua)) return 'chrome';
	if (/Safari\//.test(ua)) return 'safari';
	return 'browser';
}

const labelInput = $('label') as HTMLInputElement;
labelInput.value = q.get('label') ?? (/Android|iPhone/.test(navigator.userAgent) ? 's22' : 'pc');
$('coi').textContent = `cross-origin isolated: ${self.crossOriginIsolated ? 'yes (5 µs clock)' : 'no (the clock is coarse: 100 µs)'} · ${navigator.hardwareConcurrency} cores`;

const spawn = (): WorkerLike => {
	const w = new Worker('worker.ts', { type: 'module' });
	return {
		post: (m) => w.postMessage(m),
		on: (f) => {
			w.onmessage = (e) => f(e.data);
		},
		terminate: () => w.terminate()
	};
};

const log = (s: string) => {
	$('log').textContent += s + '\n';
};

async function run() {
	($('run') as HTMLButtonElement).disabled = true;
	$('log').textContent = '';
	const label = `${labelInput.value.trim() || 'device'}-${browser(navigator.userAgent)}`;
	const blocks = Number(q.get('blocks') ?? 2000);
	const runtimes = await runAll(spawn, { blocks }, log);
	const mem = (performance as any).memory;
	const all = {
		device: {
			label,
			os: navigator.platform,
			cpu: '',
			cores: navigator.hardwareConcurrency,
			user_agent: navigator.userAgent,
			when: new Date().toISOString().slice(0, 16).replace('T', ' ') + ' UTC',
			notes: [`crossOriginIsolated=${self.crossOriginIsolated}`, mem ? `js heap used ${(mem.usedJSHeapSize / 1048576).toFixed(1)} MB` : 'js heap: n/a']
		},
		runtimes,
		sizes: await (await fetch('dist/sizes.json', { cache: 'no-store' })).json()
	};
	const json = JSON.stringify(all, null, 2);
	const wasm = new Uint8Array(await (await fetch('dist/bench-web.wasm')).arrayBuffer());
	$('table').textContent = await table(wasm, [all]);
	const blob = URL.createObjectURL(new Blob([json], { type: 'application/json' }));
	const a = $('download') as HTMLAnchorElement;
	a.href = blob;
	a.download = `${label}.json`;
	a.hidden = false;
	try {
		const r = await fetch(`results?label=${encodeURIComponent(label)}`, { method: 'POST', body: json, headers: { 'content-type': 'application/json' } });
		log(r.ok ? `sent to the PC: ${await r.text()}` : `not sent (${r.status}) – use the download link`);
	} catch (e) {
		log(`not sent (${e}) – use the download link`);
	}
	($('run') as HTMLButtonElement).disabled = false;
}

$('run').addEventListener('click', run);
if (q.has('auto')) run();
