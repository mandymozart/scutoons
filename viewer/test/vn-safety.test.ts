// No dead ends in the VN (owner 2026-10-06: "Events queue takeovers have to be safe for accidental dead ends" – by
// logic, not by timers): the box's state machine (box-state.ts), the runner (script-runner.ts) and the page's script
// line (script-queue.ts) – the same code the viewer runs – driven by scripted cases and by random scripts, slice
// triggers and readers. The property: whenever everything has settled, the page is frozen ONLY if something on screen
// answers (a message's click, a choice, a question, a text box's tap); and the reader always gets through.
//   npm test -w viewer
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { VNEvent } from '@scutoons/content';
import { createBox, createBoxData } from '../src/lib/visual-novel/box-state.ts';
import { runEvents, runTrigger, type RunnerEnv } from '../src/lib/visual-novel/script-runner.ts';
import { createScriptQueue } from '../src/lib/visual-novel/script-queue.ts';
import { jumpProblem } from '../src/lib/visual-novel/jump-to.ts';

/** everything that can run has run (promise chains settle within a few macrotasks) */
async function settle() {
	for (let i = 0; i < 30; i++) await new Promise((r) => setImmediate(r));
}

/** a seeded random (the failures are reproducible) */
function rng(seed: number) {
	let s = seed >>> 0 || 1;
	return () => ((s = (Math.imul(s, 1664525) + 1013904223) >>> 0) / 2 ** 32);
}

/** one reader page: the pure box, a stage stub, a game, the page's script line */
function harness(labels: string[] = []) {
	const data = createBoxData<unknown, any>();
	/** who waits for the waiting text boxes (the context keeps every one – a script's and a run's) */
	const textBoxes: (() => void)[] = [];
	const box = createBox(data, { textBoxWaiting: () => textBoxes.length > 0 });
	const vars: Record<string, string | number> = {};
	const navigations: string[] = [];
	let gone = false;
	const vn = {
		...box,
		findInScene: (id: string) => (id === 'ghost' ? null : { name: id }),
		joinScene: () => {},
		leaveScene: () => {},
		showTextBox: () => true,
		setTextBoxResolver: (r: () => void) => void textBoxes.push(r),
		holds: box.holds
	};
	const game = { getVariable: (n: string) => vars[n], setVariable: (n: string, v: string | number) => void (vars[n] = v), state: { toonId: 't' } };
	const env: RunnerEnv = {
		readerKnown: () => Promise.resolve(),
		setReaderName: () => Promise.resolve(),
		readerNameMax: 32,
		setVariable: (a) => {
			if ('set' in a) vars[a.set.variable] = a.set.value;
			else vars[a.add.variable] = (Number(vars[a.add.variable]) || 0) + a.add.value;
		},
		cleanText: (t, limit) => t.trim().slice(0, limit)
	};
	const place = { toon: 't', chapter: 1, labels };
	const queue = createScriptQueue({ idle: box.idle, subscribe: box.subscribe, onChange: (s) => box.pageState(s) });
	// a page script's jump: the page goes away (goto)
	const leave = () => {
		gone = true;
		queue.cancel();
		box.reset();
	};
	const deps = (navigate = (url: string) => (navigations.push(url), leave())) => ({
		vn,
		game,
		env,
		navigate,
		jumpProblem: (target: any) => jumpProblem(target, place),
		cancelled: () => gone,
		currentLabel: () => 'here'
	});
	return {
		box,
		data,
		vars,
		navigations,
		queue,
		/** a page script through the line */
		script(id: string, events: VNEvent[]) {
			queue.register(id, true);
			queue.request(id, () => runEvents(events, 0, { ...deps(), yieldToRuns: true }));
		},
		/** a slice trigger's run (its events, or one divert into `script`) */
		trigger(events: VNEvent[], script: VNEvent[] = []) {
			// a run that jumps hands the URL back – the slice component navigates: the page goes away
			return runTrigger(events, { ...deps(), vn, script, label: 'here' }).then((url) => {
				if (url) {
					navigations.push(url);
					this.leave();
				}
				return url;
			});
		},
		/** the reader's possible moves now */
		moves(): string[] {
			const out: string[] = [];
			if (data.question) out.push('answer', 'skip');
			if (data.choicesVisible && data.pendingChoices.length) data.pendingChoices.forEach((_, i) => out.push(`pick:${i}`));
			if (data.messageVisible && data.message.wait && box.awaitingClick) out.push('click');
			if (textBoxes.length) out.push('tap');
			return out;
		},
		/** a move as the viewer's components do it (message.svelte, choices.svelte, text-box-wait.svelte) */
		move(m: string) {
			if (m === 'answer') box.answer('Zed');
			else if (m === 'skip') box.answer(null);
			else if (m === 'click') box.continueFromWait();
			else if (m === 'tap') for (const resolve of textBoxes.splice(0)) resolve(); else if (m.startsWith('pick:')) {
				const choice = data.pendingChoices[Number(m.slice(5))];
				const actions = Array.isArray(choice.onSelect) ? choice.onSelect : choice.onSelect ? [choice.onSelect] : [];
				let jumped = false;
				for (const a of actions) {
					if (a.divert) box.setPendingDivert(a.divert.to);
					if (a.jumpTo && !jumpProblem(a.jumpTo.target, place)) {
						box.setNavigationTriggered();
						jumped = true;
						break;
					}
				}
				box.clearChoices();
				if (jumped) this.leave();
			}
		},
		/** the page goes away (a jump): everything stops */
		leave,
		get gone() {
			return gone || navigations.length > 0;
		}
	};
}

