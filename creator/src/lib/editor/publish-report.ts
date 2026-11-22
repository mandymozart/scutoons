/** What a bulk publish says: what went live by name, and every refusal with the server's reason, as Log lines */

export interface PublishRefusal {
	key: string;
	reason: string;
	/** the server's code (`invalid_scene`, `dependency`, `not_publishable` …) */
	code?: string;
}

export interface PublishNotice {
	type: 'success' | 'info' | 'error';
	text: string;
	detail?: string;
}

export interface PublishWords {
	/** "{count} published: {names}." */
	done: (count: number, names: string) => string;
	/** "Not published: {name} – {reason}" */
	refused: (name: string, reason: string) => string;
	nothing: () => string;
}

/** the first names of a list; the whole list goes into the line's detail */
export function nameList(names: string[], max = 6): string {
	return names.length <= max ? names.join(', ') : `${names.slice(0, max).join(', ')} …`;
}

/** the Log lines of a publish: one naming what went live (all of them in its detail), one per refusal */
export function publishNotices(published: string[], refused: PublishRefusal[], label: (key: string) => string, words: PublishWords, extra = ''): PublishNotice[] {
	const names = published.map(label);
	const head = [names.length ? words.done(names.length, nameList(names)) : '', extra].filter(Boolean).join(' ');
	const lines: PublishNotice[] = [];
	if (head || !refused.length) {
		lines.push({ type: names.length ? 'success' : 'info', text: head || words.nothing(), ...(names.length > 6 ? { detail: names.join('\n') } : {}) });
	}
	for (const r of refused) lines.push({ type: 'error', text: words.refused(label(r.key), r.reason) });
	return lines;
}
