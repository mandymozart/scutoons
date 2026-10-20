// SCUTOONS Figma bridge – the relay's supervisor (#78): runs relay.mjs as a child and starts it again when it exits
// (backoff 1 s … 60 s, at most 30 starts an hour – past that it pauses, it never gives up). One per port: a lock file
// (logs/figma-supervisor.json) and the relay's /status. A relay that already answers on the port (started by hand) is
// watched, not fought: ours starts when that one is gone. Started DETACHED it outlives the terminal and every Claude
// session (failure F, 2026-10-19: the relay ran as a Claude background task and died with it). Nothing is installed:
// starting it at logon is the owner's choice (README.md "Setup").
// The lock outlives a crash, a reboot or a kill (nothing runs on exit then), and Windows reuses pids: so the supervisor
// writes a beat into it every 10 s, and a lock counts only while its process lives AND its beat is fresh (< 60 s), or
// the relay names that pid as its supervisor, or the pid's command line is supervise.mjs. Only such a pid is ever killed.
//
//   node tools/figma-bridge/supervise.mjs --detach    in the background (no window), returns at once – the usual way
//   node tools/figma-bridge/supervise.mjs             in the foreground (Ctrl+C stops it and the relay)
//   node tools/figma-bridge/supervise.mjs --status    the supervisor (lock) and the relay's /status
//   node tools/figma-bridge/supervise.mjs --restart   a fresh relay (new relay.mjs code) – the supervisor stays
//   node tools/figma-bridge/supervise.mjs --stop      the supervisor and the relay it started
//   options: --port 3056 (or SCUTOONS_FIGMA_PORT), --log-file <path>, --lock <path> (or SCUTOONS_FIGMA_LOG_DIR for both)
// PowerShell, the same by hand: Start-Process node -ArgumentList 'tools/figma-bridge/supervise.mjs' -WindowStyle Hidden
//   -WorkingDirectory C:\Development\scutoons