const msg = (text: string, wait = true, extra: object = {}) => ({ type: 'showMessage', params: { text, wait }, ...extra }) as VNEvent;
const choice = (label: string, onSelect?: object, cond?: object) => ({ type: 'addChoice', params: { label, ...(onSelect ? { onSelect } : {}) }, ...(cond ? { if: cond } : {}) }) as VNEvent;
const show = { type: 'showChoices', params: {} } as VNEvent;
const unset = (v: string) => ({ variable: v, op: 'set' });

// ── the deterministic rules ────────────────────────────────────────────────────────────────────────────────────────

test('every choice hidden by its condition: the question asks nothing, the script goes on (no freeze)', async () => {
	const h = harness();
	h.script('a', [choice('A', undefined, unset('nope')), choice('B', undefined, unset('nada')), show, msg('after')]);
	await settle();
	assert.equal(h.data.choicesVisible, false);
	assert.equal(h.data.message.text, 'after', 'the script went on');
	assert.deepEqual(h.moves(), ['click']);
	h.move('click');
	await settle();
	assert.equal(h.box.frozen(), false);
	assert.equal(h.queue.done, true);
});

test('a trigger parks the script’s waiting message and always gives it back – also when it fails', async () => {
	const h = harness();
	h.script('a', [msg('script'), msg('script 2')]);
	await settle();
	assert.equal(h.data.message.text, 'script');
	// a trigger with a message of its own
	const run = h.trigger([msg('trigger')]);
	await settle();
	assert.equal(h.data.message.text, 'trigger');
	assert.ok(h.box.parked);
	h.move('click');
	await run;
	await settle();
	assert.equal(h.data.message.text, 'script', 'the script’s message is back');
	assert.deepEqual(h.moves(), ['click'], 'and its click works');
	// a trigger that throws on its way (a broken event): the message comes back all the same
	const throwing = { type: 'joinScene', get params(): never { throw new Error('broken event'); } } as unknown as VNEvent;
	await h.trigger([throwing, msg('never')]);
	await settle();
	assert.equal(h.data.message.text, 'script');
	assert.equal(h.box.parked, false);
	assert.equal(h.data.runs, 0);
	h.move('click');
	await settle();
	assert.equal(h.data.message.text, 'script 2');
});

test('a run whose events throw ends and hands the box back (finally)', async () => {
	const h = harness();
	h.script('a', [msg('script')]);
	await settle();
	const throwing = { type: 'setVariable', get params(): never { throw new Error('boom'); } } as unknown as VNEvent;
	await h.trigger([throwing]);
	await settle();
	assert.equal(h.data.runs, 0);
	assert.equal(h.box.holder, 0);
	assert.deepEqual(h.moves(), ['click'], 'the script’s message answers again');
});

