// The one Publish (RULES H18, lap 2 A10): the bar's icon opens this dialog on the overlay level – every draft readers do
// not see yet, ticked (and the submits this account reviews); All · None; one primary button with the count; the
// confirm where one is needed; then the report in the same place. Publishing itself is publish-tree.ts's.
import type { ContentIndexEntity } from '@scutoons/api';
import { entityOf } from '@scutoons/content';
import { contentApi } from '$lib/api.ts';
import { fileLabels } from '$lib/labels.ts';
import { m } from '$lib/paraglide/messages.js';
import { shared } from '$lib/utils/shared.ts';
import type { PageDoc } from '$lib/shell/shell.svelte.ts';
import { notPublished, type ToonRef } from '$lib/editor/not-published.svelte.ts';
import { publishable, publishChosen, publishFailed, type PublishLine, type PublishTreeResult } from '$lib/editor/publish-tree.ts';
import type { PublishNotice } from '$lib/editor/publish-report.ts';
import { reviewGroups, reviews } from '$lib/editor/reviews.svelte.ts';
import { decisions, needsConfirm, primaryKind, startTicks, toggled } from '$lib/editor/publish-choice.ts';

export type PublishStep = 'loading' | 'list' | 'confirm' | 'sending' | 'note' | 'report';

export interface PublishList {
	files: ContentIndexEntity[];
	lines: PublishLine[];
}

/** where the list comes from and where the choice goes – the server here; the /kit page's Shell gives samples */
export interface PublishSource {
	load(toons: ToonRef[]): Promise<PublishList>;
	send(list: PublishList, chosen: ReadonlySet<string>, decisionNote: string, askNote: () => Promise<string | null>): Promise<PublishTreeResult>;
}

/** the submits this account reviews, a line each (published as submitted, its author gets the note) */
function decisionLines(): PublishLine[] {
	return reviewGroups(reviews.overview?.to_review ?? []).map((g) => {
		const first = g.items[0];
		const newer = g.items.some((r) => r.newest !== null && r.newest > r.version);
		return {
			id: `review:${g.id}`,
			label: fileLabels([...new Set(g.items.map((r) => entityOf(r.key)))]),
			type: m['shell.dash_review_asked']({ who: first.author?.email ?? '–' }),
			chip: m['creator.review_state_pending'](),
			hint: first.note ? `„${first.note}“` : newer ? m['creator.review_newer']() : '',
			review: false,
			decide: g.items.map((r) => r.id)
		};
	});
}

export const serverSource: PublishSource = {
	async load(toons) {
		const [list] = await Promise.all([publishable({ kind: 'all', toons }), reviews.load()]);
		const decide = decisionLines();
		// a file in a submit is that submit's line
		const inReview = new Set((reviews.overview?.to_review ?? []).map((r) => r.key));
		return { files: list.files, lines: [...decide, ...list.lines.filter((l) => !inReview.has(l.id))] };
	},
	async send(list, chosen, decisionNote, askNote) {
		const ids = list.lines.filter((l) => l.decide && chosen.has(l.id)).flatMap((l) => l.decide ?? []);
		const decided = ids.length ? (await contentApi.decideReviews('publish', ids, decisionNote.trim())).keys : [];
		if (ids.length) void reviews.load();
		return publishChosen(list.files, chosen, askNote, decided);
	}
};

class PublishFlow {
	open = $state(false);
	step = $state<PublishStep>('loading');
	lines = $state.raw<PublishLine[]>([]);
	ticked = $state.raw<ReadonlySet<string>>(new Set());
	/** the files the open page holds back, by key: the reason */
	held = $state.raw<ReadonlyMap<string, string>>(new Map());
	/** the open page's errors (the confirm says so) */
	errors = $state(0);
	/** the note to the authors of the submits decided here (the confirm) */
	decisionNote = $state('');
	/** the note for the reviewers (asked when something goes to review) */
	note = $state('');
	report = $state.raw<PublishNotice[]>([]);
	source: PublishSource = serverSource;
	#list: PublishList = { files: [], lines: [] };
	#page: PageDoc | null = null;
	#toons: ToonRef[] = [];
	#answer: ((note: string | null) => void) | null = null;

