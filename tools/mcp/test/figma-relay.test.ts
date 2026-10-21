// The Figma relay inside the MCP (#127): the pure parts (framing, modules, when it may start) and the relay itself
// against a stand-in plugin over a real WebSocket – the SAME scenarios run against tools/figma-bridge/relay.mjs, so the
// two speak one protocol (the plugin and the checkout's scripts can't tell them apart). Then the MCP's figma_* tools:
// with nothing on the port they start the MCP's own relay (a member's copy), with relay.mjs running they use it, and
// from a checkout (no auto start) they say how to start relay.mjs. Ports 3986–3992 (tools/figma-bridge/test*.mjs: 3993–3999).
import { strict as assert } from 'node:assert';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { request } from 'node:http';
import { connect } from 'node:net';
import { once } from 'node:events';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { after, describe, test } from 'node:test';
import { pathToFileURL } from 'node:url';
import { ROOT } from '../src/config.ts';
import { commandSource } from '../src/figma-commands.ts';
import { acceptKey, encodeFrame, FrameReader, pluginOrigin, relayAutoStart, resolveCall } from '../src/figma-relay-core.ts';
import { ownRelay, startRelay, stopOwnRelay } from '../src/figma-relay.ts';
import { registerFigmaTools } from '../src/figma.ts';

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const tmp = mkdtempSync(join(tmpdir(), 'scutoons-mcp-relay-'));
after(() => rmSync(tmp, { recursive: true, force: true }));

// ── pure ─────────────────────────────────────────────────────────────────────────────────────────────────────────────

/** a client → server frame (masked, as browsers send) */
function clientFrame(op: number, payload: Buffer, fin = true): Buffer {
	const mask = Buffer.from([1, 2, 3, 4]);
	const n = payload.length;
	const head = n < 126 ? Buffer.from([(fin ? 0x80 : 0) | op, 0x80 | n]) : n < 65536 ? Buffer.from([(fin ? 0x80 : 0) | op, 0x80 | 126, n >> 8, n & 255]) : Buffer.concat([Buffer.from([(fin ? 0x80 : 0) | op, 0x80 | 127]), (() => { const b = Buffer.alloc(8); b.writeBigUInt64BE(BigInt(n)); return b; })()]);
	const body = Buffer.from(payload);
	for (let i = 0; i < n; i++) body[i] ^= mask[i & 3];
	return Buffer.concat([head, mask, body]);
}

describe('framing', () => {
	test('server frames: 7-bit, 16-bit and 64-bit lengths read back', () => {
		for (const n of [0, 5, 125, 126, 65535, 65536, 200_000]) {
			const payload = Buffer.alloc(n, 0x61);
			const frames = new FrameReader().push(encodeFrame(0x1, payload));
			assert.equal(frames.length, 1);
			assert.deepEqual([frames[0].fin, frames[0].op, frames[0].data.length], [true, 1, n]);
		}
	});
	test('masked client frames, split byte by byte and glued together', () => {
		const a = clientFrame(0x1, Buffer.from('{"hello":1}'));
		const b = clientFrame(0x1, Buffer.alloc(70_000, 0x62));
		const all = Buffer.concat([a, b]);
		const r = new FrameReader();
		const got = [];
		for (let i = 0; i < 300; i++) got.push(...r.push(all.subarray(i, i + 1)));
		got.push(...r.push(all.subarray(300)));
		assert.equal(got.length, 2);
		assert.equal(got[0].data.toString(), '{"hello":1}');
		assert.ok(got[1].data.equals(Buffer.alloc(70_000, 0x62)));
	});
	test('a frame over the limit throws', () => {
		assert.throws(() => new FrameReader(10).push(clientFrame(0x1, Buffer.alloc(11))), /a frame of 11 B/);
	});
	test('the accept key (RFC 6455 example) and the plugin Origins', () => {
		assert.equal(acceptKey('dGhlIHNhbXBsZSBub25jZQ=='), 's3pPLMBiTxaQ9kYGzzhZRbK+xOo=');
		assert.deepEqual(['null', undefined, 'https://www.figma.com', 'https://evil.example', 'http://figma.com'].map(pluginOrigin), [true, true, true, false, false]);
	});
});