test('two triggers at once take turns – the second never writes over the first’s message', async () => {
	const h = harness();
	h.script('a', [msg('script')]);
	await settle();
	const one = h.trigger([msg('one')]);
	const two = h.trigger([msg('two')]);
	await settle();
	assert.equal(h.data.message.text, 'one');
	h.move('click');
	await one;
	await settle();
	assert.equal(h.data.message.text, 'two', 'the second run after the first');
	h.move('click');
	await two;
	await settle();
	assert.equal(h.data.message.text, 'script');
	assert.equal(h.data.runs, 0);
});

test('a trigger waits while a question is open (it never talks over an askText)', async () => {
	const h = harness();
	h.script('a', [{ type: 'askText', params: { text: 'Name?', variable: 'hero' } } as VNEvent, msg('hi {hero}')]);
	await settle();
	assert.ok(h.data.question);
	const run = h.trigger([msg('trigger')]);
	await settle();
	assert.equal(h.data.message.text, 'Name?', 'the trigger waits for its turn');
	h.move('answer');
	await settle();
	// the script paused (a run holds the box), the trigger shows its message
	assert.equal(h.data.message.text, 'trigger');
	h.move('click');
	await run;
	await settle();
	assert.equal(h.data.message.text, 'hi Zed');
});

test('a script running on (not waiting for the reader) pauses while a trigger holds the box', async () => {
	const h = harness();
	h.script('a', [{ type: 'showTextBox', params: { dialog: 'b', wait: true } } as VNEvent, msg('after the box')]);
	await settle();
	assert.deepEqual(h.moves(), ['tap']);
	const run = h.trigger([msg('trigger')]);
	await settle();
	h.move('tap'); // the box's effect ends while the trigger holds the box
	await settle();
	assert.equal(h.data.message.text, 'trigger', 'the script waits for the run, it doesn’t write over it');
	h.move('click');
	await run;
	await settle();
	assert.equal(h.data.message.text, 'after the box');
});

test('a jump to a group that is not in this chapter stays: the script goes on, the page ends', async () => {
	const h = harness(['here', 'there']);
	h.script('a', [{ type: 'jumpTo', params: { target: { type: 'chapter', value: '1', label: 'nowhere' } } } as VNEvent, msg('stayed')]);
	await settle();
	assert.equal(h.navigations.length, 0);
	assert.equal(h.data.message.text, 'stayed');
	h.move('click');
	await settle();
	assert.equal(h.queue.done, true, 'the page is done: the follow-up bar comes');
	assert.equal(h.box.frozen(), false);
	// a jump that exists goes
	const g = harness(['here', 'there']);
	g.script('a', [{ type: 'jumpTo', params: { target: { type: 'chapter', value: '1', label: 'there' } } } as VNEvent]);
	await settle();
	assert.deepEqual(g.navigations, ['/t/chapter/1?label=there']);
});

test('jumpProblem: only what this page can tell', () => {
	const place = { toon: 'catpope', chapter: 2, labels: ['a', 'b'] };
	assert.equal(jumpProblem({ type: 'chapter', value: '2', label: 'a' } as any, place), null);
	assert.match(jumpProblem({ type: 'chapter', value: '2', label: 'x' } as any, place) ?? '', /no group "x"/);
	assert.equal(jumpProblem({ type: 'chapter', value: '2', anyOf: ['x', 'b'] } as any, place), null, 'one of them is there');
	assert.match(jumpProblem({ type: 'chapter', value: '2', anyOf: ['x', 'y'] } as any, place) ?? '', /none of the groups/);
	assert.equal(jumpProblem({ type: 'chapter', value: '3', label: 'x' } as any, place), null, 'another chapter: not judged here');
	assert.equal(jumpProblem({ type: 'chapter', value: '2', toon: 'toptoy', label: 'x' } as any, place), null, 'another toon');
	assert.equal(jumpProblem({ type: 'toon', value: 'x' } as any, place), null);
	assert.equal(jumpProblem({ type: 'chapter', value: '2', label: 'x' } as any, null), null, 'no place known');
});

test('a choice that jumps into nothing: no jump, the script goes on after the question', async () => {
	const h = harness(['here']);
	h.script('a', [choice('Away', { jumpTo: { target: { type: 'chapter', value: '1', label: 'gone' } } }), show, msg('still here')]);
	await settle();
	h.move('pick:0');
	await settle();
	assert.equal(h.navigations.length, 0);
	assert.equal(h.data.message.text, 'still here');
});

