// The creator's one Log (research/redesign/LOG.md): deck-ui's messages through space-web – the open pages' problems as
// standing lines, toasts and technical lines as passing ones. The ring, the filters, the counts and the scope pills are
// the core's; this holds it (core-log-provider.svelte hands it in), queues what is posted before it is there, and
// remembers who answers a standing line's accept and jump.
import { untrack } from 'svelte';
import type { LogCore, LogCounts, LogFilter, LogLegend, LogLine, LogScopePill, LogView } from '@scutoons/content';
import { shared } from '$lib/utils/shared.ts';
import { m } from '$lib/paraglide/messages.js';

const EMPTY: LogView = { problems: [], events: [], counts: { errors: 0, warnings: 0 }, notes: 0, newest: null };

/** who posted standing lines answers for them: whether one can be accepted, accepting it, jumping to it */
export interface StandingSource {
	acceptable(key: string): boolean;
	accepted(key: string): boolean;
	accept(key: string): void;
	jump(key: string): void;
}

/** a line as a part of the creator posts it: the time, the scope and the source default to now and the open item */
export type Posted = Omit<LogLine, 'at' | 'source'> & { at?: number; source?: string };

class CreatorLog {
	status = $state<'idle' | 'loading' | 'ready' | 'error'>('idle');
	error = $state<string | null>(null);
	/** counts every change of the core's lines (what reads the core reads this, to follow) */
	version = $state(0);
	/** the open item's scope and its name (the workspace follows the front tab) */
	item = $state<string | null>(null);
	itemLabel = $state('');
	/** the selection's scope (the page sets it) */
	selection = $state<string | null>(null);
	#core: LogCore | null = null;
	#loader: (() => Promise<LogCore>) | null = null;
	#waiting: ((core: LogCore) => void)[] = [];
	#sources = new Map<string, StandingSource>();

	/** how the core loads (the space-web module: creator/static/live/, `npm run build:wasm -w shared/live-deck`) */
	provide(loader: () => Promise<LogCore>) {
		this.#loader = loader;
		if (this.#waiting.length) void this.load();
	}

	/** the one line the Log can still say when its core did not load */
	get failure(): string | null {
		return this.status === 'error' ? m['shell.log_core_failed']({ reason: this.error ?? '' }) : null;
	}

	/** runs on the core now or once it is there (nothing after the core failed to load); never a dependency of the effect that posts */
	#with(run: (core: LogCore) => void) {
		untrack(() => {
			if (this.status === 'error') return;
			if (this.#core) {
				run(this.#core);
				this.version++;
				return;
			}
			this.#waiting.push(run);
			void this.load();
		});
	}

	/** tried once: a failed load stays failed for the page (asked again on every post, it would fetch without end) */
	async load() {
		if (!this.#loader || this.status !== 'idle') return;
		this.status = 'loading';
		try {
			const core = await this.#loader();
			for (const run of this.#waiting.splice(0)) run(core);
			this.#core = core;
			this.status = 'ready';
			this.version++;
		} catch (e) {
			this.#waiting = [];
			this.error = e instanceof Error ? e.message : String(e);
			this.status = 'error';
			console.error(this.failure);
		}
	}

	/** the page's uncaught errors as technical lines, while the shell is there (its cleanup in return) */
	listen(): () => void {
		const said = (text: string, detail?: string) => this.post({ level: 'error', source: 'App', text, kind: 'technical', scope: '', ...(detail ? { detail } : {}) });
		const onerror = (e: ErrorEvent) => said(e.message || String(e.error), e.filename ? `${e.filename}:${e.lineno}` : undefined);
		const onrejection = (e: PromiseRejectionEvent) => said(e.reason instanceof Error ? e.reason.message : String(e.reason));
		addEventListener('error', onerror);
		addEventListener('unhandledrejection', onrejection);
		return () => {
			removeEventListener('error', onerror);
			removeEventListener('unhandledrejection', onrejection);
		};
	}

	follow(item: string | null, label: string) {
		this.item = item;
		this.itemLabel = label;
	}

	/** a passing line (or a standing one with a key) */
	post(line: Posted) {
		const full: LogLine = untrack(() => ({ ...line, at: line.at ?? Date.now() / 1000, source: line.source ?? this.itemLabel, scope: line.scope ?? this.item ?? '' }));
		this.#with((core) => core.post(full));
	}

	/** a poster's whole standing set under `prefix` (none: its lines go) and who answers for them */
	stand(prefix: string, lines: LogLine[], source?: StandingSource) {
		if (source && lines.length) this.#sources.set(prefix, source);
		else this.#sources.delete(prefix);
		this.#with((core) => core.stand(prefix, lines));
	}

	sourceOf(key: string): StandingSource | undefined {
		for (const [prefix, source] of this.#sources) if (key.startsWith(prefix)) return source;
		return undefined;
	}

	/** the lines through `filter` (`now` in ms; `max` caps the events) */
	view(filter: LogFilter, find: string, now: number, max?: number): LogView {
		void this.version;
		return this.#core?.view(filter, find, now / 1000, max) ?? EMPTY;
	}

	counts(filter: LogFilter): LogCounts {
		void this.version;
		return this.#core?.counts(filter) ?? EMPTY.counts;
	}

	text(filter: LogFilter, find: string): string {
		return this.#core?.text(filter, find) ?? '';
	}

	scopes(): LogScopePill[] {
		void this.version;
		return this.#core?.scopes(this.item, this.selection) ?? [];
	}

	legend(): LogLegend | null {
		void this.version;
		return this.#core?.legend() ?? null;
	}
}

export const log = shared('creator-log', () => new CreatorLog());