import { execFileSync, spawn } from 'node:child_process';
import { closeSync, existsSync, mkdirSync, openSync, readFileSync, renameSync, unlinkSync, writeFileSync, writeSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { defaultFiles, makeLogger, probe } from './relay.mjs';

const SELF = fileURLToPath(import.meta.url);
const RELAY = resolve(dirname(SELF), 'relay.mjs');
const BACKOFF = [1, 2, 5, 10, 30, 60]; // s, after quick exits in a row
const PER_HOUR = 30;
const HEALTHY = 10 * 60_000; // a relay that ran this long resets the backoff
const BEAT = 10_000, STALE = 60_000; // the lock's beat; older than STALE = its supervisor is gone (or the computer slept)

const argv = process.argv.slice(2);
const opt = (f) => { const i = argv.indexOf(f); return i >= 0 ? argv[i + 1] : undefined; };
const port = Number(opt('--port') ?? process.env.SCUTOONS_FIGMA_PORT ?? 3056);
const files = defaultFiles(port);
const logFile = opt('--log-file') ?? files.log;
const lockFile = opt('--lock') ?? files.lock;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const alive = (pid) => { try { process.kill(pid, 0); return true; } catch (e) { return e.code === 'EPERM'; } };
/** a process that is already gone (the relay dies with its supervisor: Windows kills a job's children) is no error */
const kill = (pid) => { try { process.kill(pid); return true; } catch (e) { if (e.code === 'ESRCH') return false; throw e; } };
const readLock = () => { try { return JSON.parse(readFileSync(lockFile, 'utf8')); } catch { return null; } };
const fresh = (l) => Date.now() - (Number(l.beat) || 0) < STALE;
/** the last lines of the log, for a launcher that has to say why no relay came up */
const tail = (n = 3) => { try { return readFileSync(logFile, 'utf8').trimEnd().split(/\r?\n/).slice(-n).join(' | '); } catch { return `no log yet (${logFile})`; } };

/** a process's command line (Windows: CIM through PowerShell, ~1 s; elsewhere /proc or ps), or null if it can't be read */
function commandLine(pid) {
	try {
		if (process.platform === 'win32') return execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', `(Get-CimInstance Win32_Process -Filter 'ProcessId = ${Number(pid)}').CommandLine`], { encoding: 'utf8', timeout: 20_000, windowsHide: true }).trim();
		if (existsSync(`/proc/${pid}/cmdline`)) return readFileSync(`/proc/${pid}/cmdline`, 'utf8').replace(/\0/g, ' ').trim();
		return execFileSync('ps', ['-o', 'args=', '-p', String(pid)], { encoding: 'utf8', timeout: 5_000 }).trim();
	} catch { return null; }
}
/** true / false: the pid is / is not a supervise.mjs; null: its command line can't be read */
const isSupervisor = (pid) => { const c = commandLine(pid); return c === null ? null : /supervise\.mjs/.test(c); };

/**
 * the running supervisor of this port (its lock), or null: the lock's process lives and its beat is fresh, the relay
 * names it, or it is a supervise.mjs (one without beats: an older one, or one whose beat was late after a sleep)
 */
async function current() {
	const l = readLock();
	if (!l?.pid || !alive(l.pid)) return null;
	if (fresh(l) || (await probe(port))?.supervisor === l.pid) return l;
	return isSupervisor(l.pid) ? l : null;
}
/** wait until `test(await probe(port))` holds, up to ms; → the last /status (or null) */
async function until(test, ms) {
	const t0 = Date.now();
	let s = await probe(port);
	while (!test(s) && Date.now() - t0 < ms) { await sleep(250); s = await probe(port); }
	return s;
}

async function detach() {
	const l = await current();
	if (l) {
		const s = await probe(port);
		console.log(`supervisor already running (pid ${l.pid}, since ${l.since}) – ${s ? `relay pid ${s.pid} on :${port}, plugin ${s.plugin ? 'connected' : 'not connected'}` : `but NO relay answers on :${port}; its log: ${tail()}`}`);
		if (!s) process.exitCode = 1;
		return;
	}
	const rest = argv.filter((a) => a !== '--detach');
	const child = spawn(process.execPath, [SELF, ...rest], { detached: true, stdio: 'ignore', windowsHide: true, cwd: resolve(dirname(SELF), '../..') });
	child.unref();
	const s = await until((x) => !!x, 15_000);
	if (s) console.log(`supervisor started (pid ${child.pid}) – relay pid ${s.pid} on :${port}${s.supervisor === child.pid ? '' : ' (started elsewhere – watched)'}, plugin ${s.plugin ? 'connected' : 'not connected yet'}; log ${logFile}`);
	else { console.log(`supervisor started (pid ${child.pid}), but no relay answers on :${port} after 15 s; the log: ${tail()}`); process.exitCode = 1; }
}

async function stop() {
	const l = await current(), s = await probe(port);
	try {
		// only a pid proven to be a supervisor: the relay names it, its command line says so, or (unreadable) a fresh beat
		if (l) {
			const sure = s?.supervisor === l.pid || ((k) => k === true || (k === null && fresh(l)))(isSupervisor(l.pid));
			if (!sure) console.log(`the lock names pid ${l.pid}, which is no supervise.mjs – left alone`);
			else if (kill(l.pid)) console.log(`supervisor ${l.pid} stopped`);
		}
		// only a relay a supervisor started – one started by hand in a terminal is left alone
		if (s?.pid && s.supervisor && (!l || s.supervisor === l.pid || !alive(s.supervisor))) { kill(s.pid); console.log(`relay ${s.pid} stopped`); }
		else if (s) console.log(`a relay not started by a supervisor runs on :${port} (pid ${s.pid}) – left alone`);
		if (!l && !s) console.log(`nothing runs on :${port}`);
	} finally {
		try { unlinkSync(lockFile); } catch {}
	}
	if (s?.supervisor) await until((x) => !x, 5_000);
}

async function restart() {
	const s = await probe(port);
	if (!s?.supervisor || !alive(s.supervisor)) { console.log(s ? `the relay on :${port} (pid ${s.pid}) has no supervisor – stop it and run --detach` : `no relay on :${port} – run --detach`); process.exitCode = 1; return; }
	kill(s.pid);
	const n = await until((x) => x && x.pid !== s.pid, 20_000);
	console.log(n && n.pid !== s.pid ? `relay restarted: pid ${s.pid} → ${n.pid} (relay ${n.relay})` : `relay ${s.pid} stopped – the supervisor has not brought a new one up yet, see ${logFile}`);
}

const since = new Date().toISOString();
const mine = () => JSON.stringify({ pid: process.pid, port, since, beat: Date.now(), log: logFile });
/** the lock: created exclusively; one that no longer counts (current()) is taken over */
async function lock() {
	mkdirSync(dirname(lockFile), { recursive: true });
	for (let i = 0; i < 2; i++) {
		try {
			const fd = openSync(lockFile, 'wx');
			writeSync(fd, mine());
			closeSync(fd);
			return true;
		} catch (e) {
			if (e.code !== 'EEXIST') throw e;
			if (await current()) return false;
			try { unlinkSync(lockFile); } catch {}
		}
	}
	return false;
}
/** the beat: the lock written again (to a temporary file, then renamed – a reader never sees half of it) */
function beat() {
	const tmp = `${lockFile}.${process.pid}.tmp`;
	try { writeFileSync(tmp, mine()); renameSync(tmp, lockFile); } catch { try { unlinkSync(tmp); } catch {} } // open elsewhere: the next beat
}

async function supervise() {
	if (!(await lock())) { const l = readLock(); console.log(`supervisor already running (pid ${l?.pid}, since ${l?.since}) – ${lockFile}`); return; }
	const say = makeLogger(logFile, { prefix: 'supervisor: ', rotate: false }); // the relay rotates the shared log
	let child = null, stopping = false;
	process.on('exit', () => { child?.kill(); if (readLock()?.pid === process.pid) try { unlinkSync(lockFile); } catch {} });
	for (const sig of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.on(sig, () => { stopping = true; say(`stopped (${sig})`); process.exit(0); });
	say(`started (pid ${process.pid}) for :${port}`);
	setInterval(() => {
		const l = readLock();
		// another supervisor took the lock (this one's beat was late – the computer slept): one is enough, this one goes
		if (l?.pid && l.pid !== process.pid && alive(l.pid) && fresh(l)) { say(`supervisor ${l.pid} holds the lock now – this one (${process.pid}) leaves`); stopping = true; process.exit(0); }
		beat();
	}, BEAT).unref();

	const run = () => new Promise((done) => {
		let ended = false;
		const end = (code) => { if (!ended) { ended = true; child = null; done(code); } };
		child = spawn(process.execPath, [RELAY, '--port', String(port), '--log-file', logFile], { stdio: ['ignore', 'inherit', 'pipe'], windowsHide: true, env: { ...process.env, SCUTOONS_FIGMA_SUPERVISOR: String(process.pid) } });
		child.stderr.on('data', (d) => { process.stderr.write(d); for (const line of String(d).split(/\r?\n/).filter(Boolean)) say(`relay stderr: ${line}`); });
		child.on('error', (e) => { say(`can't start the relay: ${e.message}`); end(-1); });
		child.on('exit', (code, signal) => end(code ?? signal));
	});

	const starts = []; // start times within the last hour
	let quick = 0, watching = null;
	while (!stopping) {
		const s = await probe(port);
		if (s) { // a relay answers already (by hand, or left by a supervisor before us): watch it, take over when it is gone
			if (watching !== s.pid) { watching = s.pid; say(`a relay already answers on :${port} (pid ${s.pid}) – watching it`); }
			await sleep(10_000);
			continue;
		}
		watching = null;
		const now = Date.now();
		while (starts.length && now - starts[0] > 3_600_000) starts.shift();
		if (starts.length >= PER_HOUR) {
			const resume = starts[0] + 3_600_000;
			say(`${PER_HOUR} starts in the last hour – pausing until ${new Date(resume).toISOString()}`);
			await sleep(resume - now);
			continue;
		}
		starts.push(now);
		const code = await run();
		if (stopping) break;
		const lived = Date.now() - now;
		if (lived > HEALTHY) quick = 0;
		if (await probe(port)) continue; // "already running" (exit 0): another relay won the port – watch it
		const wait = (code === 2 ? 60 : BACKOFF[Math.min(quick, BACKOFF.length - 1)]) * 1000;
		quick++;
		say(`relay exited (code ${code}) after ${Math.round(lived / 1000)} s – starting it again in ${wait / 1000} s`);
		await sleep(wait);
	}
}

if (argv.includes('--detach')) await detach();
else if (argv.includes('--stop')) await stop();
else if (argv.includes('--restart')) await restart();
else if (argv.includes('--status')) console.log(JSON.stringify({ supervisor: await current(), lock: lockFile, relay: await probe(port) }, null, 2));
else await supervise();
