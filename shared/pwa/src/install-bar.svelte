<script lang="ts">
	// A tool's install bar (creator, admin, embassy – agents/research/PWA-APPS.md §3): "Install SCUTOONS Creator as an
	// app" at the moment ./install-rules.ts picks (the 2nd visit, never the first; 30 days after "Not now"; `?install`
	// shows it at once). Chromium: "Install" → the browser's dialog. No install event (iPhone Safari, Firefox on Android,
	// or `?install` where nothing can install): the how-to in the bar itself, "OK" = seen. The update bar goes first.
	import { Button } from '@scutoons/ui';
	import type { Install } from './install.svelte.ts';
	import type { PwaLabels } from './labels.ts';

	let { install, app, labels, hidden = false }: { install: Install; app: string; labels: PwaLabels; hidden?: boolean } = $props();

	const howTo = $derived(
		install.way === 'ios' ? labels.iosHowTo : install.way === 'android-firefox' ? labels.firefoxHowTo : install.way === 'chromium-menu' ? labels.chromiumHowTo : labels.cannotHowTo
	);
</script>

{#if install.offer && !hidden}
	<div class="install-bar" role="status" data-testid="install-prompt" data-way={install.way}>
		<p>
			<strong>{labels.offer(app)}</strong>
			{#if !install.available}<span class="how" data-testid="install-howto">{howTo}</span>{/if}
		</p>
		<div class="actions">
			{#if install.available}
				<Button variant="secondary" size="sm" onclick={() => install.notNow()} data-testid="install-not-now">{labels.notNow}</Button>
				<Button size="sm" onclick={() => install.install()} data-testid="install-app">{labels.install}</Button>
			{:else}
				<Button size="sm" onclick={() => install.howTo()} data-testid="install-ok">{labels.ok}</Button>
			{/if}
		</div>
	</div>
{/if}

<style>
	/* the reader's update / install bar look: paper, an ink frame, bottom centre, above the phone's home bar */
	.install-bar {
		position: fixed;
		z-index: 2040;
		left: 0.5rem;
		right: 0.5rem;
		bottom: calc(0.5rem + env(safe-area-inset-bottom, 0rem));
		max-width: 31.25rem;
		margin: 0 auto;
		box-sizing: border-box;
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--ink);
		padding: 0.75rem;
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 0.5rem 0.75rem;
	}
	p {
		margin: 0;
		flex: 1 1 12rem;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	strong {
		font-weight: 400;
	}
	.how {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		color: var(--text-muted);
	}
	.actions {
		display: flex;
		gap: 0.5rem;
		margin-left: auto;
	}
</style>
