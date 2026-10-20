// Storyboard tooling – shared constants, the rhythm model, the safe-format envelope, reading entities, and the
// self-healing client of the SCUTOONS Figma bridge (figma(), figmaPreflight(), the run helpers – the end of the file).
// Plan + rules: agents/STORYBOARD.md (§1 formats, §6 rhythm, §9 build strategy), workflows/scenes/storyboard.md.
// Plain Node (no dependencies), so it runs in any checkout / worktree without an install.

import { execFileSync, spawn } from 'node:child_process';
import { closeSync, existsSync, openSync, readFileSync, readSync, renameSync, writeFileSync } from 'node:fs';
import { request } from 'node:http';
import { freemem } from 'node:os';
import { basename, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

/**
 * Phone canvas: width W, one screen S (390 × 844), the standard slice height SLICE_H = DEFAULT_SLICE (shared/content
 * content.ts; plain Node, so a copy): 624 since 2026-10-20 (#78, its 3x = WEBTOON's 800 × 1280); chapters 1–5 were cut at 625
 */
export const W = 390;
export const S = 844;
export const SLICE_H = 624;

/**
 * The rhythm model (STORYBOARD.md §6), calibrated on chapter 1 (owner 2026-10-09: careful reading 15–20 min; chapters
 * 1 and 2 about the same length, ch2 with more frames and less text). Seconds.
 */
export const RHYTHM = {
	scrollPerScreen: 1.2, // empty scrolling
	lookPerScreen: { careful: 10, skim: 3 }, // looking at the image; careful fitted to ch1 (see rhythm-report --fit)
	perBox: 0.8, // finding a text box / message
	wordsPerSecond: 4, // ~240 wpm
	perClick: 1, // a VN message waiting for a tap
	wall: { words: 35, boxes: 3 }, // per screen
	choicelessMessages: 6, // VN "talking heads"
	airBeat: 0.25, // × W between two beats
	airScene: [0.75, 1.25] // × W at a scene change
};

/** Length target for a chapter (owner 2026-10-09): about chapters 1 and 2 – ~70 screens, 15–20 min careful */
export const TARGET = { screens: [60, 80], minutes: [15, 20] };

/**
 * Safe formats (owner 2026-10-09: "a restriction for what we prompt and suggest"): what the generators are proven to
 * deliver today. Suggest inside, plan below the maximum; beyond = a warning, never an error. Update when a tool proves more.
 */
export const SAFE = {
	viewport: { aspect: '9:16', note: 'one screen' },
	vertical: { maxTall: 7, plan: 4, proved: 'generators (1:4 rare, 1:7 the limit)', date: '2026-10-09' }, // h/w
	loop360: { maxScreens: 13, plan: 8, proved: 'Luma Labs (widest without massive problems; usually less)', date: '2026-10-09' } // width in screens of 390
};

export const FORMATS = ['viewport', 'vertical', 'loop360', 'strip'];
export const PROJECTIONS = ['perspective', 'top-down', 'isometric', 'polar'];
export const MOVES = ['hold', 'tilt-down', 'tilt-up', 'fall', 'climb', 'travel', 'descend', 'reveal', 'look-around', 'pass'];
export const SIZES = ['ecu', 'cu', 'mcu', 'medium', 'full', 'wide', 'ews'];
export const ANGLES = ['eye', 'high', 'low', 'bird', 'worm', 'dutch'];
/** light (workflows/scenes/storyboard.md "light"): extremely hard, one key, never diffuse (owner 2026-10-09) */
export const LIGHT = {
	key: ['left', 'right', 'top', 'bottom', 'front', 'back', 'top-left', 'top-right', 'bottom-left', 'bottom-right'],
	height: ['below', 'low', 'eye', 'high', 'overhead'],
	source: ['sun', 'moon', 'snow', 'window', 'door', 'candle', 'screen', 'neon', 'fire', 'lightning', 'flash', 'energy', 'none'],
	shadow: ['long', 'short', 'none']
};
export const POSES = ['stand', 'walk', 'run', 'jump', 'fall', 'slip', 'crouch', 'kneel', 'crawl', 'sit', 'lie', 'reach', 'point', 'throw', 'cast', 'arms-up', 'hug-self', 'stamp', 'push', 'look-back', 'pray'];
export const FACING = ['front', 'away', 'left', 'right', 'three-quarter-left', 'three-quarter-right'];
export const VIEWS = ['real', 'belief', 'narration', 'memory'];
export const SOURCES = ['schematic', 'photo', 'unreal', 'direct', 'generated', 'art'];
export const STATUS = ['draft', 'layout-ok', 'refs-locked', 'generating', 'still-ok', 'in-chapter'];

/** "1:4" → { w: 1, h: 4 }; "9:16" → { w: 9, h: 16 }; "6:1" → { w: 6, h: 1 } */
export function parseAspect(a) {
	const m = /^(\d+(?:\.\d+)?):(\d+(?:\.\d+)?)$/.exec(String(a ?? ''));
	return m ? { w: Number(m[1]), h: Number(m[2]) } : null;
}

/** height in px of a shot of this aspect on the 390-wide canvas (loop360: the height of one slide row) */
export function shotHeight(aspect, format) {
	const a = parseAspect(aspect);
	if (!a) return S;
	if (format === 'loop360') return S; // a panorama is read as one screen-high stop, dragged sideways
	return Math.round((W * a.h) / a.w);
}

export function words(text) {
	const t = typeof text === 'string' ? text : (text?.de ?? Object.values(text ?? {})[0] ?? '');
	return String(t).split(/\s+/).filter((w) => /[\p{L}\p{N}]/u.test(w)).length;
}

export function textOf(text, locale = 'de') {
	if (typeof text === 'string') return text;
	return text?.[locale] ?? Object.values(text ?? {})[0] ?? '';
}

/** reading time of one text cluster (a box or a message) in seconds */
export function readSeconds(n) {
	return RHYTHM.perBox + n / RHYTHM.wordsPerSecond;
}

/**
 * Read an entity's latest version from the LOCAL dev database (read-only SELECT through the MariaDB container –
 * no client needed on Windows). status: 'published' | 'draft' | 'any'.
 */
export function readEntity(key, status = 'published') {
	const db = process.env.SCUTOONS_DB_NAME || 'scutoons';
	const where = status === 'any' ? '' : ` AND status='${status}'`;
	const sql = `SELECT json FROM \`${db}\`.entity_versions WHERE entity_key='${key.replace(/'/g, '')}'${where} ORDER BY id DESC LIMIT 1`;
	const out = execFileSync('docker', ['exec', 'scutoons-mariadb-1', 'mariadb', '-uroot', '-pscutoons-root', '-N', '--raw', '-e', sql], {
		encoding: 'utf8',
		maxBuffer: 64 * 1024 * 1024
	}).trim();
	if (!out) throw new Error(`entity ${key} (${status}) not found in ${db}`);
	return JSON.parse(out);
}

/** a chapter from a file (json) or the local DB ("catpope:1") */
export function loadChapter(ref) {
	if (existsSync(ref)) return JSON.parse(readFileSync(ref, 'utf8'));
	return readEntity(`chapter:${ref}`);
}

/** The media dev copy (gitignored, in the main checkout): for slice thumbnails. Production is the truth. */
export function mediaRoot() {
	const candidates = [process.env.SCUTOONS_MEDIA, resolve(ROOT, 'media'), resolve(ROOT, '../scutoons/media')].filter(Boolean);
	return candidates.find((p) => existsSync(p)) ?? null;
}

export function esc(s) {
	return String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
}

export const fmtMin = (sec) => `${(sec / 60).toFixed(1)} min`;

// ── The SCUTOONS Figma bridge client (#78) ──────────────────────────────────────────────────────────────────────────
// figma(cmd, args, opts): one command through the relay (tools/figma-bridge), self-healing, so a long run gets over a
// slow or restarted Figma by itself (owner 2026-10-20: "i dont like manually restarting figma and the bridge or claude
// or the machine"):
//   no relay (refused)    start it once, detached, through supervise.mjs (it outlives this script), then go on – if it
//                         does not come up, the error carries the supervisor's own words (and the log's last lines)
//   no plugin / it left   ONE line telling the owner what to click, /status every 5 s, go on when it is back (up to
//                         30 min – opts.waitPlugin, SCUTOONS_FIGMA_WAIT_MIN)
//   another file          opts.expectFile (figmaPreflight's `file`): a plugin that (re)connected is checked before the
//                         next call – in another file it is waited for like a missing one (node ids are per file)
//   timeout               never re-sent at once: wait until /status says the plugin is free, then retry after 10 / 30 /
//                         60 / 120 s, the timeout doubled each time (opts.retries = 4 retries, so 5 sends at most);
//                         Figma busy for longer than waitPlugin → plugin_stuck (a run ends: every later call would wait)
//   plugin error          thrown at once (code plugin_error) – a wrong node id does not get better by retrying
// Errors are FigmaError with a code: no_relay · plugin_disconnected · wrong_file · timeout · queue_timeout ·
// plugin_stuck · plugin_error · bad_request · low_memory · old_plugin. node:http, not fetch: fetch (undici) gives up
// after 300 s without headers, and every such failure used to read "no relay" (failure G). Before a long run:
// figmaPreflight().

export const FIGMA_PORT = Number(process.env.SCUTOONS_FIGMA_PORT ?? 3056);
/** per command (ms): past this the relay answers `timeout` – Figma keeps working, the relay keeps the queue waiting */
export const FIGMA_TIMEOUTS = { ping: 15_000, pages: 30_000, read: 120_000, export: 120_000, region: 120_000, exportScales: 120_000, thumbs: 240_000, sliceFrames: 240_000, draft: 240_000, place: 240_000 };
const FIGMA_MAX_TIMEOUT = 900_000; // the relay's cap
const FIGMA_FIX = 'in Figma desktop: Ctrl+Alt+P (Run last plugin) – or Plugins → Development → SCUTOONS Bridge';
const SUPERVISE = resolve(ROOT, 'tools/figma-bridge/supervise.mjs');
const CLIENT = basename(process.argv[1] ?? 'node');
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const minutes = (ms) => `${Math.round(ms / 60_000)} min`;

export class FigmaError extends Error {
	constructor(code, message) { super(message); this.code = code; }
}

function figmaDefaults(cmd, opts) {
	return {
		port: FIGMA_PORT, timeout: FIGMA_TIMEOUTS[cmd] ?? 120_000, wait: 600_000, retries: 4, backoff: [10_000, 30_000, 60_000, 120_000],
		waitPlugin: (Number(process.env.SCUTOONS_FIGMA_WAIT_MIN) || 30) * 60_000, poll: 5_000, probe: 30_000, relayWait: 30_000, autostart: true, client: CLIENT, expectFile: null,
		say: (m) => process.stdout.write(`\n[figma] ${m}\n`),
		...opts
	};
}

/** one HTTP exchange with the relay → { status, body }; rejects with the socket's code (ECONNREFUSED …) */
function relayHttp(port, method, path, body, timeout) {
	return new Promise((ok, fail) => {
		const data = body === undefined ? null : Buffer.from(JSON.stringify(body));
		const req = request({ host: '127.0.0.1', port, method, path, agent: false, headers: data ? { 'content-type': 'application/json', 'content-length': data.length } : {} }, (res) => {
			const chunks = [];
			let n = 0;
			res.on('data', (c) => { chunks.push(c); n += c.length; });
			res.on('end', () => {
				try { ok({ status: res.statusCode, body: JSON.parse(Buffer.concat(chunks, n).toString('utf8')) }); }
				catch { fail(Object.assign(new Error(`an unreadable answer from the relay (HTTP ${res.statusCode})`), { code: 'EBADANSWER' })); }
			});
			res.on('error', fail);
		});
		// a socket idle this long means the relay itself hangs (it answers every call within timeout + wait)
		if (timeout) req.setTimeout(timeout, () => req.destroy(Object.assign(new Error(`no answer from the relay within ${Math.round(timeout / 1000)} s`), { code: 'ETIMEDOUT' })));
		req.on('error', fail);
		req.end(data);
	});
}

/** the relay's GET /status, or null when nothing answers on the port */
export async function figmaStatus(port = FIGMA_PORT) {
	try { return (await relayHttp(port, 'GET', '/status', undefined, 5_000)).body; } catch { return null; }
}

/** one try, no healing: { ok: true, result } | { ok: false, code, error } (cli.mjs; figma() builds on it) */
export async function figmaOnce(cmd, args = {}, opts = {}) {
	const o = figmaDefaults(cmd, opts);
	try {
		const { body } = await relayHttp(o.port, 'POST', '/cmd', { cmd, args, timeout: o.timeout, wait: o.wait, client: o.client }, o.timeout + o.wait + 30_000);
		if (body.ok) return body;
		// a relay from before #78 answers without a code
		const code = body.code ?? (/no plugin/.test(body.error) ? 'no_plugin' : /^timeout/.test(body.error) ? 'timeout' : 'plugin_error');
		return { ok: false, code, error: body.error };
	} catch (e) {
		if (e.code === 'ECONNREFUSED') return { ok: false, code: 'no_relay', error: `no relay on 127.0.0.1:${o.port} – node tools/figma-bridge/supervise.mjs --detach` };
		if (e.code === 'ETIMEDOUT') return { ok: false, code: 'relay_hung', error: e.message };
		return { ok: false, code: 'relay_lost', error: `the connection to the relay broke (${e.code ?? e.message}) – it may have restarted` };
	}
}

const launched = new Map(); // port → what `supervise.mjs --detach` said when this process started it (once per port)
/** the relay answers – else start it once (detached, supervised) and wait up to relayWait (30 s); if not: why, in the error */
async function relayUp(o) {
	if (await figmaStatus(o.port)) return;
	if (o.autostart && !launched.has(o.port)) {
		o.say(`no relay on :${o.port} – starting it in the background (tools/figma-bridge/supervise.mjs --detach, log logs/figma-relay.log)`);
		launched.set(o.port, '');
		launched.set(o.port, await new Promise((done) => {
			let out = '';
			const p = spawn(process.execPath, [SUPERVISE, '--detach', '--port', String(o.port)], { stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true });
			p.stdout.on('data', (d) => (out += d));
			p.stderr.on('data', (d) => (out += d));
			p.on('error', (e) => done(`can't run it: ${e.message}`));
			p.on('close', () => done(out.trim().split(/\r?\n/).slice(-3).join(' / ')));
		}));
	}
	const t0 = Date.now();
	while (Date.now() - t0 < o.relayWait) {
		if (await figmaStatus(o.port)) return;
		await sleep(Math.min(1_000, o.relayWait / 4));
	}
	const said = launched.get(o.port);
	throw new FigmaError('no_relay', `no relay on 127.0.0.1:${o.port}${said ? ` – the supervisor said: ${said}` : ''} (node tools/figma-bridge/supervise.mjs --status; the log: logs/figma-relay.log)`);
}

const checked = new Map(); // port → pluginSince of the plugin found in opts.expectFile
/** the plugin on the relay works in opts.expectFile → true; in another file → its name; unknown right now → null */
async function fileOk(o, s) {
	if (!o.expectFile || checked.get(o.port) === s.pluginSince) return true;
	let file = s.file; // from the plugin's hello (code 2026-10-20.2 on) – else a ping says it
	if (file == null) {
		const r = await figmaOnce('ping', {}, { ...o, timeout: FIGMA_TIMEOUTS.ping });
		if (!r.ok) return null;
		file = r.result.file;
	}
	if (file !== o.expectFile) return file;
	checked.set(o.port, s.pluginSince);
	return true;
}

/** wait until the plugin is connected, answers the heartbeat and works in the run's file; one line for the owner, one when it is back */
async function pluginBack(o) {
	const t0 = Date.now();
	let told = false, other = null;
	for (;;) {
		const s = await figmaStatus(o.port);
		if (!s) { await relayUp(o); continue; }
		if (s.plugin && !s.stale) {
			const f = await fileOk(o, s);
			if (f === true) {
				if (told || other) o.say(`the plugin is back after ${Math.round((Date.now() - t0) / 1000)} s – going on`);
				return s;
			}
			if (f && f !== other) {
				other = f;
				o.say(`the SCUTOONS Bridge runs in the Figma file "${f}", this run needs "${o.expectFile}" – open "${o.expectFile}" in Figma desktop and run the plugin THERE (Ctrl+Alt+P in that tab). Waiting up to ${minutes(o.waitPlugin)}, the run goes on by itself.`);
			}
		} else if (!told) {
			told = true;
			o.say(s.plugin
				? `the connection to the plugin is dead (no heartbeat for 30 s – Figma closed, crashed or asleep). When Figma is back: ${FIGMA_FIX}. Waiting up to ${minutes(o.waitPlugin)}, the run goes on by itself.`
				: `waiting for the SCUTOONS Bridge plugin – ${FIGMA_FIX}. Waiting up to ${minutes(o.waitPlugin)}, the run goes on by itself.`);
		}
		if (Date.now() - t0 > o.waitPlugin) {
			throw other
				? new FigmaError('wrong_file', `the plugin runs in "${other}", not in "${o.expectFile}", for ${minutes(o.waitPlugin)} – run it there, then run the script again (it resumes)`)
				: new FigmaError('plugin_disconnected', `no plugin for ${minutes(o.waitPlugin)} – ${FIGMA_FIX}, then run the script again (it resumes)`);
		}
		await sleep(o.poll);
	}
}

/**
 * wait until the relay says the plugin is free (after a timeout: Figma still works on the old call). The heartbeat can't
 * see a stuck Figma (its pongs come from the network layer); an application ping every opts.probe ms can (the plugin
 * answers it between the steps of a command) – 3 unanswered in a row: the "frozen" line. Busy past waitPlugin:
 * plugin_stuck – the run stops, every later call would only queue behind it
 */
async function notBusy(o) {
	const t0 = Date.now();
	let told = false, silent = 0, probed = Date.now();
	for (;;) {
		const s = await figmaStatus(o.port);
		if (!s) { await relayUp(o); continue; }
		if (!s.plugin || s.stale) { await pluginBack(o); continue; }
		if (!s.busy) return;
		const busyFor = minutes(Date.now() - Date.parse(s.busy.since));
		if (Date.now() - probed >= o.probe) {
			probed = Date.now();
			silent = (await figmaOnce('ping', {}, { ...o, timeout: Math.min(FIGMA_TIMEOUTS.ping, o.probe) })).ok ? 0 : silent + 1;
			if (silent === 3) o.say(`Figma answers no ping (busy with ${s.busy.cmd} for ${busyFor}) – it looks frozen. If it stays so: restart Figma, open the file, ${FIGMA_FIX}. The run goes on by itself.`);
		}
		if (!told && Date.now() - t0 > 5 * 60_000) {
			told = true;
			o.say(`Figma has been busy with ${s.busy.cmd} for ${busyFor}. If Figma is frozen: close the SCUTOONS Bridge window, then ${FIGMA_FIX} (or restart Figma). The run goes on by itself.`);
		}
		if (Date.now() - t0 > o.waitPlugin) throw new FigmaError('plugin_stuck', `Figma has been busy with ${s.busy.cmd} for ${busyFor} – the run stops (restart Figma if it is frozen, ${FIGMA_FIX}; then run again – it resumes)`);
		await sleep(o.poll);
	}
}

/**
 * one bridge command, self-healing (the rules at the head of this section) → the plugin's result. opts: timeout (ms,
 * default by command – FIGMA_TIMEOUTS), retries (4), backoff (ms list), waitPlugin (ms), wait (ms in the relay's queue),
 * poll / probe (ms), port, client (for the relay's log), say (a line for the owner), autostart (the relay, true),
 * expectFile (the Figma file's name – figmaPreflight's `file`)
 */
export async function figma(cmd, args = {}, opts = {}) {
	const o = figmaDefaults(cmd, opts);
	let tries = 0, slow = 0;
	for (;;) {
		if (o.expectFile) { // a plugin that (re)connected since the last check: in the right file?
			const s = await figmaStatus(o.port);
			if (s?.plugin && checked.get(o.port) !== s.pluginSince) await pluginBack(o);
		}
		// after a timeout twice the time: a slice that needs 3 min would time out at 2 every time
		const timeout = Math.min(o.timeout * 2 ** slow, FIGMA_MAX_TIMEOUT);
		const r = await figmaOnce(cmd, args, { ...o, timeout });
		if (r.ok) return r.result;
		if (r.code === 'no_relay') { await relayUp(o); continue; } // never reached the plugin: not a try
		if (r.code === 'no_plugin') { await pluginBack(o); continue; } // never sent: not a try
		if (r.code === 'plugin_error' || r.code === 'bad_request' || r.code === 'forbidden') throw new FigmaError(r.code, `${cmd}: ${r.error}`);
		// plugin_disconnected, relay_lost, timeout, queue_timeout, relay_hung: the call may have reached Figma – a try
		if (++tries > o.retries) throw new FigmaError(r.code === 'relay_lost' ? 'plugin_disconnected' : r.code === 'relay_hung' ? 'timeout' : r.code, `${cmd}: ${r.error} (gave up after ${tries} tries)`);
		if (r.code === 'plugin_disconnected' || r.code === 'relay_lost') { await pluginBack(o); continue; }
		if (r.code === 'timeout') slow++;
		await notBusy(o);
		const pause = o.backoff[Math.min(tries - 1, o.backoff.length - 1)];
		o.say(`${cmd}: ${r.code} – the plugin is free again; retry ${tries}/${o.retries} in ${Math.round(pause / 1000)} s${slow ? ` (timeout ${Math.round(Math.min(o.timeout * 2 ** slow, FIGMA_MAX_TIMEOUT) / 1000)} s)` : ''}`);
		await sleep(pause);
	}
}

/** the plugin code's version on disk (VERSION in tools/figma-bridge/plugin/code.js) – what the running plugin must say */
export function figmaPluginVersion() {
	return /const VERSION = '([^']+)'/.exec(readFileSync(resolve(ROOT, 'tools/figma-bridge/plugin/code.js'), 'utf8'))?.[1] ?? null;
}

