// The console under the editor (RIG-EDITING.md §5): compile output now, a module's `log!()` lines later. A line buffer
// that is capped (the oldest go) and rate-limited (a module that logs every block must not drown the page): at most
// `perSecond` lines per second, the rest are counted and reported as one "… n lines suppressed" line.
export type ConsoleLevel = 'info' | 'ok' | 'warn' | 'error' | 'log';

export interface ConsoleLine {
	id: number;
	level: ConsoleLevel;
	text: string;
	/** ms on the buffer's clock */
	at: number;
}

export interface ConsoleOptions {
	/** lines kept (default 300) */
	cap?: number;
	/** lines let through per second (default 20) */
	perSecond?: number;
	now?: () => number;
}

export class ConsoleBuffer {
	private readonly cap: number;
	private readonly perSecond: number;
	private readonly now: () => number;
	private list: ConsoleLine[] = [];
	private next = 1;
	private windowStart = -Infinity;
	private inWindow = 0;
	private suppressed = 0;

	constructor(o: ConsoleOptions = {}) {
		this.cap = Math.max(1, o.cap ?? 300);
		this.perSecond = Math.max(1, o.perSecond ?? 20);
		this.now = o.now ?? (() => Date.now());
	}

	get lines(): readonly ConsoleLine[] {
		return this.list;
	}

	/** true when the line was kept, false when the rate limit swallowed it */
	push(level: ConsoleLevel, text: string): boolean {
		const at = this.now();
		if (at - this.windowStart >= 1000) {
			this.flushSuppressed(at);
			this.windowStart = at;
			this.inWindow = 0;
		}
		if (this.inWindow >= this.perSecond) {
			this.suppressed++;
			return false;
		}
		this.inWindow++;
		this.add(level, text, at);
		return true;
	}

	/** lines that must never be dropped by the rate limit (a compile's result): kept, and they count as one in the window */
	pushAlways(level: ConsoleLevel, text: string): void {
		const at = this.now();
		this.flushSuppressed(at);
		this.inWindow++;
		this.add(level, text, at);
	}

	clear(): void {
		this.list = [];
		this.suppressed = 0;
		this.inWindow = 0;
		this.windowStart = -Infinity;
	}

	private flushSuppressed(at: number): void {
		if (this.suppressed === 0) return;
		this.add('warn', `… ${this.suppressed} lines suppressed`, at);
		this.suppressed = 0;
	}

	private add(level: ConsoleLevel, text: string, at: number): void {
		this.list = [...this.list.slice(-(this.cap - 1)), { id: this.next++, level, text: text.length > 2000 ? `${text.slice(0, 2000)}…` : text, at }];
	}
}
