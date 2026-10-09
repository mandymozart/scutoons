// SCUTOONS Figma bridge – the local relay (no dependencies). Listens on localhost only (127.0.0.1 and ::1):
//   ws://localhost:3056/plugin    the Figma plugin's UI connects here (one plugin at a time; a new one replaces it)
//   POST http://127.0.0.1:3056/cmd   { "cmd": "read", "args": { … } } → the plugin's answer as JSON (cli.mjs, the MCP)
//   GET  http://127.0.0.1:3056/status
// Nothing leaves the computer. Stop with Ctrl+C.
//
//   node tools/figma-bridge/relay.mjs [--port 3056]

import { createHash, randomUUID } from 'node:crypto';
import { createServer } from 'node:http';

const GUID = '258EAFA5-E914-47DA-95CA-C5AB0DC85B11';
const TIMEOUT = 120_000;

export async function startRelay({ port = 3056, log = console.log } = {}) {
	let plugin = null; // { socket, send }
	const pending = new Map(); // id → { resolve, timer }

	function frame(text) {
		const payload = Buffer.from(text, 'utf8');
		const n = payload.length;
		const head = n < 126 ? Buffer.from([0x81, n]) : n < 65536 ? Buffer.from([0x81, 126, n >> 8, n & 255]) : Buffer.concat([Buffer.from([0x81, 127]), (() => { const b = Buffer.alloc(8); b.writeBigUInt64BE(BigInt(n)); return b; })()]);
		return Buffer.concat([head, payload]);
	}

	function attach(socket) {
		let buf = Buffer.alloc(0);
		let parts = [];
		socket.on('data', (chunk) => {
			buf = Buffer.concat([buf, chunk]);
			for (;;) {
				if (buf.length < 2) return;
				const fin = (buf[0] & 0x80) !== 0, op = buf[0] & 0x0f, masked = (buf[1] & 0x80) !== 0;
				let len = buf[1] & 0x7f, off = 2;
				if (len === 126) { if (buf.length < 4) return; len = buf.readUInt16BE(2); off = 4; }
				else if (len === 127) { if (buf.length < 10) return; len = Number(buf.readBigUInt64BE(2)); off = 10; }
				const need = off + (masked ? 4 : 0) + len;
				if (buf.length < need) return;
				const mask = masked ? buf.subarray(off, off + 4) : null;
				const data = Buffer.from(buf.subarray(off + (masked ? 4 : 0), need));
				if (mask) for (let i = 0; i < data.length; i++) data[i] ^= mask[i & 3];
				buf = buf.subarray(need);
				if (op === 0x8) { socket.end(Buffer.from([0x88, 0])); return; }
				if (op === 0x9) { socket.write(Buffer.concat([Buffer.from([0x8a, data.length]), data])); continue; }
				if (op === 0x1 || op === 0x0) {
					parts.push(data);
					if (!fin) continue;
					const text = Buffer.concat(parts).toString('utf8');
					parts = [];
					let msg;
					try { msg = JSON.parse(text); } catch { continue; }
					const p = pending.get(msg.id);
					if (p) { clearTimeout(p.timer); pending.delete(msg.id); p.resolve(msg); }
				}
			}
		});
		const me = { socket, send: (obj) => socket.write(frame(JSON.stringify(obj))) };
		socket.on('close', () => { if (plugin === me) { plugin = null; log('plugin disconnected'); } });
		socket.on('error', () => {});
		if (plugin) plugin.socket.destroy();
		plugin = me;
		log('plugin connected');
	}

	function call(cmd, args) {
		return new Promise((resolve) => {
			if (!plugin) return resolve({ ok: false, error: 'no plugin connected – open Plugins → Development → SCUTOONS Bridge in Figma desktop' });
			const id = randomUUID();
			const timer = setTimeout(() => { pending.delete(id); resolve({ ok: false, error: `timeout after ${TIMEOUT / 1000} s` }); }, TIMEOUT);
			pending.set(id, { resolve, timer });
			plugin.send({ id, cmd, args });
		});
	}

	const onRequest = (req, res) => {
		const json = (code, obj) => { res.writeHead(code, { 'content-type': 'application/json' }); res.end(JSON.stringify(obj)); };
		if (req.method === 'GET' && req.url === '/status') return json(200, { plugin: !!plugin, pending: pending.size });
		if (req.method === 'POST' && req.url === '/cmd') {
			let body = '';
			req.on('data', (c) => (body += c));
			req.on('end', async () => {
				let msg;
				try { msg = JSON.parse(body); } catch { return json(400, { ok: false, error: 'JSON body { cmd, args }' }); }
				const t0 = Date.now();
				const r = await call(msg.cmd, msg.args ?? {});
				log(`${msg.cmd} → ${r.ok ? 'ok' : r.error} (${Date.now() - t0} ms)`);
				json(r.ok ? 200 : 502, r);
			});
			return;
		}
		json(404, { ok: false, error: 'GET /status · POST /cmd' });
	};
	const onUpgrade = (req, socket) => {
		const key = req.headers['sec-websocket-key'];
		if (req.url !== '/plugin' || !key) return socket.destroy();
		const accept = createHash('sha1').update(key + GUID).digest('base64');
		socket.write(`HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ${accept}\r\n\r\n`);
		attach(socket);
	};
	// the plugin connects to ws://localhost (Figma's manifest takes no IPs) – localhost is 127.0.0.1 or ::1, so both
	const listen = (host) => new Promise((ok) => {
		const s = createServer(onRequest).on('upgrade', onUpgrade);
		s.on('error', () => ok(null)); // no IPv6 on this machine: fine
		s.listen(port, host, () => ok(s));
	});
	const servers = (await Promise.all(['127.0.0.1', '::1'].map(listen))).filter(Boolean);
	log(`SCUTOONS Figma relay on localhost:${port} – waiting for the plugin`);
	return { server: servers[0], close: () => Promise.all(servers.map((s) => new Promise((r) => { plugin?.socket.destroy(); s.close(r); }))) };
}

if (process.argv[1]?.endsWith('relay.mjs')) {
	const i = process.argv.indexOf('--port');
	startRelay({ port: i > 0 ? Number(process.argv[i + 1]) : 3056 });
}
