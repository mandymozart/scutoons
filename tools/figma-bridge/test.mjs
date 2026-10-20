// Self-test of the bridge without Figma (#78: the self-healing bridge). Relays on spare ports, stand-in plugins over a
// real WebSocket (Node's global WebSocket – it answers the relay's pings by itself) and over a raw TCP socket (which
// doesn't), the CLI's and lib.mjs's clients, the supervisor as a detached process, and plugin/code.js itself against a
// small fake of the Plugin API. Checks:
//   relay   framing (small, > 64 KiB both ways), errors with codes, /status, local tools only (an Origin or text/plain
//           on /cmd, a web page's Origin on /plugin: refused; a second socket without a hello can't take over and is
//           closed), ONE queue (FIFO, one call at the plugin; ping passes it), a late answer after a timeout keeps the
//           queue waiting (a lost one: the plugin's idle state frees it), queue_timeout, a caller that goes away takes its
//           queued call along, a disconnect answers every call in flight or queued at once, heartbeat → stale, a second
//           plugin window replaces the first (close code 4001), a second relay on the port says "already running"
//           (exit 0), a foreign server there → exit 2, the log (ISO lines, rotation)
//   client  figma() waits for a plugin that comes back (one line), after a disconnect mid-call too, and for one in the
//           run's file (expectFile); timeout → waits until not busy → retries with the timeout doubled; Figma busy too
//           long → plugin_stuck (+ the "frozen" line when pings go unanswered); preflight (code version – waits for the
//           right one; memory); no relay → starts the supervisor, which restarts a killed relay; --detach twice =
//           "already running"; --stop; a lock left by a crash / reboot (its pid alive, but no supervisor) doesn't block
//           a start, a fresh one names its reason, and --stop never kills a pid that is no supervisor
//   plugin  code.js: hello + version + file, the sweep of leftover temporary frames on start, its own queue (one export
//           at a time, ping answered while busy), region partial + layers (layer shells with the layer's look, only the
//           nodes that reach into the slice, placed right, the frame's fills, no temporary frame left), a node on a page
//           that is not loaded (dynamic-page) exports, applyDraft (layers, order, lock, idempotence) – also end to end
//   modules command modules (#93): the plugin's `run` compiles once per hash (an edited module → a new hash → compiled
//           again), a syntax error → plugin_error; the relay reads commands/<name>.js per call (by its command name or
//           `run` { name }), never source from the caller; an unknown module / a bad name → bad_request, the queue goes on
//   node tools/figma-bridge/test.mjs

import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { createServer, request } from 'node:http';
import { connect } from 'node:net';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { RELAY_VERSION, makeLogger, resolveCall, startRelay } from './relay.mjs';
import { bridge } from './cli.mjs';
import { enoughMemory, figma, figmaOnce, figmaPreflight, figmaStatus } from '../../scripts/storyboard/lib.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const PORT = 3999, PORT_HB = 3997, PORT_FOREIGN = 3996, PORT_SUP = 3995, PORT_LOCK = 3994, PORT_LOCK2 = 3993; // 3998: test-mcp.mjs
const NO_ANSWER = Symbol('no answer'); // a stand-in handler's result: the answer is lost
const tmp = mkdtempSync(join(tmpdir(), 'scutoons-bridge-test-'));
process.env.SCUTOONS_FIGMA_LOG_DIR = tmp; // the supervisor's log and lock go here, not into logs/
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const done = [];
let tid = 0; // ask(): command ids
const step = (name) => { done.push(name); console.log(`  ok  ${name}`); };
const node = (script, args) => new Promise((ok) => {
	const c = spawn(process.execPath, [resolve(here, script), ...args], { stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true });
	let out = '';
	c.stdout.on('data', (d) => (out += d));
	c.stderr.on('data', (d) => (out += d));
	c.on('exit', (code) => ok({ code, out }));
});

/** a stand-in plugin over a real WebSocket: handlers by command, a log of starts / ends, how many ran at once */
async function standIn(port, { version = 'test-1', file = 'Cat-Pope', handlers = {} } = {}) {
	const ws = new WebSocket(`ws://127.0.0.1:${port}/plugin`);
	const p = { ws, log: [], running: 0, max: 0, closeCode: null };
	const fns = { ping: () => ({ file, page: 'S1C3', version }), echo: (a) => a, slow: async (a) => { await sleep(a.ms); return { slept: a.ms, k: a.k }; }, lose: async () => { await sleep(150); return NO_ANSWER; }, ...handlers };
	ws.addEventListener('message', async (e) => {
		const msg = JSON.parse(e.data);
		const heavy = msg.cmd !== 'ping';
		p.log.push({ at: Date.now(), ev: 'start', cmd: msg.cmd, k: msg.args?.k });
		if (heavy) { p.running++; p.max = Math.max(p.max, p.running); }
		let answer;
		try {
			if (!fns[msg.cmd]) throw new Error(`unknown command ${msg.cmd}`);
			answer = { id: msg.id, ok: true, result: await fns[msg.cmd](msg.args) };
		} catch (err) { answer = { id: msg.id, ok: false, error: err.message }; }
		if (heavy) p.running--;
		p.log.push({ at: Date.now(), ev: 'end', cmd: msg.cmd, k: msg.args?.k });
		if (ws.readyState === 1 && answer.result !== NO_ANSWER) ws.send(JSON.stringify(answer));
	});
	ws.addEventListener('close', (e) => { p.closeCode = e.code; });
	await new Promise((ok, fail) => { ws.addEventListener('open', ok); ws.addEventListener('error', fail); });
	ws.send(JSON.stringify({ hello: { version, file, busy: null, queue: 0 } }));
	await sleep(30);
	p.close = async () => { if (ws.readyState !== 3) { ws.close(); await once(ws, 'close'); } await sleep(50); };
	return p;
}
const at = (p, ev, k) => p.log.find((e) => e.ev === ev && e.k === k)?.at;

