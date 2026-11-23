<script lang="ts">
	// The reader's XR mode (agents/research/XR.md §8.3, §10.8): one scene of a published experience over the camera
	// picture. The engine loads with the page, the camera starts with the reader's tap.
	import { onMount } from 'svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { openXr, XrContentError, type XrRun } from '$lib/xr/xr-engine.ts';
	import { failureOf, viewOf, type XrFailure, type XrStatus } from '$lib/xr/xr-session.ts';
	import type { PageProps } from './$types.js';

	const { data }: PageProps = $props();

	let stage = $state<HTMLElement>();
	let run: XrRun | null = null;
	let status = $state<XrStatus>('idle');
	let opened = $state(false);
	let found = $state(0);
	let failure = $state<XrFailure>('other');
	let subtitle = $state<string | null>(null);
	/** the camera runs or was asked for: a hidden page releases it, a shown one takes it again */
	let wanted = $state(false);

	const view = $derived(viewOf(status, found, opened, wanted));
	const title = $derived(data.experience.title[getLocale()] ?? data.experience.title.en ?? Object.values(data.experience.title)[0] ?? '');

	function fail(kind: XrFailure) {
		failure = kind;
		status = 'error';
	}

	async function open(host: HTMLElement) {
		status = 'idle';
		opened = false;
		try {
			run = await openXr(host, data.experience, data.place.scene, getLocale, {
				status: (next, code) => {
					if (next === 'error') failure = failureOf(code);
					status = next;
				},
				found: (count) => (found = count),
				subtitle: (text) => (subtitle = text)
			});
			opened = true;
		} catch (e) {
			console.error('[xr]', e instanceof XrContentError ? e.problems.join('\n') : e);
			fail(e instanceof XrContentError ? 'content' : 'other');
		}
	}

	function start() {
		wanted = true;
		run?.start().catch(() => fail('camera'));
	}

	async function retry() {
		await run?.dispose().catch(() => {});
		run = null;
		if (stage) await open(stage);
		if (run && wanted) start();
	}

	function visibility() {
		if (!run || !wanted) return;
		if (document.hidden) void run.stop();
		else start();
	}

	onMount(() => {
		if (stage) void open(stage);
		document.addEventListener('visibilitychange', visibility);
		return () => {
			document.removeEventListener('visibilitychange', visibility);
			void run?.dispose();
			run = null;
		};
	});
</script>

<svelte:head><title>{title}</title></svelte:head>

<div class="xr" data-testid="xr" data-view={view}>
	<div class="stage" bind:this={stage}></div>

	{#if subtitle}<p class="subtitle">{subtitle}</p>{/if}

	{#if view !== 'found' && view !== 'paused'}
		<div class="panel" role={view === 'failed' ? 'alert' : 'status'} data-testid="xr-panel">
			{#if view === 'loading'}
				<p>{m['xr.loading']()}</p>
			{:else if view === 'ready'}
				<h1>{title}</h1>
				<p>{m['xr.hint']()}</p>
				<button type="button" onclick={start} data-testid="xr-start">{m['xr.start']()}</button>
			{:else if view === 'starting'}
				<p>{m['camera.starting']()}</p>
			{:else if view === 'scanning'}
				<p>{m['xr.scanning']()}</p>
			{:else}
				<p>{failure === 'camera' ? m['camera.denied']() : failure === 'content' ? m['xr.failed_content']() : m['xr.failed_other']()}</p>
				{#if failure !== 'content'}
					<button type="button" onclick={retry} data-testid="xr-retry">{m['xr.retry']()}</button>
				{/if}
			{/if}
		</div>
	{/if}
</div>

<style>
	.xr {
		position: fixed;
		inset: 0;
		background: var(--ink);
		color: var(--paper);
	}
	.stage {
		position: absolute;
		inset: 0;
		overflow: hidden;
	}
	.panel {
		position: absolute;
		inset: auto var(--space-1) var(--space-1);
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		max-width: 22rem;
		margin-inline: auto;
		padding: var(--space-1);
		box-sizing: border-box;
		background: var(--ink);
		border: var(--border-width-regular) solid var(--paper);
	}
	.panel h1 {
		margin: 0;
		font-size: var(--type-body-size);
	}
	.panel p {
		margin: 0;
	}
	.panel button {
		margin: 0;
		background: var(--paper);
		color: var(--ink);
		border-color: var(--paper);
	}
	.subtitle {
		position: absolute;
		inset: auto var(--space-1) 6rem;
		margin: 0;
		text-align: center;
		text-shadow: 0 0 0.25rem var(--ink);
	}
</style>
