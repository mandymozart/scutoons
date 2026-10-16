// "Save a whole season" (PWA phase 3, agents/research/PWA.md §2 "Phase 3", FEATURES 10.74, owner 2026-10-16 "alle Pakete
// angehen"): one action on the toon page's season heading (components/toon/save-season.svelte) and on the account page
// ("Save the rest", components/account/saved-seasons.svelte) that saves every chapter of the season the reader may read
// and hasn't saved yet. Like one chapter: the size first (the server's list of each chapter, one request each), the
// "Everything / Without videos" choice judged on the whole season, mobile data / data saver said – nothing starts
// without the tap. Then the chapters one after another (savedChapters.save – each row's ring shows its own progress);
// stop = the running chapter is stopped, the rest is not started, the chapters already saved stay.
// Rules: lib/sw/saved-update-rules.ts `seasonTargets` / `planSeason` (tested).
import { choiceDialog, confirmDialog } from '@scutoons/ui';
import { effectiveQuality } from '@scutoons/content';
import { m } from '$lib/paraglide/messages.js';
import { mediaQuality } from '$lib/media-quality.svelte.ts';
import { dataWarnings, roundMb, type ChapterFiles, type ConnectionHint } from '$lib/sw/saved-rules.ts';
import { planSeason, seasonTargets, type SeasonChapter } from '$lib/sw/saved-update-rules.ts';
import { savedChapters } from './saved-chapters.svelte.ts';

export interface SeasonInput {
	toon: string;
	toonTitle: string;
	season: number;
	/** the season's chapters in order; `label` = how the toon page names it ("Kapitel 1", "Prolog") */
	chapters: (SeasonChapter & { title: string; label: string })[];
	locale: string;
	/** the access end a gated chapter is saved with (lib/offline/auto-save.svelte.ts `accessEnd`) */
	accessUntil: string | null;
}

export interface SeasonJob {
	toon: string;
	season: number;
	label: string;
	/** the chapters this run saves, with their planned bytes */
	chapters: { number: number; label: string; bytes: number }[];
	done: number[];
	failed: number[];
	/** the chapter saving now */
	current: number | null;
	controller: AbortController;
}

const key = (toon: string, season: number) => `${toon}/${season}`;
const connection = () => (navigator as Navigator & { connection?: ConnectionHint }).connection;

/** `fn` over `items`, at most `limit` at a time, results in order */
async function mapLimit<T, R>(items: T[], limit: number, fn: (item: T) => Promise<R>): Promise<R[]> {
	const out: R[] = new Array(items.length);
	let next = 0;
	const worker = async () => {
		while (next < items.length) {
			const i = next++;
			out[i] = await fn(items[i]);
		}
	};
	await Promise.all(Array.from({ length: Math.min(limit, items.length) }, worker));
	return out;
}

class SeasonSaves {
	/** running season saves by `<toon>/<season>` */
	jobs = $state<Record<string, SeasonJob>>({});
	/** the sizes are being asked (the button waits) */
	sizing = $state<Record<string, boolean>>({});

	job(toon: string, season: number): SeasonJob | undefined {
		return this.jobs[key(toon, season)];
	}

	/** the season's share saved so far (0..1): finished chapters + the running one's bytes */
	share(toon: string, season: number): number {
		const job = this.job(toon, season);
		if (!job) return 0;
		const total = job.chapters.reduce((sum, c) => sum + c.bytes, 0) || 1;
		const done = job.chapters.filter((c) => job.done.includes(c.number)).reduce((sum, c) => sum + c.bytes, 0);
		const running = job.current === null ? undefined : savedChapters.job(toon, job.current);
		const current = running ? (running.done / Math.max(1, running.total)) * (job.chapters.find((c) => c.number === job.current)?.bytes ?? 0) : 0;
		return Math.min(1, (done + current) / total);
	}

