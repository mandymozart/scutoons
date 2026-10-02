// One tooltip for every tool (owner 2026-10-02: "we need universal tooltip styling"). Takes over the native `title`
// tooltips of the whole document: on hover and on keyboard focus the title is shown in one `.sc-tooltip` (styled in
// @scutoons/tokens tools.css), placed below the element (above when there is no room) and kept inside the window;
// multi-line titles keep their line breaks. While shown, the title moves to `data-sc-title` so the browser's own
// tooltip doesn't appear as well; an element without an accessible name gets the title as its aria-label.
//
//   onMount(() => installTooltips());   // in a tool's root layout; returns the uninstall function

const GAP = 6;
const DELAY_MS = 350;

export function installTooltips(root: Document = document): () => void {
	const tip = root.createElement('div');
	tip.className = 'sc-tooltip';
	tip.setAttribute('role', 'tooltip');
	tip.id = 'sc-tooltip';
	tip.hidden = true;
	root.body.appendChild(tip);

	let current: HTMLElement | null = null;
	let timer: ReturnType<typeof setTimeout> | undefined;

	const textOf = (el: HTMLElement) => el.getAttribute('title') ?? el.dataset.scTitle ?? '';

	function show(el: HTMLElement) {
		const text = textOf(el);
		if (!text) return;
		if (el.hasAttribute('title')) {
			el.dataset.scTitle = text;
			el.removeAttribute('title');
		}
		if (!el.hasAttribute('aria-label') && !el.hasAttribute('aria-labelledby') && !el.textContent?.trim()) el.setAttribute('aria-label', text);
		el.setAttribute('aria-describedby', tip.id);
		current = el;
		tip.textContent = text;
		tip.hidden = false;
		place(el);
	}

	function place(el: HTMLElement) {
		const box = el.getBoundingClientRect();
		const view = root.documentElement;
		const width = tip.offsetWidth;
		const height = tip.offsetHeight;
		let left = Math.min(Math.max(box.left, GAP), view.clientWidth - width - GAP);
		let top = box.bottom + GAP;
		if (top + height > view.clientHeight - GAP && box.top - height - GAP > 0) top = box.top - height - GAP;
		tip.style.left = `${Math.max(GAP, left)}px`;
		tip.style.top = `${top}px`;
	}

	function hide() {
		clearTimeout(timer);
		if (current) {
			if (current.dataset.scTitle) {
				current.setAttribute('title', current.dataset.scTitle);
				delete current.dataset.scTitle;
			}
			current.removeAttribute('aria-describedby');
		}
		current = null;
		tip.hidden = true;
	}

	const target = (e: Event) => (e.target instanceof Element ? (e.target.closest('[title], [data-sc-title]') as HTMLElement | null) : null);

	function over(e: Event) {
		const el = target(e);
		if (el === current) return;
		hide();
		if (!el) return;
		// take the title away at once so the native tooltip never starts
		if (el.hasAttribute('title')) {
			el.dataset.scTitle = el.getAttribute('title') ?? '';
			el.removeAttribute('title');
		}
		current = el;
		timer = setTimeout(() => current === el && show(el), e.type === 'focusin' ? 0 : DELAY_MS);
	}

	function out(e: Event) {
		const next = (e as MouseEvent | FocusEvent).relatedTarget;
		if (current && next instanceof Node && current.contains(next)) return;
		hide();
	}

	const key = (e: KeyboardEvent) => e.key === 'Escape' && hide();

	root.addEventListener('pointerover', over);
	root.addEventListener('pointerout', out);
	root.addEventListener('focusin', over);
	root.addEventListener('focusout', out);
	root.addEventListener('keydown', key);
	root.addEventListener('scroll', hide, true);
	root.addEventListener('pointerdown', hide);

	return () => {
		hide();
		root.removeEventListener('pointerover', over);
		root.removeEventListener('pointerout', out);
		root.removeEventListener('focusin', over);
		root.removeEventListener('focusout', out);
		root.removeEventListener('keydown', key);
		root.removeEventListener('scroll', hide, true);
		root.removeEventListener('pointerdown', hide);
		tip.remove();
	};
}
