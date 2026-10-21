// The Figma relay's pure parts (#127): the WebSocket framing, which Origins may connect, a call → what goes to the
// plugin (command modules), the HTTP codes, the log's target text, when the MCP may start its own relay. A port of
// tools/figma-bridge/relay.mjs – the same protocol on the same port (3056), so the plugin and the checkout's scripts
// can't tell the two relays apart. The relay itself: figma-relay.ts.
import { createHash } from 'node:crypto';

/** = relay.mjs RELAY_VERSION whose protocol this port speaks (GET /status → relay: "<it>+mcp") */
export const RELAY_PROTOCOL = '2026-10-20.3';
export const GUID = '258EAFA5-E914-47DA-95CA-C5AB0DC85B11';
export const MODULE_NAME = /^[a-z][\w-]*$/;
export const TIMEOUT = 120_000;
export const MAX_TIMEOUT = 900_000; // a caller may ask for longer (a huge draft: up to 15 min)
export const WAIT = 600_000; // how long a call may wait in the queue by default
export const MAX_WAIT = 3_600_000;
export const PING_TIMEOUT = 15_000;
export const HEARTBEAT = { every: 10_000, stale: 30_000 };
export const HELLO_WAIT = 10_000; // a second plugin window must say hello within this to take over
export const MAX_MESSAGE = 512 * 1024 * 1024;
export const HTTP: Record<string, number> = { no_plugin: 503, plugin_disconnected: 503, timeout: 504, queue_timeout: 504, plugin_error: 502, bad_request: 400, forbidden: 403 };
export const NO_PLUGIN = 'no plugin connected – in Figma desktop: Ctrl+Alt+P (Run last plugin) or Plugins → Development → SCUTOONS Bridge';

/** the Origins a plugin socket may come from: Figma's plugin iframes have an opaque origin (`null`); none = a local tool */
export const pluginOrigin = (o: string | undefined): boolean => o === undefined || o === 'null' || /^https:\/\/([a-z0-9-]+\.)*figma\.com$/i.test(o);

/** Sec-WebSocket-Accept for a client's key */
export const acceptKey = (key: string): string => createHash('sha1').update(key + GUID).digest('base64');

export const kb = (n: number): string => (n >= 1048576 ? `${(n / 1048576).toFixed(1)} MB` : n >= 1024 ? `${Math.round(n / 1024)} KB` : `${n} B`);
export const clamp = (v: unknown, d: number, max: number): number => Math.min(Number(v) > 0 ? Number(v) : d, max);

/** what a call works on, for the log: node / frame id, a region's box, the scales */
export function target(a: Record<string, any> = {}): string {
	return [
		a.nodeId ?? a.node ?? a.frame ?? (a.page ? `page "${a.page}"` : ''),
		a.y != null && a.h ? `@${a.x ?? 0},${a.y} ${a.w}×${a.h}` : '',
		a.scales ? `×${a.scales.join(',')}` : '',
		a.ids ? `${a.ids.length} ids` : '',
		a.slices ? `${a.slices.length} slices` : ''
	]
		.filter(Boolean)
		.join(' ');
}

/** a server → client frame (never masked) */
export function encodeFrame(op: number, payload: Buffer): Buffer {
	const n = payload.length;
	let head: Buffer;
	if (n < 126) head = Buffer.from([0x80 | op, n]);
	else if (n < 65536) head = Buffer.from([0x80 | op, 126, n >> 8, n & 255]);
	else {
		head = Buffer.alloc(10);
		head[0] = 0x80 | op;
		head[1] = 127;
		head.writeBigUInt64BE(BigInt(n), 2);
	}
	return Buffer.concat([head, payload]);
}

export const closeFrame = (code: number, reason: string): Buffer => encodeFrame(0x8, Buffer.concat([Buffer.from([code >> 8, code & 255]), Buffer.from(reason)]));

export interface Frame {
	fin: boolean;
	op: number;
	data: Buffer;
}

/**
 * incoming bytes → whole frames (unmasked). Chunks are kept as they came and concatenated once per frame, never per
 * chunk (a 100 MB export arrives in thousands of chunks). A frame over `max` throws (the relay closes the socket).
 */
export class FrameReader {
	private chunks: Buffer[] = [];
	private have = 0;
	constructor(private readonly max = MAX_MESSAGE) {}