	/** the size first, the reader's choice, then the chapters one after another */
	async start(input: SeasonInput): Promise<'saved' | 'cancelled' | 'failed' | 'none'> {
		const id = key(input.toon, input.season);
		if (this.jobs[id] || this.sizing[id]) return 'cancelled';
		const targets = seasonTargets(input.chapters, (n) => !!savedChapters.get(input.toon, n));
		if (!targets.length) return 'none';
		const name = m['saved.season_name']({ number: input.season });
		const level = effectiveQuality(mediaQuality.setting ?? 'auto', mediaQuality.auto);
		let lists: ChapterFiles[];
		this.sizing = { ...this.sizing, [id]: true };
		try {
			lists = await mapLimit(targets, 4, (n) => savedChapters.files(input.toon, n, input.locale));
		} catch {
			return 'failed';
		} finally {
			const { [id]: _was, ...rest } = this.sizing;
			this.sizing = rest;
		}
		const plan = planSeason(lists, level);
		const warn = dataWarnings(plan.bytes, connection());
		const message = [
			targets.length === 1 ? m['saved.season_message_one']({ size: roundMb(plan.bytes) }) : m['saved.season_message']({ count: targets.length, size: roundMb(plan.bytes) }),
			warn.cellular ? m['saved.cellular']() : '',
			warn.saveData ? m['saved.data_saver']() : ''
		]
			.filter(Boolean)
			.join(' ');
		const title = m['saved.season_title']({ season: name });
		let videos = true;
		if (plan.offerLean) {
			const choice = await choiceDialog({
				title,
				message,
				cancelLabel: m['common.cancel'](),
				choices: [
					{ id: 'all', label: m['saved.with_videos']({ size: roundMb(plan.bytes) }) },
					{ id: 'lean', label: m['saved.without_videos']({ size: roundMb(plan.leanBytes) }) }
				]
			});
			if (!choice) return 'cancelled';
			videos = choice === 'all';
		} else if (!(await confirmDialog({ title, message, confirmLabel: m['saved.save_confirm'](), cancelLabel: m['common.cancel']() }))) return 'cancelled';

		const byNumber = new Map(input.chapters.map((c) => [c.number, c]));
		const controller = new AbortController();
		this.jobs = {
			...this.jobs,
			[id]: {
				toon: input.toon,
				season: input.season,
				label: `${input.toonTitle} · ${name}`,
				chapters: plan.chapters.map((c) => ({ number: c.chapter, label: byNumber.get(c.chapter)?.label ?? String(c.chapter), bytes: (videos ? c.all : c.lean).bytes })),
				done: [],
				failed: [],
				current: null,
				controller
			}
		};
		const set = (patch: Partial<SeasonJob>) => {
			const job = this.jobs[id];
			if (job) this.jobs = { ...this.jobs, [id]: { ...job, ...patch } };
		};
		try {
			for (const list of lists) {
				if (controller.signal.aborted) break;
				const chapter = byNumber.get(list.chapter);
				// saved by hand meanwhile: done
				if (savedChapters.get(input.toon, list.chapter)) {
					set({ done: [...(this.jobs[id]?.done ?? []), list.chapter] });
					continue;
				}
				set({ current: list.chapter });
				const result = await savedChapters.save(input.toon, list.chapter, {
					title: chapter?.title ?? '',
					label: chapter?.label ?? String(list.chapter),
					toonTitle: input.toonTitle,
					locale: input.locale,
					level,
					videos,
					list,
					accessUntil: input.accessUntil
				});
				const job = this.jobs[id];
				if (!job) break;
				if (result === 'saved') set({ done: [...job.done, list.chapter] });
				else if (result === 'failed') set({ failed: [...job.failed, list.chapter] });
				// 'cancelled': stopped (the season too), or the row is saving it by hand right now – skipped
				else if (controller.signal.aborted) break;
			}
			const job = this.jobs[id];
			return controller.signal.aborted ? 'cancelled' : job?.failed.length ? 'failed' : 'saved';
		} finally {
			const { [id]: _done, ...rest } = this.jobs;
			this.jobs = rest;
		}
	}

	/** stop: the running chapter stops, the rest is not started (the saved ones stay) */
	cancel(toon: string, season: number) {
		const job = this.job(toon, season);
		if (!job) return;
		job.controller.abort();
		if (job.current !== null) savedChapters.cancel(toon, job.current);
	}

	/** every saved chapter of the season off this device */
	async remove(toon: string, chapters: number[]) {
		for (const n of chapters) if (savedChapters.get(toon, n)) await savedChapters.remove(toon, n);
	}
}

/** One per page (module state). */
export const seasonSaves = new SeasonSaves();
