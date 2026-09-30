<script lang="ts">
	import type { Character } from '@scutoons/content';
	import { DEFAULT_MOOD, featuredCharacters, legacyCharacterColor, legacyCharacterImage } from '@scutoons/content';
	import { asset } from '$lib/utils/cdn.ts';
	import { isKnownMissing, markMissing } from '$lib/utils/missing-images.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * "Staffel 1 · mit …" / "featuring …": small framed avatars (the 192 px avatar view, persona colour behind the head,
	 * like the VN's left slot) with the names. `ids` = a chapter's `featuring` or the season cast (agents/ITEMS.md
	 * "Cast & featuring"); ids that aren't in the loaded cast are skipped, nothing renders when none is left.
	 */
	let {
		ids,
		characters,
		toonId,
		lead,
		size = 'small'
	}: {
		ids: string[] | null | undefined;
		characters: Character[] | null | undefined;
		toonId: string;
		/** the text in front ("Staffel 1 · mit"); default "mit" / "featuring" */
		lead?: string;
		/** small = the chapter list, large = the season line and the chapter start */
		size?: 'small' | 'large';
	} = $props();

	const shown = $derived(featuredCharacters(ids, characters));

	/** the avatar view's path (the missing-image rule applies) */
	const pathOf = (character: Character) => legacyCharacterImage(character, { mood: DEFAULT_MOOD }, 'avatar', { isMissing: isKnownMissing });
	// a leading / = from the CDN root (registry characters), else inside the toon's folder
	const url = (path: string) => (path ? asset(path.startsWith('/') ? path : `${toonId}/${path}`) : '');
</script>

{#if shown.length}
	<p class="featuring {size}" data-testid="featuring">
		<span class="lead">{lead ?? m['chapter.featuring']()}</span>
		{#each shown as character (character.name)}
			{@const path = pathOf(character)}
			<span class="who" data-character={character.name}>
				<span class="avatar" style:background-color={legacyCharacterColor(character)}>
					{#if path}<img src={url(path)} alt="" width="192" height="192" loading="lazy" onerror={() => markMissing(path)} />{/if}
				</span>
				<span class="name">{character.label}</span>
			</span>
		{/each}
	</p>
{/if}

<style>
	.featuring {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-025, 0.25rem) var(--space-075, 0.75rem);
		margin: 0;
		font-size: 0.75rem;
		line-height: 1.3;
		color: var(--text-muted, #6b6b6b);
	}
	.who {
		display: inline-flex;
		align-items: center;
		gap: var(--space-025, 0.25rem);
		color: var(--ink, #000);
		white-space: nowrap;
	}
	.avatar {
		display: inline-block;
		flex: none;
		box-sizing: border-box;
		width: 1.5rem;
		height: 1.5rem;
		overflow: hidden;
		border: 2px solid var(--ink, #000);
		border-radius: var(--radius-sm, 0.5rem);
	}
	.large .avatar {
		width: 2.25rem;
		height: 2.25rem;
	}
	.avatar img {
		display: block;
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
</style>
