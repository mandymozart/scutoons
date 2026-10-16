<script lang="ts">
	import { Icon } from '@scutoons/ui';

	/**
	 * The account pages' one back link (owner 2026-10-16: "der Zurück-Button braucht ein bisschen schönere
	 * Einheitlichkeit"): the same place, look and height on the index ("Zurück zu CatPope") and on a group's page
	 * ("Mein Konto") – a chevron and the target's name, frameless, at --control-height. `back`: go back in history
	 * instead (the page and its scroll position come back); a modified click still opens `href`.
	 */
	let { href, label, back = false, testid }: { href: string; label: string; back?: boolean; testid?: string } = $props();

	function onclick(event: MouseEvent) {
		if (!back || event.metaKey || event.ctrlKey || event.shiftKey || event.button !== 0) return;
		event.preventDefault();
		history.back();
	}
</script>

<a class="back" {href} {onclick} data-testid={testid}>
	<Icon name="chevron-left" size="1rem" />
	<span>{label}</span>
</a>

<style>
	.back {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		max-width: 100%;
		height: var(--control-height);
		color: var(--ink);
		text-decoration: none;
	}
	.back span {
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}
	.back:hover span {
		text-decoration: underline;
	}
	.back:focus-visible {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: 2px;
	}
</style>
