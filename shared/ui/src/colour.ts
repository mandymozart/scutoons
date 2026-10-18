// Colour maths for the colour popover (ColorField, owner 2026-10-18): hex ⟷ HSV, the brand palette from the tokens, the
// recent colours of this browser. No Svelte.

export const HEX = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i;

/** `#abc` → `#aabbcc`, lower case */
export function longHex(c: string): string {
	return (c.length === 4 ? `#${c[1]}${c[1]}${c[2]}${c[2]}${c[3]}${c[3]}` : c).toLowerCase();
}

export interface Hsv {
	/** 0–360 */
	h: number;
	/** 0–1 */
	s: number;
	/** 0–1 */
	v: number;
}

export function hexToHsv(hex: string): Hsv {
	const n = parseInt(longHex(hex).slice(1), 16);
	const r = ((n >> 16) & 255) / 255;
	const g = ((n >> 8) & 255) / 255;
	const b = (n & 255) / 255;
	const max = Math.max(r, g, b);
	const d = max - Math.min(r, g, b);
	let h = 0;
	if (d) h = max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
	return { h: (h * 60 + 360) % 360, s: max ? d / max : 0, v: max };
}

export function hsvToHex({ h, s, v }: Hsv): string {
	const f = (n: number) => {
		const k = (n + h / 60) % 6;
		return Math.round((v - v * s * Math.max(0, Math.min(k, 4 - k, 1))) * 255);
	};
	return `#${[f(5), f(3), f(1)].map((x) => x.toString(16).padStart(2, '0')).join('')}`;
}

/** the brand palette (DESIGN.md "Color"): name → the token it is read from */
export const BRAND_TOKENS: [string, string][] = [
	['ink', '--ink'],
	['paper', '--paper'],
	['cyan', '--primary-cyan'],
	['purple', '--acid-purple'],
	['green', '--acid-green'],
	['red', '--acid-red'],
	['muted', '--text-muted'],
	['orange', '--badge-warning'],
	['blue', '--badge-info']
];

/** the brand colours as the page's tokens define them (only whole hex values) */
export function brandPalette(): { name: string; hex: string }[] {
	if (typeof document === 'undefined') return [];
	const style = getComputedStyle(document.documentElement);
	return BRAND_TOKENS.map(([name, token]) => ({ name, hex: style.getPropertyValue(token).trim() })).filter((c) => HEX.test(c.hex)).map((c) => ({ ...c, hex: longHex(c.hex) }));
}

const RECENT_KEY = 'scutoons.recent-colours';
const RECENT_MAX = 8;

/** this browser's last picked colours, newest first */
export function recentColours(): string[] {
	try {
		const list = JSON.parse(localStorage.getItem(RECENT_KEY) ?? '[]');
		return Array.isArray(list) ? list.filter((c) => typeof c === 'string' && HEX.test(c)).slice(0, RECENT_MAX) : [];
	} catch {
		return [];
	}
}

export function rememberColour(hex: string): void {
	if (!HEX.test(hex)) return;
	const c = longHex(hex);
	try {
		localStorage.setItem(RECENT_KEY, JSON.stringify([c, ...recentColours().filter((x) => x !== c)].slice(0, RECENT_MAX)));
	} catch {
		// not remembered – fine
	}
}
