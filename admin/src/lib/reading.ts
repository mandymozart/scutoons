// Where readers are (BACKLOG 10.69): shared labels of the user page's "Reading" section and the Reading page.
import { m } from '$lib/paraglide/messages.js';

/** "Cat Pope · chapter 2" – chapter 0 is a season's prologue (@scutoons/content progress.ts PROLOGUE) */
export function chapterLabel(toon: string, chapter: number): string {
	return chapter === 0 ? m['reading.prologue']({ toon }) : m['reading.chapter']({ toon, chapter: String(chapter) });
}

/** the toon's name from the catalog's toons, else its id */
export function toonName(toons: readonly { id: string; name: string }[], id: string): string {
	return toons.find((t) => t.id === id)?.name ?? id;
}
