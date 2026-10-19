// The phone's HTTPS server on the LAN (phone-https.mjs): a GET is passed on to the studio, a write is refused, the
// WebSocket upgrade is tunnelled both ways; the certificate covers localhost + 127.0.0.1. Needs openssl (skipped without).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createServer } from 'node:http';
import { request } from 'node:https';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { connect } from 'node:tls';
import { ensureCert, phone, startPhoneServer } from '../phone-https.mjs';

let openssl = true;
try { execFileSync(process.env.OPENSSL || 'openssl', ['version'], { stdio: 'ignore' }); } catch { openssl = false; }

const listen = (server, port = 0, host = '127.0.0.1') => new Promise((ok) => server.listen(port, host, () => ok(server.address().port)));
const call = (port, method, path) => new Promise((ok, fail) => {
	const req = request({ host: '127.0.0.1', port, method, path, rejectUnauthorized: false, agent: false }, (res) => {
		let body = '';
		res.on('data', (c) => (body += c));
		res.on('end', () => ok({ status: res.statusCode, body }));
	});
	req.on('error', fail);
	req.end();
});

test('the phone server: GET through, writes refused, the upgrade tunnelled', { skip: !openssl && 'openssl not found' }, async () => {
	const dir = mkdtempSync(join(tmpdir(), 'phone-https-'));
	// the studio stand-in: answers GETs and echoes after an upgrade
	const studio = createServer((req, res) => res.end(`${req.method} ${req.url}`));
	studio.on('upgrade', (req, socket) => {
		socket.write('HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n');
		socket.on('data', (d) => socket.write(`echo:${d}`));
		socket.on('end', () => socket.destroy());
	});
	const target = await listen(studio);
	// port 0: the system's pick (test files run in parallel)
	const { server, stop } = startPhoneServer({ port: 0, host: '127.0.0.1', target, certDir: dir, log: () => {} });
	await new Promise((ok, fail) => { server.once('listening', ok); server.once('error', fail); });
	const port = server.address().port;
	try {
		assert.equal(phone.on, true);
		assert.equal(phone.port, port);
		assert.deepEqual(await call(port, 'GET', '/phone?code=1234'), { status: 200, body: 'GET /phone?code=1234' });
		for (const method of ['POST', 'PUT', 'DELETE']) assert.equal((await call(port, method, '/api/takes')).status, 403);
		// the upgrade: raw TLS, the 101 comes back, then data both ways
		const echoed = await new Promise((ok, fail) => {
			const s = connect({ host: '127.0.0.1', port, rejectUnauthorized: false }, () => s.write('GET /?token=x HTTP/1.1\r\nHost: x\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n'));
			let got = '';
			s.on('data', (d) => {
				got += d;
				if (got.includes('101') && !got.includes('echo:')) s.write('ping');
				if (got.includes('echo:ping')) { s.end(); ok(got); }
			});
			s.on('error', fail);
		});
		assert.match(echoed, /101 Switching Protocols[\s\S]*echo:ping/);
		// the certificate names this PC
		const names = readFileSync(join(dir, 'names.txt'), 'utf8');
		assert.match(names, /DNS:localhost/);
		assert.match(names, /IP:127\.0\.0\.1/);
		// a second call keeps the same certificate
		const before = readFileSync(join(dir, 'cert.pem'), 'utf8');
		ensureCert(dir);
		assert.equal(readFileSync(join(dir, 'cert.pem'), 'utf8'), before);
	} finally {
		await stop();
		studio.closeAllConnections();
		studio.close();
		rmSync(dir, { recursive: true, force: true });
	}
	assert.equal(phone.on, false);
});
