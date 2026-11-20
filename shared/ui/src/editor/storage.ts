/**
 * The editor kit's settings in this browser (panel layouts, open sections). Storage may be blocked – a private window, a
 * policy, a full quota: a read then finds nothing and a write is dropped, so the setting holds only while the page lives.
 * A value that does not parse reads as nothing stored; the caller checks the shape.
 */
export const readStoredJson = (key: string): unknown => {
	try {
		const raw = localStorage.getItem(key);
		return raw === null ? undefined : JSON.parse(raw);
	} catch {
		return undefined;
	}
};

/** Stores a value as JSON; undefined removes the key */
export const writeStoredJson = (key: string, value: unknown): void => {
	try {
		if (value === undefined) localStorage.removeItem(key);
		else localStorage.setItem(key, JSON.stringify(value));
	} catch {
		// blocked or full: the value holds while the page lives
	}
};
