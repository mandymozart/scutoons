<script lang="ts">
	import { KANON_SECTIONS, type KanonRole } from '@scutoons/content';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The Kanon's writing rules (research/KANON.md §4): the sections of one role – or of every role (the help panel) –
	 * in their order, in the UI's language.
	 */
	let { role = null }: { role?: KanonRole | null } = $props();

	const ROLE_LABEL: Record<KanonRole, () => string> = {
		toon: () => m['creator.kanon_role_toon'](),
		character: () => m['creator.kanon_role_character'](),
		persona: () => m['creator.kanon_role_persona'](),
		look: () => m['creator.kanon_role_look'](),
		place: () => m['creator.kanon_role_place']()
	};
	const roles = $derived(role ? [role] : (Object.keys(KANON_SECTIONS) as KanonRole[]));
	const heading = (s: { de: string; en: string }) => (getLocale() === 'de' ? s.de : s.en);
</script>

<div class="rules" data-testid="kanon-rules">
	<p>{m['creator.kanon_rules_intro']()}</p>
	{#each roles as r (r)}
		<p class="role">{m['creator.kanon_rules']({ role: ROLE_LABEL[r]() })}</p>
		<p class="sections">{KANON_SECTIONS[r].map((s) => `## ${heading(s)}`).join(' · ')}</p>
	{/each}
</div>

<style>
	.rules p {
		margin: 0 0 0.5rem;
	}
	.role {
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		font-size: var(--type-caption-size);
	}
</style>
