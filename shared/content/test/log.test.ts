// The Log's typed calls hand JSON to space-web's Log and read its answers back – no rule of their own (the rules:
// deck-ui messages.rs).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { logCore, type LogModule } from '../src/log.ts';

test('the typed calls pass lines and filters as JSON and read the answers', () => {
	const calls: string[] = [];
	const mod: LogModule = {
		Log: class {
			post(m: string) {
				calls.push(`post ${m}`);
			}
			stand(p: string, l: string) {
				calls.push(`stand ${p} ${l}`);
			}
			clear(k: string) {
				calls.push(`clear ${k}`);
			}
			view(r: string) {
				calls.push(`view ${r}`);
				return JSON.stringify({ problems: [], events: [], counts: { errors: 1, warnings: 0 }, notes: 0, newest: null });
			}
			counts() {
				return '{"errors":0,"warnings":2}';
			}
			text(r: string) {
				return `text ${r}`;
			}
		},
		log_scopes: (r) => JSON.stringify([{ scope: 'item', words: 'Item', prefix: JSON.parse(r).item }]),
		log_kinds: () => '{"kinds":[],"shows":[]}'
	};
	const log = logCore(mod);
	log.post({ level: 'success', source: 'Chapter 2', text: 'Saved', at: 1, kind: 'save' });
	log.stand('p#', [{ level: 'warning', source: 'Chapter', text: 'short', at: 0, key: 'p#1' }]);
	log.clear('p#1');
	assert.equal(log.view({ show: 'problems', scope: 'toon/a' }, 'x', 5).counts.errors, 1);
	assert.deepEqual(log.counts({}), { errors: 0, warnings: 2 });
	assert.equal(log.text({ show: 'all' }, 'f'), 'text {"show":"all","find":"f"}');
	assert.equal(log.scopes('toon/a', null)[0].prefix, 'toon/a');
	assert.deepEqual(log.legend(), { kinds: [], shows: [] });
	assert.deepEqual(calls, [
		'post {"level":"success","source":"Chapter 2","text":"Saved","at":1,"kind":"save"}',
		'stand p# [{"level":"warning","source":"Chapter","text":"short","at":0,"key":"p#1"}]',
		'clear p#1',
		'view {"show":"problems","scope":"toon/a","find":"x","now":5}'
	]);
});