describe('command modules', () => {
	const src = (name: string) => ({ where: 'return 1;' })[name] ?? null;
	test('a command with a module → run { name, hash, src, args }; ping and core commands as they are', () => {
		const r = resolveCall('where', { k: 1 }, src);
		assert.equal(r.cmd, 'run');
		assert.deepEqual({ ...(r.args as object), hash: undefined }, { name: 'where', hash: undefined, src: 'return 1;', args: { k: 1 } });
		assert.match(r.module!, /^where@[0-9a-f]{8}$/);
		assert.deepEqual(resolveCall('run', { name: 'where', args: { k: 2 } }, src).args, (resolveCall('where', { k: 2 }, src).args));
		assert.deepEqual(resolveCall('read', { nodeId: '1:2' }, src), { cmd: 'read', args: { nodeId: '1:2' } });
		assert.deepEqual(resolveCall('ping', {}, () => 'x'), { cmd: 'ping', args: {} });
	});
	test('run without a module / with a bad name throws', () => {
		assert.throws(() => resolveCall('run', { name: 'nope' }, src), /no module commands\/nope\.js/);
		assert.throws(() => resolveCall('run', { name: '../x' }, src), /a module name/);
	});
	test('the checkout\'s modules are read from disk (the ones relay.mjs sends)', () => {
		assert.match(commandSource('export') ?? '', /exportAsync/);
		assert.equal(commandSource('../relay'), null);
		assert.equal(commandSource('no-such-module'), null);
	});
});

test('relayAutoStart: a member\'s copy yes, a checkout only when asked, off anywhere', () => {
	assert.equal(relayAutoStart({}, true), true);
	assert.equal(relayAutoStart({}, false), false, 'a checkout never by itself (the owner\'s supervise.mjs owns the port)');
	assert.equal(relayAutoStart({ SCUTOONS_FIGMA_RELAY: 'mcp' }, false), true);
	assert.equal(relayAutoStart({ SCUTOONS_FIGMA_RELAY: 'off' }, true), false);
});

// ── the relay against a stand-in plugin – the same scenarios for relay.mjs and the TS port ──────────────────────────

type Started = { close: () => Promise<void> };
type Starter = (port: number, log: (l: string) => void, opts: { modulesDir: string; helloWait: number }) => Promise<Started>;

/** POST /cmd → { status, body } */
async function cmd(port: number, c: string, args: Record<string, unknown> = {}, extra: Record<string, unknown> = {}) {
	const res = await fetch(`http://127.0.0.1:${port}/cmd`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ cmd: c, args, client: 'test', ...extra }) });
	return { status: res.status, body: (await res.json()) as any };
}
const status = async (port: number) => (await (await fetch(`http://127.0.0.1:${port}/status`)).json()) as any;

const NO_ANSWER = Symbol('no answer');
/** a stand-in plugin: handlers by command, a log of starts / ends, how many ran at once */
async function standIn(port: number, { version = 'test-1', file = 'Cat-Pope', handlers = {} as Record<string, (a: any) => unknown> } = {}) {
	const ws = new WebSocket(`ws://127.0.0.1:${port}/plugin`);
	const p = { ws, log: [] as { at: number; ev: string; cmd: string; k?: string; msg?: any }[], running: 0, max: 0, closeCode: null as number | null, close: async () => {} };
	const fns: Record<string, (a: any) => unknown> = {
		ping: () => ({ file, version }),
		echo: (a) => a,
		slow: async (a) => { await sleep(a.ms); return { k: a.k }; },
		lose: async () => { await sleep(150); return NO_ANSWER; },
		run: (a) => ({ module: a.name, hash: a.hash, src: a.src, args: a.args }),
		...handlers
	};
	ws.addEventListener('message', async (e) => {
		const msg = JSON.parse(String(e.data));
		const heavy = msg.cmd !== 'ping';
		p.log.push({ at: Date.now(), ev: 'start', cmd: msg.cmd, k: msg.args?.k, msg });
		if (heavy) { p.running++; p.max = Math.max(p.max, p.running); }
		let answer: any;
		try {
			if (!fns[msg.cmd]) throw new Error(`unknown command ${msg.cmd}`);
			answer = { id: msg.id, ok: true, result: await fns[msg.cmd](msg.args) };
		} catch (err) {
			answer = { id: msg.id, ok: false, error: (err as Error).message };
		}
		if (heavy) p.running--;
		p.log.push({ at: Date.now(), ev: 'end', cmd: msg.cmd, k: msg.args?.k });
		if (ws.readyState === 1 && answer.result !== NO_ANSWER) ws.send(JSON.stringify(answer));
	});
	ws.addEventListener('close', (e) => { p.closeCode = e.code; });
	await new Promise((ok, fail) => { ws.addEventListener('open', ok); ws.addEventListener('error', fail); });
	ws.send(JSON.stringify({ hello: { version, file, busy: null, queue: 0 } }));
	await sleep(40);
	p.close = async () => { if (ws.readyState !== 3) { ws.close(); await once(ws as unknown as NodeJS.EventEmitter, 'close').catch(() => {}); await sleep(60); } };
	return p;
}
const at = (p: Awaited<ReturnType<typeof standIn>>, ev: string, k: string) => p.log.find((e) => e.ev === ev && e.k === k)?.at;

