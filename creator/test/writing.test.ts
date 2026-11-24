// `npm test -w scutoons-creator`: the writing page's reading of a text (lib/editor/writing.ts).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	inlineMarks,
	joinParagraphs,
	lineInfo,
	paragraphs,
	pieces,
	readingMinutes,
	selectionState,
	toggleDirection,
	toggleLinePrefix,
	toggleWrap,
	toLink,
	toSpeaker,
	wordCount,
	type WritingEdit
} from '../src/lib/editor/writing.ts';

/** an edit applied (changes from the end, so the offsets hold) */
function apply(text: string, edit: WritingEdit): string {
	let out = text;
	for (const c of [...edit.changes].sort((a, b) => b.from - a.from)) out = out.slice(0, c.from) + c.insert + out.slice(c.to);
	return out;
}

test('a line is a heading, a speaker, a direction, a quote, an item or text', () => {
	assert.deepEqual(lineInfo('## Wer dort wohnt'), { kind: 'heading', mark: 3 });
	assert.deepEqual(lineInfo('#Kein Titel'), { kind: 'text', mark: 0 });
	assert.deepEqual(lineInfo('KATZENPAPST'), { kind: 'speaker', mark: 0 });
	assert.deepEqual(lineInfo('KRÄHE (V.O.)'), { kind: 'speaker', mark: 0 });
	assert.equal(lineInfo('I').kind, 'text', 'one capital is no name');
	assert.equal(lineInfo('Setz dich.').kind, 'text');
	assert.deepEqual(lineInfo('(er rückt die Mitra zurecht)'), { kind: 'direction', mark: 0 });
	assert.deepEqual(lineInfo('> Ein Zitat'), { kind: 'quote', mark: 2 });
	assert.deepEqual(lineInfo('- eins'), { kind: 'item', mark: 2 });
	assert.deepEqual(lineInfo('   '), { kind: 'blank', mark: 0 });
});

test('the inline marks: strong, em, a link', () => {
	const line = 'ein **lauter** und *leiser* Satz, [hier](https://x.y)';
	const marks = inlineMarks(line);
	assert.deepEqual(marks.map((m) => m.kind), ['strong', 'em', 'link']);
	assert.equal(line.slice(...marks[0].inner), 'lauter');
	assert.equal(line.slice(...marks[1].inner), 'leiser');
	assert.equal(line.slice(...marks[2].inner), 'hier');
	assert.deepEqual(pieces('## Ein *kleiner* Titel'), [{ text: 'Ein ' }, { text: 'kleiner', em: true }, { text: ' Titel' }]);
	assert.deepEqual(inlineMarks('2 * 3 * 4'), [], 'a lone star between spaces is no mark');
});

test('paragraphs split on blank lines and join again; empty ones at the end leave nothing', () => {
	const text = 'Eins.\nnoch eins\n\n## Zwei\n\n\nDrei';
	assert.deepEqual(paragraphs(text), ['Eins.\nnoch eins', '## Zwei', 'Drei']);
	assert.equal(joinParagraphs(paragraphs(text)), 'Eins.\nnoch eins\n\n## Zwei\n\nDrei');
	assert.equal(joinParagraphs(['a', 'b', '', '']), 'a\n\nb');
	assert.deepEqual(paragraphs(''), []);
});

test('words and the reading time', () => {
	assert.equal(wordCount('Die Kathedrale liegt unter dem Meer – wer’s weiß.'), 8);
	assert.equal(wordCount(''), 0);
	assert.equal(readingMinutes(0), 0);
	assert.equal(readingMinutes(12), 1);
	assert.equal(readingMinutes(412), 2);
});

test('the pill wraps the selection and takes the marks off again', () => {
	const text = 'so leise, dass';
	const bold = toggleWrap(text, 3, 8, '**');
	assert.equal(apply(text, bold), 'so **leise**, dass');
	const back = toggleWrap(apply(text, bold), bold.selection.anchor, bold.selection.head, '**');
	assert.equal(apply(apply(text, bold), back), text);
	assert.equal(apply('**fett**', toggleWrap('**fett**', 0, 8, '**')), 'fett');
	assert.equal(apply('a *b* c', toggleWrap('a *b* c', 3, 4, '*')), 'a b c');
	assert.equal(apply('a **b** c', toggleWrap('a **b** c', 4, 5, '*')), 'a ***b*** c', 'em inside strong');
});

test('headings, quotes, speakers and directions work on whole lines', () => {
	const text = 'Wer dort wohnt\nDer Papst.';
	assert.equal(apply(text, toggleLinePrefix(text, 2, 4, '## ')), '## Wer dort wohnt\nDer Papst.');
	assert.equal(apply('## A', toggleLinePrefix('## A', 0, 4, '## ')), 'A');
	assert.equal(apply('## A', toggleLinePrefix('## A', 0, 4, '> ')), '> A', 'a quote replaces a heading');
	assert.equal(apply('Katzenpapst\nSetz dich.', toSpeaker('Katzenpapst\nSetz dich.', 0, 3)), 'KATZENPAPST\nSetz dich.');
	assert.equal(apply('er lacht', toggleDirection('er lacht', 0, 2)), '(er lacht)');
	assert.equal(apply('(er lacht)', toggleDirection('(er lacht)', 1, 2)), 'er lacht');
	const link = toLink('hier lang', 0, 4);
	assert.equal(apply('hier lang', link), '[hier]() lang');
	assert.equal(link.selection.anchor, 7);
});

test('the pill knows what the selection already is', () => {
	assert.deepEqual(selectionState('a **b** c', 4, 5), { strong: true, em: false, heading: false, quote: false, speaker: false, direction: false });
	assert.equal(selectionState('## Titel', 3, 5).heading, true);
	assert.equal(selectionState('KRÄHE\nWir', 0, 2).speaker, true);
	assert.equal(selectionState('a *b* c', 3, 4).em, true);
});