	/** the first n bytes (n ≤ have) – concatenates only the chunks they span */
	private peek(n: number): Buffer {
		if (this.chunks[0].length >= n) return this.chunks[0];
		let k = 0;
		let len = 0;
		while (len < n) len += this.chunks[k++].length;
		const b = Buffer.concat(this.chunks.splice(0, k), len);
		this.chunks.unshift(b);
		return b;
	}

	private take(n: number): Buffer {
		const b = this.peek(n);
		if (b.length === n) this.chunks.shift();
		else this.chunks[0] = b.subarray(n);
		this.have -= n;
		return b.subarray(0, n);
	}

	push(chunk: Buffer): Frame[] {
		this.chunks.push(chunk);
		this.have += chunk.length;
		const out: Frame[] = [];
		while (this.have >= 2) {
			const h = this.peek(2);
			const fin = (h[0] & 0x80) !== 0;
			const op = h[0] & 0x0f;
			const masked = (h[1] & 0x80) !== 0;
			let len = h[1] & 0x7f;
			const off = len === 126 ? 4 : len === 127 ? 10 : 2;
			const headLen = off + (masked ? 4 : 0);
			if (this.have < headLen) break;
			const head = this.peek(headLen);
			if (len === 126) len = head.readUInt16BE(2);
			else if (len === 127) len = Number(head.readBigUInt64BE(2));
			if (len > this.max) throw Object.assign(new Error(`a frame of ${kb(len)}`), { code: 'too_big' });
			if (this.have < headLen + len) break;
			const mask = masked ? Buffer.from(head.subarray(off, off + 4)) : null;
			this.take(headLen);
			// unmasked in place: these bytes are taken – nothing else reads them
			const data = len ? this.take(len) : Buffer.alloc(0);
			if (mask) for (let i = 0; i < len; i++) data[i] ^= mask[i & 3];
			out.push({ fin, op, data });
		}
		return out;
	}
}

/** a command module's source by its name, or null when there is none (figma-commands.ts) */
export type ModuleSource = (name: string) => string | null;

export interface ResolvedCall {
	cmd: string;
	args: unknown;
	/** "<name>@<hash8>" for the log when it is a module */
	module?: string;
}

/**
 * a call → what goes to the plugin: a command with a module file (or `run` { name, args }) → `run` { name, hash, src,
 * args }, read now; anything else as it is. A `run` without such a module throws (code bad_request).
 */
export function resolveCall(cmd: string, args: any, source: ModuleSource): ResolvedCall {
	const explicit = cmd === 'run';
	const name = explicit ? args?.name : cmd;
	if (cmd === 'ping' || typeof name !== 'string' || !MODULE_NAME.test(name)) {
		if (explicit) throw new Error(`run: a module name (${MODULE_NAME}), not ${JSON.stringify(name ?? null)}`);
		return { cmd, args };
	}
	const src = source(name);
	if (src === null) {
		if (explicit) throw new Error(`no module commands/${name}.js`);
		return { cmd, args }; // a core command (or an unknown one – the plugin says which it has)
	}
	const hash = createHash('sha256').update(src).digest('hex').slice(0, 16);
	return { cmd: 'run', args: { name, hash, src, args: (explicit ? args.args : args) ?? {} }, module: `${name}@${hash.slice(0, 8)}` };
}

/**
 * may this MCP start its own relay when nothing listens on the port? Only a member's one-file copy / extension (no
 * checkout) – or when asked for (`SCUTOONS_FIGMA_RELAY=mcp`). Never from a checkout by default: the owner's production
 * scripts run relay.mjs under supervise.mjs on the same port, and an MCP relay taking the port during a relay restart
 * would lock the supervisor out (owner's ch3 pipeline, 2026-10-21). `SCUTOONS_FIGMA_RELAY=off` turns it off anywhere.
 */
export function relayAutoStart(env: NodeJS.ProcessEnv, bundled: boolean): boolean {
	const v = (env.SCUTOONS_FIGMA_RELAY ?? '').trim().toLowerCase();
	if (v === 'mcp' || v === 'on') return true;
	if (v === 'off' || v === 'external' || v === 'supervisor') return false;
	return bundled;
}
