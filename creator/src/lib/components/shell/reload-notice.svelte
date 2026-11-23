<script lang="ts">
	import { Button, Pill } from '@scutoons/ui/editor';
	import { NEUTRAL_LOCALE } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { fileLabel } from '$lib/labels.ts';
	import { watched } from '$lib/editor/doc-session.svelte.ts';

	/**
	 * A newer version of the open item announces itself (RULES H19, as the deck's on D3): quietly above the status bar, left,
	 * with Reload – and, when changes of this page are not saved, Keep mine; Reload then asks first (the overlay level).
	 */
	const session = $derived(watched.session);
	const notice = $derived(session?.notice ?? null);
	const file = $derived(notice ? (notice.locale && notice.locale !== NEUTRAL_LOCALE ? `${fileLabel(notice.doc.entity)} (${notice.locale})` : fileLabel(notice.doc.entity)) : '');
	const text = $derived(notice ? (notice.unsaved ? m['creator.newer_unsaved']({ file }) : m['creator.newer']({ file })) : '');
</script>

{#if session && notice}
	<Pill class="reload-notice" role="status" data-testid="reload-notice">
		<span class="words" title={text}><span class="dot" aria-hidden="true"></span>{text}</span>
		{#if notice.unsaved}
			<Button variant="quiet" onclick={() => session.keepMine()} title={m['creator.keep_mine_hint']()} data-testid="reload-notice-keep">{m['creator.keep_mine']()}</Button>
		{/if}
		<Button onclick={() => session.reload()} title={notice.unsaved ? m['creator.reload_unsaved_hint']() : m['creator.reload_hint']()} data-testid="reload-notice-reload">{m['creator.reload']()}</Button>
	</Pill>
{/if}

<style>
	/* left, above the status bar (D3), over the panels; it waits, nothing else waits on it */
	:global(.kit-pill.reload-notice) {
		position: fixed;
		z-index: 55;
		left: var(--kit-margin);
		bottom: calc(var(--kit-margin) * 2 + var(--kit-bar));
		max-width: calc(100vw - var(--kit-margin) * 2);
		padding-left: var(--editor-pad);
		animation: reload-notice-in var(--motion-base) var(--motion-ease);
	}
	.words {
		display: inline-flex;
		align-items: center;
		gap: calc(var(--editor-pad) * 0.6);
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		margin-right: calc(var(--editor-pad) * 0.6);
	}
	.dot {
		flex: none;
		width: 0.5rem;
		height: 0.5rem;
		border-radius: 50%;
		background: var(--kit-status-warning-mark);
	}
	@keyframes reload-notice-in {
		from {
			opacity: 0;
			transform: translateY(0.5rem);
		}
	}
</style>
