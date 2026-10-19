// The generator jobs (owner 2026-10-19: "Ja, generator pluggable ist smart. Erstmal soweit bauen, dass ich die Runway
// API anbinden kann."): one registry of generators, the running jobs of this studio session (in memory), the session's
// spend, and the results written as the shot's generated variants – storyboards/_out/<ch>/generated/<shot>--g<n>.<ext>
// + a line in _out/<ch>/generated.jsonl, the same place and log as scripts/storyboard/comfy.mjs, so Variants →
// "Approve as chosen" works unchanged. README "Generators", agents/research/STORYBOARD.md §3 "As built".
//
// A generator (generators/*.mjs):
//   { id, label, kinds: ['image' | 'video'], paid: boolean,
//     transport: 'api' | 'local' | 'manual',         – how a job gets made (research/GENERATORS.md §2): a provider's API,
//       this PC, or by hand (luma-manual.mjs: never started, never run – the Generate panel shows the prompt to copy
//       instead of Generate; the result is dropped onto Variants)
//     models: [{ id, label, kind, mode, inputs: ['still' | 'control' | 'characters' | 'style' | 'prompt' | 'duration'
//               | 'ratio'], durations?: { min, max } | number[], ratios?: string[], price?: { creditsPerSecond, minCredits } }],
//     status(): Promise<{ configured, how? }>        – "not configured" + a one-line how-to (never the key itself)
//     estimate(job): { credits, usd, seconds, ratio?, note? }   – before anything is sent
//     run(job, ctx): Promise<{ files?: [{ bytes, ext }], written?: [file name it wrote itself], meta? }>
//        job = { model, toon, ch, shot, prompt, duration, aspect, seed, stillFile, controlFile, characterFiles, styleFiles }
//        ctx = { signal (AbortSignal – cancel), progress(p 0…1 | null, message?), remote(taskId, credits?), cost(credits) }
//   }
// Safety (owner): a paid job starts only with `confirm` ≥ the server's own estimate (the UI shows it first); a job is
// never retried by itself; the spend of this session is counted (final cost when the provider reports it).