/** at least minFreeGB of RAM free – else one line and wait (the 2026-10-19 crash: the machine out of memory) */
export async function enoughMemory(opts = {}) {
	const o = { minFreeGB: 4, ...figmaDefaults('', { poll: 30_000, ...opts }) };
	const gb = () => freemem() / 2 ** 30;
	if (gb() >= o.minFreeGB) return gb();
	o.say(`only ${gb().toFixed(1)} GB RAM free (want ${o.minFreeGB}) – close something (dev servers, browser tabs); waiting, the run goes on by itself`);
	const t0 = Date.now();
	while (gb() < o.minFreeGB) {
		if (Date.now() - t0 > o.waitPlugin) throw new FigmaError('low_memory', `still only ${gb().toFixed(1)} GB RAM free after ${minutes(o.waitPlugin)}`);
		await sleep(o.poll);
	}
	o.say(`${gb().toFixed(1)} GB RAM free – going on`);
	return gb();
}

/**
 * before a long run: the relay up (started if not), the plugin connected and answering, a ping round trip, the
 * plugin's code = code.js on disk (else one line: re-run the plugin – and wait for it), enough free RAM.
 * opts: version (expected; false = don't check), minFreeGB (4), + figma()'s → { file, page, version, pingMs, freeGB }.
 * The run then passes `expectFile: file` to every figma() call: the plugin re-run in another file is waited for
 */
