// SCUTOONS Figma bridge – the local relay (no dependencies). Listens on localhost only (127.0.0.1 and ::1):
//   ws://localhost:3056/plugin       the Figma plugin's UI connects here (one plugin at a time; a new one replaces it)
//   POST http://127.0.0.1:3056/cmd   { cmd, args, timeout?, wait?, client? } → the plugin's answer as JSON
//   GET  http://127.0.0.1:3056/status → { plugin, stale, busy, queue, version, file, relay, pid, supervisor, … }
// Nothing leaves the computer. Self-healing rules (#78; the failures they answer: tools/figma-bridge/README.md "Recovery"):
//   - ONE queue (FIFO), one call at the plugin at a time. `ping` is the only exception: it goes at once (the health
//     probe; the plugin answers it even while busy)
//   - a call past its `timeout` is answered `timeout`, but the plugin counts as busy until its late answer arrives (or
//     it reconnects): the next call waits instead of piling heavy work onto a slow Figma (failures B, D, E)
//   - a call that waits longer than `wait` in the queue is answered `queue_timeout` and never sent; a caller that goes
//     away (Ctrl+C) takes its queued call with it
//   - the plugin gone → every call in flight or queued is answered `plugin_disconnected` at once
//   - heartbeat: a WebSocket ping every 10 s, `stale` after 30 s without a pong (or any other frame) = the connection is
//     dead (Figma closed, crashed, asleep). Pongs come from Figma's network layer, not the plugin's code: a plugin stuck
//     in a long step still answers them – lib.mjs notBusy() finds that with application pings
//   - single instance: a taken port with a relay behind it → "already running", exit 0; anything else there → exit 2
//   - command modules (#93): a call whose command has a file tools/figma-bridge/commands/<cmd>.js (or `run` with
//     { name, args }) goes to the plugin's generic `run` as { name, hash, src, args } – the file read from disk at each
//     send, never source from the caller (what runs in Figma = what is in the repo). A new or edited module needs no
//     relay restart and no plugin re-run; a module named like a core command replaces it
//   - local tools only, never a web page: /cmd takes `content-type: application/json` without an Origin header (browsers
//     send one, and JSON needs their preflight, which this relay never answers); /plugin takes the Origin of Figma's
//     plugin iframes (`null`), none, or figma.com. A second plugin window takes over only with its hello
// Answers that are not ok carry a `code`: no_plugin · plugin_disconnected · timeout · queue_timeout · plugin_error ·
// bad_request · forbidden. The log (ISO time, command, node, ms, bytes in / out, close codes) goes to stdout and is appended to
// logs/figma-relay.log (rotated to figma-relay.1.log at ~5 MB). Run it under the supervisor so it outlives terminals
// and Claude sessions: node tools/figma-bridge/supervise.mjs --detach (README.md).
//
//   node tools/figma-bridge/relay.mjs [--port 3056] [--log-file <path> | --no-log-file]

import { createHash, randomUUID } from 'node:crypto';
import { appendFileSync, mkdirSync, readFileSync, renameSync, statSync } from 'node:fs';
import { createServer, request } from 'node:http';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

/** the relay's own version (GET /status → relay) – bump it with a change of behaviour */
export const RELAY_VERSION = '2026-10-20.3';
const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const COMMANDS = resolve(ROOT, 'tools/figma-bridge/commands');
const MODULE_NAME = /^[a-z][\w-]*$/;
const GUID = '258EAFA5-E914-47DA-95CA-C5AB0DC85B11';
const TIMEOUT = 120_000, MAX_TIMEOUT = 900_000; // a caller may ask for longer (a huge draft: up to 15 min)
const WAIT = 600_000; // how long a call may wait in the queue by default
const PING_TIMEOUT = 15_000;
const HEARTBEAT = { every: 10_000, stale: 30_000 };
const MAX_MESSAGE = 512 * 1024 * 1024;
const HTTP = { no_plugin: 503, plugin_disconnected: 503, timeout: 504, queue_timeout: 504, plugin_error: 502, bad_request: 400, forbidden: 403 };
const HELLO_WAIT = 10_000; // a second plugin window must say hello within this to take over
const NO_PLUGIN = 'no plugin connected – in Figma desktop: Ctrl+Alt+P (Run last plugin) or Plugins → Development → SCUTOONS Bridge';

