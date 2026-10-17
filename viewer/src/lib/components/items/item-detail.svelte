<script lang="ts">
	import { getInventoryContext, localized } from '$lib/contexts/inventory.svelte.ts';
	import { getItemsUiContext } from '$lib/contexts/items-ui.svelte.ts';
	import { getHomesContext } from '$lib/contexts/homes.svelte.ts';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import { isGiftable, isSoldOut, mediaPlayback, playerAlbums, type Album, type AlbumTrack, type Entry, type ProductCatalog } from '@scutoons/content';
	import { loadProducts, productOf } from '$lib/products.ts';
	import { shopUrl } from '$lib/config.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { Field, Icon, IconButton, StickyActions } from '@scutoons/ui';
	import type { InviteOffer } from '@scutoons/api';
	import { m } from '$lib/paraglide/messages.js';
	import ItemIcon from './item-icon.svelte';
	import ItemModel from './item-model.svelte';
	import ContainerActions from './container-actions.svelte';
	import ScoreActions from './score-actions.svelte';
	import { albumDownloadUrl } from '$lib/protected-files.ts';
	import { albumRequest } from './record-shelf.ts';

	let {
		entry,
		onback,
		oncombine,
		onopencontainer = () => {},
		onpack = () => {}
	}: { entry: Entry; onback: () => void; oncombine: (entry: Entry) => void; onopencontainer?: (container: Entry) => void; onpack?: (entry: Entry) => void } = $props();

	const inventory = getInventoryContext();
	const ui = getItemsUiContext();

	const def = $derived(inventory.item(entry.item_id));
	const model = $derived(inventory.modelOf(entry));
	// the VR headset: the reader's home from anywhere (research/CAVE.md D2)
	const homes = getHomesContext();
	const skills = $derived(def?.skills ?? []);
	// the old play_audio skill is offered as "Abspielen" below (mediaPlayback), not as "Benutzen"
	const usable = $derived(skills.some((s) => s.type !== 'play_audio'));

	// Players, albums, related items (agents/ITEMS.md): a record plays only with a player for its format; without one the
	// detail names the missing player and offers the album download (if this reader may have it, `album.access`). A
	// player lists the records it plays: play the ones the reader may listen to, buy the others.
	const membership = getMembershipContext();
	const listener = $derived({ owned: inventory.entries.map((e) => e.item_id), member: membership.isValid });
	const playback = $derived(mediaPlayback(inventory.catalog, def, listener));
	const playerList = $derived(playerAlbums(inventory.catalog, def, listener));
	let products = $state<ProductCatalog | null>(null);
	$effect(() => {
		if (playerList.some((row) => !row.listen)) loadProducts().then((c) => (products = c));
	});
	// a physical release's tracks live in the vault: the player asks for each one's signed URL when it starts (the URLs
	// live ten minutes, an album longer – owner 2026-10-14: "play the whole album")
	let audioError = $state(false);
	function play(album: Album, tracks: AlbumTrack[], from: Entry | null) {
		audioError = false;
		// the record's 3D model rides along (its sleeve with the record inside – the card plays it on a 3D turntable)
		ui.playAlbum(albumRequest(album, tracks, from, from ? inventory.modelOf(from) || null : null));
	}
	async function download(album: Album, path: string) {
		audioError = false;
		try {
			window.open(await albumDownloadUrl(album.id, path), '_blank', 'noopener');
		} catch {
			audioError = true;
		}
	}
	const entryOf = (itemId: string) => inventory.entries.find((e) => e.item_id === itemId) ?? null;
	// a substance (ITEMS.md "Substances"): "Benutzen" uses one piece up and applies it
	const substance = $derived(skills.some((s) => s.type === 'apply'));
	// food (cucumber, tuna can) is eaten: "Essen" instead of "Benutzen", same mechanics (appetite down)
	const useLabel = $derived(def?.category === 'food' ? m['items.eat']() : m['items.use']());
	let useError = $state<string | null>(null);
	async function useIt() {
		useError = null;
		working = true;
		const result = await inventory.useSubstance(entry);
		working = false;
		if (!result.ok) useError = result.error ?? 'error';
	}
	// a recipe takes it, or another piece of the same kind can go onto its field (research/PACKING.md §6)
	const combinable = $derived(
		(inventory.catalog?.recipes ?? []).some((r) => r.inputs.some((i) => i.item === entry.item_id)) ||
			inventory.twinsOf(entry).length > 0
	);
	const equipped = $derived(inventory.isEquipped(entry));
	const total = $derived(inventory.editionTotal(entry.item_id));
	// "Anziehen" swaps (ITEMS.md "Wardrobe"): what was on that slot goes back into the inventory, or – while "Hide
	// sensitive content" is on – the swim pants come back on; the newest such note is shown under the buttons
	const note = $derived([...inventory.notices].reverse().find((n) => n.kind === 'returned'));
	const noteText = $derived(
		note ? m['items.returned']({ name: inventory.nameOf({ item_id: note.item_id, variant: null }) }) : ''
	);
	let working = $state(false);

	const sourceLabel = $derived(
		({
			qr: m['items.source_qr'](),
			story: m['items.source_story'](),
			scan: m['items.source_scan'](),
			craft: m['items.source_craft'](),
			starter: m['items.source_starter'](),
			gift: m['items.source_gift']()
		})[entry.source] ?? entry.source
	);

	// "Verschenken" (ITEMS.md "Ownership, claims, gifting"): members only, giftable items; a worn piece comes off first
	// (a stackable keeps its worn piece back); the server checks everything again (cooldown, daily limit, recipient)
	const giftable = $derived(inventory.isAccount && isGiftable(def));
	const giftMax = $derived(Math.max(0, entry.qty - (equipped ? 1 : 0)));
	let giving = $state(false);
	const giftFormId = $derived(`gift-form-${entry.id}`);
	let giftTo = $state('');
	let giftQty = $state(1);
	let giftError = $state<string | null>(null);
	/**
	 * no account for the address while a gift invite campaign runs (agents/research/GIFT-INVITES.md): not an error but
	 * an offer – invite them, the gift waits for them (until `inviteUntil`) and can be called back until then
	 */
	let inviteOffer = $state<InviteOffer | null>(null);
	const GIFT_ERRORS = ['recipient_not_found', 'worn', 'cooldown', 'max_reached', 'daily_limit', 'self_gift', 'invalid_email', 'not_giftable'];
	const INVITE_ERRORS = ['has_account', 'already_invited', 'campaign_limit', 'sold_out', 'no_campaign', 'daily_limit', 'code_needs_account', 'container_needs_account'];
	const giftCount = $derived(Math.min(Math.max(1, giftQty), giftMax || 1));
	async function sendGift(event: SubmitEvent) {
		event.preventDefault();
		if (!giftTo.includes('@')) {
			giftError = m['items.gift_error_invalid_email']();
			return;
		}
		giftError = null;
		inviteOffer = null;
		working = true;
		const result = await inventory.gift(entry, giftTo, giftCount);
		working = false;
		if (result.ok) {
			giving = false;
			onback();
			return;
		}
		// a campaign with the member's own item, invites left, a piece without a code: offer the invite
		if (result.error === 'recipient_not_found' && result.invite?.gift === 'own' && result.invite.left > 0 && !entry.code && !def?.holds) {
			inviteOffer = result.invite;
			return;
		}
		const code = GIFT_ERRORS.includes(result.error ?? '') ? result.error : 'failed';
		giftError = (m as unknown as Record<string, () => string>)[`items.gift_error_${code}`]();
	}
	async function sendInvite() {
		working = true;
		const result = await inventory.invite(giftTo, entry, giftCount);
		working = false;
		if (result.ok) {
			giving = false;
			inviteOffer = null;
			onback();
			return;
		}
		inviteOffer = null;
		const code = INVITE_ERRORS.includes(result.error ?? '') ? result.error : 'failed';
		giftError = (m as unknown as Record<string, () => string>)[`items.invite_error_${code}`]();
	}
	const inviteUntil = $derived(inviteOffer?.wait_until ? new Date(inviteOffer.wait_until).toLocaleDateString(getLocale()) : '');

	async function toggleWear() {
		if (!def?.slot) return;
		working = true;
		try {
			if (equipped) await inventory.unequipEntry(entry);
			else await inventory.equip(entry);
		} catch (e) {
			console.warn('Avatar update failed', e);
		} finally {
			working = false;
		}
	}
