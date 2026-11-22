/** the pause after the last change before it saves (as OSCT's and the deck editor's autosave) */
export const AUTOSAVE_MS = 1200;

/**
 * The creator's autosave timing: `schedule()` after every edit saves `delay` ms after the last one, `flush()` saves now.
 * One save at a time – a flush during a save waits for it and saves again right after (two at once would refuse each
 * other); `save` answers whether it went through.
 */
export class Autosaver {
	private timer: ReturnType<typeof setTimeout> | null = null;
	private current: Promise<boolean> | null = null;
	private queued: Promise<boolean> | null = null;

	constructor(
		private readonly save: () => Promise<boolean>,
		private readonly delay = AUTOSAVE_MS
	) {}

	/** a save waits for its pause */
	get pending(): boolean {
		return this.timer !== null;
	}
	/** a save runs */
	get busy(): boolean {
		return this.current !== null;
	}

	schedule(): void {
		this.cancel();
		this.timer = setTimeout(() => {
			this.timer = null;
			void this.flush();
		}, this.delay);
	}

	cancel(): void {
		if (this.timer) clearTimeout(this.timer);
		this.timer = null;
	}

	/** saves now; during a save: once more right after it (one follow-up, however many flushes ask) */
	flush(): Promise<boolean> {
		this.cancel();
		if (this.queued) return this.queued;
		if (this.current) {
			const queued = this.current.then(() => {
				this.queued = null;
				return this.run();
			});
			this.queued = queued;
			return queued;
		}
		return this.run();
	}

	private run(): Promise<boolean> {
		const current = this.save().then(
			(ok) => ok,
			() => false
		);
		this.current = current;
		return current.finally(() => {
			if (this.current === current) this.current = null;
		});
	}
}
