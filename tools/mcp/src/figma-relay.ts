// The Figma relay inside the MCP (#127; agents/research/STUDIO-IN-CREATOR.md §2.4 "the local kit"; owner 2026-10-20:
// "they can just install the stuff and connect their claude and go"): a TypeScript port of tools/figma-bridge/relay.mjs
// – the same protocol, the same port, the same self-healing rules (its header and tools/figma-bridge/README.md
// "Recovery"): ONE queue, one call at the plugin (ping passes it), a timed-out call keeps the plugin busy until its late
// answer (or the plugin's idle state), queue_timeout, a caller that goes away takes its queued call along, a disconnect
// answers every call at once, the heartbeat (stale), local tools only (/cmd: no Origin, JSON only; /plugin: Figma's
// Origins), a second plugin window takes over only with its hello (the old one gets 4001), command modules read per send.
// What it leaves to relay.mjs + supervise.mjs: the supervisor and the log file. It lives as long as the MCP process; the
// plugin reconnects by itself (ui.html: every ≤ 10 s) to whichever relay holds the port.
// ~500 lines (RULES "Code": over 400 with a reason): a line-by-line port of relay.mjs, kept in one piece so the two can
// be compared and changed together until relay.mjs retires (#127's second part).
// The MCP starts it only when nothing listens on the port AND relayAutoStart() allows it (figma-relay-core.ts: a
// member's copy, or SCUTOONS_FIGMA_RELAY=mcp) – a running relay.mjs (the owner's checkout) is simply used.
import { randomUUID } from 'node:crypto';
import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'node:http';
import type { Socket } from 'node:net';
import {
	acceptKey,
	clamp,
	closeFrame,
	encodeFrame,
	FrameReader,
	HEARTBEAT,
	HELLO_WAIT,
	HTTP,
	kb,
	MAX_TIMEOUT,
	MAX_WAIT,
	NO_PLUGIN,
	PING_TIMEOUT,
	pluginOrigin,
	RELAY_PROTOCOL,
	resolveCall,
	target,
	TIMEOUT,
	WAIT,
	type ModuleSource
} from './figma-relay-core.ts';

type Answer = { ok: boolean; code?: string; error?: string; text: string; bytes?: number };
interface Call {
	id: string;
	cmd: string;
	args: any;
	target: string;
	client: string;
	timeout: number;
	wait: number;
	t0: number;
	bytesIn: number;
	sent?: number;
	answered?: boolean;
	timedOut?: boolean;
	timer?: NodeJS.Timeout;
	waitTimer?: NodeJS.Timeout;
	done: (a: Answer) => void;
}
interface PluginState {
	busy: { cmd: string; id?: string; since?: string } | null;
	queue: number;
}
interface Plugin {
	socket: Socket;
	origin: string | undefined;
	since: string;
	lastSeen: number;
	stale: boolean;
	version: string | null;
	file: string | null;
	state: PluginState | null;
	closeCode: number | null;
	pending: NodeJS.Timeout | null;
	said?: string;
	send: (obj: unknown) => void;
}

export interface RelayOptions {
	port?: number;
	log?: (line: string) => void;
	heartbeat?: { every: number; stale: number };
	helloWait?: number;
	/** the command modules (figma-commands.ts commandSource, or the embedded ones in the member's copy) */
	modules: ModuleSource;
}

export interface Relay {
	port: number;
	status: () => Record<string, unknown>;
	close: () => Promise<void>;
}

const fail = (code: string, error: string): Answer => ({ ok: false, code, error, text: JSON.stringify({ ok: false, code, error }) });