export async function figmaPreflight(opts = {}) {
	const o = { version: figmaPluginVersion(), minFreeGB: 4, ...figmaDefaults('ping', opts) };
	await relayUp(o);
	await pluginBack(o);
	let t0 = Date.now();
	let ping = await figma('ping', {}, o);
	let pingMs = Date.now() - t0;
	if (o.version && ping.version !== o.version) {
		o.say(`the plugin runs ${ping.version ? `code ${ping.version}` : 'old code'}, code.js is ${o.version} – close the SCUTOONS Bridge window, then ${FIGMA_FIX}. Waiting, the run goes on by itself.`);
		const t1 = Date.now();
		for (;;) {
			await sleep(o.poll);
			const s = await figmaStatus(o.port);
			if (s?.plugin && !s.stale && s.version === o.version) break;
			if (Date.now() - t1 > o.waitPlugin) throw new FigmaError('old_plugin', `the plugin still runs ${ping.version ?? 'old code'}, not ${o.version} – ${FIGMA_FIX}`);
		}
		t0 = Date.now();
		ping = await figma('ping', {}, o);
		pingMs = Date.now() - t0;
		o.say(`the plugin runs ${ping.version} now – going on`);
	}
	if (pingMs > 5_000) o.say(`a ping took ${(pingMs / 1000).toFixed(1)} s – Figma is slow right now`);
	const freeGB = await enoughMemory({ ...o, poll: 30_000 });
	return { file: ping.file, page: ping.page, version: ping.version ?? null, pingMs, freeGB: +freeGB.toFixed(1) };
}