/** the default log and lock files of a port: logs/figma-relay.log, logs/figma-supervisor.json (+ "-<port>" when not 3056) */
export function defaultFiles(port) {
	const dir = process.env.SCUTOONS_FIGMA_LOG_DIR || resolve(ROOT, 'logs');
	const suffix = Number(port) === 3056 ? '' : `-${port}`;
	return { log: resolve(dir, `figma-relay${suffix}.log`), lock: resolve(dir, `figma-supervisor${suffix}.json`) };
}

/**
 * a logger: ISO-timestamped lines to stdout and appended to `file`, rotated to <name>.1.log past maxBytes – by ONE writer
 * only (the relay; the supervisor writes to the same file with rotate: false – two renames at once would put a few
 * lines over the previous 5 MB)
 */
export function makeLogger(file, { maxBytes = 5 * 1024 * 1024, echo = true, prefix = '', rotate = true } = {}) {
	if (file) mkdirSync(dirname(file), { recursive: true });
	return (msg) => {
		const line = `${new Date().toISOString()} ${prefix}${msg}`;
		if (echo) console.log(line);
		if (!file) return;
		try {
			if (rotate && (statSync(file, { throwIfNoEntry: false })?.size ?? 0) > maxBytes) renameSync(file, file.replace(/(\.log)?$/, '.1.log'));
		} catch {} // open elsewhere this moment – the next line rotates
		try { appendFileSync(file, `${line}\n`); } catch {}
	};
}

/** the Origins a plugin socket may come from: Figma's plugin iframes have an opaque origin (`null`); none = a local tool */
const pluginOrigin = (o) => o === undefined || o === 'null' || /^https:\/\/([a-z0-9-]+\.)*figma\.com$/i.test(o);

/** GET /status of whatever listens on 127.0.0.1:port – the parsed answer, or null (nothing there / not JSON) */
export function probe(port, timeout = 3_000) {
	return new Promise((ok) => {
		const req = request({ host: '127.0.0.1', port, path: '/status', agent: false, timeout }, (res) => {
			const chunks = [];
			res.on('data', (c) => chunks.push(c));
			res.on('end', () => { try { ok(JSON.parse(Buffer.concat(chunks).toString('utf8'))); } catch { ok(null); } });
			res.on('error', () => ok(null));
		});
		req.on('timeout', () => req.destroy());
		req.on('error', () => ok(null));
		req.end();
	});
}

const kb = (n) => (n >= 1048576 ? `${(n / 1048576).toFixed(1)} MB` : n >= 1024 ? `${Math.round(n / 1024)} KB` : `${n} B`);
const clamp = (v, d, max) => Math.min(Number(v) > 0 ? Number(v) : d, max);
/** what a call works on, for the log: node / frame id, a region's box, the scales */
const target = (a = {}) => [a.nodeId ?? a.node ?? a.frame ?? (a.page ? `page "${a.page}"` : ''), a.y != null && a.h ? `@${a.x ?? 0},${a.y} ${a.w}×${a.h}` : '', a.scales ? `×${a.scales.join(',')}` : '', a.ids ? `${a.ids.length} ids` : '', a.slices ? `${a.slices.length} slices` : ''].filter(Boolean).join(' ');

/**
 * a call → what goes to the plugin: a command with a module file (or `run` { name, args }) → `run` { name, hash, src,
 * args }, read from `dir` now; anything else as it is. A `run` without such a file throws (code bad_request).
 */
