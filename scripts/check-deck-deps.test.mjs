// check-deck-deps' self-tests (#363 part B): a platform library or an unlisted crate in a core crate fails, main.rs and the
// platform logic files only shrink, the deny list only grows.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { functions, judge, native, ratchet } from './check-deck-deps.mjs';

const lists = { allow: { serde: 'serialization', toml: 'scenes' }, deny: { cpal: 'audio device', tokio: 'async runtime / sockets' } };

test('a core crate: listed crates pass, a platform library or an unlisted crate fails', () => {
	assert.deepEqual(judge('deck-core', ['serde', 'toml'], lists), []);
	const e = judge('deck-core', ['serde', 'cpal', 'smallvec'], lists);
	assert.equal(e.length, 2);
	assert.match(e[0], /deck-core depends on cpal – a platform library \(audio device\)/);
	assert.match(e[1], /smallvec, which scripts\/deck-deps\.json does not list/);
});

test('main.rs: no new function, no new line; a moved logic function is a note', () => {
	const base = { main: { file: 'm.rs', lines: 5, functions: ['main', 'hud_lines'], logic: { hud_lines: '#374 – the HUD' } }, files: {} };
	const ok = native(base, () => 'fn main() {}\nfn hud_lines() {}\n');
	assert.deepEqual(ok.errors, []);
	assert.deepEqual(ok.fns, ['main', 'hud_lines']);
	const grown = native(base, () => 'fn main() {}\nfn hud_lines() {}\npub(crate) fn stats_line() {}\n\n\n\n');
	assert.match(grown.errors.join('\n'), /6 lines, it may have 5/);
	assert.match(grown.errors.join('\n'), /fn stats_line is new/);
	const moved = native(base, () => 'fn main() {}\n');
	assert.match(moved.notes[0], /hud_lines left \(#374\)/);
	assert.deepEqual(functions('    pub fn a() {}\nconst fn b() {}\n// fn c\n'), ['a', 'b']);
});

test('the ratchet: native-logic.json only shrinks, the deny list only grows, the owner may approve', () => {
	const main = { main: { lines: 10, functions: ['a'] }, files: { 'x.rs': { lines: 5 } } };
	const now = { main: { lines: 11, functions: ['a', 'b'] }, files: { 'x.rs': { lines: 6 }, 'y.rs': { lines: 1 } } };
	const e = ratchet(main, now, { deny: { cpal: 'a', winit: 'w' } }, { deny: { cpal: 'a' } }, false).join('\n');
	assert.match(e, /main\.rs may have 10 lines on main, 11 here/);
	assert.match(e, /fn b is new/);
	assert.match(e, /x\.rs 5 → 6/);
	assert.match(e, /y\.rs new → 1/);
	assert.match(e, /winit left the deny list/);
	assert.deepEqual(ratchet(main, now, null, { deny: {} }, true), []);
	assert.deepEqual(ratchet(main, main, null, { deny: {} }, false), []);
});
