// The fixture every side parses: deck/protocol/fixtures/messages.json (the Rust round trip is deck/protocol/src/lib.rs).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import type { Envelope, Message, Role } from '../src/index.ts';
import { PROTOCOL_VERSION } from '../src/index.ts';

const fixture = fileURLToPath(new URL('../../../deck/protocol/fixtures/messages.json', import.meta.url));
const messages = JSON.parse(readFileSync(fixture, 'utf8')) as Envelope[];

/** The type-level check: every kind the schema knows narrows its payload. */
const describe = (m: Message): string => {
	switch (m.kind) {
		case 'hello':
			return `${m.p.role as Role}@${m.p.code}`;
		case 'device':
			return m.p.msgs.map((d) => (d.kind === 'midi' ? d.p.bytes.join(',') : d.kind)).join(';');
		case 'presence':
			return Object.keys(m.p.groups).join(',');
		default:
			return m.kind;
	}
};

test('fixture: every envelope has v 1, a seq and a kind', () => {
	assert.ok(messages.length >= 20);
	for (const m of messages) {
		assert.equal(m.v, PROTOCOL_VERSION);
		assert.equal(typeof m.seq, 'number');
		assert.equal(typeof m.from, 'string');
		assert.equal(typeof m.kind, 'string');
		assert.equal(typeof m.p, 'object');
	}
});

test('fixture: the narrowed payloads', () => {
	const out = messages.map(describe);
	assert.equal(out[0], 'output@A7K2');
	assert.ok(out.includes('159,12,100'));
	assert.ok(out.includes('left,right'));
});

test('fixture covers each kind of the schema at least once, except the loose ones not routed yet', () => {
	const kinds = new Set(messages.map((m) => m.kind));
	for (const k of ['hello', 'welcome', 'ping', 'pong', 'state', 'cue', 'cell', 'media.preload', 'media.ready', 'device', 'device.in', 'devices', 'scene.sync', 'deck.synced', 'error'])
		assert.ok(kinds.has(k as Message['kind']), k);
});
