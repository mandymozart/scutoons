#!/usr/bin/env node
// The two FX items (owner 2026-10-08) in the LOCAL catalog through the admin API as the test user (history + undo in
// admin → Catalog): `punschkrapfen` – food that sobers up (a running trip lasts half as long, appetite −10) – and
// `sunglasses` – worn on the face, everything orange while worn (worn_tint). Icons: media/scutoons/items/*.svg (local
// only – production: the owner creates both in admin and uploads real pictures). Rerun = update.
//   npm run dev:test-user && node scripts/dev/fx-items.mjs
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const API = local('api', '/api').replace(/\/$/, '');
if (!/^https?:\/\/(localhost|127\.0\.0\.1)(:\d+)?\//.test(API + '/')) throw new Error('local API only');
const token = JSON.parse(readFileSync(resolve(ROOT, 'logs/dev/test-user.json'), 'utf8')).token;
const headers = { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' };

const ITEMS = [
	{
		id: 'punschkrapfen',
		toon: null,
		name: { de: 'Punschkrapfen', en: 'Punschkrapfen' },
		description: {
			de: 'Rosa glasiert, süß und mit einem Hauch Rum. Wer einen isst, wird schneller wieder klar: ein laufender Rausch dauert nur noch halb so lang.',
			en: 'Pink glazed, sweet, with a hint of rum. Eating one sobers you up: a running trip lasts only half as long.'
		},
		kind: 'virtual',
		category: 'food',
		icon: 'scutoons/items/punschkrapfen.svg',
		stackable: true,
		starter: false,
		grantable: ['story', 'scan'],
		slot: null,
		skills: [{ type: 'apply' }],
		edition_id: null,
		product: null,
		variants: null,
		effects: [{ stat: 'appetite', delta: -10 }],
		sober: 0.5,
		respawn: { default: { hours: 6 }, min: { hours: 1 }, max: { days: 7 } }
	},
	{
		id: 'sunglasses',
		toon: null,
		name: { de: 'Sonnenbrille', en: 'Sunglasses' },
		description: {
			de: 'Coole Gläser in Orange. Solange du sie trägst, sieht alles nach Sommerabend aus.',
			en: 'Cool orange lenses. While you wear them, everything looks like a summer evening.'
		},
		kind: 'virtual',
		category: 'wearable',
		icon: 'scutoons/items/sunglasses.svg',
		stackable: false,
		starter: false,
		grantable: ['story'],
		slot: 'face',
		skills: [],
		edition_id: null,
		product: null,
		variants: null,
		worn_tint: '#ff9a2e'
	}
];

for (const data of ITEMS) {
	const res = await fetch(`${API}/admin/catalog/items/${data.id}`, { method: 'PUT', headers, body: JSON.stringify({ data, note: 'FX items (local, scripts/dev/fx-items.mjs)' }) });
	const json = await res.json().catch(() => null);
	console.log(`${data.id}: ${res.status}${res.ok ? (json?.created ? ' created' : ' updated') : ` ${JSON.stringify(json?.problems ?? json?.message ?? json).slice(0, 300)}`}`);
}
