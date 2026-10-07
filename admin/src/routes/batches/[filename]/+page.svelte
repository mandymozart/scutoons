<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import { onDestroy, untrack } from 'svelte';
	import { beforeNavigate, goto, replaceState } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, Badge, Button, confirmDialog, Skeleton } from '@scutoons/ui';
	import { admin } from '$lib/api.ts';
	import { downloadPrintSheet, downloadZip } from '$lib/batch-actions.ts';
	import { errorText, number } from '$lib/format.ts';
	import { BatchRun, PIPELINE, type PipelineStep } from '$lib/qr/batch.svelte.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';

	/**
	 * One batch: the manifest's codes with the pipeline steps (sign → db → svg → pdf → sig), run / pause / abort,
	 * resumable after a reload (completed steps are stored in the manifest after every code), ZIP and print sheet.
	 */
	const toaster = getToaster();
	const filename = $derived(page.params.filename ?? '');

	let run = $state<BatchRun | null>(null);
	let error = $state<string | null>(null);
	let busy = $state<string | null>(null);

	$effect(() => {
		const current = filename;
		const autorun = untrack(() => page.url.searchParams.get('run') === '1');
		run = null;
		error = null;
		admin
			.getBatch(current)
			.then(({ batch }) => {
				run = new BatchRun(batch);
				if (autorun) {
					// a reload must not start it again by itself
					const url = new URL(untrack(() => page.url));
					url.searchParams.delete('run');
					replaceState(url, {});
					if (!untrack(() => run!.complete)) untrack(start);
				}
			})
			.catch((e) => (error = errorText(e)));
	});

	function start() {
		run?.run().then(() => {
			if (run?.state === 'complete') toaster.addToast({ message: m['batches.done_toast']({ count: run.done }), type: 'success' });
		});
	}

	// a running batch: leaving asks in-page (beforeNavigate can't wait, so the navigation is held and repeated)
	let leaving = false;
	beforeNavigate((navigation) => {
		if (leaving || !(run?.state === 'running' || run?.state === 'paused')) return;
		navigation.cancel();
		const to = navigation.to?.url;
		if (navigation.type === 'leave' || !to) return;
		confirmDialog({ title: m['batches.leave_title'](), message: m['batches.leave_confirm'](), confirmLabel: m['batches.leave_label']() }).then((ok) => {
			if (!ok) return;
			run?.abort();
			leaving = true;
			goto(to).finally(() => (leaving = false));
		});
	});
	onDestroy(() => run?.abort());

	const STEP_LABELS: Record<PipelineStep, () => string> = {
		sign: () => m['batches.step_sign'](),
		db: () => m['batches.step_db'](),
		svg: () => m['batches.step_svg'](),
		pdf: () => m['batches.step_pdf'](),
		sig: () => m['batches.step_sig']()
	};

	async function action(key: string, fn: () => Promise<void>) {
		busy = key;
		try {
			await fn();
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		} finally {
			busy = null;
		}
	}

	let rowEls: Record<number, HTMLTableRowElement> = {};
	$effect(() => {
		const n = run?.current;
		if (n != null) rowEls[n]?.scrollIntoView({ block: 'nearest' });
	});
</script>

