// log.ts (#298): the engine's log lines – a bounded buffer a page renders (the host writes every status the engine
// says: the relay, the 3D, the sound, a rig played, an error). Pure.
export type LogLevel = 'info' | 'warn' | 'error';

export interface LogLine {
	/** counts up from 1 – a stable key for a list */
	id: number;
	/** ms since the epoch (`now()`) */
	time: number;
	level: LogLevel;
	/** who said it: `host` · `deck` · `3d` · `sound` · `rig` */
	source: string;
	text: string;
	/** a line repeated straight away is counted, not listed again */
	count: number;
}

export class EngineLog {
	private list: LogLine[] = [];
	private nextId = 1;
	private readonly listeners = new Set<(lines: readonly LogLine[]) => void>();

	constructor(
		private readonly max = 200,
		private readonly now: () => number = () => Date.now()
	) {}

	get lines(): readonly LogLine[] {
		return this.list;
	}

	push(source: string, text: string, level: LogLevel = 'info'): void {
		const t = text.trim();
		if (!t) return;
		const last = this.list[this.list.length - 1];
		if (last && last.source === source && last.text === t && last.level === level) {
			this.list = [...this.list.slice(0, -1), { ...last, count: last.count + 1, time: this.now() }];
		} else {
			this.list = [...this.list.slice(-(this.max - 1)), { id: this.nextId++, time: this.now(), level, source, text: t, count: 1 }];
		}
		for (const f of this.listeners) f(this.list);
	}

	clear(): void {
		this.list = [];
		for (const f of this.listeners) f(this.list);
	}

	/** `f` is called with the whole buffer now and on every change; returns the unsubscribe */
	subscribe(f: (lines: readonly LogLine[]) => void): () => void {
		this.listeners.add(f);
		f(this.list);
		return () => this.listeners.delete(f);
	}
}

/** `12:03:41.250 [3d] 3D webgpu` (×3 when repeated) */
export function formatLine(l: LogLine): string {
	const d = new Date(l.time);
	const p = (n: number, w = 2) => String(n).padStart(w, '0');
	const time = `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}.${p(d.getMilliseconds(), 3)}`;
	return `${time} [${l.source}] ${l.text}${l.count > 1 ? ` ×${l.count}` : ''}`;
}

/** an Error or anything thrown, as a message */
export const messageOf = (e: unknown): string => (e instanceof Error ? e.message : String(e));
