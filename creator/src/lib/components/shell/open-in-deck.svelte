<script lang="ts">
	import { deckLink } from '@scutoons/api';
	import { Button } from '@scutoons/ui/editor';
	import { library } from '$lib/editor/library.svelte.ts';
	import { onDeckLink } from '$lib/shell/deck.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * "Open in the deck" (lap2 P4, DECK-LINK.md) – the one control on an asset, a scene, a rig and a module: the link that
	 * opens the deck on the item, in the workspace the core names for it (deck-catalog `library::open`, editor deck). What
	 * the deck has no workspace for yet is off, the core's sentence why as its hover. `edit`: the creator only previews
	 * the item (a scene, a file), so the link says "Edit in deck"; `framed` where it sits on a picture.
	 */
	let { address, testid = 'open-in-deck', edit = false, variant = 'quiet' }: { address: string; testid?: string; edit?: boolean; variant?: 'quiet' | 'framed' } = $props();

	let href = $state<string | null>(null);
	let why = $state<string | null>(null);
	$effect(() => {
		const asked = address;
		let current = true;
		library
			.opener('deck')
			.then((rule) => {
				if (!current) return;
				const answer = rule.open(asked);
				href = answer.ok ? deckLink(answer.workspace, answer.address) : null;
				why = answer.ok ? null : answer.why;
			})
			.catch(() => current && ((href = null), (why = null)));
		return () => (current = false);
	});
</script>

<Button {variant} icon="external-link" href={href ?? undefined} disabled={!href} title={why ?? (edit ? m['shell.edit_in_deck_title']() : m['shell.open_in_deck_title']())} onclick={onDeckLink} data-testid={testid}>{edit ? m['shell.edit_in_deck']() : m['shell.open_in_deck']()}</Button>