const supervised = new Set(); // ports whose supervisor a failed check must stop
try {
	// ── relay: basics ─────────────────────────────────────────────────────────────────────────────────────────────
	const quiet = [];
	const mods = join(tmp, 'commands'); // the relay's commands/ for this test
	mkdirSync(mods);
	const relay = await startRelay({ port: PORT, log: (m) => quiet.push(m), helloWait: 300, commandsDir: mods });
	let r = await bridge('ping', {}, PORT);
	assert.equal(r.ok, false);
	assert.equal(r.code, 'no_plugin');
	assert.match(r.error, /no plugin connected/);
	let s = await figmaStatus(PORT);
	for (const k of ['plugin', 'stale', 'busy', 'queue', 'version', 'file', 'relay']) assert.ok(k in s, `/status has ${k}`);
	assert.deepEqual([s.plugin, s.stale, s.busy, s.queue, s.version, s.file, s.relay], [false, false, null, 0, null, null, RELAY_VERSION]);
	step('no plugin: code no_plugin; /status { plugin, stale, busy, queue, version, file, relay }');

	let p = await standIn(PORT);
	s = await figmaStatus(PORT);
	assert.deepEqual([s.plugin, s.stale, s.version, s.file, s.busy, s.queue], [true, false, 'test-1', 'Cat-Pope', null, 0]);
	r = await bridge('ping', {}, PORT);
	assert.deepEqual(r, { id: r.id, ok: true, result: { file: 'Cat-Pope', page: 'S1C3', version: 'test-1' } });
	const big = 'x'.repeat(200_000); // > 64 KiB both ways (16-bit and 64-bit frame lengths)
	r = await bridge('echo', { big }, PORT);
	assert.equal(r.result.big.length, 200_000);
	r = await bridge('nope', {}, PORT);
	assert.equal(r.code, 'plugin_error');
	assert.match(r.error, /unknown command/);
	assert.ok(quiet.some((l) => /^echo → ok \d+ ms \(in \d+ KB, out \d+ KB, cli\.mjs\)$/.test(l)), 'a log line with ms and bytes');
	step('plugin connected: hello → version; round trip, > 64 KiB frames, plugin_error; log lines with ms / bytes');

	// ── relay: local tools only ───────────────────────────────────────────────────────────────────────────────────
	const post = (headers) => new Promise((ok) => {
		const q = request({ host: '127.0.0.1', port: PORT, method: 'POST', path: '/cmd', agent: false, headers }, (a) => {
			const b = [];
			a.on('data', (d) => b.push(d));
			a.on('end', () => ok({ status: a.statusCode, body: JSON.parse(Buffer.concat(b).toString('utf8')) }));
		});
		q.end(JSON.stringify({ cmd: 'echo', args: { k: 'web' } }));
	});
	let h = await post({ 'content-type': 'text/plain', origin: 'https://evil.example' });
	assert.deepEqual([h.status, h.body.code], [403, 'forbidden'], 'a web page (it sends an Origin)');
	h = await post({ 'content-type': 'text/plain' });
	assert.deepEqual([h.status, h.body.code], [415, 'bad_request'], 'not JSON');
	await sleep(50);
	assert.equal(at(p, 'start', 'web'), undefined, 'neither reached the plugin');
	/** a raw WebSocket upgrade on /plugin with this Origin → { sock, got() } */
	const upgrade = async (origin) => {
		const sock = connect(PORT, '127.0.0.1');
		await once(sock, 'connect');
		const got = [];
		sock.on('data', (d) => got.push(d));
		sock.on('error', () => {});
		sock.write(`GET /plugin HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n${origin ? `Origin: ${origin}\r\n` : ''}\r\n`);
		await sleep(100);
		return { sock, got: () => Buffer.concat(got) };
	};
	let u = await upgrade('https://evil.example');
	assert.match(u.got().toString('latin1'), /^HTTP\/1\.1 403 /, 'a web page\'s socket is refused');
	u.sock.destroy();
	u = await upgrade('null'); // Figma's plugin iframe origin – a second window, but it never says hello
	assert.match(u.got().toString('latin1'), /^HTTP\/1\.1 101 /);
	s = await figmaStatus(PORT);
	assert.deepEqual([s.plugin, s.version], [true, 'test-1'], 'a socket without a hello does not push the plugin out');
	assert.equal((await bridge('echo', { k: 'still' }, PORT)).ok, true);
	await sleep(350);
	assert.ok(u.got().includes(Buffer.from([0x88, 0x0a, 0x03, 0xf0])), 'closed after helloWait with code 1008');
	assert.equal(p.closeCode, null, 'the plugin stays');
	u.sock.destroy();
	assert.ok(quiet.some((l) => /refused POST \/cmd from Origin https:\/\/evil\.example/.test(l)));
	assert.ok(quiet.some((l) => /refused a plugin connection from Origin https:\/\/evil\.example/.test(l)));
	step('local tools only: /cmd with an Origin → 403, not JSON → 415; a web page\'s socket → 403; a second socket without a hello can\'t take over (closed, 1008)');

	// ── relay: the queue ──────────────────────────────────────────────────────────────────────────────────────────
	const calls = [];
	for (const k of ['a', 'b', 'c']) { calls.push(figmaOnce('slow', { ms: 300, k }, { port: PORT })); await sleep(25); }
	s = await figmaStatus(PORT);
	assert.equal(s.busy.cmd, 'slow');
	assert.equal(s.queue, 2);
	let t0 = Date.now();
	r = await figmaOnce('ping', {}, { port: PORT });
	assert.ok(r.ok && Date.now() - t0 < 200, 'ping passes the queue');
	const answers = await Promise.all(calls);
	assert.deepEqual(answers.map((x) => x.result.k), ['a', 'b', 'c']);
	assert.equal(p.max, 1, 'one call at the plugin at a time');
	assert.deepEqual(p.log.filter((e) => e.ev === 'start' && e.cmd === 'slow').map((e) => e.k), ['a', 'b', 'c']);
	assert.ok(at(p, 'start', 'b') >= at(p, 'end', 'a') && at(p, 'start', 'c') >= at(p, 'end', 'b'));
	step('the queue: FIFO, one call at the plugin, ping past it');

	r = await figmaOnce('slow', { ms: 500, k: 'late' }, { port: PORT, timeout: 100 });
	assert.equal(r.code, 'timeout');
	s = await figmaStatus(PORT);
	assert.equal(s.busy.cmd, 'slow');
	assert.equal(s.busy.timedOut, true);
	r = await figmaOnce('echo', { k: 'after' }, { port: PORT });
	assert.ok(r.ok);
	assert.ok(at(p, 'start', 'after') >= at(p, 'end', 'late'), 'the next call waits for the late answer');
	assert.ok(quiet.some((l) => /answered late/.test(l)));
	// the late answer never comes, but the plugin says it is idle: the queue goes on (no reconnect needed)
	r = await figmaOnce('lose', {}, { port: PORT, timeout: 50 });
	assert.equal(r.code, 'timeout');
	assert.equal((await figmaStatus(PORT)).busy.timedOut, true);
	await sleep(200);
	p.ws.send(JSON.stringify({ state: { busy: null, queue: 0 } }));
	await sleep(50);
	assert.equal((await figmaStatus(PORT)).busy, null);
	assert.equal((await figmaOnce('echo', { k: 'on' }, { port: PORT })).ok, true);
	assert.ok(quiet.some((l) => /its answer is lost; the queue goes on/.test(l)));
	step('a timed-out call keeps the plugin busy until its late answer – nothing is sent on top (a lost answer: the idle state frees it)');

	const slow = figmaOnce('slow', { ms: 400, k: 'long' }, { port: PORT });
	await sleep(30);
	r = await figmaOnce('echo', { k: 'impatient' }, { port: PORT, wait: 100 });
	assert.equal(r.code, 'queue_timeout');
	// a caller that goes away (Ctrl+C) while queued
	const gone = request({ host: '127.0.0.1', port: PORT, method: 'POST', path: '/cmd', headers: { 'content-type': 'application/json' } });
	gone.on('error', () => {});
	gone.end(JSON.stringify({ cmd: 'echo', args: { k: 'gone' } }));
	await sleep(60);
	gone.destroy();
	await slow;
	await sleep(100);
	assert.equal(at(p, 'start', 'impatient'), undefined, 'queue_timeout: never sent');
	assert.equal(at(p, 'start', 'gone'), undefined, 'a caller gone: its call is dropped');
	assert.ok(quiet.some((l) => /caller went away, dropped from the queue/.test(l)));
	step('queue_timeout (never sent); a caller that went away takes its queued call along');

	const inflight = figmaOnce('slow', { ms: 3000, k: 'cut' }, { port: PORT });
	await sleep(30);
	const q1 = figmaOnce('echo', { k: 'q1' }, { port: PORT }), q2 = figmaOnce('echo', { k: 'q2' }, { port: PORT });
	await sleep(30);
	t0 = Date.now();
	await p.close();
	const cut = await Promise.all([inflight, q1, q2]);
	assert.deepEqual(cut.map((x) => x.code), ['plugin_disconnected', 'plugin_disconnected', 'plugin_disconnected']);
	assert.ok(Date.now() - t0 < 1000, 'answered at once');
	s = await figmaStatus(PORT);
	assert.deepEqual([s.plugin, s.busy, s.queue], [false, null, 0]);
	step('a disconnect answers the call in flight and the queued ones at once (plugin_disconnected)');

	// ── client: figma() ───────────────────────────────────────────────────────────────────────────────────────────
	let lines = [];
	const say = (m) => lines.push(m);
	setTimeout(async () => { p = await standIn(PORT); }, 600);
	assert.deepEqual(await figma('echo', { k: 'back' }, { port: PORT, poll: 100, say }), { k: 'back' });
	assert.equal(lines.filter((l) => /waiting for the SCUTOONS Bridge plugin – in Figma desktop: Ctrl\+Alt\+P/.test(l)).length, 1);
	assert.match(lines.at(-1), /back after/);
	step('figma(): no plugin → ONE line what to click, waits, goes on when it is back');

	lines = [];
	const was = p;
	setTimeout(async () => { await was.close(); setTimeout(async () => { p = await standIn(PORT, { handlers: { slow: () => ({ fast: true }) } }); }, 300); }, 200);
	assert.deepEqual(await figma('slow', { ms: 5000, k: 'mid' }, { port: PORT, poll: 100, say }), { fast: true });
	assert.equal(lines.filter((l) => /waiting for the SCUTOONS Bridge plugin/.test(l)).length, 1);
	step('figma(): the plugin goes away mid-call → waits for it, sends again');

	lines = [];
	let first = true;
	await p.close();
	// the first send takes 1.3 s (timeout 1 s), the retry 1.2 s – it only fits because the retry gets twice the time
	p = await standIn(PORT, { handlers: { once: async () => { if (first) { first = false; await sleep(1300); return { n: 1 }; } await sleep(1200); return { n: 2 }; } } });
	assert.deepEqual(await figma('once', {}, { port: PORT, timeout: 1000, backoff: [100], poll: 50, say }), { n: 2 });
	const runs = p.log.filter((e) => e.cmd === 'once');
	assert.deepEqual(runs.map((e) => e.ev), ['start', 'end', 'start', 'end']);
	assert.ok(runs[2].at >= runs[1].at + 100, 'retried only after the first answered, plus the backoff');
	assert.equal(p.max, 1);
	assert.ok(lines.some((l) => /once: timeout – the plugin is free again; retry 1\/4 in 0 s \(timeout 2 s\)/.test(l)), lines.join('\n'));
	step('figma(): timeout → waits until not busy → retries after the backoff with the timeout doubled (never on top)');

	// Figma stuck: no late answer, pings unanswered too → ONE "frozen" line, then plugin_stuck (the run stops)
	lines = [];
	let frozen = false;
	await p.close();
	p = await standIn(PORT, { handlers: { hang: () => new Promise(() => {}), ping: () => (frozen ? new Promise(() => {}) : { file: 'Cat-Pope', version: 'test-1' }) } });
	frozen = true;
	await assert.rejects(figma('hang', {}, { port: PORT, timeout: 100, poll: 40, probe: 60, waitPlugin: 1200, say }), (e) => e.code === 'plugin_stuck');
	assert.equal(lines.filter((l) => /Figma answers no ping \(busy with hang .*\) – it looks frozen/.test(l)).length, 1, lines.join('\n'));
	await p.close(); // the frozen one goes (Figma restarted)
	p = await standIn(PORT);
	step('figma(): Figma busy past waitPlugin → plugin_stuck; unanswered pings → one "frozen" line');

	lines = [];
	let pre = await figmaPreflight({ port: PORT, version: 'test-1', minFreeGB: 0, say });
	assert.equal(pre.version, 'test-1');
	const old = p;
	setTimeout(async () => { p = await standIn(PORT, { version: 'test-2' }); }, 400);
	pre = await figmaPreflight({ port: PORT, version: 'test-2', minFreeGB: 0, poll: 100, say });
	assert.equal(pre.version, 'test-2');
	assert.ok(lines.some((l) => /the plugin runs code test-1, code\.js is test-2 – close the SCUTOONS Bridge window/.test(l)));
	await sleep(50);
	assert.equal(old.closeCode, 4001, 'the replaced window is told so (4001) – it does not reconnect');
	await assert.rejects(enoughMemory({ minFreeGB: 1e6, waitPlugin: 200, poll: 50, say }), (e) => e.code === 'low_memory');
	step('preflight: version check waits for the re-run plugin; a second window replaces the first (4001); low memory');

	// the plugin re-run in ANOTHER Figma file (Ctrl+Alt+P in the wrong tab): node ids are per file – wait for the right one
	lines = [];
	await p.close();
	const elsewhere = p = await standIn(PORT, { file: 'Other' });
	setTimeout(async () => { await elsewhere.close(); p = await standIn(PORT); }, 400);
	assert.deepEqual(await figma('echo', { k: 'file' }, { port: PORT, poll: 100, say, expectFile: 'Cat-Pope' }), { k: 'file' });
	assert.equal(lines.filter((l) => /runs in the Figma file "Other", this run needs "Cat-Pope" – open "Cat-Pope" in Figma desktop and run the plugin THERE/.test(l)).length, 1, lines.join('\n'));
	assert.equal(at(elsewhere, 'start', 'file'), undefined, 'nothing sent to the plugin in the other file');
	assert.equal(at(p, 'start', 'file') > 0, true);
	assert.equal((await figmaStatus(PORT)).file, 'Cat-Pope');
	step('figma(): expectFile – a plugin in another file is waited for (one line), the call goes to the right one');

	// ── relay: heartbeat ──────────────────────────────────────────────────────────────────────────────────────────
	const quietHb = [];
	const relayHb = await startRelay({ port: PORT_HB, log: (m) => quietHb.push(m), heartbeat: { every: 100, stale: 300 } });
	const raw = connect(PORT_HB, '127.0.0.1');
	await once(raw, 'connect');
	const rawData = [];
	raw.on('data', (d) => rawData.push(d));
	raw.write('GET /plugin HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n');
	await sleep(150);
	s = await figmaStatus(PORT_HB);
	assert.deepEqual([s.plugin, s.stale], [true, false]);
	await sleep(400);
	s = await figmaStatus(PORT_HB);
	assert.equal(s.stale, true, 'no pong for 300 ms → stale');
	assert.ok(Buffer.concat(rawData).includes(Buffer.from([0x89, 0x00])), 'the relay pings');
	assert.ok(quietHb.some((l) => /plugin stale/.test(l)));
	const ws2 = await standIn(PORT_HB);
	await sleep(500);
	s = await figmaStatus(PORT_HB);
	assert.equal(s.stale, false, 'a socket that answers the pings stays fresh');
	assert.ok(Buffer.concat(rawData).includes(Buffer.from([0x0f, 0xa1])), 'the replaced socket got close code 4001');
	raw.destroy();
	await ws2.close();
	await relayHb.close();
	step('heartbeat: pings every interval, stale without pongs, fresh with them');

	// ── relay: single instance ────────────────────────────────────────────────────────────────────────────────────
	let x = await node('relay.mjs', ['--port', String(PORT), '--no-log-file']);
	assert.equal(x.code, 0, x.out);
	assert.match(x.out, /already running on :3999 \(pid \d+, plugin connected\)/);
	const foreign = createServer((q, a) => { a.writeHead(404); a.end('not a relay'); }).listen(PORT_FOREIGN, '127.0.0.1');
	await once(foreign, 'listening');
	x = await node('relay.mjs', ['--port', String(PORT_FOREIGN), '--no-log-file']);
	assert.equal(x.code, 2, x.out);
	assert.match(x.out, /taken by another program/);
	foreign.close();
	await assert.rejects(startRelay({ port: PORT, log: () => {} }), (e) => e.code === 'EADDRINUSE');
	step('a second relay on the port: "already running", exit 0; a foreign server: exit 2; the bind error has its code');

	// ── the log ───────────────────────────────────────────────────────────────────────────────────────────────────
	const logFile = join(tmp, 'rot.log');
	const lg = makeLogger(logFile, { maxBytes: 300, echo: false });
	for (let i = 0; i < 20; i++) lg(`line ${i} ${'x'.repeat(20)}`);
	assert.ok(existsSync(join(tmp, 'rot.1.log')), 'rotated');
	assert.ok(statSync(logFile).size < 400);
	assert.match(readFileSync(logFile, 'utf8').split('\n')[0], /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}Z line \d+ x+$/);
	step('the log: ISO timestamps, appended, rotated past maxBytes');

	// ── supervisor: started by figma() when there is no relay ─────────────────────────────────────────────────────
	lines = [];
	supervised.add(PORT_SUP);
	await assert.rejects(figma('ping', {}, { port: PORT_SUP, waitPlugin: 1500, poll: 200, say }), (e) => e.code === 'plugin_disconnected');
	assert.match(lines[0], /no relay on :3995 – starting it in the background/);
	s = await figmaStatus(PORT_SUP);
	assert.ok(s?.supervisor, 'the relay runs under the supervisor');
	const lockNow = JSON.parse(readFileSync(join(tmp, `figma-supervisor-${PORT_SUP}.json`), 'utf8'));
	assert.ok(lockNow.pid === s.supervisor && Date.now() - lockNow.beat < 15_000, 'the lock carries a fresh beat');
	process.kill(s.pid); // the supervisor brings it back
	let s2 = null;
	for (let i = 0; i < 60 && !(s2 && s2.pid !== s.pid); i++) { await sleep(250); s2 = await figmaStatus(PORT_SUP); }
	assert.ok(s2 && s2.pid !== s.pid, 'a new relay after the kill');
	assert.equal(s2.supervisor, s.supervisor);
	x = await node('supervise.mjs', ['--detach', '--port', String(PORT_SUP)]);
	assert.match(x.out, /supervisor already running \(pid \d+/);
	x = await node('supervise.mjs', ['--stop', '--port', String(PORT_SUP)]);
	assert.match(x.out, /supervisor \d+ stopped/);
	assert.match(x.out, /relay \d+ stopped/);
	supervised.delete(PORT_SUP);
	assert.equal(await figmaStatus(PORT_SUP), null);
	const supLog = readFileSync(join(tmp, `figma-relay-${PORT_SUP}.log`), 'utf8');
	assert.match(supLog, /supervisor: started \(pid \d+\)/);
	assert.match(supLog, /supervisor: relay exited \(code .*\) after \d+ s – starting it again in 1 s/);
	step('no relay → figma() starts the supervisor (detached, its lock with a beat); it restarts a killed relay; --detach twice; --stop');

	// a lock left by a crash / reboot / hard kill: its pid alive again (Windows reuses pids – here: this test's own pid),
	// its beat an hour old, no relay names it, its command line is no supervise.mjs → it doesn't count
	const lockFile = (port) => join(tmp, `figma-supervisor-${port}.json`);
	writeFileSync(lockFile(PORT_LOCK), JSON.stringify({ pid: process.pid, port: PORT_LOCK, since: '2026-10-19T08:00:00.000Z', beat: Date.now() - 3_600_000 }));
	lines = [];
	supervised.add(PORT_LOCK);
	await assert.rejects(figma('ping', {}, { port: PORT_LOCK, waitPlugin: 1500, poll: 200, say }), (e) => e.code === 'plugin_disconnected');
	s = await figmaStatus(PORT_LOCK);
	assert.ok(s?.supervisor && s.supervisor !== process.pid, 'the stale lock did not block the start');
	assert.equal(JSON.parse(readFileSync(lockFile(PORT_LOCK), 'utf8')).pid, s.supervisor, 'the lock taken over');
	x = await node('supervise.mjs', ['--stop', '--port', String(PORT_LOCK)]);
	assert.match(x.out, /supervisor \d+ stopped/);
	supervised.delete(PORT_LOCK);
	// a fresh lock whose pid is no supervisor (a pid reused within a minute): the start is refused WITH its reason, and
	// --stop leaves that process alone (it would be this test)
	writeFileSync(lockFile(PORT_LOCK2), JSON.stringify({ pid: process.pid, port: PORT_LOCK2, since: '2026-10-20T08:00:00.000Z', beat: Date.now() }));
	await assert.rejects(figma('ping', {}, { port: PORT_LOCK2, waitPlugin: 500, poll: 100, relayWait: 500, say }), (e) => e.code === 'no_relay' && /the supervisor said: supervisor already running \(pid \d+.*NO relay answers on :3993/.test(e.message));
	x = await node('supervise.mjs', ['--stop', '--port', String(PORT_LOCK2)]);
	assert.match(x.out, new RegExp(`the lock names pid ${process.pid}, which is no supervise\\.mjs – left alone`));
	assert.equal(existsSync(lockFile(PORT_LOCK2)), false, '--stop removes the lock');
	step('a lock left by a crash / reboot doesn\'t block a start; a fresh foreign one names its reason; --stop kills only a proven supervisor');

	// ── plugin/code.js against a fake Plugin API ──────────────────────────────────────────────────────────────────
	const TEMP = 'scutoons-export (temporary)';
	let fig = fakeFigma();
	const page2 = fig.createPage();
	page2.name = 'other';
	const left1 = fig.createFrame(); left1.name = TEMP; fig.currentPage.appendChild(left1);
	const left2 = fig.createFrame(); left2.name = TEMP; page2.appendChild(left2);
	fig.root.setPluginData('scutoons-temp', JSON.stringify([left2.id]));
	let plugin = loadPlugin(fig);
	assert.equal(fig.ui.posted[0].hello, plugin.VERSION);
	assert.equal(fig.ui.posted[0].file, 'Cat-Pope');
	assert.deepEqual(fig.ui.posted[0].state, { busy: null, queue: 0 });
	assert.equal((await ask(fig, 'pages')).ok, true);
	assert.ok(left1.removed && left2.removed, 'leftover temporary frames removed on start');
	assert.equal(fig.root.getPluginData('scutoons-temp'), '');
	assert.ok(fig.ui.posted.some((m) => m.note === 'removed 2 leftover export frames'));
	step('plugin: hello + version + file; leftover temporary frames swept on start (current page + registered ones)');

	const art = fig.createFrame(); art.name = 'art'; art.resize(390, 100); fig.currentPage.appendChild(art);
	fig.exportDelay = 60;
	const exports = ['e1', 'e2', 'e3'].map(() => ask(fig, 'export', { nodeId: art.id, format: 'PNG', scale: 1 }));
	const pong = await ask(fig, 'ping');
	assert.equal(pong.result.version, plugin.VERSION);
	assert.equal(pong.result.busy.cmd, 'run', 'ping answered while an export (a module) runs');
	assert.equal(pong.result.queue, 2);
	const exported = await Promise.all(exports);
	assert.ok(exported.every((e) => e.ok));
	assert.equal(fig.exporting.max, 1, 'one export at a time');
	assert.deepEqual(fig.ui.posted.filter((m) => m.state).at(-1).state, { busy: null, queue: 0 });
	fig.exportDelay = 0;
	step('plugin: its own queue (one command at a time), ping answered at once with { busy, version }');

	// region partial + layers: a (dark) draft frame with layer frames, a slice of 624 at y 624
	const draftFrame = fig.createFrame(); draftFrame.name = 'draft'; draftFrame.resize(390, 2000); fig.currentPage.appendChild(draftFrame);
	draftFrame.fills = [{ type: 'SOLID', color: { r: 0, g: 0, b: 0 } }];
	const layer = (name, kids) => {
		const l = fig.createFrame(); l.name = name; l.resize(390, 2000); draftFrame.appendChild(l);
		for (const [kn, y, h, extra] of kids) { const k = fig.createRectangle(); k.name = kn; k.y = y; k.resize(390, h); Object.assign(k, extra ?? {}); l.appendChild(k); }
		return l;
	};
	layer('storyboard', [['s1', 0, 600], ['s2', 600, 700], ['s3', 1400, 600]]);
	layer('guides', [['g1', 624, 1]]);
	layer('text', [['t1', 700, 40]]);
	const lay = layer('layout', [['l1', 900, 100], ['l2', 800, 50, { visible: false }], ['l3', 1000, 50]]);
	Object.assign(lay, { opacity: 0.5, fills: [{ type: 'SOLID', color: { r: 0, g: 0, b: 1 }, opacity: 0.2 }], clipsContent: true }); // a hand-made layer's look
	fig.exports.length = 0;
	const reg = await ask(fig, 'region', { frame: draftFrame.id, x: 0, y: 624, w: 390, h: 624, hide: ['text', 'guides'], hideIds: [lay.children[2].id], partial: true, layers: true, scales: [1, 2, 3] });
	assert.equal(reg.ok, true, reg.error);
	assert.deepEqual(reg.result.map((e) => e.scale), [1, 2, 3]);
	assert.deepEqual(fig.exports.map((e) => e.scale), [1, 2, 3]);
	const shot = fig.exports[0];
	assert.equal(shot.node, TEMP);
	assert.deepEqual(shot.children.map((c) => [c.name, c.y]), [['storyboard', -624], ['layout', -624]], 'a shell per shown layer, in the layer\'s place');
	assert.deepEqual(shot.children.map((c) => c.children.map((k) => [k.name, k.y])), [[['s2', 600]], [['l1', 900]]], 'only what reaches into the slice (in the slice: −24 / 276)');
	assert.deepEqual(shot.fills.map((f) => f.color), [{ r: 1, g: 1, b: 1 }, { r: 0, g: 0, b: 0 }], 'white, then the frame\'s own fills');
	assert.deepEqual([shot.children[1].opacity, shot.children[1].fills.length, shot.children[1].clipsContent], [0.5, 1, true], 'the layer\'s look on its shell');
	assert.deepEqual([shot.children[0].fills, shot.children[0].clipsContent], [[], false], 'a plain layer: a transparent, unclipped shell');
	assert.ok(!fig.currentPage.children.some((c) => c.name === TEMP), 'no temporary frame left');
	assert.equal(fig.root.getPluginData('scutoons-temp'), '');
	assert.equal(draftFrame.children.length, 4, 'the draft is untouched');
	step('plugin: region partial + layers – a shell per layer with its look, only the nodes in the slice, the frame\'s fills, hide / hideIds, no temporary frame left');

	// dynamic-page: a node on a page that is not loaded – that page's `children` throws; export and region still work
	const page3 = fig.createPage(); page3.name = 'not loaded';
	const far = fig.createFrame(); far.name = 'far'; far.resize(390, 1300); page3.appendChild(far);
	const farLayer = fig.createFrame(); farLayer.name = 'storyboard'; farLayer.resize(390, 1300); far.appendChild(farLayer);
	const farArt = fig.createRectangle(); farArt.name = 'a1'; farArt.resize(390, 1300); farLayer.appendChild(farArt);
	page3._kids = page3.children;
	Object.defineProperty(page3, 'children', { get() { throw new Error('Cannot access children on a page that has not been explored'); } });
	r = await ask(fig, 'export', { nodeId: far.id, format: 'PNG', scale: 1 });
	assert.equal(r.ok, true, r.error);
	r = await ask(fig, 'region', { frame: far.id, x: 0, y: 624, w: 390, h: 624, partial: true, layers: true, scales: [1] });
	assert.equal(r.ok, true, r.error);
	assert.ok(!fig.currentPage.children.some((c) => c.name === TEMP), 'no temporary frame left');
	step('plugin: a node on a page that is not loaded (dynamic-page) – export and region work, the box lives on the current page');

	// applyDraft (the draft command) – a small spec always, the real one when figma-draft.mjs made it
	const specs = [{ page: 'T', frame: { name: 'f', x: 0, w: 390, h: 844 }, guides: [{ kind: 'screen', y: 844 }], rail: [{ y: 0, label: 'scene 1' }], shots: [{ id: 'sb-1-1', svg: '<svg width="390" height="300"></svg>', y: 0, texts: [{ text: 'Hallo', x: 10, y: 10, w: 100, variant: 'bubble' }] }] }];
	const specFile = resolve(here, '../../storyboards/_out/s1c3/figma/scene-25.spec.json');
	if (existsSync(specFile)) specs.push(JSON.parse(readFileSync(specFile, 'utf8')));
	for (const spec of specs) {
		r = await ask(fig, 'draft', { spec });
		assert.equal(r.ok, true, r.error);
		assert.equal(r.result.shots, spec.shots.length);
		const again = await ask(fig, 'draft', { spec }); // idempotent: replaced by id, not doubled
		assert.equal(again.result.shots, spec.shots.length);
		assert.equal(again.result.texts, spec.shots.reduce((a, sh) => a + (sh.texts ?? []).length, 0));
		const pg = fig.root.children.find((c) => c.name === spec.page);
		const frame = pg.children.find((n) => n.name === spec.frame.name);
		assert.deepEqual(frame.children.map((c) => c.name), ['storyboard', 'guides', 'text', 'layout']);
		assert.equal(frame.children[0].locked, true);
		assert.equal(frame.children[1].visible, false);
		assert.equal(pg.children.filter((n) => n.name.endsWith('· rail')).length, 1);
	}
	step(`plugin: applyDraft – layers, order, lock, idempotent (${specs.length > 1 ? `+ the real ${specs[1].frame.name}` : 'no real spec – run figma-draft.mjs for that check'})`);

	// slice-guides (#107): only the guides' children and the rail are redrawn – the frame the owner works in stays as it is
	r = await ask(fig, 'draft', { spec: { page: 'G', frame: { name: 's1c3 – draft', x: 0, w: 390, h: 1300 }, guides: [{ kind: 'slice', y: 625 }], rail: [{ y: 0, label: '— scene-25' }, { y: 16, label: 'sb-25-1  luma' }], shots: [{ id: 'sb-25-1', svg: '<svg width="390" height="700"></svg>', y: 0, texts: [{ text: 'Hallo', x: 10, y: 10, w: 100, variant: 'bubble' }] }, { id: 'sb-25-2', svg: '<svg width="390" height="600"></svg>', y: 700 }] } });
	assert.equal(r.ok, true, r.error);
	const gp = fig.root.children.find((c) => c.name === 'G');
	const gf = gp.children.find((n) => n.name === 's1c3 – draft');
	const [gStory, gGuides, gText, gLayout] = gf.children;
	const owners = fig.createRectangle(); gLayout.appendChild(owners); owners.name = 'owner art';
	gText.children[0].characters = 'Hallo, as the owner edited it';
	gf.x = 1472; gf.y = -225; gGuides.visible = true; // the owner moved the frame and shows the guides
	gp.children.find((n) => n.name === 's1c3 – draft · rail').x = 7802; // the rail left where it was (Figma reports a group's box)
	const gLines = [{ y: 624, kind: 'slice' }, { y: 1300 - 24, kind: 'end', label: 'end · +652 px' }, { y: 5000, kind: 'slice' }];
	const before = { size: [gf.width, gf.height], kids: gf.children.map((c) => [c.name, c.width, c.height, c.locked, c.visible]), story: gStory.children.map((c) => c.name), text: gText.children.map((c) => c.characters) };
	const gArgs = { page: 'G', frame: 's1c3 – draft', guides: gLines, rail: [{ y: 0, label: 'scene-25 · 2 slices + 52 px' }, { y: 16, label: 'sb-25-1  render', shot: 'sb-25-1' }, { y: 716, label: 'sb-25-2  render', shot: 'sb-25-2' }], shots: [{ id: 'sb-25-1', y: 0 }, { id: 'sb-25-2', y: 700 }, { id: 'mama-1', y: 1300 }] };
	r = await ask(fig, 'slice-guides', gArgs);
	assert.equal(r.ok, true, r.error);
	assert.deepEqual([r.result.drawn, r.result.guides, r.result.labels, r.result.outside, r.result.rail, r.result.railX, r.result.missing, r.result.hidden], [true, 2, 1, 1, 3, 7802, ['mama-1'], false]);
	assert.deepEqual(gGuides.children.map((c) => c.name), ['slice 624', 'end 1276 · end · +652 px', 'end 1276 label'], 'only the new lines (+ a label), the 625 line gone, none outside the frame');
	assert.deepEqual({ size: [gf.width, gf.height], kids: gf.children.map((c) => [c.name, c.width, c.height, c.locked, c.visible]), story: gStory.children.map((c) => c.name), text: gText.children.map((c) => c.characters) }, before, 'frame, layers (size, order, lock, visibility), storyboard and text untouched');
	assert.deepEqual(gLayout.children.map((c) => c.name), ['owner art'], 'the owner\'s layout untouched');
	const rails = gp.children.filter((n) => n.name === 's1c3 – draft · rail');
	assert.equal(rails.length, 1, 'one rail');
	assert.deepEqual(rails[0].children.map((t) => [t.characters, t.x, t.y]), [['scene-25 · 2 slices + 52 px', 7802, -225], ['sb-25-1  luma', 7802, -209], ['sb-25-2  render', 7802, 491]], 'the group\'s cut; a shot keeps its old label; at the rail\'s x, y from the frame\'s top');
	r = await ask(fig, 'slice-guides', { ...gArgs, guides: [{ y: 100, kind: 'slice' }], shots: [{ id: 'sb-25-2', y: 624 }] });
	assert.deepEqual([r.result.drawn, r.result.moved], [false, ['sb-25-2 at 700, the cut expects 624']]);
	assert.equal(gGuides.children.length, 3, 'a shot not where the cut expects it: nothing drawn');
	r = await ask(fig, 'slice-guides', { ...gArgs, frame: 's1c3 – draft 2026' });
	assert.match(r.error, /no frame "s1c3 – draft 2026" on "G"/, 'the exact name only');
	const twin = fig.createFrame(); twin.name = 's1c3 – draft'; gp.appendChild(twin);
	r = await ask(fig, 'slice-guides', gArgs);
	assert.match(r.error, /2 frames named "s1c3 – draft"/);
	step('module slice-guides: only the guides and the rail redrawn (frame, layers, storyboard, text, layout untouched), the rail kept at its x with its shot labels, a moved shot → nothing drawn, exact name, never two');

	// command modules in the plugin (#93): compiled once per hash, `figma` a global, `lib` the core's helpers
	const mod = (name, hash, src, args) => ask(fig, 'run', { name, hash, src, args });
	const base = (await ask(fig, 'ping')).result; // export / region ran as modules above
	r = await mod('hello', 'h1', 'return { file: figma.root.name, k: args.k, core: lib.VERSION, has: typeof lib.describe };', { k: 2 });
	assert.deepEqual(r.result, { file: 'Cat-Pope', k: 2, core: plugin.VERSION, has: 'function' }, r.error);
	r = await mod('hello', 'h1', undefined, { k: 3 }); // the same hash: no source needed, not compiled again
	assert.equal(r.result.k, 3, r.error);
	const pp = (await ask(fig, 'ping')).result;
	assert.deepEqual([pp.modules - base.modules, pp.compiles - base.compiles, pp.compiler], [1, 1, 'Function']);
	r = await mod('hello', 'h2', 'return { edited: true };');
	assert.deepEqual(r.result, { edited: true });
	assert.equal((await ask(fig, 'ping')).result.compiles - base.compiles, 2, 'a new hash → compiled again');
	r = await mod('bad', 'h3', 'return {');
	assert.equal(r.ok, false);
	assert.match(r.error, /^module bad: /);
	r = await mod('ghost', 'h4');
	assert.match(r.error, /no source for module ghost/);
	r = await mod('throws', 'h5', 'await null; throw new Error("nope " + args.k);', { k: 7 });
	assert.equal(r.error, 'nope 7');
	assert.equal((await ask(fig, 'pages')).ok, true, 'the queue goes on');
	step('modules in the plugin: run compiles once per hash (Function), an edited source recompiles, figma / args / lib in scope; syntax error, no source, a throw → errors');

	// end to end: code.js (on the fake) as THE plugin behind the relay, wired like ui.html
	await p.close();
	fig = fakeFigma();
	plugin = loadPlugin(fig);
	const wire = new WebSocket(`ws://127.0.0.1:${PORT}/plugin`);
	await new Promise((ok, fail) => { wire.onopen = ok; wire.onerror = fail; });
	const forward = (m) => { if (wire.readyState !== 1) return; if (m.hello) wire.send(JSON.stringify({ hello: { version: m.hello, file: m.file, ...m.state } })); else if (m.state) wire.send(JSON.stringify({ state: m.state })); if (m.id) wire.send(JSON.stringify(m)); };
	fig.ui.posted.forEach(forward);
	fig.ui.listeners.push(forward);
	wire.onmessage = (e) => fig.ui.onmessage(JSON.parse(e.data));
	await sleep(50);
	assert.deepEqual([(await figmaStatus(PORT)).version, (await figmaStatus(PORT)).file], [plugin.VERSION, 'Cat-Pope']);
	r = await bridge('draft', { spec: specs[0] }, PORT);
	assert.equal(r.ok, true, r.error);
	r = await bridge('ping', {}, PORT);
	assert.deepEqual([r.result.version, r.result.busy, r.result.file], [plugin.VERSION, null, 'Cat-Pope']);
	s = await figmaStatus(PORT);
	assert.deepEqual([s.busy, s.queue], [null, 0]);
	step('end to end: code.js behind the relay – hello → /status version, draft, ping, state messages');

	// command modules through the relay (#93): the file is read per call – no relay restart, no plugin re-run
	writeFileSync(join(mods, 'where.js'), 'return { file: figma.root.name, page: figma.currentPage.name, k: args.k };');
	r = await bridge('run', { name: 'where', args: { k: 1 } }, PORT);
	assert.deepEqual(r.result, { file: 'Cat-Pope', page: 'S1C3', k: 1 }, r.error);
	r = await bridge('where', { k: 2 }, PORT); // by its own name, like a core command
	assert.equal(r.result.k, 2, r.error);
	assert.equal((await bridge('ping', {}, PORT)).result.compiles, 1, 'the same file: compiled once');
	assert.ok(quiet.some((l) => /^where where@[0-9a-f]{8} → ok /.test(l)), 'the log names the module and its hash');
	writeFileSync(join(mods, 'where.js'), 'return { edited: args.k };');
	r = await bridge('where', { k: 3 }, PORT);
	assert.deepEqual(r.result, { edited: 3 }, 'an edited file runs at once');
	assert.equal((await bridge('ping', {}, PORT)).result.compiles, 2);
	r = await bridge('run', { name: 'where', src: 'return "from the caller"', args: {} }, PORT);
	assert.deepEqual(r.result, {}, 'source from the caller is ignored – the file runs');
	r = await bridge('run', { name: 'missing' }, PORT);
	assert.deepEqual([r.ok, r.code], [false, 'bad_request']);
	assert.match(r.error, /no module commands\/missing\.js/);
	r = await bridge('run', { name: '../relay' }, PORT);
	assert.equal(r.code, 'bad_request', 'a path is no module name');
	writeFileSync(join(mods, 'broken.js'), 'return {');
	r = await bridge('broken', {}, PORT);
	assert.deepEqual([r.ok, r.code], [false, 'plugin_error']);
	assert.match(r.error, /^module broken: /);
	r = await bridge('nope', {}, PORT);
	assert.match(r.error, /unknown command nope/, 'no file, no core command: the plugin says so');
	r = await bridge('pages', {}, PORT);
	assert.equal(r.ok, true, 'the queue goes on');
	wire.close();
	await relay.close();
	step('modules through the relay: commands/<name>.js read per call (by name or run), an edit runs at once (new hash), caller source ignored; missing / bad name → bad_request; syntax error → plugin_error');

	// the repo's own modules: each compiles as a module body, in plugin-sandbox syntax (no `?.`, no object spread)
	const real = readdirSync(resolve(here, 'commands')).filter((f) => f.endsWith('.js'));
	for (const f of real) {
		const src = readFileSync(resolve(here, 'commands', f), 'utf8');
		assert.match(f, /^[a-z][\w-]*\.js$/, `${f}: a module name`);
		new Function(`return (async function (args, lib) {
${src}
})`)();
		const code = src.replace(/\/\/.*$/gm, '').replace(/(['"`])(?:\.|(?!).)*/g, '""');
		assert.ok(!/\?\.(?!\d)/.test(code), `${f}: no ?. in the plugin sandbox`);
		assert.ok(!/\{\s*\.\.\.|,\s*\.\.\.[\w.]+\s*\}/.test(code), `${f}: no object spread in the plugin sandbox`);
	}
	step(`the repo's modules (${real.length}): compile, plugin-sandbox syntax`);

	console.log(`bridge self-test ok – ${done.length} checks`);
} finally {
	for (const port of supervised) await node('supervise.mjs', ['--stop', '--port', String(port)]);
	try { rmSync(tmp, { recursive: true, force: true }); } catch {}
}
process.exit(process.exitCode ?? 0); // stand-in sockets / timers of failed paths must not keep the test alive

// ── the fake Plugin API: just enough for code.js ────────────────────────────────────────────────────────────────
function fakeFigma() {
	let nid = 0;
	const fig = { exportDelay: 0, exporting: { now: 0, max: 0 }, exports: [] };
	const node = (type, extra = {}) => {
		const n = { id: `0:${++nid}`, type, name: '', x: 0, y: 0, width: 0, height: 0, children: [], parent: null, locked: false, visible: true, ...extra };
		n.appendChild = (c) => { if (c.parent) c.parent.children = c.parent.children.filter((k) => k !== c); c.parent = n; n.children.push(c); };
		n.remove = () => { if (n.parent) n.parent.children = n.parent.children.filter((k) => k !== n); n.parent = null; n.removed = true; };
		n.resize = (w, h) => { n.width = w; n.height = h; };
		n.rescale = (sc) => { n.width *= sc; n.height *= sc; };
		n.clone = () => { const c = node(n.type, { name: n.name, x: n.x, y: n.y, width: n.width, height: n.height, visible: n.visible, locked: n.locked }); for (const k of n.children) c.appendChild(k.clone()); return c; };
		n.loadAsync = async () => {};
		n.exportAsync = async (settings) => {
			fig.exporting.now++;
			fig.exporting.max = Math.max(fig.exporting.max, fig.exporting.now);
			const tree = (c) => ({ name: c.name, x: c.x, y: c.y, fills: c.fills, opacity: c.opacity, clipsContent: c.clipsContent, children: c.children.map(tree) });
			fig.exports.push({ node: n.name, scale: settings.constraint?.value, fills: n.fills, children: n.children.map(tree) });
			await sleep(fig.exportDelay);
			fig.exporting.now--;
			return new Uint8Array([137, 80, 78, 71]);
		};
		// positions: x / y relative to the parent, the page at 0 / 0
		Object.defineProperty(n, 'absoluteTransform', { get: () => { let ax = 0, ay = 0; for (let q = n; q && q.type !== 'PAGE' && q.type !== 'DOCUMENT'; q = q.parent) { ax += q.x; ay += q.y; } return [[1, 0, ax], [0, 1, ay]]; } });
		Object.defineProperty(n, 'absoluteBoundingBox', { get: () => ({ x: n.absoluteTransform[0][2], y: n.absoluteTransform[1][2], width: n.width, height: n.height }) });
		Object.defineProperty(n, 'relativeTransform', { get: () => [[1, 0, n.x], [0, 1, n.y]], set: (m) => { n.x = m[0][2]; n.y = m[1][2]; } });
		return n;
	};
	const data = {};
	const root = node('DOCUMENT', { name: 'Cat-Pope' });
	root.getPluginData = (k) => data[k] ?? '';
	root.setPluginData = (k, v) => { data[k] = v; };
	const createPage = () => { const pg = node('PAGE'); root.appendChild(pg); return pg; };
	let current = createPage();
	current.name = 'S1C3';
	// like Figma: a new node lands on the current page
	const onPage = (n) => { current.appendChild(n); return n; };
	const find = (n, id) => { if (n.id === id) return n; for (const c of n._kids ?? n.children) { const f = find(c, id); if (f) return f; } return null; };
	const ui = { posted: [], listeners: [], onmessage: null, postMessage: (m) => { ui.posted.push(m); for (const l of ui.listeners) l(m); } };
	return Object.assign(fig, {
		root, ui, editorType: 'figma', currentUser: { name: 'Test' },
		get currentPage() { return current; },
		showUI: () => {},
		createPage,
		setCurrentPageAsync: async (pg) => { current = pg; },
		loadFontAsync: async () => {},
		createFrame: () => onPage(node('FRAME')),
		createRectangle: () => onPage(node('RECTANGLE')),
		createText: () => { const t = onPage(node('TEXT')); t.height = 16; return t; },
		createNodeFromSvg: (svg) => { const m = /width="(\d+)" height="(\d+)"/.exec(svg); return onPage(node('FRAME', { width: +m[1], height: +m[2] })); },
		group: (nodes, parent) => { const g = node('GROUP'); parent.appendChild(g); nodes.forEach((k) => g.appendChild(k)); return g; },
		createImage: () => ({ hash: 'hash' }),
		base64Decode: () => new Uint8Array(),
		base64Encode: (b) => Buffer.from(b).toString('base64'),
		getNodeByIdAsync: async (id) => find(root, id)
	});
}

/** runs plugin/code.js against a fake (as Figma would: figma + __html__ in scope) → its commands and VERSION */
function loadPlugin(fig) {
	const code = readFileSync(resolve(here, 'plugin/code.js'), 'utf8');
	globalThis.figma = fig; // a global in Figma – command modules compile in global scope
	return new Function('figma', '__html__', `${code}\nreturn { commands, VERSION };`)(fig, '<html></html>');
}

/** a command into the plugin (as ui.html passes it; a module of commands/ as the relay sends it) → its answer */
function ask(fig, cmd, args = {}) {
	const out = cmd === 'run' ? { cmd, args } : resolveCall(cmd, args, resolve(here, 'commands'));
	return new Promise((ok) => {
		const id = `t${++tid}`;
		const l = (m) => { if (m.id === id) { fig.ui.listeners.splice(fig.ui.listeners.indexOf(l), 1); ok(m); } };
		fig.ui.listeners.push(l);
		fig.ui.onmessage({ id, cmd: out.cmd, args: out.args });
	});
}
