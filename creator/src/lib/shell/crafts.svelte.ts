// The member's crafts (server docs/API.md "Crafts" – what someone does, not the catalog's recipe crafting): the list, the
// member's own, the workspaces they start with (resolved by the server) and whether the first-login question is open.
// The API decides who may change them; this only reads and sends.
import { createCraftsApi, type Craft, type MemberCrafts } from '@scutoons/api';
import { api } from '$lib/api.ts';
import { shared } from '$lib/utils/shared.ts';
import { startKinds } from './workspaces.ts';
import type { WorkspaceKind } from './address.ts';

const craftsApi = createCraftsApi(api);

class Crafts {
	list = $state<Craft[]>([]);
	mine = $state<MemberCrafts | null>(null);
	/** the crafts' workspaces this creator knows ([] = no crafts: every workspace) */
	readonly starts = $derived<WorkspaceKind[]>(startKinds(this.mine?.workspaces));
	private loading: Promise<void> | null = null;

	/** once per page; a failed load leaves the member without crafts (every workspace, no question) */
	load(): Promise<void> {
		this.loading ??= Promise.all([craftsApi.list(), craftsApi.mine()])
			.then(([l, me]) => {
				this.list = l.crafts;
				this.mine = me;
			})
			.catch(() => {
				this.loading = null;
			});
		return this.loading;
	}

	/** the member's answer (first login) or a change where the API allows it */
	async save(ids: string[]): Promise<void> {
		this.mine = await craftsApi.setMine(ids);
	}
}

export const crafts = shared('creator-crafts', () => new Crafts());
