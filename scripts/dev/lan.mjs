// The viewer on the LAN, to test /live/deck on a phone without pushing (owner 2026-10-31: "open the port in the
// local LAN, then I can test on the phone without it having to be pushed"):
//
//   npm run dev:lan [-- --port=4100] [--ip=192.168.1.116] [--relay=ws://127.0.0.1:7070]
//
// HTTPS with a self-signed certificate for this machine's LAN address (made once with openssl, kept in .dev-certs/,
// gitignored): a phone needs a secure context for the audio worklet, shared memory and WebGPU. The phone shows a
// certificate warning once – "Advanced → proceed". The production API and the local relay go through the dev server
// (viewer/vite.config.ts "LAN mode"), so the phone talks to one origin. Windows: the port needs an inbound firewall
// rule on the private network once (the owner's step, elevated – the command is printed below).
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdirSync } from 'node:fs';
import { networkInterfaces } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const arg = (name, fallback) => process.argv.find((a) => a.startsWith(`--${name}=`))?.split('=')[1] ?? fallback;

/** the LAN address: a private IPv4 (192.168.* first, then 10.*, 172.16–31.*), not a virtual adapter's */
function lanAddress() {
	const all = Object.entries(networkInterfaces())
		.filter(([name]) => !/vethernet|wsl|docker|virtualbox|vmware|hyper-v/i.test(name))
		.flatMap(([, list]) => list ?? [])
		.filter((a) => a.family === 'IPv4' && !a.internal)
		.map((a) => a.address);
	const rank = (ip) => (ip.startsWith('192.168.') ? 0 : ip.startsWith('10.') ? 1 : /^172\.(1[6-9]|2\d|3[01])\./.test(ip) ? 2 : 3);
	return all.sort((a, b) => rank(a) - rank(b))[0];
}

const ip = arg('ip', lanAddress());
const port = Number(arg('port', '4100'));
const relay = arg('relay', 'ws://127.0.0.1:7070');
if (!ip) {
	console.error('no LAN address found – pass --ip=…');
	process.exit(1);
}

// the certificate for this address (a new one when the address changes)
const certs = join(root, '.dev-certs');
const base = join(certs, `lan-${ip}`);
if (!existsSync(`${base}.crt`)) {
	mkdirSync(certs, { recursive: true });
	execFileSync('openssl', ['req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-sha256', '-days', '825', '-subj', `/CN=SCUTOONS dev ${ip}`, '-addext', `subjectAltName=IP:${ip},DNS:localhost`, '-keyout', `${base}.key`, '-out', `${base}.crt`], { stdio: 'ignore' });
}

const origin = `https://${ip}:${port}`;
console.log(`
  SCUTOONS viewer on the LAN
  phone:  ${origin}/de/live/deck
  relay:  ${origin}/de/live/deck?relay=wss://${ip}:${port}/relay/live/CODE   (the relay here: ${relay})
  first visit: the certificate warning → Advanced → proceed.
  Windows firewall, once (elevated PowerShell):
    New-NetFirewallRule -DisplayName "SCUTOONS dev LAN ${port}" -Direction Inbound -Protocol TCP -LocalPort ${port} -Profile Private -Action Allow
`);

const child = spawn('npm', ['run', 'dev', '-w', 'viewer', '--', '--port', String(port), '--strictPort'], {
	cwd: root,
	stdio: 'inherit',
	shell: process.platform === 'win32',
	env: {
		...process.env,
		SCUTOONS_LAN_CERT: base,
		SCUTOONS_LAN_RELAY: relay,
		VITE_API_URL: `${origin}/api`,
		VITE_CDN_URL: 'https://cdn.scutoons.com'
	}
});
child.on('exit', (code) => process.exit(code ?? 0));
