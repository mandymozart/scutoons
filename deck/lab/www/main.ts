// The #182 lab page: starts the worker, measures the main ↔ worker round trip with one audio block (128 floats,
// copied and transferred), runs the suite in the worker and shows the rows. `window.__results` holds them for a
// script (the morning report's table was read from there).
type Row = { path: string; workload: string; us: number };

const out = document.querySelector('#out') as HTMLElement;
const say = (s: string) => (out.textContent += s + '\n');

const worker = new Worker(new URL('./worker.js', import.meta.url), { type: 'module' });

function roundTrip(transfer: boolean, n: number): Promise<number> {
	return new Promise((resolve) => {
		const times: number[] = [];
		let t = 0;
		const send = () => {
			const buf = new Float32Array(128);
			t = performance.now();
			worker.postMessage({ type: 'echo', buf, transfer }, transfer ? [buf.buffer] : []);
		};
		worker.onmessage = () => {
			times.push(performance.now() - t);
			if (times.length < n) send();
			else {
				times.sort((a, b) => a - b);
				resolve(times[Math.floor(n / 2)] * 1000);
			}
		};
		send();
	});
}

async function main() {
	const rows: Row[] = [];
	say(`crossOriginIsolated: ${self.crossOriginIsolated} (timer resolution)`);
	rows.push({ path: 'postMessage (Chromium)', workload: 'round trip, 128 floats copied', us: await roundTrip(false, 1000) });
	rows.push({ path: 'postMessage (Chromium)', workload: 'round trip, 128 floats transferred', us: await roundTrip(true, 1000) });
	say('round trips done, running the suite in the worker …');
	const suite: Row[] = await new Promise((resolve) => {
		worker.onmessage = (ev) => ev.data.type === 'rows' && resolve(ev.data.rows);
		worker.postMessage({ type: 'run' });
	});
	rows.push(...suite);
	(window as unknown as { __results: Row[] }).__results = rows;
	say('| path | workload | µs |\n|---|---|---|');
	for (const r of rows) say(`| ${r.path} | ${r.workload} | ${r.us.toFixed(3)} |`);
	document.title = 'done';
}

main();