/**
 * pacing for long export runs: after every `every` calls, and after a call slower than twice the median so far, a
 * pause (Figma catches up – its memory, its autosave); RAM below minFreeGB waits (enoughMemory). → async (ms) => reason
 */
export function figmaPacer({ every = 25, pause = 15_000, minFreeGB = 4, say } = {}) {
	const times = [];
	return async (ms) => {
		const sorted = [...times].sort((a, b) => a - b), median = sorted[sorted.length >> 1];
		times.push(ms);
		const why = times.length % every === 0 ? `${times.length} calls` : sorted.length >= 5 && ms > 2 * median ? `a slow call (${(ms / 1000).toFixed(1)} s, median ${(median / 1000).toFixed(1)} s)` : null;
		if (why) await sleep(pause);
		if (freemem() / 2 ** 30 < minFreeGB) await enoughMemory({ minFreeGB, ...(say ? { say } : {}) });
		return why;
	};
}

/**
 * a run manifest (JSON, written atomically after every item): which outputs of a long run are done and from what, so a
 * crashed run resumes where it stopped → { data, get(name), set(name, entry) }
 */
export function runManifest(file, meta = {}) {
	let data = { ...meta, items: {} };
	try { const old = JSON.parse(readFileSync(file, 'utf8')); data = { ...old, ...meta, items: old.items ?? {} }; } catch {}
	const save = () => { writeFileSync(`${file}.tmp`, JSON.stringify(data, null, '\t')); renameSync(`${file}.tmp`, file); };
	return {
		data,
		get: (name) => data.items[name],
		set(name, entry) { data.items[name] = { ...entry, at: new Date().toISOString() }; save(); }
	};
}

