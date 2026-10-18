<script lang="ts">
	// A tool's update bar (agents/research/PWA-APPS.md §2): a new version is installed and waits – "Later" · "Reload".
	// Reload = the waiting worker takes over, then the page reloads (./tool-pwa.svelte.ts).
	import { Button } from '@scutoons/ui';
	import type { ToolPwa } from './tool-pwa.svelte.ts';
	import type { PwaLabels } from './labels.ts';

	let { pwa, app, labels }: { pwa: ToolPwa; app: string; labels: PwaLabels } = $props();
</script>

{#if pwa.updateReady}
	<div class="update-bar" role="status" data-testid="update-prompt">
		<p>{labels.updateReady(app)}</p>
		<div class="actions">
			<Button variant="secondary" size="sm" onclick={() => pwa.later()}>{labels.later}</Button>
			<Button size="sm" onclick={() => pwa.reload()} data-testid="update-reload">{labels.reload}</Button>
		</div>
	</div>
{/if}

<style>
	.update-bar {
		position: fixed;
		z-index: 2041;
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
	}
	.actions {
		display: flex;
		gap: 0.5rem;
		margin-left: auto;
	}
</style>