export function resolveCall(cmd, args, dir = COMMANDS) {
	const explicit = cmd === 'run';
	const name = explicit ? args?.name : cmd;
	if (cmd === 'ping' || typeof name !== 'string' || !MODULE_NAME.test(name)) {
		if (explicit) throw new Error(`run: a module name (${MODULE_NAME}), not ${JSON.stringify(name ?? null)}`);
		return { cmd, args };
	}
	let src;
	try { src = readFileSync(resolve(dir, `${name}.js`), 'utf8'); } catch (e) {
		if (explicit) throw new Error(`no module commands/${name}.js${e.code === 'ENOENT' ? '' : ` (${e.code ?? e.message})`}`);
		return { cmd, args }; // a core command (or an unknown one – the plugin says which it has)
	}
	const hash = createHash('sha256').update(src).digest('hex').slice(0, 16);
	return { cmd: 'run', args: { name, hash, src, args: (explicit ? args.args : args) ?? {} }, module: `${name}@${hash.slice(0, 8)}` };
}

export async function startRelay({ port = 3056, log = console.log, heartbeat = HEARTBEAT, helloWait = HELLO_WAIT, commandsDir = COMMANDS } = {}) {
	let plugin = null; // { socket, send, since, lastSeen, stale, version, file, state }
	let busy = null; // the call at the plugin (stays after its timeout until the late answer)
	const queue = []; // calls waiting for the plugin, oldest first
	const side = new Map(); // pings sent past the queue: id → call
	const startedAt = new Date().toISOString();

	const frame = (op, payload) => {
		const n = payload.length;
		let head;
		if (n < 126) head = Buffer.from([0x80 | op, n]);
		else if (n < 65536) head = Buffer.from([0x80 | op, 126, n >> 8, n & 255]);
		else { head = Buffer.alloc(10); head[0] = 0x80 | op; head[1] = 127; head.writeBigUInt64BE(BigInt(n), 2); }
		return Buffer.concat([head, payload]);
	};
	const isStale = () => !!plugin && Date.now() - plugin.lastSeen > heartbeat.stale;

	// ── calls ──
	function answer(c, a) {
		if (c.answered) return;
		c.answered = true;
		clearTimeout(c.timer); clearTimeout(c.waitTimer);
		const ms = Date.now() - c.t0, queued = c.sent ? c.sent - c.t0 : 0;
		log(`${c.cmd}${c.target ? ` ${c.target}` : ''} → ${a.ok ? 'ok' : `${a.code}${a.code === 'plugin_error' ? `: ${a.error}` : ''}`} ${ms} ms (in ${kb(c.bytesIn)}, out ${kb(a.bytes ?? 0)}${queued > 50 ? `, ${queued} ms in the queue` : ''}${c.client ? `, ${c.client}` : ''})`);
		c.done(a);
	}
	const fail = (code, error) => ({ ok: false, code, error, text: JSON.stringify({ ok: false, code, error }) });

	function enqueue(c) {
		if (!plugin) return answer(c, fail('no_plugin', NO_PLUGIN));
		if (c.cmd === 'ping') return sendSide(c);
		c.waitTimer = setTimeout(() => {
			const i = queue.indexOf(c);
			if (i >= 0) queue.splice(i, 1);
			const ahead = busy ? `${busy.cmd}${busy.timedOut ? ' (past its timeout, Figma still working on it)' : ''}` : 'the queue';
			answer(c, fail('queue_timeout', `waited ${Math.round(c.wait / 1000)} s behind ${ahead} – not sent; try again when GET /status says busy: null`));
		}, c.wait);
		queue.push(c);
		pump();
	}

	function pump() {
		if (busy || !plugin || !queue.length) return;
		// the plugin still works on something this relay did not send (it was busy when the relay restarted)
		if (plugin.state?.busy) return;
		const c = queue.shift();
		clearTimeout(c.waitTimer);
		let out;
		try { out = resolveCall(c.cmd, c.args, commandsDir); } catch (e) { answer(c, fail('bad_request', e.message)); return pump(); }
		if (out.module) c.target = [out.module, c.target].filter(Boolean).join(' ');
		busy = c;
		c.sent = Date.now();
		c.timer = setTimeout(() => {
			c.timedOut = true; // the caller gets its answer; the plugin keeps working – busy until the late answer
			answer(c, fail('timeout', `timeout after ${Math.round(c.timeout / 1000)} s – Figma is still working on it; the bridge sends the next call when it answers (GET /status → busy)`));
		}, c.timeout);
		plugin.send({ id: c.id, cmd: out.cmd, args: out.args });
	}

	function sendSide(c) {
		side.set(c.id, c);
		c.sent = Date.now();
		c.timer = setTimeout(() => { side.delete(c.id); answer(c, fail('timeout', `no answer to ${c.cmd} within ${Math.round(c.timeout / 1000)} s – Figma is frozen or the plugin's code is old`)); }, Math.min(c.timeout, PING_TIMEOUT));
		plugin.send({ id: c.id, cmd: c.cmd, args: c.args });
	}

	function onReply(msg, text, bytes) {
		const ok = msg.ok === true;
		const a = ok ? { ok: true, text, bytes } : { ok: false, code: 'plugin_error', error: String(msg.error ?? 'plugin error'), bytes, text: JSON.stringify({ ...msg, code: 'plugin_error' }) };
		if (busy && msg.id === busy.id) {
			const c = busy;
			busy = null;
			clearTimeout(c.timer);
			if (c.timedOut) log(`${c.cmd}${c.target ? ` ${c.target}` : ''} answered late, after ${Date.now() - c.sent} ms (out ${kb(bytes)}) – the queue goes on`);
			else answer(c, a);
			pump();
			return;
		}
		const s = side.get(msg.id);
		if (s) { side.delete(msg.id); answer(s, a); return; }
		log(`an answer for an unknown call (${kb(bytes)}) – dropped (from before a relay restart?)`);
	}

	/** the plugin went away: every call in flight or queued is answered at once */
	function lost(why) {
		const calls = [busy, ...queue, ...side.values()].filter(Boolean);
		busy = null; queue.length = 0; side.clear();
		for (const c of calls) answer(c, fail('plugin_disconnected', `the plugin went away (${why}) – in Figma desktop: Ctrl+Alt+P (Run last plugin) or Plugins → Development → SCUTOONS Bridge`));
	}

	// ── the plugin's socket ──
	function attach(socket, origin) {
		const chunks = []; // incoming bytes, as they came – concatenated once per frame, never per chunk
		let have = 0, parts = [], partsLen = 0;
		/** the first n bytes (n ≤ have) – concatenates only the chunks they span */
		const peek = (n) => {
			if (chunks[0].length >= n) return chunks[0];
			let k = 0, len = 0;
			while (len < n) len += chunks[k++].length;
			const b = Buffer.concat(chunks.splice(0, k), len);
			chunks.unshift(b);
			return b;
		};
		const take = (n) => {
			const b = peek(n);
			if (b.length === n) chunks.shift(); else chunks[0] = b.subarray(n);
			have -= n;
			return b.subarray(0, n);
		};
		const me = { socket, origin, since: new Date().toISOString(), lastSeen: Date.now(), stale: false, version: null, file: null, state: null, closeCode: null, pending: null, send: (obj) => socket.write(frame(0x1, Buffer.from(JSON.stringify(obj), 'utf8'))) };

		socket.on('data', (chunk) => {
			chunks.push(chunk);
			have += chunk.length;
			while (have >= 2) {
				const h = peek(2);
				const fin = (h[0] & 0x80) !== 0, op = h[0] & 0x0f, masked = (h[1] & 0x80) !== 0;
				let len = h[1] & 0x7f;
				const off = len === 126 ? 4 : len === 127 ? 10 : 2, headLen = off + (masked ? 4 : 0);
				if (have < headLen) return;
				const head = peek(headLen);
				if (len === 126) len = head.readUInt16BE(2);
				else if (len === 127) len = Number(head.readBigUInt64BE(2));
				if (len > MAX_MESSAGE) { log(`a frame of ${kb(len)} from the plugin – closing the socket`); socket.destroy(); return; }
				if (have < headLen + len) return;
				const mask = masked ? Buffer.from(head.subarray(off, off + 4)) : null;
				take(headLen);
				const data = len ? take(len) : Buffer.alloc(0);
				if (mask) for (let i = 0; i < len; i++) data[i] ^= mask[i & 3];
				me.lastSeen = Date.now();
				if (me.stale) { me.stale = false; if (plugin === me) log('the plugin answers again'); }
				if (op === 0x8) { // close: its code for the log, then close back
					me.closeCode = len >= 2 ? data.readUInt16BE(0) : 1005;
					const reason = len > 2 ? ` "${data.subarray(2).toString('utf8')}"` : '';
					if (plugin === me) log(`the plugin closes the socket (code ${me.closeCode}${reason})`);
					socket.end(Buffer.from([0x88, 0]));
					return;
				}
				if (op === 0x9) { socket.write(frame(0xa, Buffer.from(data))); continue; } // ping → pong
				if (op === 0xa) continue; // pong: lastSeen is set
				if (op === 0x0 || op === 0x1 || op === 0x2) {
					parts.push(data);
					partsLen += len;
					if (!fin) continue;
					const text = (parts.length === 1 ? parts[0] : Buffer.concat(parts, partsLen)).toString('utf8');
					const bytes = partsLen;
					parts = []; partsLen = 0;
					if (plugin === me || me.pending) onMessage(me, text, bytes);
				}
			}
		});

		function onMessage(p, text, bytes) {
			let msg;
			try { msg = JSON.parse(text); } catch { log(`an unreadable message from the plugin (${kb(bytes)}) – dropped`); return; }
			if (msg.hello) { // the plugin's code version, its file and its own state, on every (re)connect (the UI may send it twice)
				const said = `plugin code version ${msg.hello.version ?? null}${msg.hello.file ? `, file "${msg.hello.file}"` : ''}`;
				p.version = msg.hello.version ?? null;
				p.file = msg.hello.file ?? null;
				p.state = { busy: msg.hello.busy ?? null, queue: msg.hello.queue ?? 0 };
				if (p.pending) takeOver(p);
				if (said !== p.said || p.state.busy) log(`${said}${p.state.busy ? ` – still busy with ${p.state.busy.cmd} from before` : ''}`);
				p.said = said;
				pump();
				return;
			}
			if (p.pending) return; // a second window says nothing else before its hello
			if (msg.state) {
				p.state = msg.state;
				// the call that timed out never got its late answer, yet the plugin is idle: the answer is lost (the plugin
				// sends it before this state, on the same socket) – the queue goes on instead of waiting for a reconnect
				if (busy && busy.timedOut && !msg.state.busy && !msg.state.queue) {
					log(`${busy.cmd}${busy.target ? ` ${busy.target}` : ''}: timed out, and the plugin is idle – its answer is lost; the queue goes on`);
					busy = null;
				}
				if (!msg.state.busy) pump();
				return;
			}
			if (msg.id) onReply(msg, text, bytes);
		}

		socket.setNoDelay(true);
		socket.on('close', () => {
			clearTimeout(me.pending);
			if (plugin !== me) return;
			plugin = null;
			const why = me.closeCode ? `close code ${me.closeCode}` : 'connection lost';
			log(`plugin disconnected (${why}, connected since ${me.since})`);
			lost(why);
		});
		socket.on('error', () => {});
		if (plugin) {
			// a second plugin window: it takes over with its hello (takeOver) – a socket that never says hello can't push
			// the running plugin out (the replaced one does not reconnect: code 4001)
			me.pending = setTimeout(() => {
				log('a second plugin socket without a hello – closed');
				socket.end(closeFrame(1008, 'no hello'));
				setTimeout(() => socket.destroy(), 1000).unref();
			}, helloWait);
			me.pending.unref();
			return;
		}
		plugin = me;
		log(`plugin connected (Origin ${origin ?? 'none'})`);
	}

	const closeFrame = (code, reason) => frame(0x8, Buffer.concat([Buffer.from([code >> 8, code & 255]), Buffer.from(reason)]));
	/** a second plugin window said hello: it replaces the first, which is told so (code 4001) and does not reconnect */
	function takeOver(me) {
		clearTimeout(me.pending);
		me.pending = null;
		const old = plugin;
		plugin = null;
		log(`a second plugin window connected (Origin ${me.origin ?? 'none'}) – it replaces the first (two Figma windows running the bridge?)`);
		if (old) {
			lost('replaced by another plugin window');
			old.socket.end(closeFrame(4001, 'replaced by another plugin window'));
			setTimeout(() => old.socket.destroy(), 1000).unref();
		}
		plugin = me;
	}

	const beat = setInterval(() => {
		if (!plugin) return;
		if (isStale() && !plugin.stale) { plugin.stale = true; log(`plugin stale – no pong for ${Math.round((Date.now() - plugin.lastSeen) / 1000)} s: the connection is dead (Figma closed, crashed or asleep)`); }
		plugin.socket.write(Buffer.from([0x89, 0]));
	}, heartbeat.every);
	beat.unref();

	const status = () => {
		const b = busy ?? (plugin?.state?.busy ? { ...plugin.state.busy, orphan: true } : null);
		return {
			plugin: !!plugin,
			stale: isStale(),
			busy: b && { cmd: b.cmd, id: b.id ?? null, since: new Date(b.sent ?? b.since ?? Date.now()).toISOString(), ...(b.target ? { target: b.target } : {}), ...(b.client ? { client: b.client } : {}), ...(b.timedOut ? { timedOut: true } : {}), ...(b.orphan ? { orphan: true } : {}) },
			queue: queue.length,
			version: plugin?.version ?? null,
			file: plugin?.file ?? null,
			relay: RELAY_VERSION,
			pid: process.pid,
			port,
			supervisor: Number(process.env.SCUTOONS_FIGMA_SUPERVISOR) || null,
			since: startedAt,
			pluginSince: plugin?.since ?? null
		};
	};

	// ── HTTP ──
	const onRequest = (req, res) => {
		const send = (code, body) => { res.writeHead(code, { 'content-type': 'application/json' }); res.end(typeof body === 'string' ? body : JSON.stringify(body)); };
		if (req.method === 'GET' && req.url === '/status') return send(200, status());
		if (req.method === 'POST' && req.url === '/cmd') {
			// a web page in the owner's browser could post here (text/plain needs no preflight): browsers send an Origin,
			// local tools (node:http, Node's fetch) don't; JSON from a page would need a preflight, which gets no answer
			if (req.headers.origin !== undefined) {
				log(`refused POST /cmd from Origin ${req.headers.origin}`);
				return send(403, { ok: false, code: 'forbidden', error: 'the relay takes no calls from web pages' });
			}
			if (!/^application\/json\b/i.test(req.headers['content-type'] ?? '')) return send(415, { ok: false, code: 'bad_request', error: 'content-type: application/json' });
			const chunks = [];
			let n = 0;
			req.on('data', (c) => { chunks.push(c); n += c.length; });
			req.on('end', () => {
				let msg;
				try { msg = JSON.parse(Buffer.concat(chunks, n).toString('utf8')); } catch {}
				if (!msg || typeof msg.cmd !== 'string') return send(400, { ok: false, code: 'bad_request', error: 'a JSON body { cmd, args, timeout?, wait?, client? }' });
				const c = {
					id: randomUUID(), cmd: msg.cmd, args: msg.args ?? {}, target: target(msg.args ?? {}), client: typeof msg.client === 'string' ? msg.client.slice(0, 60) : '',
					timeout: clamp(msg.timeout, TIMEOUT, MAX_TIMEOUT), wait: clamp(msg.wait, WAIT, 3_600_000), t0: Date.now(), bytesIn: n,
					done: (a) => { if (!res.writableEnded && !res.destroyed) send(a.ok ? 200 : HTTP[a.code] ?? 502, a.text); }
				};
				res.on('close', () => {
					if (c.answered) return;
					// the caller went away (Ctrl+C): a queued call goes with it; one at the plugin runs on (it can't be stopped)
					const i = queue.indexOf(c);
					if (i >= 0) queue.splice(i, 1);
					c.answered = true;
					clearTimeout(c.waitTimer);
					if (side.has(c.id)) { side.delete(c.id); clearTimeout(c.timer); }
					log(`${c.cmd}${c.target ? ` ${c.target}` : ''} – the caller went away${i >= 0 ? ', dropped from the queue' : busy === c ? ', Figma keeps working on it' : ''}${c.client ? ` (${c.client})` : ''}`);
				});
				enqueue(c);
			});
			return;
		}
		send(404, { ok: false, code: 'bad_request', error: 'GET /status · POST /cmd' });
	};
	const onUpgrade = (req, socket) => {
		const key = req.headers['sec-websocket-key'];
		if (req.url !== '/plugin' || !key) return socket.destroy();
		const origin = req.headers.origin;
		if (!pluginOrigin(origin)) {
			log(`refused a plugin connection from Origin ${origin} (a web page?)`);
			socket.end('HTTP/1.1 403 Forbidden\r\nConnection: close\r\nContent-Length: 0\r\n\r\n');
			return;
		}
		const accept = createHash('sha1').update(key + GUID).digest('base64');
		socket.write(`HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ${accept}\r\n\r\n`);
		attach(socket, origin);
	};
	// the plugin connects to ws://localhost (Figma's manifest takes no IPs) – localhost is 127.0.0.1 or ::1, so both.
	// Bind errors are not swallowed: 127.0.0.1 taken (or ::1 taken by someone else) → the error, with its code
	const listen = (host) => new Promise((ok) => {
		const s = createServer({ requestTimeout: 0 }, onRequest).on('upgrade', onUpgrade); // a call may take minutes
		s.once('error', (error) => ok({ error }));
		s.listen(port, host, () => { s.on('error', (e) => log(`server error on ${host}: ${e.message}`)); ok({ server: s }); });
	});
	const bindError = (e, host) => Object.assign(new Error(`${host}:${port} – ${e.code === 'EADDRINUSE' ? 'the port is taken' : e.message}`), { code: e.code });
	const v4 = await listen('127.0.0.1');
	if (v4.error) { clearInterval(beat); throw bindError(v4.error, '127.0.0.1'); }
	const servers = [v4.server];
	const v6 = await listen('::1');
	if (v6.error?.code === 'EADDRINUSE') { clearInterval(beat); v4.server.close(); throw bindError(v6.error, '[::1]'); }
	if (v6.error) log(`no IPv6 here (${v6.error.code}) – 127.0.0.1 only`);
	else servers.push(v6.server);
	log(`SCUTOONS Figma relay ${RELAY_VERSION} on localhost:${port} (pid ${process.pid}) – waiting for the plugin`);
	return {
		server: servers[0],
		status,
		close: async () => {
			clearInterval(beat);
			const p = plugin;
			plugin = null;
			lost('the relay is closing');
			p?.socket.destroy();
			await Promise.all(servers.map((s) => new Promise((r) => { s.closeAllConnections?.(); s.close(r); })));
		}
	};
}

