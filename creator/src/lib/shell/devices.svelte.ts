// Resume across devices (lib/shell/devices.ts): this browser's state goes to the server per device as it settles, beside
// the local copy; on start this device's own state counts first and another device's newer state is offered once. A
// missing API, offline or an error leaves the local copy as it is. Taking another device's state writes it into this
// browser's stores and reloads – every workspace then starts from it as from its own.
import { createDeviceSessionApi } from '@scutoons/api';
import { PANELS_KEY } from '@scutoons/ui/editor';
import { api } from '$lib/api.ts';
import { shared } from '$lib/utils/shared.ts';
import { DEVICE_KEY, deviceName, fitPanels, hostPanels, newDeviceId, readMeta, resumption, seenKey, withHostPanels, type DeviceMeta, type DeviceRow, type DeviceState } from './devices.ts';
import { CHOSEN_KEY, layouts } from './layouts.svelte.ts';
import { RESUME_KEY, readResume } from './resume.ts';
import { resume } from './resume.svelte.ts';
import { readSettings, SETTINGS_KEY } from './settings.ts';
import { readJson, shell, TABS_KEY, writeJson } from './shell.svelte.ts';

const sessionApi = createDeviceSessionApi(api);

/** how often the state is looked at (ms); it is sent once it held still for one look */
const LOOK = 2000;

export interface Offer {
	name: string;
	at: string | null;
	state: DeviceState;
	key: string;
}

const random = (n: number) => crypto.getRandomValues(new Uint8Array(n));

class DeviceSync {
	meta: DeviceMeta & { at: number } = { id: '', seen: null, at: 0 };
	/** another device's newer state, offered until taken or put away */
	offer = $state.raw<Offer | null>(null);
	#member: string | null = null;
	#available = false;
	#sent = '';
	#looked = '';
	#timer: ReturnType<typeof setInterval> | null = null;

	constructor() {
		if (typeof localStorage === 'undefined') return;
		const raw = readJson(DEVICE_KEY);
		const meta = readMeta(raw, () => newDeviceId(random));
		const at = typeof (raw as { at?: unknown } | null)?.at === 'number' ? (raw as { at: number }).at : 0;
		this.meta = { ...meta, at };
		this.#keepMeta();
	}

	#keepMeta() {
		writeJson(DEVICE_KEY, this.meta);
	}

	/** this browser's state now */
	snapshot(): DeviceState {
		const { dashboard, full, scroll } = resume.store;
		return {
			v: 1,
			window: { w: innerWidth, h: innerHeight },
			tabs: $state.snapshot(shell.state),
			chosen: $state.snapshot(layouts.chosen),
			panels: hostPanels(readJson(PANELS_KEY)),
			views: $state.snapshot(shell.settings.views),
			session: { dashboard, full, scroll }
		};
	}

	/** the signed-in member: their rows read, the start decided, then the state kept going out */
	async start(member: string, read: (device: string) => Promise<{ available: boolean; mine: DeviceRow | null; other: DeviceRow | null }> = sessionApi.mine): Promise<void> {
		if (this.#member === member) return;
		this.#member = member;
		this.offer = null;
		this.#available = false;
		this.#sent = this.#looked = JSON.stringify(this.#comparable(this.snapshot()));
		try {
			const rows = await read(this.meta.id);
			if (this.#member !== member || !rows.available) return;
			this.#available = true;
			const next = resumption(this.snapshot(), this.meta.at, rows.mine, rows.other, this.meta.seen);
			if (next.kind === 'own') this.apply(next.state);
			else if (next.kind === 'offer') this.offer = { name: next.row.name, at: next.row.updated_at, state: next.state, key: seenKey(next.row) };
			// what the server holds for this device: sent already unless this browser changed since
			if (rows.mine && rows.mine.state) this.#sent = JSON.stringify(this.#comparable(rows.mine.state as DeviceState));
		} catch {
			// no API, offline: the local copy as now
		} finally {
			this.#run();
		}
	}

	/** what counts as a change: the window's size alone is not one */
	#comparable(state: DeviceState): Omit<DeviceState, 'window'> {
		const { window: _w, ...rest } = state;
		return rest;
	}

	#run() {
		if (this.#timer || typeof window === 'undefined') return;
		this.#timer = setInterval(() => this.#look(), LOOK);
		addEventListener('pagehide', this.#flush);
		document.addEventListener('visibilitychange', this.#onHidden);
	}

	stop() {
		if (this.#timer) clearInterval(this.#timer);
		this.#timer = null;
		removeEventListener('pagehide', this.#flush);
		document.removeEventListener('visibilitychange', this.#onHidden);
	}

	#onHidden = () => {
		if (document.visibilityState === 'hidden') this.#flush();
	};

	#flush = () => this.#look(true);

	/** debounced: a state that changed is sent once it held still for one look (or the page goes away) */
	#look(now = false) {
		const state = this.snapshot();
		const json = JSON.stringify(this.#comparable(state));
		const changed = json !== this.#looked;
		this.#looked = json;
		if (changed) {
			this.meta = { ...this.meta, at: Date.now() };
			this.#keepMeta();
			if (!now) return;
		}
		if (json === this.#sent || !this.#available || !this.#member) return;
		this.#sent = json;
		sessionApi.save({ device: this.meta.id, name: deviceName(navigator.userAgent), state }).catch(() => {
			// offline or refused: sent again with the next change
			this.#sent = '';
		});
	}

	/** another device's state taken: written into this browser's stores, the panels fitted into this window, then reloaded */
	apply(state: DeviceState) {
		const panels = fitPanels(state.panels, state.window, { w: innerWidth, h: innerHeight });
		writeJson(TABS_KEY, state.tabs);
		writeJson(CHOSEN_KEY, state.chosen);
		writeJson(PANELS_KEY, withHostPanels(readJson(PANELS_KEY), panels));
		writeJson(SETTINGS_KEY, { ...readSettings(readJson(SETTINGS_KEY)), views: state.views });
		writeJson(RESUME_KEY, { ...readResume(readJson(RESUME_KEY)), member: this.#member, ...state.session });
		this.meta = { ...this.meta, at: Date.now() };
		this.#keepMeta();
		this.stop();
		const front = state.tabs.tabs.find((t) => t.address === state.tabs.front);
		location.assign(front?.href ?? '/');
	}

	accept() {
		const offer = this.offer;
		if (!offer) return;
		this.#seen(offer.key);
		this.apply(offer.state);
	}

	dismiss() {
		if (this.offer) this.#seen(this.offer.key);
		this.offer = null;
	}

	#seen(key: string) {
		this.meta = { ...this.meta, seen: key };
		this.#keepMeta();
	}
}

export const devices = shared('creator-devices', () => new DeviceSync());