</script>

<div class="item-detail">
	<!-- the item's 3D model when it has one (FEATURES 10.102), else / meanwhile its icon -->
	<div class="hero" class:model>
		{#snippet icon()}<ItemIcon srcs={[inventory.iconOf(entry), def?.icon ?? '']} name={inventory.nameOf(entry)} lettering={inventory.letteringOf(entry)} />{/snippet}
		{#if model}<ItemModel path={model} label={inventory.nameOf(entry)} fallback={icon} />{:else}{@render icon()}{/if}
	</div>

	<h2>{inventory.nameOf(entry)}</h2>
	{#if def}
		<p class="description">{localized(def.description)}</p>
	{/if}

	<dl class="meta">
		{#if entry.edition_number}
			<dt>Edition</dt>
			<dd>
				{total
					? m['items.edition']({ number: entry.edition_number, total })
					: m['items.edition_number']({ number: entry.edition_number })}
			</dd>
		{/if}
		{#if entry.qty > 1}
			<dt>{m['items.qty']()}</dt>
			<dd>{entry.qty}</dd>
		{/if}
		<dt>{m['items.source']()}</dt>
		<dd>{sourceLabel}{entry.code ? ` · ${entry.code}` : ''}</dd>
		{#if equipped}
			<dt>{m['items.tab_avatar']()}</dt>
			<dd data-testid="item-worn">{m['items.worn']()}</dd>
		{/if}
	</dl>

	{#if playback?.kind === 'needs_player' && playback.players.length}
		<p class="swap-note" role="status" data-testid="item-needs-player">{m['items.needs_player']({ player: localized(playback.players[0].name) })}</p>
	{/if}
	{#if playerList.length}
		<section class="player-albums" data-testid="player-albums">
			<h3>{m['items.player_albums']()}</h3>
			<ul>
				{#each playerList as row (row.album.id)}
					{@const product = productOf(products, row.carrier)}
					{@const carrierName = localized(row.carrier.name)}
					<li data-album={row.album.id} data-testid="player-album">
						<div class="thumb"><ItemIcon srcs={[row.carrier.icon]} name={carrierName} /></div>
						<div class="album-info">
							<span>{localized(row.album.title)}</span>
							<span class="album-meta">{row.owned ? m['items.player_owned']() : carrierName}</span>
						</div>
						{#if row.listen}
							<button class="btn-secondary" onclick={() => play(row.album, row.album.tracks, entryOf(row.carrier.id))} data-testid="player-play">{m['items.play']()}</button>
						{:else if product && isSoldOut(product)}
							<span class="album-meta" data-testid="player-sold-out">{m['items.player_sold_out']()}</span>
						{:else}
							<a class="btn btn-secondary" href={product?.link ?? shopUrl(getLocale())} target="_blank" rel="noopener noreferrer" data-testid="player-buy">{m['items.player_buy']()}</a>
						{/if}
					</li>
				{/each}
			</ul>
		</section>
	{/if}
	{#if giving}
		<form class="gift" id={giftFormId} onsubmit={sendGift} data-testid="gift-form">
			<h3>{m['items.gift_title']({ name: inventory.nameOf(entry) })}</h3>
			{#if giftMax < 1}
				<p class="swap-note" role="status">{m['items.gift_worn']()}</p>
			{:else}
				<Field label={m['items.gift_to']()} error={giftError}>
					{#snippet children({ id, describedby, invalid })}
						<input {id} type="email" name="gift_to" autocomplete="off" inputmode="email" required aria-describedby={describedby} aria-invalid={invalid} bind:value={giftTo} oninput={() => (inviteOffer = null)} />
					{/snippet}
				</Field>
				{#if giftMax > 1}
					<Field label={m['items.gift_qty']()}>
						{#snippet children({ id, describedby })}
							<input {id} type="number" name="gift_qty" min="1" max={giftMax} aria-describedby={describedby} bind:value={giftQty} />
						{/snippet}
					</Field>
				{/if}
				{#if inviteOffer}
					<div class="invite-offer" role="status" data-testid="gift-invite-offer">
						<span class="invite-title">{m['items.invite_no_account']()}</span>
						<span>{m['items.invite_offer']({ name: inventory.nameOf(entry), until: inviteUntil })}</span>
					</div>
					<p class="gift-note">{m['items.invite_note']()}</p>
				{:else}
					<p class="gift-note">{m['items.gift_note']()}{entry.code ? ` ${m['items.gift_note_code']()}` : ''}</p>
				{/if}
			{/if}
		</form>
	{/if}
	{#if noteText}<p class="swap-note" role="status" data-testid="item-note">{noteText}</p>{/if}
	{#if useError}<p class="swap-note" role="alert">{m['items.use_failed']()}</p>{/if}
	{#if audioError}<p class="swap-note" role="alert">{m['items.release_failed']()}</p>{/if}

	<!-- the item's actions stuck to the panel's bottom, the detail scrolls under them (owner 2026-10-16, DESIGN.md
	     "Sticky action bar"); while a gift is written, its cancel / send instead -->
	<StickyActions testid="item-actions">
		{#if giving && giftMax >= 1}
			<button type="button" class="btn-secondary" onclick={() => ((giving = false), (inviteOffer = null))}>{m['items.gift_cancel']()}</button>
			{#if inviteOffer}
				<button type="button" class:disabled={working} disabled={working} onclick={sendInvite} data-testid="gift-invite">{m['items.invite_confirm']()}</button>
			{:else}
				<button type="submit" form={giftFormId} class:disabled={working} disabled={working} data-testid="gift-confirm">{m['items.gift_confirm']()}</button>
			{/if}
		{:else}
			<!-- scores and containers (research/SCORES.md, research/PACKING.md): open the score, its PDF; open a container,
			     pack (the pack mode picks the target) / take out -->
			<ScoreActions {entry} />
			<ContainerActions {entry} onopen={onopencontainer} {onpack} />
			<!-- the less frequent ones as round icons (their name on hover / for screen readers), the main ones in words
			     filling the row (owner 2026-10-16: "mit Icons arbeiten, um Platz zu sparen") -->
			{#if playback?.download}
				<IconButton label={m['items.download_album']()} onclick={() => playback.download && download(playback.album, playback.download)} data-testid="item-download"><Icon name="download" size="1rem" /></IconButton>
			{/if}
			{#if combinable}
				<IconButton label={m['items.combine']()} onclick={() => oncombine(entry)} data-testid="item-combine"><Icon name="layers" size="1rem" /></IconButton>
			{/if}
			{#if def?.slot}
				<IconButton label={equipped ? m['items.take_off']() : m['items.put_on']()} pressed={equipped} disabled={working} onclick={toggleWear} data-testid="item-wear"><Icon name={equipped ? 'user-minus' : 'user-plus'} size="1rem" /></IconButton>
			{/if}
			{#if giftable && !giving}
				<IconButton label={m['items.gift']()} onclick={() => ((giving = true), (giftError = null), (giftQty = 1))} data-testid="item-gift"><Icon name="gift" size="1rem" /></IconButton>
			{/if}
			{#if usable}
				<button class="btn-secondary" class:disabled={working} disabled={working} onclick={() => (substance ? useIt() : skills.some((s) => s.type === 'enter_home') ? homes.openOwn('vr') : ui.use(entry, skills))} data-testid="item-use">{useLabel}</button>
			{/if}
			{#if playback?.kind === 'play'}
				<button class="btn-secondary" onclick={() => playback.kind === 'play' && play(playback.album, playback.tracks, entry)} data-testid="item-play">{m['items.play']()}</button>
			{/if}
		{/if}
	</StickyActions>
</div>

<style>
	.item-detail {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.hero {
		width: 10rem;
		max-width: 60%;
		align-self: center;
	}
	/* the 3D view over the panel's whole width, out of its body's padding (like the record card's stage) */
	.hero.model {
		align-self: stretch;
		width: auto;
		max-width: none;
		margin-inline: calc(-1rem - env(safe-area-inset-left)) calc(-1rem - env(safe-area-inset-right));
	}
	h2 {
		margin: 0;
		font-size: 1rem;
		font-weight: 700;
	}
	.description {
		margin: 0;
	}
	.meta {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 0.25rem 1rem;
		margin: 0.5rem 0;
		font-size: 0.75rem;
		line-height: 1.3;
	}
	dt {
		color: var(--text-muted);
	}
	dd {
		margin: 0;
	}
	.swap-note {
		margin: 0;
		font-size: 0.75rem;
		line-height: 1.3;
	}
	.item-detail :global(.sc-sticky-actions button) {
		padding: 0 1rem;
	}
	.player-albums {
		border-top: 2px solid var(--ink, black);
		padding-top: 0.75rem;
	}
	.player-albums h3 {
		margin: 0 0 0.5rem;
		font-size: 1rem;
	}
	.player-albums ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.player-albums li {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	.player-albums .thumb {
		width: 3rem;
		flex: none;
	}
	.album-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}
	.album-meta {
		font-size: 0.75rem;
		color: var(--text-muted);
	}
	.player-albums a.btn,
	.player-albums button {
		flex: none;
		padding: 0 0.75rem;
		text-decoration: none;
	}
	.gift {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		border-top: 2px solid var(--ink, black);
		padding-top: 0.75rem;
	}
	.gift h3 {
		margin: 0;
		font-size: 1rem;
	}
	.gift input {
		width: 100%;
		box-sizing: border-box;
	}
	.gift-note {
		margin: 0;
		font-size: 0.75rem;
		line-height: 1.3;
	}
	/* no account for the address: a warning that offers the invite (research/GIFT-INVITES.md), not an error */
	.invite-offer {
		display: grid;
		gap: 0.25rem;
		padding-left: 0.6rem;
		border-left: 0.5rem solid var(--state-warning);
		font-size: 0.875rem;
		line-height: 1.35;
	}
	.invite-title {
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
</style>
