<script lang="ts">
	import { Button, Chip, Overlay } from '@scutoons/ui/editor';
	import { craftName } from '@scutoons/api';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { crafts } from '$lib/shell/crafts.svelte.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { toaster } from '$lib/toasts.ts';

	/**
	 * The first login's question (lap2 draft A7), on the overlay level: what do you do? The crafts as the kit's chips, an
	 * invite's ticked; Continue saves them and opens the switcher with their workspaces. It stays until answered.
	 */
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
		<h1 class="ask">{m['shell.crafts_ask']()}</h1>
		<div class="crafts">
			{#each crafts.list as craft (craft.id)}
				<Chip
					label={craftName(craft, getLocale())}
					on={picked.includes(craft.id)}
					title={invited.includes(craft.id) ? m['shell.crafts_from_invite']() : undefined}
					onclick={() => toggle(craft.id)}
					testid="craft-{craft.id}"
				/>
			{/each}
		</div>
		{#if invited.length}<p class="invite">{m['shell.crafts_from_invite']()}</p>{/if}
		<div><Button variant="primary" disabled={saving} onclick={onContinue} data-testid="crafts-continue">{m['shell.crafts_continue']()}</Button></div>
	</div>
</Overlay>

<style>
	.body {
		display: flex;
		flex-direction: column;
		gap: var(--editor-gap);
		width: min(32rem, calc(100vw - 4rem));
		padding: var(--editor-gap);
	}
	.ask {
		margin: 0;
		font-size: var(--editor-title);
		font-weight: inherit;
	}
	.crafts {
		display: flex;
		flex-wrap: wrap;
		gap: var(--kit-gap);
	}
	.invite {
		margin: 0;
		font-size: var(--editor-small);
		color: var(--tool-muted);
	}
</style>
