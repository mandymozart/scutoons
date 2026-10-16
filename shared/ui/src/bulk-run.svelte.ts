// Bulk actions over long lists (research/ASSET-LISTS.md): thousands of rows go to the server in batches, one request
// after the other, with live progress, a cancel between batches and every batch's result kept for the report.
// `BulkProgress` shows a run; the page owns what a batch does and what the report says.

/** Splits items into batches of about `size` units; an item is never split (e.g. an asset's files stay together) */
export function chunk<T>(items: T[], size: number, weight: (item: T) => number = () => 1): T[][] {
	const out: T[][] = [];
	let batch: T[] = [];
	let units = 0;
	for (const item of items) {
		const w = Math.max(1, weight(item));
		if (batch.length && units + w > size) {
			out.push(batch);
			batch = [];
			units = 0;
		}
		batch.push(item);
		units += w;
	}
	if (batch.length) out.push(batch);
	return out;
}

export interface BulkReport<R> {
	/** one per finished batch */
	results: R[];
	/** units done / all (files, rows …) */
	done: number;
	total: number;
	cancelled: boolean;
	/** the error that stopped the run (a failed request); per-row failures belong in the results */
	error: string | null;
}

/** what a progress display reads of a run */
export interface BulkState {
	readonly running: boolean;
	readonly done: number;
	readonly total: number;
	readonly percent: number;
	cancel(): void;
}

/**
 * One bulk run: `start(batches, step)` runs the batches in order; `done` / `total` / `running` are reactive. `cancel()`
 * stops before the next batch (the running request finishes – nothing is cut in half). A thrown step stops the run.
 */
export class BulkRun<B, R> implements BulkState {
	running = $state(false);
	done = $state(0);
	total = $state(0);
	cancelled = $state(false);
	error = $state<string | null>(null);
	#stop = false;
	readonly #weight: (batch: B) => number;

	/** `weight` = the units a batch counts for in the progress (default 1 per batch) */
	constructor(weight: (batch: B) => number = () => 1) {
		this.#weight = weight;
	}

	get percent(): number {
		return this.total ? Math.min(100, Math.round((this.done / this.total) * 100)) : 0;
	}

	cancel() {
		if (this.running) this.#stop = true;
	}

	async start(batches: B[], step: (batch: B, index: number) => Promise<R>, describe: (error: unknown) => string = String): Promise<BulkReport<R>> {
		this.#stop = false;
		this.running = true;
		this.cancelled = false;
		this.error = null;
		this.done = 0;
		this.total = batches.reduce((n, b) => n + this.#weight(b), 0);
		const results: R[] = [];
		try {
			for (let i = 0; i < batches.length; i++) {
				if (this.#stop) {
					this.cancelled = true;
					break;
				}
				results.push(await step(batches[i], i));
				this.done += this.#weight(batches[i]);
			}
		} catch (e) {
			this.error = describe(e);
		} finally {
			this.running = false;
		}
		return { results, done: this.done, total: this.total, cancelled: this.cancelled, error: this.error };
	}
}