/** width × height of a PNG or WebP from its header, or null */
export function imageSize(file) {
	const b = Buffer.alloc(32);
	try { const fd = openSync(file, 'r'); try { readSync(fd, b, 0, 32, 0); } finally { closeSync(fd); } } catch { return null; }
	if (b.readUInt32BE(0) === 0x89504e47) return { w: b.readUInt32BE(16), h: b.readUInt32BE(20) };
	if (b.toString('latin1', 0, 4) !== 'RIFF' || b.toString('latin1', 8, 12) !== 'WEBP') return null;
	const kind = b.toString('latin1', 12, 16);
	if (kind === 'VP8 ') return { w: b.readUInt16LE(26) & 0x3fff, h: b.readUInt16LE(28) & 0x3fff };
	if (kind === 'VP8L') { const v = b.readUInt32LE(21); return { w: (v & 0x3fff) + 1, h: ((v >>> 14) & 0x3fff) + 1 }; }
	if (kind === 'VP8X') return { w: 1 + b.readUIntLE(24, 3), h: 1 + b.readUIntLE(27, 3) };
	return null;
}

/** the slice's three WebPs (<name>.webp, .2x, .3x) exist in `dir` at w × h, 2w × 2h, 3w × 3h – a 625 file is no 624 one */
export function variantsOk(dir, name, w, h) {
	return [['', 1], ['.2x', 2], ['.3x', 3]].every(([suf, s]) => {
		const size = imageSize(resolve(dir, `${name}${suf}.webp`));
		return size?.w === w * s && size?.h === h * s;
	});
}

/** <dir>/png/<name>@{1,2,3}x.png → <dir>/slices/<name>{,.2x,.3x}.webp (PIL, quality 90; written to .tmp, then renamed) */
export function webpVariants(dir, name) {
	execFileSync('python', ['-I', '-c', `import os, sys
from PIL import Image
d, n = sys.argv[1], sys.argv[2]
for s, suf in ((1, ''), (2, '.2x'), (3, '.3x')):
    out = f'{d}/slices/{n}{suf}.webp'
    Image.open(f'{d}/png/{n}@{s}x.png').convert('RGB').save(out + '.tmp', 'WEBP', quality=90, method=6)
    os.replace(out + '.tmp', out)`, dir, name]);
}
