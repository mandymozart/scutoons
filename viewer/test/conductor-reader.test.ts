// #529: the conductor page's linked reader – the link it hands out is the core's (deck-space linked.rs `reader_link`,
// relay-web's wasm) for the picked chapter and label, the label choices are the chapter's jump places (@scutoons/content
// `jumpLabels`), and the linked readers are the core's view (relay-web `readers()`, handed on by live-vote-link.ts) – with
// a reader driving on the real relay the conductor names it and the last story event. The wasm tests skip, saying so,
// when relay-web isn't built (`npm run build:wasm -w shared/live-deck -- --relay`); the relay test when the relay isn't
// (`cargo build -p relay`).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import type { Envelope } from '@scutoons/live';
import { jumpLabels } from '@scutoons/content';
import { loadRelay, type RelayModule } from '@scutoons/live-deck';
import { readerLink, type LinkModule, type LinkedReaders } from '../src/lib/conductor-reader.ts';
import { RelayVote } from '../src/lib/live-vote-link.ts';
import { deckTarget } from '../../shared/live-core/scripts/wasm.ts';

const wasmDir = new URL('../../shared/live-deck/wasm/', import.meta.url);
const NO_CORE = existsSync(new URL('relay_web_bg.wasm', wasmDir)) ? false : 'relay-web is not built (npm run build:wasm -w shared/live-deck -- --relay)';
const core = () => loadRelay(wasmDir.href, readFileSync(new URL('relay_web_bg.wasm', wasmDir))) as Promise<RelayModule & LinkModule>;
const relayExe = `${deckTarget()}/debug/relay.exe`;
const NO_RELAY = existsSync(relayExe) ? NO_CORE : `${relayExe} not built (cargo build -p relay)`;

test("the link to hand out is the core's for the picked chapter and label; the labels are the chapter's jump places", { skip: NO_CORE }, async () => {
	const c = await core();
	const groups = [{ label: 'start' }, {}, { label: 'dialog-start' }, { label: 'start' }];
	const labels = jumpLabels(groups);
	assert.deepEqual(labels, ['start', 'dialog-start']);
	const link = readerLink(c, 'https://scutoons.com', 'A7K2', { toon: 'catpope', chapter: 3, label: labels[1] });
	assert.equal(link, c.reader_link('https://scutoons.com', 'catpope', 3, 'dialog-start', 'A7K2'));
	assert.equal(link, 'https://scutoons.com/catpope/chapter/3?label=dialog-start&live=A7K2');
	assert.equal(readerLink(c, 'https://scutoons.com', 'A7K2', { toon: 'catpope', chapter: 3, label: '' }), 'https://scutoons.com/catpope/chapter/3?live=A7K2', "no label: the chapter's start");
	assert.equal(readerLink(c, 'https://scutoons.com', 'A7K2', { toon: 'catpope', chapter: null, label: '' }), '', 'no chapter: no link');
	assert.equal(readerLink(c, 'https://scutoons.com', 'A7K2', { toon: '', chapter: 3, label: '' }), '', 'no toon: no link');
});

/** a raw client on the relay: its messages */
function raw(url: string, hello: Record<string, unknown>): Promise<{ msgs: Envelope[]; ws: WebSocket; client: string }> {
	const code = url.split('/live/')[1];
	const ws = new WebSocket(`${url}?role=${hello.role}`);
	const msgs: Envelope[] = [];
	return new Promise((ok, fail) => {
		ws.onopen = () => ws.send(JSON.stringify({ v: 1, seq: 0, from: '', kind: 'hello', p: { code, ...hello } }));
		ws.onmessage = (e) => {
			const m = JSON.parse(String(e.data)) as Envelope;
			msgs.push(m);
			if (m.kind === 'welcome') ok({ msgs, ws, client: (m.p as { client: string }).client });
		};
		ws.onerror = () => fail(new Error('no relay'));
	});
}

const PATIENCE = 60_000;
const until = async (what: () => boolean) => {
	const end = Date.now() + PATIENCE;
	while (!what()) {
		if (Date.now() > end) throw new Error('timed out');
		await new Promise((r) => setTimeout(r, 20));
	}
};

test('in process: the conductor page names the driving reader and the last story event it heard', { skip: NO_RELAY }, async () => {
	const port = Number(process.env.LAB_RELAY_PORT ?? 7072) + 6;
	const env = { ...process.env };
	delete env.LIVE_TOKEN_SECRET;
	const relay = spawn(relayExe, [`127.0.0.1:${port}`], { env, stdio: 'ignore' });
	let conductor: RelayVote | null = null;
	try {
		const url = `ws://127.0.0.1:${port}/live/LINK`;
		let reader: Awaited<ReturnType<typeof raw>> | null = null;
		const end = Date.now() + PATIENCE;
		while (!reader) {
			try {
				reader = await raw(url, { role: 'reader', nick: 'Mira' });
			} catch (e) {
				if (Date.now() > end) throw e;
				await new Promise((r) => setTimeout(r, 100));
			}
		}
		let seen: LinkedReaders | null = null;
		conductor = new RelayVote({ relay: url, role: 'conductor', token: 'dev', core: core(), socket: WebSocket as never, onReaders: (r) => (seen = r) });
		const me = reader.client;
		await until(() => !!seen && me in seen.linked);
		assert.equal(seen!.linked[me], 'Mira', 'by the name it said');
		assert.equal(seen!.driver, null, 'nobody drives yet');
		reader.ws.send(JSON.stringify({ v: 1, seq: 1, from: '', kind: 'reader.drive', p: {} }));
		await until(() => seen?.driver === me);
		const join = { type: 'joinScene', id: 'ch3-e12', params: { characters: [{ character: 'catpope' }] } };
		reader.ws.send(JSON.stringify({ v: 1, seq: 2, from: '', kind: 'story', p: join }));
		await until(() => !!seen?.last);
		assert.deepEqual(seen!.last, { type: 'joinScene', characters: ['character:catpope'], from: me });
		reader.ws.close();
	} finally {
		conductor?.dispose();
		relay.kill();
	}
});
