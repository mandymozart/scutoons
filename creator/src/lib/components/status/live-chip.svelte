<script lang="ts">
	import { clickOutside } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { hasStatus, notLive, stateWords, STATES, type NotLive } from '$lib/components/status/file-status.svelte';

	/**
	 * The page's not-live languages as ONE chip in the ActionBar, next to Publish (UI-CONCEPTS E.1, owner 2026-10-07 –
	 * it replaces the file-status row above the editor): the languages that aren't live + the worst state ("de en ·
	 * draft ▾"; phones: "de ▾"), its colour = that state (tools.css .sc-chip). A language-neutral chapter / scene /
	 * season stands for all its text languages (they publish together). It opens the file-status list upwards – per
	 * file its label, the languages and the state in words. Nothing when everything is live.
	 * The list stays in the DOM while closed (hidden), so the browser checks read it as before.
	 */
	let { files }: { files: { doc: ContentDoc; label: string }[] } = $props();

	const uid = $props.id();
	let open = $state(false);
	let toggle: HTMLButtonElement | null = $state(null);

	const shown = $derived(files.filter((f) => hasStatus(f.doc)));
	/** a stored language → the reader languages it stands for: a neutral document with texts = all its text languages */
	const languagesOf = (doc: ContentDoc, l: string) => (l !== '_' ? [l] : (doc.texts?.languages ?? []));
	/** per shown file its not-live rows: the languages, the state, its words (+ • for unsaved edits) */
	const rows = $derived(
		shown.map((f) => ({
			file: f,
			states: f.doc.locales.flatMap((l) => {
				const state = notLive(f.doc, l);
				return state ? [{ l, state, languages: languagesOf(f.doc, l), words: `${stateWords(f.doc, l)}${f.doc.dirtyLocales.includes(l) ? ' •' : ''}` }] : [];
			})
		}))
	);
	const states = $derived(rows.flatMap((r) => r.states));
	/** the language codes that aren't live (a neutral entity without texts – cast, characters … – carries none) */
	const codes = $derived([...new Set(states.flatMap((s) => s.languages))].join(' '));
	const worst = $derived(STATES.find((w) => states.some((s) => s.state === w)) ?? 'draft');
	const word = (state: NotLive) => (state === 'missing' ? m['creator.live_chip_missing']() : state === 'unsaved' ? m['creator.live_chip_unsaved']() : m['creator.live_chip_draft']());
	const tone = (state: NotLive) => (state === 'missing' ? 'is-error' : state === 'unsaved' ? 'is-warning' : 'is-draft');
	/** the chip's name and tooltip: every file with its not-live languages */
	const title = $derived(
		m['creator.live_chip']({ states: rows.map((r) => `${r.file.label}: ${r.states.map((s) => `${s.languages.length ? `${s.languages.join(' ')} ` : ''}${word(s.state)}`).join(', ')}`).join(' · ') })
	);

	$effect(() => {
		if (!shown.length) open = false;
	});

	function close(event?: Event) {
		if (!open) return;
		open = false;
		if (event instanceof KeyboardEvent) toggle?.focus();
	}
</script>

{#if shown.length}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="live-chip" data-testid="file-status" {@attach clickOutside(close)} onkeydown={(e) => e.key === 'Escape' && close(e)}>
		<button
			bind:this={toggle}
			type="button"
			class="ghost toggle"
			aria-expanded={open}
			aria-controls="{uid}-files"
			aria-label={title}
			{title}
			onclick={() => (open = !open)}
			data-testid="file-status-toggle"
		>
			<span class="sc-chip is-dot {tone(worst)}" aria-hidden="true"></span>
			{#if codes}<span class="codes">{codes}</span>{/if}<span class="word" class:solo={!codes}>{codes ? ' · ' : ''}{word(worst)}</span>
			<span class="caret" aria-hidden="true">▾</span>
		</button>
		<div class="popover" id="{uid}-files" hidden={!open}>
			{#each rows as r (r.file.doc.entity)}
				<p class="file-status">
					<span class="name">{r.file.label}</span>
					{#each r.states as s (s.l)}
						<span class="sc-chip {tone(s.state)}">{s.languages.length ? `${s.languages.join(' ')} · ` : ''}{s.words}</span>
					{/each}
				</p>
			{/each}
		</div>
	</div>
{/if}

<style>
	.live-chip {
		position: relative;
		flex: none;
	}
	.toggle {
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
		white-space: nowrap;
	}
	.toggle[aria-expanded='true'] {
		border-color: var(--ink);
	}
	.caret {
		font-size: var(--type-caption-size);
	}
	/* opens upwards out of the ActionBar, right-aligned with the chip */
	.popover {
		position: absolute;
		right: 0;
		bottom: calc(100% + var(--space-05) + var(--border-width-regular));
		z-index: 100;
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--space-05);
		box-sizing: border-box;
		width: max-content;
		max-width: min(28rem, calc(100vw - 2rem));
		max-height: 60dvh;
		overflow: auto;
		padding: var(--space-075) var(--space-1);
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
	}
	.popover[hidden] {
		display: none;
	}
	.file-status {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05);
		margin: 0;
	}
	.name {
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
	/* phones: the language codes + ▾ only (owner 2026-10-07: More · Save · de ▾ · Publish fits at 375 px) */
	@media (max-width: 479px) {
		.word:not(.solo),
		.toggle .sc-chip {
			display: none;
		}
		.toggle {
			gap: var(--space-025);
		}
	}
</style>