/** the protocol, as relay.mjs answers it – run against both relays */
async function scenarios(start: Starter, port: number, name: string) {
	const logs: string[] = [];
	const modulesDir = mkdtempSync(join(tmp, `${name}-commands-`));
	writeFileSync(join(modulesDir, 'where.js'), "return { where: 'module', args };");
	const relay = await start(port, (l) => logs.push(l), { modulesDir, helloWait: 300 });
	try {
		// no plugin
		let r = await cmd(port, 'ping');
		assert.deepEqual([r.status, r.body.code], [503, 'no_plugin'], `${name}: no plugin`);
		let s = await status(port);
		assert.deepEqual([s.plugin, s.stale, s.busy, s.queue, s.version, s.file], [false, false, null, 0, null, null]);

		// connected: hello, round trip, > 64 KiB both ways, plugin_error
		let p = await standIn(port);
		s = await status(port);
		assert.deepEqual([s.plugin, s.version, s.file], [true, 'test-1', 'Cat-Pope'], `${name}: hello`);
		r = await cmd(port, 'echo', { big: 'x'.repeat(200_000) });
		assert.equal(r.body.result.big.length, 200_000, `${name}: big frames`);
		r = await cmd(port, 'nope');
		assert.deepEqual([r.status, r.body.code], [502, 'plugin_error']);
		assert.match(r.body.error, /unknown command nope/);

		// local tools only
		const post = (headers: Record<string, string>) =>
			new Promise<number>((ok) => {
				const q = request({ host: '127.0.0.1', port, method: 'POST', path: '/cmd', agent: false, headers }, (a) => { a.resume(); a.on('end', () => ok(a.statusCode ?? 0)); });
				q.end(JSON.stringify({ cmd: 'echo', args: { k: 'web' } }));
			});
		assert.equal(await post({ 'content-type': 'text/plain', origin: 'https://evil.example' }), 403);
		assert.equal(await post({ 'content-type': 'text/plain' }), 415);
		const evil = connect(port, '127.0.0.1');
		await once(evil, 'connect');
		const got: Buffer[] = [];
		evil.on('data', (d) => got.push(d));
		evil.on('error', () => {});
		evil.write('GET /plugin HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\nOrigin: https://evil.example\r\n\r\n');
		await sleep(100);
		assert.match(Buffer.concat(got).toString('latin1'), /^HTTP\/1\.1 403 /, `${name}: a web page's socket`);
		evil.destroy();
		// a second socket without a hello can't take over; it is closed with 1008
		const quiet = connect(port, '127.0.0.1');
		await once(quiet, 'connect');
		const qgot: Buffer[] = [];
		quiet.on('data', (d) => qgot.push(d));
		quiet.on('error', () => {});
		quiet.write('GET /plugin HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\nOrigin: null\r\n\r\n');
		await sleep(80);
		assert.match(Buffer.concat(qgot).toString('latin1'), /^HTTP\/1\.1 101 [\s\S]*Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK\+xOo=/);
		assert.equal((await cmd(port, 'echo', { k: 'still' })).body.ok, true);
		await sleep(350);
		assert.ok(Buffer.concat(qgot).includes(Buffer.from([0x88, 0x0a, 0x03, 0xf0])), `${name}: closed with 1008`);
		assert.equal(p.closeCode, null);
		quiet.destroy();

		// the queue: FIFO, one at the plugin, ping past it
		const calls = [];
		for (const k of ['a', 'b', 'c']) { calls.push(cmd(port, 'slow', { ms: 250, k })); await sleep(25); }
		s = await status(port);
		assert.deepEqual([s.busy?.cmd, s.queue], ['slow', 2], `${name}: busy + queue`);
		const t0 = Date.now();
		assert.equal((await cmd(port, 'ping')).body.ok, true);
		assert.ok(Date.now() - t0 < 200, 'ping passes the queue');
		assert.deepEqual((await Promise.all(calls)).map((x) => x.body.result.k), ['a', 'b', 'c']);
		assert.equal(p.max, 1, `${name}: one call at the plugin`);

		// a timed-out call keeps the plugin busy until its late answer
		r = await cmd(port, 'slow', { ms: 400, k: 'late' }, { timeout: 100 });
		assert.deepEqual([r.status, r.body.code], [504, 'timeout']);
		s = await status(port);
		assert.deepEqual([s.busy?.cmd, s.busy?.timedOut], ['slow', true]);
		assert.equal((await cmd(port, 'echo', { k: 'after' })).body.ok, true);
		assert.ok(at(p, 'start', 'after')! >= at(p, 'end', 'late')!, `${name}: the next call waited for the late answer`);
		assert.ok(logs.some((l) => /answered late/.test(l)));
		// a lost answer: the plugin's idle state frees the queue
		r = await cmd(port, 'lose', {}, { timeout: 50 });
		assert.equal(r.body.code, 'timeout');
		await sleep(200);
		p.ws.send(JSON.stringify({ state: { busy: null, queue: 0 } }));
		await sleep(50);
		assert.equal((await status(port)).busy, null, `${name}: the idle state frees a lost answer`);

		// queue_timeout: never sent
		const slow = cmd(port, 'slow', { ms: 300, k: 'long' });
		await sleep(30);
		r = await cmd(port, 'echo', { k: 'impatient' }, { wait: 80 });
		assert.equal(r.body.code, 'queue_timeout');
		await slow;
		await sleep(50);
		assert.equal(at(p, 'start', 'impatient'), undefined);

		// command modules: read from the folder per call, sent as run; caller source ignored; unknown → bad_request
		r = await cmd(port, 'where', { k: 'm' });
		assert.deepEqual([r.body.result.module, r.body.result.src, r.body.result.args], ['where', "return { where: 'module', args };", { k: 'm' }], `${name}: module by command name`);
		const hash1 = r.body.result.hash;
		writeFileSync(join(modulesDir, 'where.js'), 'return 2;');
		r = await cmd(port, 'run', { name: 'where', src: 'evil()', args: { k: 'n' } });
		assert.deepEqual([r.body.result.src, r.body.result.args], ['return 2;', { k: 'n' }], `${name}: an edited module, the caller's source ignored`);
		assert.notEqual(r.body.result.hash, hash1);
		r = await cmd(port, 'run', { name: 'nope' });
		assert.deepEqual([r.status, r.body.code], [400, 'bad_request']);
		assert.equal((await cmd(port, 'echo', { k: 'goes-on' })).body.ok, true, 'the queue goes on');

		// a second plugin window with its hello replaces the first (4001)
		const second = await standIn(port, { version: 'test-2' });
		await sleep(80);
		assert.equal(p.closeCode, 4001, `${name}: the replaced window gets 4001`);
		assert.equal((await status(port)).version, 'test-2');
		p = second;

		// a disconnect answers the call in flight and the queued ones at once
		const inflight = cmd(port, 'slow', { ms: 3000, k: 'cut' });
		await sleep(30);
		const q1 = cmd(port, 'echo', { k: 'q1' });
		await sleep(30);
		const t1 = Date.now();
		await p.close();
		assert.deepEqual((await Promise.all([inflight, q1])).map((x) => x.body.code), ['plugin_disconnected', 'plugin_disconnected'], name);
		assert.ok(Date.now() - t1 < 1000);
		s = await status(port);
		assert.deepEqual([s.plugin, s.busy, s.queue], [false, null, 0]);
	} finally {
		await relay.close();
	}
}

