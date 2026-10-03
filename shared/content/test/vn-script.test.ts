// The VN script logic (CUTOVER 10.27, owner 2026-10-03).   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { actionsOf, formatClock, interpolateText, labelIndex, pickJumpLabel, scriptLabels, testCondition, validateFile } from '../src/index.ts';

test('conditions: numbers as numbers, text as text, set / unset', () => {
	const vars: Record<string, string | number> = { time: 420, name: 'Pope', zero: 0 };
	const get = (v: string) => vars[v];
	assert.equal(testCondition({ variable: 'time', op: '>=', value: 420 }, get), true);
	assert.equal(testCondition({ variable: 'time', op: '>=', value: '421' }, get), false);
	assert.equal(testCondition({ variable: 'time', op: '<', value: 420 }, get), false);
	assert.equal(testCondition({ variable: 'name', op: '==', value: 'Pope' }, get), true);
	assert.equal(testCondition({ variable: 'name', op: '!=', value: 'Pope' }, get), false);
	assert.equal(testCondition({ variable: 'missing', op: 'unset' }, get), true);
	assert.equal(testCondition({ variable: 'zero', op: 'set' }, get), true, '0 is a value');
	assert.equal(testCondition({ variable: 'missing', op: '<', value: 5 }, get), false, 'no value never compares');
	assert.equal(testCondition(undefined, get), true);
});

test('actions, points, random jumps, clock, text', () => {
	assert.deepEqual(actionsOf(undefined), []);
	assert.deepEqual(actionsOf({ divert: { to: 'a' } }), [{ divert: { to: 'a' } }]);
	assert.equal(actionsOf([{ add: { variable: 'time', value: 15 } }, { divert: { to: 'a' } }]).length, 2);
	const script = [{ type: 'showChoices' }, { type: 'label', params: { name: 'join' } }, { type: 'label', params: { name: 'go-on' } }] as never;
	assert.equal(labelIndex(script, 'go-on'), 2);
	assert.equal(labelIndex(script, 'nope'), -1);
	assert.deepEqual(scriptLabels(script), ['join', 'go-on']);
	const strolls = ['scene-14', 'scene-15', 'scene-16'];
	for (let i = 0; i < 20; i++) assert.notEqual(pickJumpLabel(strolls, 'scene-15'), 'scene-15');
	assert.equal(pickJumpLabel(strolls, 'scene-15', () => 0), 'scene-14');
	assert.equal(pickJumpLabel(strolls, 'scene-15', () => 0.99), 'scene-16');
	assert.equal(pickJumpLabel(['scene-14'], 'scene-14'), 'scene-14', 'the only one is taken even when current');
	assert.equal(pickJumpLabel([], 'x'), null);
	assert.equal(formatClock(330), '5:30');
	assert.equal(formatClock(420), '7:00');
	assert.equal(formatClock(1445), '0:05');
	const get = (v: string) => ({ time: 345, who: 'Pope' })[v as 'time'];
	assert.equal(interpolateText('Es ist {time:clock} Uhr, {who}. {nope}', get), 'Es ist 5:45 Uhr, Pope. {nope}');
});

test('validation: points, diverts, conditions, action lists, random targets', () => {
	const group = (events: object[], slices: object[] = []) => ({ label: 'g', renderer: { type: 'vertical' }, slices, events });
	const errs = (g: object) => validateFile('group', g, 'de', { file: 'x' } as never).filter((p) => p.level === 'error').map((p) => p.path.join('.'));
	assert.deepEqual(
		errs(group([
			{ type: 'label', params: { name: 'join' } },
			{ type: 'divert', params: { to: 'join' }, if: { variable: 'time', op: '>=', value: 420 } },
			{ type: 'setVariable', params: { variable: 'time', value: 15, op: 'add' } },
			{ type: 'addChoice', params: { label: 'Weiter', onSelect: [{ add: { variable: 'time', value: 15 } }, { jumpTo: { target: { type: 'chapter', value: '2', anyOf: ['scene-14', 'scene-15'] } } }] } },
			{ type: 'addChoice', params: { label: 'Ja', onSelect: { divert: { to: 'join' } } } },
			{ type: 'showChoices' }
		])),
		[]
	);
	assert.deepEqual(errs(group([{ type: 'divert', params: { to: 'nowhere' } }])), ['events.0.params.to']);
	assert.deepEqual(errs(group([{ type: 'label', params: { name: 'a' } }, { type: 'label', params: { name: 'a' } }])), ['events.1.params.name']);
	assert.deepEqual(errs(group([{ type: 'hideMessage', if: { variable: 'x', op: '~' } }])), ['events.0.if.op']);
	assert.deepEqual(errs(group([{ type: 'hideMessage', if: { variable: 'x', op: '<' } }])), ['events.0.if.value']);
	assert.deepEqual(errs(group([{ type: 'addChoice', params: { label: 'x', onSelect: [{ divert: { to: 'none' } }] } }])), ['events.0.params.onSelect.0.divert.to']);
	assert.deepEqual(errs(group([{ type: 'jumpTo', params: { target: { type: 'chapter', value: '2', anyOf: [] } } }])), ['events.0.params.target.anyOf']);
	// a slice event diverts into its group's script
	const slice = { image: 'a.webp', width: 390, height: 625, events: [{ type: 'divert', params: { to: 'plattenspieler' }, trigger: { type: 'intersect' } }] };
	assert.deepEqual(errs(group([{ type: 'label', params: { name: 'plattenspieler' } }], [slice])), []);
	assert.deepEqual(errs(group([], [slice])), ['slices.0.events.0.params.to']);
});
