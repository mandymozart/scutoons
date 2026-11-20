import { readStoredJson, writeStoredJson } from './storage.ts';

/** The open state of the panels' sections per section id, in this browser; one nobody toggled takes its own default */
const KEY = 'sc-editor-sections';

const read = (): Record<string, boolean> => {
	const parsed = readStoredJson(KEY);
	if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return {};
	return Object.fromEntries(Object.entries(parsed).filter((e): e is [string, boolean] => typeof e[1] === 'boolean'));
};

class Sections {
	#open = $state<Record<string, boolean>>(read());

	isOpen(id: string, fallback: boolean): boolean {
		return this.#open[id] ?? fallback;
	}

	setOpen(id: string, open: boolean): void {
		this.#open = { ...this.#open, [id]: open };
		writeStoredJson(KEY, this.#open);
	}

	toggle(id: string, fallback: boolean): void {
		this.setOpen(id, !this.isOpen(id, fallback));
	}
}

export const sections = new Sections();
