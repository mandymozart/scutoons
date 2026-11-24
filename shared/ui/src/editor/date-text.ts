// A date typed into the kit's Field (type="date"): the day as stored, ISO `YYYY-MM-DD`.

const ISO = /^(\d{4})-(\d{1,2})-(\d{1,2})$/;
const DOTTED = /^(\d{1,2})\.(\d{1,2})\.(\d{4})$/;

/** '' for an empty field (unset), the ISO day for a day that exists (`2026-1-5`, `5.1.2026`), else null (not a date) */
export function dateText(text: string): string | null {
	const t = text.trim();
	if (t === '') return '';
	const iso = ISO.exec(t);
	const dotted = iso ? null : DOTTED.exec(t);
	if (!iso && !dotted) return null;
	const [y, mo, d] = iso ? [iso[1], iso[2], iso[3]].map(Number) : [dotted![3], dotted![2], dotted![1]].map(Number);
	const day = new Date(Date.UTC(y, mo - 1, d));
	if (day.getUTCFullYear() !== y || day.getUTCMonth() !== mo - 1 || day.getUTCDate() !== d) return null;
	return `${String(y).padStart(4, '0')}-${String(mo).padStart(2, '0')}-${String(d).padStart(2, '0')}`;
}