export async function startRelay({ port = 3056, log = () => {}, heartbeat = HEARTBEAT, helloWait = HELLO_WAIT, modules }: RelayOptions): Promise<Relay> {
	let plugin: Plugin | null = null;
	let busy: Call | null = null; // the call at the plugin (stays after its timeout until the late answer)
	const queue: Call[] = []; // calls waiting for the plugin, oldest first
	const side = new Map<string, Call>(); // pings sent past the queue
	const startedAt = new Date().toISOString();
	const isStale = () => !!plugin && Date.now() - plugin.lastSeen > heartbeat.stale;
	const label = (c: Call) => `${c.cmd}${c.target ? ` ${c.target}` : ''}`;

	// ── calls ──
	function answer(c: Call, a: Answer) {
		if (c.answered) return;
		c.answered = true;
		clearTimeout(c.timer);
		clearTimeout(c.waitTimer);
		const ms = Date.now() - c.t0;
		const queued = c.sent ? c.sent - c.t0 : 0;
		log(`${label(c)} → ${a.ok ? 'ok' : `${a.code}${a.code === 'plugin_error' ? `: ${a.error}` : ''}`} ${ms} ms (in ${kb(c.bytesIn)}, out ${kb(a.bytes ?? 0)}${queued > 50 ? `, ${queued} ms in the queue` : ''}${c.client ? `, ${c.client}` : ''})`);
		c.done(a);
	}

	function enqueue(c: Call) {
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

	function pump(): void {
		if (busy || !plugin || !queue.length) return;
		// the plugin still works on something this relay did not send (it was busy when the relay (re)started)
		if (plugin.state?.busy) return;
		const c = queue.shift()!;
		clearTimeout(c.waitTimer);
		let out;
		try {
			out = resolveCall(c.cmd, c.args, modules);
		} catch (e) {
			answer(c, fail('bad_request', (e as Error).message));
			return pump();
		}
		if (out.module) c.target = [out.module, c.target].filter(Boolean).join(' ');
		busy = c;
		c.sent = Date.now();
		c.timer = setTimeout(() => {
			c.timedOut = true; // the caller gets its answer; the plugin keeps working – busy until the late answer
			answer(c, fail('timeout', `timeout after ${Math.round(c.timeout / 1000)} s – Figma is still working on it; the bridge sends the next call when it answers (GET /status → busy)`));
		}, c.timeout);
		plugin.send({ id: c.id, cmd: out.cmd, args: out.args });
	}

	function sendSide(c: Call) {
		side.set(c.id, c);
		c.sent = Date.now();
		c.timer = setTimeout(() => {
			side.delete(c.id);
			answer(c, fail('timeout', `no answer to ${c.cmd} within ${Math.round(c.timeout / 1000)} s – Figma is frozen or the plugin's code is old`));
		}, Math.min(c.timeout, PING_TIMEOUT));
		plugin!.send({ id: c.id, cmd: c.cmd, args: c.args });
	}

	function onReply(msg: any, text: string, bytes: number) {
		const a: Answer = msg.ok === true ? { ok: true, text, bytes } : { ok: false, code: 'plugin_error', error: String(msg.error ?? 'plugin error'), bytes, text: JSON.stringify({ ...msg, code: 'plugin_error' }) };
		if (busy && msg.id === busy.id) {
			const c = busy;
			busy = null;
			clearTimeout(c.timer);
			if (c.timedOut) log(`${label(c)} answered late, after ${Date.now() - (c.sent ?? c.t0)} ms (out ${kb(bytes)}) – the queue goes on`);
			else answer(c, a);
			pump();
			return;
		}
		const s = side.get(msg.id);
		if (s) {
			side.delete(msg.id);
			answer(s, a);
			return;
		}
		log(`an answer for an unknown call (${kb(bytes)}) – dropped (from before a relay restart?)`);
	}

	/** the plugin went away: every call in flight or queued is answered at once */
	function lost(why: string) {
		const calls = [busy, ...queue, ...side.values()].filter((c): c is Call => !!c);
		busy = null;
		queue.length = 0;
		side.clear();
		for (const c of calls) answer(c, fail('plugin_disconnected', `the plugin went away (${why}) – in Figma desktop: Ctrl+Alt+P (Run last plugin) or Plugins → Development → SCUTOONS Bridge`));
	}

	// ── the plugin's socket ──
	function attach(socket: Socket, origin: string | undefined) {
		const reader = new FrameReader();
		let parts: Buffer[] = [];
		let partsLen = 0;
		const me: Plugin = {
			socket,
			origin,
			since: new Date().toISOString(),
			lastSeen: Date.now(),
			stale: false,
			version: null,
			file: null,
			state: null,
			closeCode: null,
			pending: null,
			send: (obj) => socket.write(encodeFrame(0x1, Buffer.from(JSON.stringify(obj), 'utf8')))
		};

		socket.on('data', (chunk: Buffer) => {
			let frames;
			try {
				frames = reader.push(chunk);
			} catch (e) {
				log(`${(e as Error).message} from the plugin – closing the socket`);
				socket.destroy();
				return;
			}
			for (const { fin, op, data } of frames) {
				me.lastSeen = Date.now();
				if (me.stale) {
					me.stale = false;
					if (plugin === me) log('the plugin answers again');
				}
				if (op === 0x8) {
					// close: its code for the log, then close back
					me.closeCode = data.length >= 2 ? data.readUInt16BE(0) : 1005;
					const reason = data.length > 2 ? ` "${data.subarray(2).toString('utf8')}"` : '';
					if (plugin === me) log(`the plugin closes the socket (code ${me.closeCode}${reason})`);
					socket.end(Buffer.from([0x88, 0]));
					return;
				}
				if (op === 0x9) {
					socket.write(encodeFrame(0xa, Buffer.from(data)));
					continue;
				}
				if (op === 0xa) continue; // pong: lastSeen is set
				if (op === 0x0 || op === 0x1 || op === 0x2) {
					parts.push(data);
					partsLen += data.length;
					if (!fin) continue;
					const text = (parts.length === 1 ? parts[0] : Buffer.concat(parts, partsLen)).toString('utf8');
					const bytes = partsLen;
					parts = [];
					partsLen = 0;
					if (plugin === me || me.pending) onMessage(me, text, bytes);
				}
			}
		});

		function onMessage(p: Plugin, text: string, bytes: number) {
			let msg: any;
			try {
				msg = JSON.parse(text);
			} catch {
				log(`an unreadable message from the plugin (${kb(bytes)}) – dropped`);
				return;
			}
			if (msg.hello) {
				// the plugin's code version, its file and its own state, on every (re)connect (the UI may send it twice)
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
				// the call that timed out never got its late answer, yet the plugin is idle: the answer is lost – the queue goes on
				if (busy && busy.timedOut && !msg.state.busy && !msg.state.queue) {
					log(`${label(busy)}: timed out, and the plugin is idle – its answer is lost; the queue goes on`);
					busy = null;
				}
				if (!msg.state.busy) pump();
				return;
			}
			if (msg.id) onReply(msg, text, bytes);
		}

		socket.setNoDelay(true);
		socket.on('close', () => {
			if (me.pending) clearTimeout(me.pending);
			if (plugin !== me) return;
			plugin = null;
			const why = me.closeCode ? `close code ${me.closeCode}` : 'connection lost';
			log(`plugin disconnected (${why}, connected since ${me.since})`);
			lost(why);
		});
		socket.on('error', () => {});
		if (plugin) {
			// a second plugin window: it takes over with its hello – a socket that never says hello can't push the running plugin out
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

	/** a second plugin window said hello: it replaces the first, which is told so (code 4001) and does not reconnect */
	function takeOver(me: Plugin) {
		if (me.pending) clearTimeout(me.pending);
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
		if (isStale() && !plugin.stale) {
			plugin.stale = true;
			log(`plugin stale – no pong for ${Math.round((Date.now() - plugin.lastSeen) / 1000)} s: the connection is dead (Figma closed, crashed or asleep)`);
		}
		plugin.socket.write(Buffer.from([0x89, 0]));
	}, heartbeat.every);
	beat.unref();

	const status = () => {
		const b: any = busy ?? (plugin?.state?.busy ? { ...plugin.state.busy, orphan: true } : null);
		return {
			plugin: !!plugin,
			stale: isStale(),
			busy: b && {
				cmd: b.cmd,
				id: b.id ?? null,
				since: new Date(b.sent ?? b.since ?? Date.now()).toISOString(),
				...(b.target ? { target: b.target } : {}),
				...(b.client ? { client: b.client } : {}),
				...(b.timedOut ? { timedOut: true } : {}),
				...(b.orphan ? { orphan: true } : {})
			},
			queue: queue.length,
			version: plugin?.version ?? null,
			file: plugin?.file ?? null,
			relay: `${RELAY_PROTOCOL}+mcp`,
			host: 'scutoons-mcp',
			pid: process.pid,
			port,
			supervisor: null,
			since: startedAt,
			pluginSince: plugin?.since ?? null
		};
	};

	// ── HTTP ──
	const onRequest = (req: IncomingMessage, res: ServerResponse) => {
		const send = (code: number, body: unknown) => {
			res.writeHead(code, { 'content-type': 'application/json' });
			res.end(typeof body === 'string' ? body : JSON.stringify(body));
		};
		if (req.method === 'GET' && req.url === '/status') return send(200, status());
		if (req.method === 'POST' && req.url === '/cmd') {
			// browsers send an Origin, local tools don't; JSON from a page would need a preflight, which gets no answer
			if (req.headers.origin !== undefined) {
				log(`refused POST /cmd from Origin ${req.headers.origin}`);
				return send(403, { ok: false, code: 'forbidden', error: 'the relay takes no calls from web pages' });
			}
			if (!/^application\/json\b/i.test(req.headers['content-type'] ?? '')) return send(415, { ok: false, code: 'bad_request', error: 'content-type: application/json' });
			const chunks: Buffer[] = [];
			let n = 0;
			req.on('data', (c: Buffer) => {
				chunks.push(c);
				n += c.length;
			});
			req.on('end', () => {
				let msg: any;
				try {
					msg = JSON.parse(Buffer.concat(chunks, n).toString('utf8'));
				} catch {}
				if (!msg || typeof msg.cmd !== 'string') return send(400, { ok: false, code: 'bad_request', error: 'a JSON body { cmd, args, timeout?, wait?, client? }' });
				const c: Call = {
					id: randomUUID(),
					cmd: msg.cmd,
					args: msg.args ?? {},
					target: target(msg.args ?? {}),
					client: typeof msg.client === 'string' ? msg.client.slice(0, 60) : '',
					timeout: clamp(msg.timeout, TIMEOUT, MAX_TIMEOUT),
					wait: clamp(msg.wait, WAIT, MAX_WAIT),
					t0: Date.now(),
					bytesIn: n,
					done: (a) => {
						if (!res.writableEnded && !res.destroyed) send(a.ok ? 200 : (HTTP[a.code ?? ''] ?? 502), a.text);
					}
				};
				res.on('close', () => {
					if (c.answered) return;
					// the caller went away (Ctrl+C): a queued call goes with it; one at the plugin runs on (it can't be stopped)
					const i = queue.indexOf(c);
					if (i >= 0) queue.splice(i, 1);
					c.answered = true;
					clearTimeout(c.waitTimer);
					if (side.has(c.id)) {
						side.delete(c.id);
						clearTimeout(c.timer);
					}
					log(`${label(c)} – the caller went away${i >= 0 ? ', dropped from the queue' : busy === c ? ', Figma keeps working on it' : ''}${c.client ? ` (${c.client})` : ''}`);
				});
				enqueue(c);
			});
			return;
		}
		send(404, { ok: false, code: 'bad_request', error: 'GET /status · POST /cmd' });
	};
	const onUpgrade = (req: IncomingMessage, socket: Socket) => {
		const key = req.headers['sec-websocket-key'];
		if (req.url !== '/plugin' || !key) return socket.destroy();
		const origin = req.headers.origin;
		if (!pluginOrigin(origin)) {
			log(`refused a plugin connection from Origin ${origin} (a web page?)`);
			socket.end('HTTP/1.1 403 Forbidden\r\nConnection: close\r\nContent-Length: 0\r\n\r\n');
			return;
		}
		socket.write(`HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ${acceptKey(key)}\r\n\r\n`);
		attach(socket, origin);
	};
	// the plugin connects to ws://localhost (Figma's manifest takes no IPs) – localhost is 127.0.0.1 or ::1, so both
	const listen = (host: string) =>
		new Promise<{ server?: Server; error?: NodeJS.ErrnoException }>((ok) => {
			const s = createServer({ requestTimeout: 0 }, onRequest).on('upgrade', onUpgrade); // a call may take minutes
			s.once('error', (error) => ok({ error }));
			s.listen(port, host, () => {
				s.on('error', (e) => log(`server error on ${host}: ${e.message}`));
				ok({ server: s });
			});
		});
	const bindError = (e: NodeJS.ErrnoException, host: string) => Object.assign(new Error(`${host}:${port} – ${e.code === 'EADDRINUSE' ? 'the port is taken' : e.message}`), { code: e.code });
	const v4 = await listen('127.0.0.1');
	if (v4.error) {
		clearInterval(beat);
		throw bindError(v4.error, '127.0.0.1');
	}
	const servers = [v4.server!];
	const v6 = await listen('::1');
	if (v6.error?.code === 'EADDRINUSE') {
		clearInterval(beat);
		v4.server!.close();
		throw bindError(v6.error, '[::1]');
	}
	if (v6.error) log(`no IPv6 here (${v6.error.code}) – 127.0.0.1 only`);
	else servers.push(v6.server!);
	log(`SCUTOONS Figma relay (in the MCP, protocol ${RELAY_PROTOCOL}) on localhost:${port} (pid ${process.pid}) – waiting for the plugin`);
	return {
		port,
		status,
		close: async () => {
			clearInterval(beat);
			const p = plugin;
			plugin = null;
			lost('the relay is closing');
			p?.socket.destroy();
			await Promise.all(servers.map((s) => new Promise<void>((r) => { s.closeAllConnections?.(); s.close(() => r()); })));
		}
	};
}

// ── the MCP's own relay: started once, on demand ──

let own: Promise<Relay> | null = null;

/** the relay this MCP started (or is starting), if any */
export const ownRelay = (): Promise<Relay> | null => own;

/**
 * start this process's relay on `port` once. A port taken in the meantime (another MCP, relay.mjs) is fine – the caller
 * just talks to whoever holds it; any other bind error is thrown. The log goes to stderr (stdout is the MCP's channel).
 */
export async function ensureRelay(port: number, modules: ModuleSource, log: (line: string) => void = (l) => process.stderr.write(`[figma-relay] ${l}\n`)): Promise<'started' | 'taken'> {
	if (own) {
		const r = await own.catch(() => null);
		if (r && r.port === port) return 'started';
	}
	own = startRelay({ port, modules, log });
	try {
		await own;
		return 'started';
	} catch (e) {
		own = null;
		if ((e as NodeJS.ErrnoException).code === 'EADDRINUSE') return 'taken';
		throw e;
	}
}

/** stop this process's relay (tests) */
export async function stopOwnRelay(): Promise<void> {
	const r = await own?.catch(() => null);
	own = null;
	await r?.close();
}
