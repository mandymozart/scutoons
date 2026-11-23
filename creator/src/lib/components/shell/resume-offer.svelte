<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { Button, IconButton, Pill } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { whenLabel } from '$lib/shell/devices.ts';
	import { devices } from '$lib/shell/devices.svelte.ts';

	/** Another device's newer state, offered once and quietly above the status bar: Continue takes it, × keeps this device's */
	const offer = $derived(devices.offer);
	const when = $derived(offer?.at ? whenLabel(offer.at, new Date(), getLocale()) : '');
</script>

{#if offer}
	<Pill class="resume-offer" role="status" data-testid="resume-offer">
		<span class="words"><Icon name="monitor" size="1.1rem" strokeWidth={1.5} />{m['shell.resume_other']({ device: offer.name || '—' })}{#if when}<span class="when"> · {when}</span>{/if}</span>
		<Button variant="primary" onclick={() => devices.accept()} data-testid="resume-offer-continue">{m['shell.resume_other_continue']()}</Button>
		<IconButton icon="x" label={m['shell.resume_other_dismiss']()} onclick={() => devices.dismiss()} data-testid="resume-offer-dismiss" />
	</Pill>
{/if}

<style>
	/* centred above the status bar, over the panels and the dashboard; it waits, nothing else waits on it */
	:global(.kit-pill.resume-offer) {
		position: fixed;
		z-index: 55;
		left: 50%;
		bottom: calc(var(--kit-margin) * 2 + var(--kit-bar));
		max-width: calc(100vw - var(--kit-margin) * 2);
		transform: translateX(-50%);
		padding-left: var(--editor-pad);
		animation: resume-offer-in var(--motion-base) var(--motion-ease);
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
	.when {
		color: var(--tool-muted);
	}
	@keyframes resume-offer-in {
		from {
			opacity: 0;
			transform: translate(-50%, 0.5rem);
		}
	}
</style>