const tsRelay: Starter = (port, log, { modulesDir, helloWait }) => startRelay({ port, log, helloWait, modules: (n) => commandSource(n, modulesDir) });
const mjsRelay: Starter = async (port, log, { modulesDir, helloWait }) => {
	const { startRelay: start } = await import(pathToFileURL(resolve(ROOT, 'tools/figma-bridge/relay.mjs')).href);
	return start({ port, log, helloWait, commandsDir: modulesDir });
};

describe('one protocol', () => {
	test('relay.mjs (the reference)', async () => scenarios(mjsRelay, 3992, 'relay.mjs'));
	test('the MCP\'s relay (figma-relay.ts)', async () => scenarios(tsRelay, 3991, 'figma-relay.ts'));
	test('the TS relay\'s /status names itself; a second one on the port: EADDRINUSE', async () => {
		const r = await startRelay({ port: 3990, modules: () => null });
		try {
			const s = await status(3990);
			assert.deepEqual([s.relay, s.host, s.supervisor, s.pid], ['2026-10-20.3+mcp', 'scutoons-mcp', null, process.pid]);
			await assert.rejects(startRelay({ port: 3990, modules: () => null }), (e: NodeJS.ErrnoException) => e.code === 'EADDRINUSE');
		} finally {
			await r.close();
		}
	});
	test('heartbeat: stale without pongs', async () => {
		const logs: string[] = [];
		const r = await startRelay({ port: 3990, modules: () => null, log: (l) => logs.push(l), heartbeat: { every: 80, stale: 250 } });
		try {
			const raw = connect(3990, '127.0.0.1');
			await once(raw, 'connect');
			const data: Buffer[] = [];
			raw.on('data', (d) => data.push(d));
			raw.write('GET /plugin HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n');
			await sleep(100);
			assert.equal((await status(3990)).stale, false);
			await sleep(350);
			assert.equal((await status(3990)).stale, true);
			assert.ok(Buffer.concat(data).includes(Buffer.from([0x89, 0x00])), 'the relay pings');
			raw.destroy();
		} finally {
			await r.close();
		}
	});
});