test('the page goes away mid-run: the box is reset, nothing waits any more', async () => {
	const h = harness();
	h.script('a', [msg('one')]);
	await settle();
	void h.trigger([msg('t')]);
	await settle();
	h.leave();
	await settle();
	assert.equal(h.box.frozen(), false);
	assert.equal(h.data.runs, 0);
});

// ── the property: random scripts, triggers and readers ───────────────────────────────────────────────────────────────

/** a random event list; diverts only lead forward (a reader always gets to the end) */
function randomEvents(r: () => number, n: number, points: string[]): VNEvent[] {
	const out: VNEvent[] = [];
	const pick = <T>(list: T[]) => list[Math.floor(r() * list.length)];
	const cond = () => (r() < 0.4 ? { if: r() < 0.5 ? unset(pick(['x', 'y', 'nope'])) : { variable: 'x', op: '<', value: 3 } } : {});
	let open = 0;
	for (let i = 0; i < n; i++) {
		const later = points.filter((p) => !out.some((e: any) => e.type === 'label' && e.params.name === p));
		const k = r();
		if (k < 0.18) out.push(msg(`m${i}`, r() < 0.8, cond()));
		else if (k < 0.3) (out.push(choice(`c${i}`, r() < 0.3 && later.length ? { divert: { to: pick(later) } } : r() < 0.15 ? { jumpTo: { target: { type: 'chapter', value: '1', label: pick(['gone', 'here']) } } } : { set: { variable: 'x', value: 1 } }, (cond() as { if?: object }).if)), open++);
		else if (k < 0.4) (out.push({ ...show, ...cond() } as VNEvent), (open = 0));
		else if (k < 0.47) out.push({ type: 'askText', params: { text: `q${i}`, variable: 'y', ...(r() < 0.5 ? { default: 'D' } : {}) }, ...cond() } as VNEvent);
		else if (k < 0.55 && later.length) out.push({ type: 'divert', params: { to: pick(r() < 0.8 ? later : ['missing']) }, ...cond() } as VNEvent);
		else if (k < 0.62 && later.length) out.push({ type: 'label', params: { name: later[0] } } as VNEvent);
		else if (k < 0.68) out.push({ type: 'setVariable', params: { variable: pick(['x', 'y']), value: 1, op: 'add' } } as VNEvent);
		else if (k < 0.74) out.push({ type: 'showTextBox', params: { dialog: 'b', wait: r() < 0.6 } } as VNEvent);
		else if (k < 0.79) out.push({ type: 'hideMessage' } as VNEvent);
		else if (k < 0.83) out.push({ type: 'jumpTo', params: { target: { type: 'chapter', value: '1', label: pick(['gone', 'gone', 'here']) }, wait: r() < 0.5 }, ...cond() } as VNEvent);
		else if (k < 0.88) out.push(msg(`speaker${i}`, true, { params: { text: `s${i}`, wait: true, character: pick(['cat', 'ghost']) } }));
		else out.push(msg(`n${i}`, false));
	}
	void open;
	return out;
}

test('random scripts, triggers and readers: frozen only with something to answer, and the reader always gets through', async () => {
	const CASES = Number(process.env.VN_SAFETY_CASES ?? 1500);
	let moves = 0;
	for (let seed = 1; seed <= CASES; seed++) {
		const r = rng(seed);
		const h = harness(['here']);
		const scripts = 1 + Math.floor(r() * 3);
		const bodies: VNEvent[][] = [];
		for (let s = 0; s < scripts; s++) {
			const body = randomEvents(r, 4 + Math.floor(r() * 14), ['p1', 'p2', 'p3']);
			bodies.push(body);
			h.script(`s${s}`, body);
		}
		const runs: Promise<unknown>[] = [];
		let step = 0;
		for (; step < 400; step++) {
			await settle();
			const trace = () => `seed ${seed}, step ${step}: ${JSON.stringify({ moves: h.moves(), data: { ...h.data, question: !!h.data.question }, parked: h.box.parked, holder: h.box.holder })}\n${JSON.stringify(bodies)}`;
			// THE property: a frozen page always has something to answer
			if (h.box.frozen()) assert.ok(h.box.answerable(), `frozen with nothing to answer – ${trace()}`);
			if (h.gone) break;
			const possible = h.moves();
			// now and then a slice trigger fires (its own events, or a divert into the first script)
			if (r() < 0.15 && runs.length < 6) {
				runs.push(h.trigger(r() < 0.3 ? ([{ type: 'divert', params: { to: 'p1' } }] as VNEvent[]) : randomEvents(r, 1 + Math.floor(r() * 4), []), bodies[0]));
				continue;
			}
			if (!possible.length) {
				assert.ok(!h.box.frozen(), `stuck – ${trace()}`);
				if (h.queue.done && h.data.runs === 0) break;
				assert.fail(`not done and nothing to do – ${trace()}`);
			}
			h.move(possible[Math.floor(r() * possible.length)]);
			moves++;
		}
		assert.ok(step < 400, `seed ${seed}: the reader never got through`);
		if (!h.gone) {
			await Promise.all(runs);
			await settle();
			assert.equal(h.box.frozen(), false, `seed ${seed}: frozen at the end`);
		}
	}
	assert.ok(moves > CASES, `the readers moved (${moves})`);
});

