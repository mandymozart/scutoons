// log.ts (#767): the Log (deck-ui `messages`) over space-web's exports – standing problems and passing events, the
// filters, the counts, the scope pills, the event types and their legend. Typed calls, JSON in and out, no rule here;
// the module itself comes from @scutoons/live-deck/log (the wasm the page loads).

export type LogLevel = 'info' | 'ok' | 'warn' | 'error';
export type LogKind = 'logic' | 'save' | 'publish' | 'history' | 'generation' | 'library' | 'show' | 'sync' | 'technical';
export type LogShow = 'all' | 'problems' | 'events';
export type LogScope = 'selection' | 'item' | 'chapter' | 'season' | 'toon' | 'show' | 'all';

/** a line as posted: standing with a `key`, passing without */
export interface LogLine {
	/** 'warning' and 'success' are read as 'warn' and 'ok' */
	level: LogLevel | 'warning' | 'success';
	/** where it comes from, in words (a problem's place, the item's name) */
	source: string;
	text: string;
	/** seconds since the Unix epoch */
	at: number;
	/** the address path (`toon/catpope/season/1/chapter/3/…`); empty: the app */
	scope?: string;
	key?: string;
	kind?: LogKind;
	/** where a press goes (an address or a problem's key) */
	jump?: string;
	/** the address-book entry it is about (`event:bell`) */
	ref?: string;
	detail?: string;
	accepted?: boolean;
}

/** a line as the Log shows it */
export interface LogShown extends Omit<LogLine, 'level'> {
	level: LogLevel;
	times: number;
	/** how long ago ('' for a standing line) */
	age: string;
	/** HH:MM:SS (UTC) */
	clock: string;
}

export interface LogFilter {
	level?: 'all' | 'errors' | 'warnings' | 'info' | 'technical';
	show?: LogShow;
	/** none: every type */
	kinds?: LogKind[];
	/** the scope prefix; '': everything */
	scope?: string;
	/** standing info lines (notes) */
	notes?: boolean;
}

export interface LogCounts {
	errors: number;
	warnings: number;
}

export interface LogView {
	problems: LogShown[];
	events: LogShown[];
	counts: LogCounts;
	/** the standing info lines the filter hides or shows */
	notes: number;
	newest: LogShown | null;
}

export interface LogScopePill {
	scope: LogScope;
	words: string;
	/** null: the open item has no such place */
	prefix: string | null;
}

export interface LogLegend {
	kinds: { kind: LogKind; label: string; legend: string }[];
	shows: { show: LogShow; words: string }[];
}

/** space-web's `Log` instance */
interface LogHandle {
	post(message: string): void;
	stand(prefix: string, lines: string): void;
	clear(key: string): void;
	view(request: string): string;
	counts(filter: string): string;
	text(request: string): string;
	free?(): void;
}

/** the space-web exports the Log uses */
export interface LogModule {
	Log: new () => LogHandle;
	log_scopes(request: string): string;
	log_kinds(): string;
}

export interface LogCore {
	post(line: LogLine): void;
	/** a poster's whole standing set: every standing line whose key starts with `prefix` is replaced */
	stand(prefix: string, lines: LogLine[]): void;
	clear(key: string): void;
	view(filter: LogFilter, find: string, now: number, max?: number): LogView;
	counts(filter: LogFilter): LogCounts;
	text(filter: LogFilter, find: string): string;
	scopes(item: string | null, selection: string | null): LogScopePill[];
	legend(): LogLegend;
	free(): void;
}

export function logCore(mod: LogModule): LogCore {
	const log = new mod.Log();
	return {
		post: (line) => log.post(JSON.stringify(line)),
		stand: (prefix, lines) => log.stand(prefix, JSON.stringify(lines)),
		clear: (key) => log.clear(key),
		view: (filter, find, now, max) => JSON.parse(log.view(JSON.stringify({ ...filter, find, now, max }))),
		counts: (filter) => JSON.parse(log.counts(JSON.stringify(filter))),
		text: (filter, find) => log.text(JSON.stringify({ ...filter, find })),
		scopes: (item, selection) => JSON.parse(mod.log_scopes(JSON.stringify({ item, selection }))),
		legend: () => JSON.parse(mod.log_kinds()),
		free: () => log.free?.()
	};
}
