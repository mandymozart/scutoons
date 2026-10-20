// The generator jobs (owner 2026-10-19: "Ja, generator pluggable ist smart. Erstmal soweit bauen, dass ich die Runway
// API anbinden kann."): one registry of generators, the running jobs of this studio session (in memory), the session's
// spend, and the results written as the shot's generated variants – storyboards/_out/<ch>/generated/<shot>--g<n>.<ext>
// + a v2 record in _out/<ch>/generated.jsonl (scripts/storyboard/generated-log.mjs – the same log as comfy.mjs: the
// inputs as keys with their roles, the parents, the take, the seed; a failed or cancelled job writes its line too), so
// Variants → "Approve as chosen" works unchanged. README "Generators", agents/research/VARIANTS-CANVAS.md §3.7.
//
// A generator (generators/*.mjs):
//   { id, label, kinds: ['image' | 'video'], paid: boolean, creditUsd?: number (what one of its credits costs),
//     transport: 'api' | 'local' | 'manual',         – how a job gets made (research/GENERATORS.md §2): a provider's API,
//       this PC, or by hand (luma-manual.mjs: never started, never run – the Generate panel shows the prompt to copy
//       instead of Generate; the result is dropped onto Variants)
//     models: [{ id, label, kind, mode, inputs: ['still' | 'control' | 'characters' | 'style' | 'prompt' | 'duration'
//               | 'ratio'], durations?: { min, max } | number[], ratios?: string[], price?: { creditsPerSecond, minCredits },
//               seed?: true (it takes a seed: the manager picks one, 0 … 2³² − 1, when the job has none) }],
//     status(): Promise<{ configured, how? }>        – "not configured" + a one-line how-to (never the key itself)
//     estimate(job): { credits, usd, seconds, ratio?, note? }   – before anything is sent
//     run(job, ctx): Promise<{ files?: [{ bytes, ext }], written?: [file name it wrote itself], meta? }>
//        job = { model, toon, ch, shot, prompt, duration, aspect, seed, stillFile, controlFile, characterFiles, styleFiles }
//        ctx = { signal (AbortSignal – cancel), progress(p 0…1 | null, message?), remote(taskId, credits?), cost(credits) }
//   }
// Safety (owner): a paid job starts only with `confirm` ≥ the server's own estimate (the UI shows it first); a job is
// never retried by itself; the spend of this session is counted (final cost when the provider reports it).
// Money here is the provider's: credits + USD (the estimate, the cost, the spend, `confirm`, generated.jsonl); the UI
// shows it in euro at the rate GET /api/generators serves (rate.mjs – owner 2026-10-20: "preise in euro").

import { SEED_MAX, appendLine, kindOf, nextName, parentsOf, pickSeed, record, writeNew } from '../../../scripts/storyboard/generated-log.mjs';

const ACTIVE = new Set(['queued', 'running']);
const cents = (usd) => Math.round(usd * 100) / 100;

