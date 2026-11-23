// Reopening resumes (lib/shell/resume.ts): the store in this browser, the start, another member signing in, items gone,
// each workspace's full screen and the scroll positions of the stage page and of panel lists.
import { shared } from '$lib/utils/shared.ts';
import { NO_RESUME, panelKey, readResume, RESUME_KEY, stageKey, startOf, stillThere, switchMember, withFull, withoutGone, withScroll, type DashboardState, type Opened, type ResumeStore, type Scroll, type Start } from './resume.ts';
import type { TabsState } from './tabs.ts';
import { readJson, shell, writeJson } from './shell.svelte.ts';

/** how long a resumed scroll waits for its page or list to grow tall enough (ms) */
const SCROLL_WAIT = 6000;
/** a scroll is kept this long after it settles (ms) */
const SCROLL_SETTLE = 250;

class Resumer {
	store: ResumeStore = typeof localStorage === 'undefined' ? NO_RESUME : readResume(readJson(RESUME_KEY));
	/** the resumed tabs not yet checked against the lists (a link opened is the member's own ask: not checked) */
	#unchecked = new Set<string>();
	/** the workspaces whose kept full screen was taken back (or given up) in this page's life */
	#fullDone = new Set<string>();

	#write(next: ResumeStore) {
		if (next === this.store) return;
		this.store = next;
		writeJson(RESUME_KEY, next);
	}

	/** the start (lib/shell/resume.ts `startOf`): the tabs take it at once; the page to go to and the dashboard are the caller's */
	start(opened: Opened): Start {
		const start = startOf(shell.state, this.store, shell.settings.reopenTabs, opened);
		shell.restore(start.tabs);
		this.#unchecked = new Set(start.tabs.tabs.map((t) => t.address).filter((a) => a !== opened.address));
		return start;
	}

	/** the signed-in member: another than the one before takes their own tabs back – the page to go to then, else null */
	member(id: string): string | null {
		const { store, tabs } = switchMember(this.store, shell.state, id);
		const switched = this.store.member !== null && store.member !== this.store.member;
		this.#write(store);
		if (!switched) return null;
		shell.restore(tabs);
		this.#unchecked = new Set(tabs.tabs.map((t) => t.address));
		this.#fullDone.clear();
		shell.dashboard = tabs.front ? store.dashboard : 'cards';
		return tabs.tabs.find((t) => t.address === tabs.front)?.href ?? '/';
	}

	dashboard(state: DashboardState) {
		if (this.store.dashboard !== state) this.#write({ ...this.store, dashboard: state });
	}

	/**
	 * The resumed tabs checked as their lists arrive (`lists` by kind word, `toons` null while not loaded): those gone or
	 * not the member's leave the strip; their names are returned (the caller logs them).
	 */
	check(lists: Readonly<Record<string, readonly string[] | undefined>>, toons: readonly string[] | null): { gone: string[]; tabs: TabsState } | null {
		if (!this.#unchecked.size) return null;
		const gone: string[] = [];
		for (const address of [...this.#unchecked]) {
			const there = stillThere(address, lists, toons);
			if (there === null) continue;
			this.#unchecked.delete(address);
			if (!there) gone.push(address);
		}
		if (!gone.length) return null;
		const names = gone.map((a) => shell.state.tabs.find((t) => t.address === a)?.label ?? a);
		shell.restore(withoutGone(shell.state, gone));
		return { gone: names, tabs: shell.state };
	}

	/**
	 * A workspace's full screen: the kept one comes back once its view has content (once per page life); after that every
	 * change is kept.
	 */
	full(workspace: string, full: string | null, available: ReadonlySet<string>): string | null {
		const kept = this.store.full[workspace];
		if (!this.#fullDone.has(workspace) && kept && full === null) {
			if (!available.has(kept)) return null;
			this.#fullDone.add(workspace);
			return kept;
		}
		this.#fullDone.add(workspace);
		this.#write(withFull(this.store, workspace, full));
		return null;
	}

	scroll(key: string, scroll: Scroll) {
		this.#write(withScroll(this.store, key, scroll));
	}
}

export const resume = shared('creator-resume', () => new Resumer());

const STAGE = '[data-testid="stage"]';

/** an element's child path from `root` (the panel), or null when it is not inside */
function pathFrom(root: Element, el: Element): number[] | null {
	const path: number[] = [];
	for (let at: Element | null = el; at && at !== root; at = at.parentElement) {
		const parent: Element | null = at.parentElement;
		if (!parent) return null;
		path.unshift(Array.prototype.indexOf.call(parent.children, at));
	}
	return path;
}

const atPath = (root: Element, path: readonly number[]): Element | null => path.reduce<Element | null>((el, i) => el?.children[i] ?? null, root);

/** the panel's key: its workspace, its id and the tab in front (a panel without tabs: its id) */
function panelOf(el: Element): { key: string; panel: Element } | null {
	const panel = el.closest('[data-float]');
	const workspace = shell.floats?.workspace;
	if (!panel || !workspace) return null;
	const id = panel.getAttribute('data-float') ?? '';
	const tab = panel.querySelector('[data-tab][aria-selected="true"]')?.getAttribute('data-tab') ?? id;
	return { key: panelKey(workspace, id, tab), panel };
}

/** Keeps the stage page's and the panel lists' scroll positions as they settle; returns the stop */
export function keepScroll(): () => void {
	const due = new Map<Element, ReturnType<typeof setTimeout>>();
	const take = (el: Element) => {
		const stage = el.closest(STAGE);
		if (stage) {
			const path = pathFrom(stage, el);
			if (path) resume.scroll(stageKey(shell.state.front), { top: el.scrollTop, path });
			return;
		}
		const at = panelOf(el);
		const path = at && pathFrom(at.panel, el);
		if (at && path) resume.scroll(at.key, { top: el.scrollTop, path });
	};
	const onScroll = (event: Event) => {
		const el = event.target;
		if (!(el instanceof Element) || !el.closest(`${STAGE}, [data-float]`)) return;
		clearTimeout(due.get(el));
		due.set(
			el,
			setTimeout(() => {
				due.delete(el);
				take(el);
			}, SCROLL_SETTLE)
		);
	};
	document.addEventListener('scroll', onScroll, { capture: true, passive: true });
	return () => {
		document.removeEventListener('scroll', onScroll, { capture: true });
		for (const t of due.values()) clearTimeout(t);
	};
}

/**
 * Takes the kept scroll positions back – the front item's stage page and the front workspace's panel lists – as each
 * grows tall enough (its content loads after the start), until the member scrolls, clicks or types themselves.
 */
export function resumeScroll(): void {
	const kept = resume.store.scroll;
	const stage = kept[stageKey(shell.state.front)];
	const todo = new Map<string, Scroll>(Object.entries(kept).filter(([k]) => k.startsWith('panel:')));
	if (stage) todo.set(stageKey(shell.state.front), stage);
	if (!todo.size) return;
	const until = performance.now() + SCROLL_WAIT;
	let stopped = false;
	const stop = () => (stopped = true);
	const hands = ['wheel', 'pointerdown', 'keydown', 'touchstart'] as const;
	for (const h of hands) addEventListener(h, stop, { once: true, capture: true, passive: true });
	const step = () => {
		const workspace = shell.floats?.workspace;
		for (const [key, s] of todo) {
			let el: Element | null = null;
			if (key.startsWith('stage:')) {
				const stage = document.querySelector(STAGE);
				el = stage && atPath(stage, s.path ?? []);
			} else if (workspace && key.startsWith(`panel:${workspace}/`)) {
				const panel = document.querySelector(`[data-float="${CSS.escape(key.slice(`panel:${workspace}/`.length).split('/')[0])}"]`);
				if (panel && s.path && panelOf(panel)?.key === key) el = atPath(panel, s.path);
			}
			if (el && el.scrollHeight - el.clientHeight >= s.top - 1) {
				el.scrollTop = s.top;
				todo.delete(key);
			}
		}
		if (todo.size && !stopped && performance.now() < until) requestAnimationFrame(step);
		else for (const h of hands) removeEventListener(h, stop, { capture: true });
	};
	requestAnimationFrame(step);
}
