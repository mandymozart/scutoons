// Expressions in the reader (agents/EXPRESSIONS.md §7 step 2): the viewer's resolver for texts – text boxes (through
// @scutoons/reader's `expressions` prop), VN messages and choice labels (runner.ts). The names come from reader.ts over
// the contexts; the visits of sequences / cycles / once-only lists are the reader's history in the one save game
// (`scutoons_game_scutoons.history`, @scutoons/content history.ts – synced for members like the variables).
//
// Visits: a text box counts once per PAGE VIEW (the URL's path + its `?label=`; a reload is a new one) however often it
// re-renders – it shows the count from before this page view all the while. A VN message / choice label counts each time
// the script shows it (a script that comes back to a message within one page goes on in the list, as in ink).
// Seed (shuffles, random()): the reader's start (the save game's created_at) + the text's identity.
import { getContext, setContext, untrack } from 'svelte';
import { page } from '$app/state';
import { evaluateText, textHistoryKey, type ExprResolver, type ExprValue } from '@scutoons/content';
import type { TextExpressions } from '@scutoons/reader';
import type { GameContext } from '$lib/contexts/game.svelte';
import { fetchMap } from '$lib/api.ts';
import { readerCall, readerName, type ReaderSources } from './reader.ts';

const KEY = Symbol('reader-expressions');

export type ReaderExpressions = ReturnType<typeof createReaderExpressions>;

let _current: ReaderExpressions | undefined;

export function createReaderExpressions(sources: ReaderSources, game: GameContext | undefined, language: () => string) {
	const name = (path: readonly string[]): ExprValue => readerName(sources, path);
	const call = (fn: string, args: readonly ExprValue[]): ExprValue => readerCall(sources, fn, args);
	const seed = (text: string) => `${game?.state.created_at ?? ''}|${text}`;
	/** the document a text belongs to: the toon + the chapter of the page */
	const doc = () => `${sources.toon}/${page.params?.chapterNumber ?? ''}`;

	// ── the page view: a text box's visits are counted once per view ──
	let view = '';
	/** history count at the first showing in this page view, by history key */
	const before = new Map<string, number>();
	const counted = new Set<string>();
	function inView() {
		const url = page.url;
		const key = `${url.pathname}?${url.searchParams.get('label') ?? ''}`;
		if (key !== view) {
			view = key;
			before.clear();
			counted.clear();
		}
	}

	/** a text box's resolver (`text` = its identity in the slice: the box id + line) */
	function boxResolver(text: string, lang?: string): ExprResolver {
		const id = `box/${text}`;
		return {
			name,
			call,
			visit(index) {
				inView();
				const key = textHistoryKey(doc(), id, index);
				if (!before.has(key)) before.set(key, untrack(() => game?.visits(key) ?? 0));
				return before.get(key)!;
			},
			seed: seed(id),
			language: lang || language()
		};
	}

	/** a text box showed these lists: one visit each, once per page view */
	function boxShown(text: string, visited: number[]) {
		inView();
		const id = `box/${text}`;
		for (const index of visited) {
			const key = textHistoryKey(doc(), id, index);
			if (counted.has(key)) continue;
			counted.add(key);
			game?.recordVisit(key);
		}
	}

	/** the text-box answers for @scutoons/reader's `Slice` / `Dialogs` */
	const textBoxes: TextExpressions = { resolver: boxResolver, shown: boxShown };

	/**
	 * A VN message / choice label as the reader sees it now (`at` = its identity: group + script + event index); each call
	 * is a showing – its lists count one visit.
	 */
	function evaluate(text: string, at: string): string {
		const id = `vn/${at}`;
		const result = evaluateText(text, {
			name,
			call,
			visit: (index) => game?.visits(textHistoryKey(doc(), id, index)) ?? 0,
			seed: seed(id),
			language: language()
		});
		for (const index of result.visited) game?.recordVisit(textHistoryKey(doc(), id, index));
		return result.text;
	}

	return { name, call, textBoxes, evaluate };
}

let worldPlaces = $state<readonly { id: string; name?: string }[] | null>(null);
let worldLoading = '';

/**
 * The world map's places for `map.total` / `map.here` when the toon's map (toon/map/map.svelte → the world context) has
 * none – a toon not on the map, or before the map's idle load: the whole world (`/library/map`), loaded once per language
 * when a text first asks (null meanwhile; texts re-render when it arrives).
 */
export function lazyWorldPlaces(locale: string): readonly { id: string; name?: string }[] | null {
	if (worldLoading !== locale) {
		worldLoading = locale;
		fetchMap(locale)
			.then((map) => {
				if (worldLoading === locale) worldPlaces = (map?.locations ?? []).map((p) => ({ id: p.id, name: p.name }));
			})
			.catch(() => {});
	}
	return worldPlaces;
}

/** a short stable identity for a longer one (a slice trigger's JSON): FNV-1a, base 36 – history keys stay small */
export function shortHash(text: string): string {
	let h = 0x811c9dc5;
	for (let i = 0; i < text.length; i++) h = Math.imul(h ^ text.charCodeAt(i), 0x01000193);
	return (h >>> 0).toString(36);
}

/** the toon's reader expressions (the toon layout), for the slices and the VN runner */
export function setReaderExpressions(expressions: ReaderExpressions): ReaderExpressions {
	setContext(KEY, expressions);
	_current = expressions;
	return expressions;
}

export function getReaderExpressions(): ReaderExpressions | undefined {
	return getContext<ReaderExpressions | undefined>(KEY);
}

/** for code outside the component tree (the VN runner) */
export function getCurrentReaderExpressions(): ReaderExpressions | undefined {
	return _current;
}