async function main() {
	const argv = process.argv.slice(2);
	const opt = (f) => { const i = argv.indexOf(f); return i >= 0 ? argv[i + 1] : undefined; };
	const port = Number(opt('--port') ?? process.env.SCUTOONS_FIGMA_PORT ?? 3056);
	const file = argv.includes('--no-log-file') ? null : (opt('--log-file') ?? defaultFiles(port).log);
	try {
		await startRelay({ port, log: makeLogger(file) });
	} catch (e) {
		if (e.code === 'EADDRINUSE') {
			const s = await probe(port);
			if (s && 'plugin' in s) {
				console.log(`already running on :${port} (pid ${s.pid ?? '?'}${s.supervisor ? `, supervisor ${s.supervisor}` : ''}, plugin ${s.plugin ? 'connected' : 'not connected'})`);
				process.exit(0);
			}
			console.error(`port ${port} is taken by another program (${s ? 'not a SCUTOONS relay' : 'no answer on /status'}) – free it, or use --port / SCUTOONS_FIGMA_PORT`);
			process.exit(2);
		}
		console.error(`the relay can't listen on ${port}: ${e.message}`);
		process.exit(2);
	}
	if (file) console.log(`log: ${file}`);
	for (const sig of ['SIGINT', 'SIGTERM']) process.on(sig, () => process.exit(130));
}

if (process.argv[1]?.endsWith('relay.mjs')) main();
