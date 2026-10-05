// The Ask panel's answers are Markdown (owner 2026-10-04: "the md is not transformed, ** ugly output"). A small renderer
// for what the agent writes – paragraphs, headings, - / 1. lists, **bold**, *italic*, `code`, ``` blocks – instead of a
// library: everything is HTML-escaped FIRST, so nothing in an answer (or in content the agent quotes) becomes markup;
// only the tags made here exist. Links stay text.

const escape = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

/** inline formatting of one escaped line: `code` first (its content stays literal), then **bold**, *italic* / _italic_ */
function inline(text: string): string {
	const codes: string[] = [];
	let s = escape(text).replace(/`([^`\n]+)`/g, (_, c: string) => `\u0000${codes.push(c) - 1}\u0000`);
	s = s
		.replace(/\*\*([^*\n]+?)\*\*/g, '<strong>$1</strong>')
		.replace(/(^|[^*\w])\*([^*\s][^*\n]*?)\*(?![*\w])/g, '$1<em>$2</em>')
		.replace(/(^|[^\w])_([^_\s][^_\n]*?)_(?!\w)/g, '$1<em>$2</em>');
	return s.replace(/\u0000(\d+)\u0000/g, (_, i: string) => `<code>${codes[Number(i)]}</code>`);
}

/** Markdown → safe HTML for the chat */
export function chatMarkdown(markdown: string): string {
	const out: string[] = [];
	const lines = markdown.replace(/\r\n?/g, '\n').split('\n');
	let paragraph: string[] = [];
	let list: { tag: 'ul' | 'ol'; items: string[] } | null = null;
	const flush = () => {
		if (paragraph.length) out.push(`<p>${paragraph.map(inline).join('<br>')}</p>`);
		paragraph = [];
		if (list) out.push(`<${list.tag}>${list.items.map((i) => `<li>${inline(i)}</li>`).join('')}</${list.tag}>`);
		list = null;
	};
	for (let i = 0; i < lines.length; i++) {
		const line = lines[i];
		if (/^\s*```/.test(line)) {
			flush();
			const code: string[] = [];
			while (++i < lines.length && !/^\s*```/.test(lines[i])) code.push(lines[i]);
			out.push(`<pre><code>${escape(code.join('\n'))}</code></pre>`);
			continue;
		}
		const heading = /^(#{1,4})\s+(.*)$/.exec(line);
		const bullet = /^\s*[-*•]\s+(.*)$/.exec(line);
		const numbered = /^\s*\d+[.)]\s+(.*)$/.exec(line);
		if (!line.trim()) flush();
		else if (heading) {
			flush();
			out.push(`<p class="heading">${inline(heading[2])}</p>`);
		} else if (bullet || numbered) {
			const tag = bullet ? 'ul' : 'ol';
			if (paragraph.length || (list && list.tag !== tag)) flush();
			list ??= { tag, items: [] };
			list.items.push((bullet ?? numbered)![1]);
		} else if (list && /^\s{2,}\S/.test(line)) {
			// a list item's continuation line
			list.items[list.items.length - 1] += ` ${line.trim()}`;
		} else {
			if (list) flush();
			paragraph.push(line);
		}
	}
	flush();
	return out.join('');
}
