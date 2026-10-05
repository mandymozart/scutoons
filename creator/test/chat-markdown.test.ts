// The Ask panel's Markdown renderer (@scutoons/ask, shared/ask/src/chat-markdown.ts): formatting, lists, code – and nothing else
// becomes markup.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chatMarkdown } from '../../shared/ask/src/chat-markdown.ts';

test('bold, italic, code inline', () => {
	assert.equal(chatMarkdown('**Slice 5:** the `bells` *sample*'), '<p><strong>Slice 5:</strong> the <code>bells</code> <em>sample</em></p>');
});

test('lists and paragraphs', () => {
	assert.equal(
		chatMarkdown('I replaced:\n- **Slice 5:** bells\n- Slice 45\n\nDone.'),
		'<p>I replaced:</p><ul><li><strong>Slice 5:</strong> bells</li><li>Slice 45</li></ul><p>Done.</p>'
	);
	assert.equal(chatMarkdown('1. one\n2. two'), '<ol><li>one</li><li>two</li></ol>');
});

test('headings and code blocks', () => {
	assert.equal(chatMarkdown('## Plan'), '<p class="heading">Plan</p>');
	assert.equal(chatMarkdown('```\n<b>x</b> **y**\n```'), '<pre><code>&lt;b&gt;x&lt;/b&gt; **y**</code></pre>');
});

test('HTML in an answer stays text', () => {
	assert.equal(chatMarkdown('<img src=x onerror=alert(1)> **ok**'), '<p>&lt;img src=x onerror=alert(1)&gt; <strong>ok</strong></p>');
	assert.equal(chatMarkdown('`<script>`'), '<p><code>&lt;script&gt;</code></p>');
});

test('snake_case and multiplication stay as they are', () => {
	assert.equal(chatMarkdown('choir_once and 2 * 3 * 4'), '<p>choir_once and 2 * 3 * 4</p>');
});