test('migrate: the story waits for the welcome; no `who` = the reader (agents/MAP.md §8.10)', async () => {
	const h = harness();
	const moves: string[] = [];
	let close: () => void = () => {};
	const events: VNEvent[] = [
		{ type: 'migrate', params: { region: 'alps' } },
		{ type: 'migrate', params: { region: 'dunes', who: 'boy', silent: true } },
		{ type: 'setVariable', params: { variable: 'after', value: 1, op: 'set' } } as VNEvent
	];
	const env: RunnerEnv = {
		readerKnown: () => Promise.resolve(),
		setReaderName: () => Promise.resolve(),
		readerNameMax: 32,
		setVariable: (a) => void ('set' in a && (h.vars[a.set.variable] = a.set.value)),
		cleanText: (t) => t,
		migrate: (who, region, o) => {
			moves.push(`${who ?? 'reader'}→${region}${o.silent ? ' (silent)' : ''}`);
			return o.silent ? Promise.resolve() : new Promise<void>((r) => (close = r));
		}
	};
	const vn = { ...h.box, findInScene: () => null, joinScene: () => {}, leaveScene: () => {}, showTextBox: () => true, setTextBoxResolver: () => {}, holds: h.box.holds };
	const run = runEvents(events, 0, { vn, game: { getVariable: () => undefined, setVariable: () => {}, state: { toonId: 't' } }, env, navigate: () => {}, cancelled: () => false, currentLabel: () => 'here' } as any);
	await settle();
	assert.deepEqual(moves, ['reader→alps'], 'waits at the welcome');
	assert.equal(h.vars.after, undefined);
	close();
	await run;
	assert.deepEqual(moves, ['reader→alps', 'boy→dunes (silent)']);
	assert.equal(h.vars.after, 1, 'then the story goes on');
});

test('setAvatarLocation: reading moves the avatar, after the unlock before it (owner 2026-10-15)', async () => {
	const h = harness();
	const log: string[] = [];
	const events: VNEvent[] = [
		{ type: 'unlockLocation', params: { location: 'cathedral' } },
		{ type: 'setAvatarLocation', params: { location: 'cathedral' } },
		{ type: 'setVariable', params: { variable: 'after', value: 1, op: 'set' } } as VNEvent
	];
	const env: RunnerEnv = {
		readerKnown: () => Promise.resolve(),
		setReaderName: () => Promise.resolve(),
		readerNameMax: 32,
		setVariable: (a) => void ('set' in a && (h.vars[a.set.variable] = a.set.value)),
		cleanText: (t) => t,
		unlockPlace: (location) => (log.push(`unlock ${location}`), Promise.resolve()),
		setAvatarLocation: (location) => (log.push(`avatar ${location}`), Promise.resolve())
	};
	const vn = { ...h.box, findInScene: () => null, joinScene: () => {}, leaveScene: () => {}, showTextBox: () => true, setTextBoxResolver: () => {}, holds: h.box.holds };
	await runEvents(events, 0, { vn, game: { getVariable: () => undefined, setVariable: () => {}, state: { toonId: 't' } }, env, navigate: () => {}, cancelled: () => false, currentLabel: () => 'here' } as any);
	assert.deepEqual(log, ['unlock cathedral', 'avatar cathedral']);
	assert.equal(h.vars.after, 1);
});
