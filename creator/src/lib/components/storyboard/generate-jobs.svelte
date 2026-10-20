<script lang="ts">
	// The current shot's running provider jobs (#121): a row each with the model, its status, the estimate in euro and the
	// house progress bar; Cancel where the provider can stop a task (Runway). The open jobs of the chapter are asked about
	// every few seconds while the page is visible – a hidden tab or nothing open stops it; a finished one goes into the
	// workspace folder (storyboard-generate.svelte.ts poll).
	import { CloseButton } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { toast } from '$lib/toasts.ts';
	import { POLL_MS, canCancel, shouldPoll } from '$lib/editor/storyboard-generate.ts';
	import { figuresTitle, money } from '$lib/editor/storyboard-money.ts';
	import type { Generation } from '$lib/editor/storyboard-generations.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

	let { sb }: { sb: StoryboardPage } = $props();
	const gen = $derived(sb.gen);
	const rate = $derived(gen.budget?.rate ?? null);

	let visible = $state(typeof document === 'undefined' || document.visibilityState === 'visible');
	const polling = $derived(shouldPoll(sb.generations, visible));
	$effect(() => {
		if (!polling) return;
		let busy = false;
		const timer = setInterval(() => {
			if (busy) return;
			busy = true;
			gen.poll()
				.then(({ saved }) => saved.forEach((g) => toast(m['storyboard.gen_saved']({ file: g.file ?? '' }), { type: 'success' })))
				.catch((e) => toast(e instanceof Error ? e.message : String(e), { type: 'error' }))
				.finally(() => (busy = false));
		}, POLL_MS);
		return () => clearInterval(timer);
	});

	async function cancel(g: Generation) {
		try {
			await gen.cancel(g);
		} catch (e) {
			toast(e instanceof Error ? e.message : String(e), { type: 'error' });
		}
	}
	const status = (g: Generation) => (g.status === 'queued' ? m['storyboard.gen_status_queued']() : m['storyboard.gen_status_running']());
</script>

<svelte:document onvisibilitychange={() => (visible = document.visibilityState === 'visible')} />

{#if gen.running.length}
	<fieldset>
		<legend>{m['storyboard.gen_running']()}</legend>
		<ul>
			{#each gen.running as g (g.id)}
				<li data-testid="job-{g.id}">
					<div class="row">
						<span title={figuresTitle(g.estimate, g.currency, rate)}>{g.model ?? g.generator} · {status(g)}{#if g.estimate !== null} · {money(g.estimate, g.currency, rate, true)}{/if}</span>
						{#if canCancel(g)}<CloseButton label={m['storyboard.gen_cancel']()} title={m['storyboard.gen_cancel_hint']()} onclick={() => cancel(g)} />{/if}
					</div>
					<progress max="1" value={g.progress ?? undefined}></progress>
				</li>
			{/each}
		</ul>
	</fieldset>
{/if}

<style>
	fieldset {
		margin: 0;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: var(--space-05);
	}
	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-05);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	/* the house bar (@scutoons/ui BulkProgress: ink on paper in an ink frame); nothing reported yet = the empty frame */
	progress {
		display: block;
		box-sizing: border-box;
		width: 100%;
		height: 1rem;
		appearance: none;
		border: var(--border-width-regular) solid var(--ink);
		border-radius: 0;
		background: var(--paper);
		color: var(--ink);
	}
	progress::-webkit-progress-bar {
		background: var(--paper);
	}
	progress::-webkit-progress-value {
		background: var(--ink);
	}
	progress::-moz-progress-bar {
		background: var(--ink);
	}
</style>