// ── the MCP's figma_* tools: own relay, relay.mjs, or the hint ──────────────────────────────────────────────────────

type Tool = { handler: (a: any) => Promise<{ content: { type: string; text?: string }[]; isError?: boolean }> };
function tools(opts: Parameters<typeof registerFigmaTools>[1]) {
	const t: Record<string, Tool> = {};
	registerFigmaTools({ registerTool: (name: string, _def: unknown, handler: Tool['handler']) => (t[name] = { handler }) } as never, opts);
	return t;
}
const PNG = Buffer.from('89504e470d0a1a0a', 'hex').toString('base64');
const figmaPlugin = (port: number) =>
	standIn(port, {
		handlers: {
			pages: () => [{ id: '0:1', name: 'S1C3' }],
			// export is a command module (#93): it arrives as run with the checkout's (or the embedded) source
			run: (a) => {
				assert.equal(a.name, 'export');
				assert.match(a.src, /exportAsync/);
				return { name: 'photo', format: 'PNG', bytes: 8, base64: PNG };
			}
		}
	});

describe('figma_* tools', () => {
	test('a member\'s copy: nothing on the port → its own relay starts; the plugin connects; status and view work', async () => {
		const port = 3989;
		const t = tools({ port, autoStart: true, pluginWait: 300 });
		try {
			let r = await t.figma_status.handler({});
			assert.equal(r.isError, true);
			assert.match(r.content[0].text!, /no plugin is connected/);
			assert.match(r.content[0].text!, /scutoons\.com\/figma-plugin/, 'says where the plugin comes from');
			assert.ok(ownRelay(), 'the MCP started its relay');
			assert.equal((await status(port)).host, 'scutoons-mcp');
			const p = await figmaPlugin(port);
			r = await t.figma_status.handler({});
			assert.equal(r.isError, undefined, r.content[0].text);
			assert.deepEqual(JSON.parse(r.content[0].text!).pages, [{ id: '0:1', name: 'S1C3' }]);
			r = await t.figma_view.handler({ node: '1-2' });
			assert.equal(r.content[1].type, 'image');
			await p.close();
		} finally {
			await stopOwnRelay();
		}
	});

	test('relay.mjs already runs (the owner\'s checkout): it is used, no own relay', async () => {
		const port = 3988;
		const { startRelay: start } = await import(pathToFileURL(resolve(ROOT, 'tools/figma-bridge/relay.mjs')).href);
		const old = await start({ port, log: () => {} });
		try {
			const p = await figmaPlugin(port);
			const r = await tools({ port, autoStart: true }).figma_status.handler({});
			assert.equal(r.isError, undefined, r.content[0].text);
			assert.equal(ownRelay(), null, 'nothing started');
			assert.equal((await status(port)).relay, (await import(pathToFileURL(resolve(ROOT, 'tools/figma-bridge/relay.mjs')).href)).RELAY_VERSION);
			await p.close();
		} finally {
			await old.close();
		}
	});

	test('a checkout without SCUTOONS_FIGMA_RELAY=mcp never starts one – the hint names supervise.mjs', async () => {
		const port = 3987;
		const r = await tools({ port, autoStart: false }).figma_status.handler({});
		assert.equal(r.isError, true);
		assert.match(r.content[0].text!, /supervise\.mjs --detach/);
		assert.equal(ownRelay(), null);
		assert.equal(await fetch(`http://127.0.0.1:${port}/status`).then(() => true, () => false), false, 'the port stays free');
	});
});
