<script lang="ts">
	import type { SampleTrigger } from '@scutoons/content';

	let { trigger }: { trigger?: SampleTrigger } = $props();
	$inspect(trigger);
</script>

{#if trigger?.rect && (trigger.type === 'click' || trigger.type === 'hover')}
	<div
		class="sample-trigger"
		style:left="{trigger.rect.x}%"
		style:top="{trigger.rect.y}%"
		style:width="{trigger.rect.width}%"
		style:height="{trigger.rect.height}%"
		data-trigger-type={trigger.type}
	></div>
{/if}

<style>
	/* the outline only with "Show hints" (account page, off by default – owner 2026-10-07, BACKLOG M.1): without it the
	   area is still tappable, just not drawn */
	:global(html:not(.sc-show-hints)) .sample-trigger {
		border-color: transparent !important;
		background: none !important;
		box-shadow: none !important;
	}

	/* Audio Hotzones */
	.sample-trigger {
		position: absolute;
		border: 2px dashed rgba(255, 193, 7, 0.8);
		background: rgba(255, 193, 7, 0.1);
		pointer-events: auto;
		opacity: 0.5;
		transition: opacity 0.3s ease;
		z-index: 1;
		box-shadow: inset 0 0 20px rgba(255, 193, 7, 0.3);
		cursor: pointer;
	}

	.sample-trigger:hover {
		opacity: 0.8;
	}

	.sample-trigger:active {
		opacity: 1;
	}

	/* Flash animation when hotzone comes into view */
	@keyframes hotzoneFlash {
		0% {
			opacity: 0;
		}

		50% {
			opacity: 1;
		}

		100% {
			opacity: 0.5;
		}
	}

	.sample-trigger.flash {
		animation: hotzoneFlash 0.5s ease-in-out;
	}
</style>