import { appendFileSync, existsSync, mkdirSync, readdirSync, writeFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';

const ACTIVE = new Set(['queued', 'running']);
const cents = (usd) => Math.round(usd * 100) / 100;

export function createManager({ generators, out, now = () => new Date() }) {
	const byId = new Map(generators.map((g) => [g.id, g]));
	const jobs = [];
	const spend = { usd: 0, credits: 0, jobs: 0 };
	let seq = 0;

	const gen = (id) => { const g = byId.get(id); if (!g) throw new Error(`no generator ${id}`); return g; };
	const modelOf = (g, id) => g.models.find((m) => m.id === id) ?? g.models[0] ?? null;
	const view = (j) => ({ id: j.id, generator: j.generator, model: j.model, shot: j.shot, ch: j.ch, status: j.status, progress: j.progress, message: j.message, task: j.task, estimate: j.estimate, cost: j.cost, files: j.files, created: j.created, ended: j.ended });

	/** the next free <shot>--g<n> (the highest n + 1, so a deleted file never gets overwritten) */
	function nextName(dir, shot, ext) {
		const used = existsSync(dir) ? readdirSync(dir).map((f) => new RegExp(`^${shot}--g(\\d+)\\.`).exec(f)?.[1]).filter(Boolean).map(Number) : [];
		const n = Math.max(0, ...used, existsSync(dir) ? readdirSync(dir).filter((f) => f.startsWith(`${shot}--`)).length : 0) + 1;
		return `${shot}--g${n}.${ext}`;
	}

	function writeResults(j, res) {
		const dir = resolve(out, j.ch, 'generated');
		mkdirSync(dir, { recursive: true });
		const files = [...(res.written ?? [])];
		for (const f of res.files ?? []) {
			const name = nextName(dir, j.shot, f.ext);
			writeFileSync(resolve(dir, name), f.bytes);
			files.push(name);
			// the log line of comfy.mjs, plus the generator, the model, the cost and the provider's task id
			appendFileSync(resolve(out, j.ch, 'generated.jsonl'), JSON.stringify({
				date: now().toISOString(), shot: j.shot, file: name, engine: j.generator, generator: j.generator, model: j.model,
				job: j.task ?? undefined, mode: j.mode, duration: j.estimate?.seconds, ratio: j.estimate?.ratio,
				estimate: j.estimate ? { credits: j.estimate.credits, usd: j.estimate.usd } : undefined,
				cost: j.cost ?? undefined, ...(res.meta ?? {}), images: j.inputs, prompt: j.input.prompt ?? ''
			}) + '\n');
		}
		return files;
	}

	async function execute(j, g) {
		const ctx = {
			signal: j.abort.signal,
			progress: (p, message) => { if (j.status === 'running') { j.progress = p; if (message) j.message = message; } },
			remote: (task, credits) => { j.task = task; if (credits != null && j.estimate) j.estimate.provider = credits; },
			cost: (credits) => { j.cost = { credits, usd: cents(credits * (g.creditUsd ?? 0.01)) }; }
		};
		j.status = 'running';
		try {
			const res = await g.run(j.input, ctx);
			if (j.abort.signal.aborted) throw Object.assign(new Error('cancelled'), { name: 'AbortError' });
			j.files = writeResults(j, res);
			j.status = 'succeeded';
			j.progress = 1;
			j.message = j.files.length ? `${j.files.join(', ')}` : 'no file came back';
		} catch (e) {
			j.status = j.abort.signal.aborted || e.name === 'AbortError' ? 'cancelled' : 'failed';
			j.message = j.status === 'cancelled' ? 'Cancelled.' : e.message;
		} finally {
			j.ended = now().toISOString();
			// the session's spend: the provider's final cost, else the estimate for a success (never less than it cost)
			const usd = j.cost?.usd ?? (j.status === 'succeeded' && g.paid ? j.estimate?.usd ?? 0 : 0);
			if (usd || j.cost) { spend.usd = cents(spend.usd + usd); spend.credits += j.cost?.credits ?? j.estimate?.credits ?? 0; spend.jobs++; }
		}
	}

	return {
		async list() {
			return Promise.all(generators.map(async (g) => ({ id: g.id, label: g.label, transport: g.transport, kinds: g.kinds, paid: !!g.paid, models: g.models, note: g.note ?? '', ...(await g.status().catch((e) => ({ configured: false, how: e.message }))) })));
		},
		estimate(job) {
			const g = gen(job.generator);
			const m = modelOf(g, job.model);
			return { paid: !!g.paid, model: m?.id ?? null, ...g.estimate({ ...job, model: m?.id }) };
		},
		/** starts a job; a paid one needs `confirm` (the USD the owner saw) ≥ the estimate */
		async start(job, { confirm } = {}) {
			const g = gen(job.generator);
			// by hand there is nothing to send – refused before a job exists (no failed entry in the shot's jobs)
			if (g.transport === 'manual') throw new Error(`${g.label} is done by hand – nothing to send`);
			const st = await g.status();
			if (!st.configured) throw new Error(`${g.label} is not configured – ${st.how ?? ''}`.trim());
			const m = modelOf(g, job.model);
			const input = { ...job, model: m?.id };
			const est = g.estimate(input);
			if (g.paid && !(Number(confirm) >= est.usd)) throw new Error(`confirm the cost first (${est.usd.toFixed(2)} USD)`);
			if (!/^[\w-]+$/.test(job.shot ?? '') || !/^[a-z0-9-]+$/.test(job.ch ?? '')) throw new Error('bad shot');
			const j = {
				id: `j${++seq}`, generator: g.id, model: m?.id ?? null, mode: m?.mode ?? null, shot: job.shot, ch: job.ch, status: 'queued', progress: null,
				message: '', task: null, estimate: est, cost: null, files: [], created: now().toISOString(), ended: null,
				input, inputs: [job.still, job.control, ...(job.characters ?? []), ...(job.style ?? [])].filter(Boolean).map((k) => basename(String(k))),
				abort: new AbortController()
			};
			jobs.push(j);
			j.done = execute(j, g);
			return view(j);
		},
		cancel(id) {
			const j = jobs.find((x) => x.id === id);
			if (!j) throw new Error(`no job ${id}`);
			if (ACTIVE.has(j.status)) j.abort.abort();
			return view(j);
		},
		jobs: ({ ch, shot } = {}) => jobs.filter((j) => (!ch || j.ch === ch) && (!shot || j.shot === shot)).map(view).reverse(),
		spend: () => ({ ...spend }),
		/** for tests: wait until a job has ended */
		wait: (id) => jobs.find((x) => x.id === id)?.done
	};
}
