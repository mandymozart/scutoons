<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { Button, Overlay, Tile } from '@scutoons/ui/editor';
	import { craftName } from '@scutoons/api';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { greetName } from '$lib/shell/dashboard.ts';
	import { crafts } from '$lib/shell/crafts.svelte.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { toaster } from '$lib/toasts.ts';

	/**
	 * The first login's question (research/redesign/lap/crafts-v2.html view `crafts`), on the overlay level: what do you do?
	 * The crafts as tiles, an invite's ticked; Continue saves them and opens the switcher with their workspaces. It stays
	 * until answered.
	 */
	const session = getCreatorSession();
	let picked = $state<string[]>([...(crafts.mine?.crafts ?? [])]);
	let saving = $state(false);
	const invited = $derived(crafts.mine?.invited ?? []);

	const toggle = (id: string) => (picked = picked.includes(id) ? picked.filter((p) => p !== id) : [...picked, id]);

	async function onContinue() {
		saving = true;
		try {
			await crafts.save(picked);
			shell.toggleSwitcher(true);
		} catch (e) {
			toaster.addToast({ message: e instanceof Error ? e.message : String(e), type: 'error' });
		} finally {
			saving = false;
		}
	}
</script>

<Overlay place="center" label={m['shell.crafts_ask']()} data-testid="crafts-question">
	<div class="body">
		<h1 id="crafts-hello" class="hello">{m['shell.dash_welcome']({ name: greetName(session.user?.email ?? '') })}</h1>
		<p class="ask">{m['shell.crafts_ask']()}</p>
		<div class="tiles">
			{#each crafts.list as craft (craft.id)}
				{@const on = picked.includes(craft.id)}
				<Tile label={craftName(craft, getLocale())} sub={invited.includes(craft.id) ? m['shell.crafts_from_invite']() : undefined} {on} onclick={() => toggle(craft.id)} data-testid="craft-{craft.id}">
					{#snippet picture()}<Icon name={craft.icon} size="1.3rem" strokeWidth={1.5} />{#if on}<Icon name="check-circle" size="1.1rem" strokeWidth={1.5} />{/if}{/snippet}
				</Tile>
			{/each}
		</div>
		<div class="end"><Button variant="primary" disabled={saving} onclick={onContinue} data-testid="crafts-continue">{m['shell.crafts_continue']()}</Button></div>
	</div>
</Overlay>

<style>
	.body {
		display: flex;
		flex-direction: column;
		gap: var(--editor-gap);
		width: min(46rem, calc(100vw - 4rem));
	}
	.hello {
		margin: 0;
		font-size: 1.6rem;
		font-weight: inherit;
	}
	.ask {
		margin: 0 0 var(--editor-gap);
		color: var(--tool-muted);
	}
	.tiles {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		gap: var(--editor-gap);
	}
	.tiles :global(.kit-tile) {
		min-height: 7rem;
		justify-content: space-between;
	}
	.end {
		display: flex;
		justify-content: flex-end;
		margin-top: var(--editor-gap);
	}
</style>
