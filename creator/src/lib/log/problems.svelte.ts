// A page's problems as the Log's standing lines: posted while the page is mounted, recomputed as it edits, gone when it
// leaves. The page keeps what only it knows – the words of a place, accepting (an edit of its document) and the jump.
import type { LogLine, Problem } from '@scutoons/content';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import { acceptStore, isAccepted, toggleAccepted } from '$lib/editor/accepted.ts';
import { log } from './log.svelte.ts';

export interface ProblemsSource {
	problems: Problem[];
	/** the page's documents: a warning about one of them can be accepted (stored in it) */
	docs?: ContentDoc[];
	/** the page's own file: its problems are scoped by their path inside the item */
	home?: string;
	/** a problem's place in words */
	place: (problem: Problem) => string;
	/** a problem's message in the creator's language */
	message: (problem: Problem) => string;
	ongoto?: (problem: Problem) => void;
}

let mounts = 0;

/** a problem's scope: the item's, and inside its own file the path (`…/chapter/3/groups/1/slices/2`) */
export function problemScope(item: string, problem: Problem, home: string | undefined): string {
	const path = problem.file === home ? problem.path.map(String).join('/') : '';
	return [item, path].filter(Boolean).join('/');
}

/** the problems as standing lines under `prefix`, keyed by their place in the list */
export function problemLines(prefix: string, item: string, s: Pick<ProblemsSource, 'problems' | 'home' | 'place' | 'message'>, accepted: (p: Problem) => boolean): LogLine[] {
	return s.problems.map((p, i) => ({ level: p.level, source: s.place(p), text: s.message(p), at: 0, key: `${prefix}${i}`, scope: problemScope(item, p, s.home), jump: `${prefix}${i}`, accepted: accepted(p) }));
}

/** the page posts its problems to the Log while it is mounted (call during component init) */
export function useProblems(source: () => ProblemsSource | null) {
	const prefix = `problems:${++mounts}#`;
	$effect(() => {
		const s = source();
		const item = log.item ?? '';
		if (!s) return void log.stand(prefix, []);
		const docs = s.docs ?? [];
		const lines = problemLines(prefix, item, s, (p) => isAccepted(docs, p));
		const of = (key: string) => s.problems[Number(key.slice(prefix.length))];
		log.stand(prefix, lines, {
			acceptable: (key) => !!of(key) && !!acceptStore(docs, of(key)),
			accepted: (key) => !!of(key) && isAccepted(docs, of(key)),
			accept: (key) => of(key) && toggleAccepted(docs, of(key)),
			jump: (key) => of(key) && s.ongoto?.(of(key))
		});
	});
	$effect(() => () => log.stand(prefix, []));
}
