<script lang="ts">
	import '../hint-areas.css';
	import { fireflies } from '../fireflies.ts';
	import type { SampleTrigger } from '@scutoons/content';

	/**
	 * A sample's click / hover area (the outline with "Show hints"). The slice plays it (`sliceAudio`, the old way) –
	 * or, with `onplay`, the area itself: it is a row of the slice's layer stack (the author ordered it, BACKLOG 10.54)
	 * and takes the pointer at its place there, so what lies in front of it covers it.
	 */
	let { trigger, onplay }: { trigger?: SampleTrigger; onplay?: () => void } = $props();
</script>

{#if trigger?.rect && (trigger.type === 'click' || trigger.type === 'hover')}
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<div
		class="sample-trigger sc-hint-area"
		data-hint="sound"
		{@attach fireflies()}
		style:left="{trigger.rect.x}%"
		style:top="{trigger.rect.y}%"
		style:width="{trigger.rect.width}%"
		style:height="{trigger.rect.height}%"
		data-trigger-type={trigger.type}
		class:own={!!onplay}
		onclick={onplay && trigger.type === 'click' ? onplay : undefined}
		onmouseenter={onplay && trigger.type === 'hover' ? onplay : undefined}
	></div>
{/if}

<style>
	/* a sound area – drawn only with "Show hints" (account page, off by default – BACKLOG M.1): corner marks + a speaker
	   (hint-areas.css) */
	.sample-trigger {
		position: absolute;
		pointer-events: auto;
		z-index: 1;
		cursor: pointer;
	}

	/* in the layer stack: at its place there (no z-index of its own) */
	.sample-trigger.own {
		z-index: auto;
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
