// The handheld camera over Wi-Fi (owner 2026-10-19: "the handheld camera should be connectable through wifi network. so
// expose the port with https:// on the local network. i don t want to plug in a cable"). The PC keeps the studio on
// http://localhost:<studio port> (127.0.0.1 only); a second server on the LAN, https://<this PC>:<phone port>, passes the
// phone's requests on to it – GET / HEAD and the WebSocket (the handheld relay rides on it) only: other machines read,
// every write stays on this PC (api.mjs's rule; behind this proxy everything looks local, so the proxy enforces it).
// HTTPS because the phone's sensors / WebXR need a secure context: a self-signed certificate for localhost + this PC's
// LAN addresses, made once with openssl into .cert/ (gitignored) and again when the PC gets an address it doesn't cover
// – the phone confirms the warning once per certificate. The phone port = the studio's + 443 (vite.config.js: 14443);
// STUDIO_LAN=0 turns it off. README "Connecting the phone".
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { Agent, request } from 'node:http';
import { createServer } from 'node:https';
import { connect } from 'node:net';
import { networkInterfaces } from 'node:os';
import { resolve } from 'node:path';

/** what the studio's pairing shows (api.mjs GET /api/handheld/urls) */
export const phone = { on: false, port: 0, error: '' };

/** this PC's IPv4 addresses on the LAN – the Wi-Fi / Ethernet ones; virtual adapters (WSL, Hyper-V, Docker, VirtualBox,
 * VMware) only when there is nothing else (a phone can't reach them) */
const VIRTUAL = /vEthernet|WSL|Hyper-V|Docker|VirtualBox|VMware|Loopback/i;
export function lanIps() {
	const all = Object.entries(networkInterfaces()).flatMap(([name, list]) => (list ?? []).filter((a) => a.family === 'IPv4' && !a.internal).map((a) => ({ name, ip: a.address })));
	const real = all.filter((a) => !VIRTUAL.test(a.name));
	return (real.length ? real : all).map((a) => a.ip);
}

/** the certificate for localhost + the LAN addresses (made with openssl when missing or an address is new) */
export function ensureCert(dir) {
	const key = resolve(dir, 'key.pem');
	const cert = resolve(dir, 'cert.pem');
	const list = resolve(dir, 'names.txt');
	const want = ['DNS:localhost', 'IP:127.0.0.1', ...lanIps().map((ip) => `IP:${ip}`)];
	const have = existsSync(list) ? readFileSync(list, 'utf8').trim().split(',') : [];
	if (!existsSync(key) || !existsSync(cert) || !want.every((n) => have.includes(n))) {
		mkdirSync(dir, { recursive: true });
		// the addresses it had stay in (a laptop between networks keeps one certificate for both)
		const names = [...new Set([...have.filter(Boolean), ...want])];
		execFileSync(process.env.OPENSSL || 'openssl', ['req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-sha256', '-days', '825', '-subj', '/CN=SCUTOONS storyboard studio', '-keyout', key, '-out', cert, '-addext', `subjectAltName=${names.join(',')}`], { stdio: 'pipe' });
		writeFileSync(list, names.join(','));
	}
	return { key: readFileSync(key), cert: readFileSync(cert) };
}

/** the LAN's HTTPS server in front of the studio on `host`:`target` (this PC only); `stop()` ends it with its open
 * connections and sockets (a studio restart) */
export function startPhoneServer({ port, host = '127.0.0.1', target, certDir, log = console.log }) {
	let tls;
	try {
		tls = ensureCert(certDir);
	} catch (e) {
		phone.error = `No certificate (${e.code === 'ENOENT' ? 'openssl not found – install Git for Windows or set OPENSSL=<path>' : e.message.split('\n')[0]}) – the phone can still come over USB.`;
		log(`  storyboard studio: ${phone.error}`);
		return null;
	}
	// its own keep-alive connections to the studio (stop() ends them)
	const agent = new Agent({ keepAlive: true });
	const server = createServer(tls, (req, res) => {
		if (req.method !== 'GET' && req.method !== 'HEAD') {
			res.writeHead(403, { 'content-type': 'application/json' });
			res.end(JSON.stringify({ error: 'writes only from this PC' }));
			return;
		}
		const up = request({ host, port: target, method: req.method, path: req.url, headers: req.headers, agent }, (r) => {
			res.writeHead(r.statusCode ?? 502, r.headers);
			r.pipe(res);
		});
		up.on('error', () => { if (!res.headersSent) res.writeHead(502); res.end(); });
		req.pipe(up);
	});
	// the WebSocket (Vite's – the relay's channel): the upgrade request as it came, then both ways as they are
	const tunnels = new Set();
	server.on('upgrade', (req, socket, head) => {
		tunnels.add(socket);
		socket.on('close', () => tunnels.delete(socket));
		const up = connect(target, host, () => {
			const lines = [`${req.method} ${req.url} HTTP/${req.httpVersion}`];
			for (let i = 0; i < req.rawHeaders.length; i += 2) lines.push(`${req.rawHeaders[i]}: ${req.rawHeaders[i + 1]}`);
			up.write(`${lines.join('\r\n')}\r\n\r\n`);
			if (head?.length) up.write(head);
			socket.pipe(up).pipe(socket);
		});
		// one side gone = both gone
		up.on('error', () => socket.destroy());
		socket.on('error', () => up.destroy());
		up.on('close', () => socket.destroy());
		socket.on('close', () => up.destroy());
	});
	server.on('error', (e) => {
		phone.on = false;
		phone.error = e.code === 'EADDRINUSE' ? `Port ${port} is taken – the phone can't come over Wi-Fi.` : e.message;
		log(`  storyboard studio: ${phone.error}`);
	});
	server.on('close', () => (phone.on = false));
	server.listen(port, '0.0.0.0', () => {
		Object.assign(phone, { on: true, port: server.address().port, error: '' });
		for (const ip of lanIps()) log(`  ➜  Phone (Wi-Fi): https://${ip}:${phone.port}/phone`);
	});
	return {
		server,
		stop: () => new Promise((done) => {
			server.close(() => done());
			server.closeAllConnections();
			for (const s of tunnels) s.destroy();
			agent.destroy();
		})
	};
}