	get count(): number {
		return this.ticked.size;
	}
	get kind(): 'publish' | 'submit' {
		return primaryKind(this.lines, this.ticked);
	}
	/** the ticked submits of others (the confirm's note goes to their authors) */
	get decided(): PublishLine[] {
		return decisions(this.lines, this.ticked);
	}

	/** the dialog opens: what the open page has waiting is saved first, then the list */
	async show(toons: ToonRef[], page: PageDoc | null): Promise<void> {
		this.open = true;
		this.step = 'loading';
		this.#page = page;
		this.#toons = toons;
		this.report = [];
		this.note = '';
		this.decisionNote = '';
		this.errors = page?.blocking ?? 0;
		this.held = new Map((page?.held ?? []).map((h) => [h.key, h.reason]));
		try {
			if (page?.prepare && !(await page.prepare())) return this.#end([{ type: 'error', text: m['shell.publish_unsaved']() }]);
			const list = await this.source.load(toons);
			if (!this.open) return;
			this.#list = list;
			this.lines = list.lines;
			this.ticked = startTicks(list.lines, new Set(this.held.keys()));
			if (!list.lines.length) return this.#end([{ type: 'info', text: m['creator.nothing_to_publish']() }]);
			this.step = 'list';
		} catch (e) {
			this.#end(publishFailed(e).notices);
		}
	}

	/** Esc, a click outside, Cancel, Close – not while it sends */
	close(): void {
		if (this.step === 'sending') return;
		if (this.#answer) return this.answer(false);
		this.open = false;
	}

	all(): void {
		this.ticked = startTicks(this.lines, new Set());
	}
	none(): void {
		this.ticked = new Set();
	}
	tick(id: string, on: boolean): void {
		this.ticked = toggled(this.ticked, id, on);
	}

	/** the primary button: the confirm first where one is needed, then it sends */
	go(): void {
		if (!this.count) return;
		if (this.step === 'list' && needsConfirm(this.lines, this.ticked, this.errors)) {
			this.step = 'confirm';
			return;
		}
		void this.#send();
	}
	back(): void {
		if (this.step === 'confirm') this.step = 'list';
	}

	/** the note for the reviewers: sent (true) or not submitted (false) */
	answer(send: boolean): void {
		const answer = this.#answer;
		this.#answer = null;
		this.step = 'sending';
		answer?.(send ? this.note : null);
	}

	async #send(): Promise<void> {
		this.step = 'sending';
		let result: PublishTreeResult;
		try {
			result = await this.source.send(this.#list, this.ticked, this.decisionNote, () => this.#askNote());
		} catch (e) {
			result = publishFailed(e);
		}
		this.#end(result.notices);
		try {
			await this.#page?.published?.(result);
		} catch (e) {
			console.warn('the page could not read again what went live', e);
		}
		if (result.published.length) void notPublished.load(this.#toons, true);
	}

	#askNote(): Promise<string | null> {
		this.note = '';
		this.step = 'note';
		return new Promise((resolve) => (this.#answer = resolve));
	}

	#end(report: PublishNotice[]): void {
		this.report = report;
		this.step = 'report';
	}

	/** the /kit page's Shell: a step drawn with sample lines, nothing sent */
	sample(lines: PublishLine[], step: PublishStep = 'list', report: PublishNotice[] = []): void {
		this.open = true;
		this.#page = null;
		this.errors = 0;
		this.held = new Map();
		this.#list = { files: [], lines };
		this.lines = lines;
		this.ticked = startTicks(lines, new Set());
		this.report = report;
		this.step = step;
	}
}

export const publishFlow = shared('creator-publish-flow', () => new PublishFlow());