{#if error}
	<PageHeader type={m['batches.type_label']()} title={filename} />
	<p class="error-box" role="alert">{error}</p>
{:else if !run}
	<PageHeader type={m['batches.type_label']()} title={filename} />
	<Skeleton variant="table" columns={8} label={m['shell.loading']()} />
{:else}
	{@const b = run.manifest}
	<PageHeader type={m['batches.type_label']()} title={b.edition} meta={b.filename} />

	<ActionBar label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={b.edition}>
		{#snippet info()}<PageHelp page="batch" />{/snippet}
		{#snippet start()}
			<a href="/batches">{m['batches.back']()}</a>
			<span class="bar-summary">{m['batches.summary']({ done: number(run!.done), total: number(b.total), errors: number(run!.failed) })}</span>
		{/snippet}
		{#snippet secondary()}
			{#if run!.done > 0}
				<Button size="sm" variant="secondary" disabled={busy === 'zip'} onclick={() => action('zip', () => downloadZip(b.filename))}>{m['batches.zip']()}</Button>
				<Button size="sm" variant="secondary" disabled={busy === 'print'} onclick={() => action('print', () => downloadPrintSheet(b.filename))}>
					{busy === 'print' ? m['batches.generating']() : m['batches.print_sheet']()}
				</Button>
			{/if}
		{/snippet}
		{#snippet end()}
			{#if run!.state === 'running' || run!.state === 'paused'}
				<Button size="sm" variant="secondary" onclick={() => run?.abort()}>{m['batches.abort']()}</Button>
				<Button size="sm" onclick={() => run?.pause()}>{run!.state === 'paused' ? m['batches.continue']() : m['batches.pause']()}</Button>
			{:else if !run!.complete}
				<Button size="sm" onclick={start}>{run!.done > 0 || run!.failed > 0 ? m['batches.resume']() : m['batches.start']()}</Button>
			{/if}
		{/snippet}
	</ActionBar>

	<div class="controls">
		<div class="progress" role="progressbar" aria-valuemin="0" aria-valuemax={b.total} aria-valuenow={run.done} aria-label={m['batches.col_progress']()}>
			<span class="fill" style:width="{b.total ? (run.done / b.total) * 100 : 0}%"></span>
		</div>
		<p class="summary" aria-live="polite">
			{m['batches.summary']({ done: number(run.done), total: number(b.total), errors: number(run.failed) })}
			{#if run.state === 'complete' || run.complete}
				<Badge tone="ink">{m['batches.complete']()}</Badge>
			{:else if run.state === 'paused'}
				<Badge tone="warning">{m['batches.paused']()}</Badge>
			{:else if run.state === 'stopped'}
				<Badge tone="warning">{m['batches.stopped']()}</Badge>
			{:else if run.state === 'running'}
				<Badge tone="accent">{m['batches.running']()}</Badge>
			{/if}
		</p>
	</div>

	<div class="table-scroll">
		<table class="progress-table">
			<thead>
				<tr>
					<th scope="col">#</th>
					<th scope="col">{m['codes.col_code']()}</th>
					{#each PIPELINE as step (step)}
						<th scope="col" class="step">{STEP_LABELS[step]()}</th>
					{/each}
					<th scope="col">{m['codes.col_status']()}</th>
				</tr>
			</thead>
			<tbody>
				{#each b.codes as item (item.n)}
					<tr bind:this={rowEls[item.n]} class:current={run.current === item.n}>
						<td>{item.n}</td>
						<td>
							{#if item.status === 'done' || item.completedSteps.includes('db')}
								<a class="mono-code" href="/codes/{encodeURIComponent(item.code)}">{item.code}</a>
							{:else}
								<span class="mono-code">{item.code}</span>
							{/if}
						</td>
						{#each PIPELINE as step (step)}
							{@const ok = run.stepDone(item, step)}
							<td class="step" class:ok>
								<span aria-hidden="true">{ok ? '✓' : '—'}</span><span class="sr-only">{ok ? m['common.yes']() : m['common.no']()}</span>
							</td>
						{/each}
						<td>
							{#if item.status === 'done'}<Badge tone="ink">{m['batches.status_done']()}</Badge>
							{:else if item.status === 'error'}<Badge tone="error">{m['batches.status_error']()}</Badge>
								{#if run.errors[item.n]}<span class="err muted">{run.errors[item.n]}</span>{/if}
							{:else if run.current === item.n}<Badge tone="accent">{m['batches.running']()}</Badge>
							{:else}<span class="muted">{m['batches.status_pending']()}</span>{/if}
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
{/if}

<style>
	.controls {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-075) var(--space-15);
		margin-bottom: var(--space-1);
	}
	.progress {
		flex: 1 1 16rem;
		height: 1rem;
		border: var(--border-width-regular) solid var(--ink);
	}
	.fill {
		display: block;
		height: 100%;
		background: var(--primary-cyan);
		transition: width var(--duration-base) var(--easing);
	}
	.summary {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		margin: 0;
		font-variant-numeric: tabular-nums;
	}
	.bar-summary {
		white-space: nowrap;
		font-variant-numeric: tabular-nums;
	}
	.table-scroll {
		overflow: auto;
		max-height: 70dvh;
		border: var(--border-width-regular) solid var(--ink);
		margin-bottom: var(--space-15);
	}
	.progress-table {
		width: 100%;
		border-collapse: collapse;
		font-variant-numeric: tabular-nums;
	}
	.progress-table th,
	.progress-table td {
		padding: var(--space-025) var(--space-075);
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
		text-align: left;
	}
	.progress-table thead th {
		position: sticky;
		top: 0;
		background: var(--paper);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	.step {
		text-align: center !important;
		width: 3.5rem;
	}
	.step.ok {
		font-weight: var(--font-weight-regular);
	}
	/* the row in progress: not a selection (DESIGN.md "Selection") – a cyan tint + its status badge, not inverted */
	tr.current {
		background: var(--surface-tint-cyan);
	}
	.err {
		display: block;
		font-size: var(--type-caption-size);
	}
	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
	}
</style>
