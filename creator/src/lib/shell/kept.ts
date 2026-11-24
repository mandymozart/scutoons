// What the shell keeps in this browser (tabs, workspace use, chosen layouts, settings, resume) is read and written here
// only, so a sample of the shell (the /kit page's Shell) can hold every write while it runs. Pure: test/kept.test.ts.

let holds = 0;

export function readJson(key: string): unknown {
	try {
		return JSON.parse(localStorage.getItem(key) ?? 'null');
	} catch {
		return null;
	}
}

export function writeJson(key: string, value: unknown): void {
	if (holds > 0) return;
	try {
		localStorage.setItem(key, JSON.stringify(value));
	} catch {
		// storage blocked or full: it holds for this visit
	}
}

/** nothing is kept in this browser until the returned function is called (once; holds nest) */
export function holdWrites(): () => void {
	holds++;
	let ended = false;
	return () => {
		if (ended) return;
		ended = true;
		holds--;
	};
}

export const writesHeld = (): boolean => holds > 0;