/** parentOf(key) → the id of the generated file an input is a copy of (index.mjs: the approved image), or null */
export function createManager({ generators, out, now = () => new Date(), parentOf = null }) {
	const byId = new Map(generators.map((g) => [g.id, g]));
	const jobs = [];
	const spend = { usd: 0, credits: 0, jobs: 0 };
	let seq = 0;

	const gen = (id) => { const g = byId.get(id); if (!g) throw new Error(`no generator ${id}`); return g; };
	const modelOf = (g, id) => g.models.find((m) => m.id === id) ?? g.models[0] ?? null;
	const view = (j) => ({ id: j.id, generator: j.generator, model: j.model, shot: j.shot, ch: j.ch, status: j.status, progress: j.progress, message: j.message, task: j.task, estimate: j.estimate, cost: j.cost, seed: j.input.seed, files: j.files, created: j.created, ended: j.ended });

	/** the record of one result, or of a job that ended without one (generated-log.mjs) */
	const line = (j, fields) => record({
		shot: j.shot, generator: j.generator, model: j.model, mode: j.mode, job: j.task, inputs: j.inputs, parents: j.parents, prompt: j.input.prompt ?? '',
		seed: j.input.seed, ratio: j.estimate?.ratio, duration: j.estimate?.seconds,
		estimate: j.estimate ? { credits: j.estimate.credits, usd: j.estimate.usd } : null, cost: j.cost, date: now().toISOString(), ...fields
	});

	function writeResults(j, res) {
		// comfy-local: comfy.mjs wrote its file and its record itself
		const files = [...(res.written ?? [])];
		for (const f of res.files ?? []) {
			const { id, file } = writeNew(out, j.ch, j.shot, f.ext, f.bytes);
			files.push(file);
			appendLine(out, j.ch, line(j, { id, file, kind: kindOf(file), status: 'succeeded', params: res.meta }));
		}
		if (!files.length) appendLine(out, j.ch, line(j, { id: nextName(out, j.ch, j.shot).id, kind: j.kind, status: 'succeeded', error: 'no file came back' }));
		return files;
	}

	/** what the job sends, as keys with their roles (VARIANTS-CANVAS §3.7), only what the model takes – the main input
	 *  first: a video to video's control video (the still is its keyframe at 0), else the still */
	function inputsOf(m, job) {
		const takes = (what) => (m?.inputs ?? []).includes(what);
		const list = [];
		if (takes('control') && job.control) list.push({ role: 'control', key: String(job.control) });
		if (takes('still') && job.still) list.push(m.mode === 'video_to_video' ? { role: 'keyframe', key: String(job.still), at: 0 } : { role: 'still', key: String(job.still) });
		if (takes('characters')) for (const k of job.characters ?? []) list.push({ role: 'character', key: String(k) });
		if (takes('style')) for (const k of job.style ?? []) list.push({ role: 'style', key: String(k) });
		return list;
	}
	/** the job's seed: the given one (0 … 2³² − 1), else one picked here when the model takes a seed, else null */
	function seedOf(m, seed) {
		if (!m?.seed) return null;
		if (seed == null || seed === '') return pickSeed();
		const n = Number(seed);
		if (!Number.isInteger(n) || n < 0 || n > SEED_MAX) throw new Error(`bad seed ${seed} (0 … ${SEED_MAX})`);
		return n;
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
			// a failed or cancelled job keeps its line too: an id of the shot's (no file), its status, its cost
			try { appendLine(out, j.ch, line(j, { id: nextName(out, j.ch, j.shot).id, kind: j.kind, status: j.status, error: j.status === 'failed' ? j.message : null })); } catch {}
		} finally {
			j.ended = now().toISOString();
			// the session's spend: the provider's final cost, else the estimate for a success (never less than it cost)
			const usd = j.cost?.usd ?? (j.status === 'succeeded' && g.paid ? j.estimate?.usd ?? 0 : 0);
			if (usd || j.cost) { spend.usd = cents(spend.usd + usd); spend.credits += j.cost?.credits ?? j.estimate?.credits ?? 0; spend.jobs++; }
		}
	}

	return {
		async list() {
			return Promise.all(generators.map(async (g) => ({ id: g.id, label: g.label, transport: g.transport, kinds: g.kinds, paid: !!g.paid, creditUsd: g.creditUsd ?? null, models: g.models, note: g.note ?? '', ...(await g.status().catch((e) => ({ configured: false, how: e.message }))) })));
		},
		estimate(job) {
			const g = gen(job.generator);
			const m = modelOf(g, job.model);
			return { paid: !!g.paid, model: m?.id ?? null, ...g.estimate({ ...job, model: m?.id }) };
		},
		/** starts a job; a paid one needs `confirm` (the USD behind the euro figure the owner saw) ≥ the estimate */
		async start(job, { confirm } = {}) {
			const g = gen(job.generator);
			// by hand there is nothing to send – refused before a job exists (no failed entry in the shot's jobs)
			if (g.transport === 'manual') throw new Error(`${g.label} is done by hand – nothing to send`);
			const st = await g.status();
			if (!st.configured) throw new Error(`${g.label} is not configured – ${st.how ?? ''}`.trim());
			const m = modelOf(g, job.model);
			const input = { ...job, model: m?.id, seed: seedOf(m, job.seed) };
			const est = g.estimate(input);
			if (g.paid && !(Number(confirm) >= est.usd)) throw new Error(`confirm the cost first (${est.credits} credits)`);
			if (!/^[\w-]+$/.test(job.shot ?? '') || !/^[a-z0-9-]+$/.test(job.ch ?? '')) throw new Error('bad shot');
			const j = {
				id: `j${++seq}`, generator: g.id, model: m?.id ?? null, mode: m?.mode ?? null, kind: m?.kind ?? null, shot: job.shot, ch: job.ch,
				status: 'queued', progress: null, message: '', task: null, estimate: est, cost: null, files: [], created: now().toISOString(),
				ended: null, input, inputs: inputsOf(m, job), abort: new AbortController()
			};
			// the parents as they are now (an approval during the run doesn't change what this one was made from)
			j.parents = parentsOf(j.inputs, (key) => { try { return parentOf?.(key) ?? null; } catch { return null; } });
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
