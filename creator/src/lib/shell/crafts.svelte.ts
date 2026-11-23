// The member's crafts (server docs/API.md "Crafts" – what someone does, not the catalog's recipe crafting): the list, the
// member's own, the workspaces they start with (resolved by the server) and whether the first-login question is open.
// The API decides who may change them; this only reads and sends. A missing or failing API = no crafts (crafts.ts).
import { createCraftsApi, type Craft, type MemberCrafts } from '@scutoons/api';
import { api } from '$lib/api.ts';
import { shared } from '$lib/utils/shared.ts';
import { asksCrafts, readCrafts } from './crafts.ts';
import { startKinds } from './workspaces.ts';
import type { WorkspaceKind } from './address.ts';

const craftsApi = createCraftsApi(api);

class Crafts {
	list = $state<Craft[]>([]);
	mine = $state<MemberCrafts | null>(null);
	/** the crafts' workspaces this creator knows ([] = no crafts: every workspace) */
	readonly starts = $derived<WorkspaceKind[]>(startKinds(this.mine?.workspaces));
	/** the first login's question is shown */
	readonly ask = $derived(asksCrafts({ list: this.list, mine: this.mine }));
	private loading: Promise<void> | null = null;

	/** once per page */
	load(): Promise<void> {
		this.loading ??= readCrafts(craftsApi.list, craftsApi.mine).then((s) => {
			this.list = s.list;
			this.mine = s.mine;
		});
		return this.loading;
	}

	/** the member's answer (first login) or a change where the API allows it */
	async save(ids: string[]): Promise<void> {
		this.mine = await craftsApi.setMine(ids);
	}
}

export const crafts = shared('creator-crafts', () => new Crafts());
