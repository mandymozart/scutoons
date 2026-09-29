// Certificate templates (text with {{variable[:type]}}), port of generate/app/utils/template.js.
import cos from './templates/cos.md?raw';
import qr from './templates/qr.md?raw';

export interface TemplateField {
	id: string;
	label: string;
	type: 'text' | 'number' | 'date' | 'textarea';
	placeholder: string;
}

/** Certificate of Authenticity (membership) and access certificate / printable QR (promo) */
export const TEMPLATES = { cos, qr } as const;

/** membership → CoA, promo → printable QR; event / xr / page have no PDF */
export function templateFor(type: string): string | null {
	if (type === 'membership') return TEMPLATES.cos;
	if (type === 'promo') return TEMPLATES.qr;
	return null;
}

const AUTO = ['code', 'qr', 'url', 'created_at', 'expires_at'];
const TYPES: Record<string, TemplateField['type']> = { text: 'text', number: 'number', date: 'date', textfield: 'textarea' };
const PLACEHOLDERS: Record<string, string> = {
	receiver: 'John Doe',
	productname: 'SCUTOONS Hoodie with QR Code',
	edition_number: '1',
	creators: 'Artist 1, Artist 2',
	email: 'your@email.com'
};

const label = (name: string) =>
	name
		.split('_')
		.map((word) => word.charAt(0).toUpperCase() + word.slice(1))
		.join(' ');

/** The fillable fields of a template in order of first appearance (auto-generated variables skipped) */
export function parseTemplate(content: string): TemplateField[] {
	const fields = new Map<string, TemplateField>();
	for (const match of content.matchAll(/\{\{(\w+)(?::(\w+))?\}\}/g)) {
		const name = match[1];
		const kind = match[2] || 'text';
		if (AUTO.includes(name) || fields.has(name)) continue;
		fields.set(name, {
			id: name,
			label: label(name),
			type: TYPES[kind] ?? 'text',
			placeholder: PLACEHOLDERS[name] ?? (kind === 'number' ? '0' : '')
		});
	}
	return [...fields.values()];
}

/** Fills the variables; `{{qr}}` becomes the image placeholder line */
export function renderTemplate(content: string, data: Record<string, unknown>): string {
	return content.replace(/\{\{(\w+)(?::\w+)?\}\}/g, (_match, name: string) => {
		if (name === 'qr') return '[QR_CODE_PLACEHOLDER]';
		const value = data[name];
		return value !== undefined && value !== null ? String(value) : '';
	});
}
