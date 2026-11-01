<script lang="ts">
	// The conductor opens a vote (#231): the question and the options by language, the rule, the time, the fallback, who
	// votes. Until the show's choice events open votes by themselves (LIVE.md B.5), the conductor types it. One primary
	// action: Open vote.
	import { Button } from '@scutoons/ui';
	import type { VoteOpen } from '@scutoons/live';
	import { buildOpen, type VoteForm } from '$lib/live-vote.ts';

	let { onopen, next }: { onopen: (p: VoteOpen) => void; next: string } = $props();

	let f = $state<VoteForm>({
		id: '',
		question: { en: '', de: '' },
		options: { en: 'Yes\nNo', de: '' },
		rule: 'majority',
		n: 20,
		seconds: 30,
		fallback: 0,
		groups: '',
		show: true
	});
	let error = $state('');
	const RULES = [
		['majority', 'Majority'],
		['groups', 'Per group'],
		['first', 'First to n']
	] as const;
	const count = $derived(f.options.en.split('\n').filter((l) => l.trim()).length);

	function submit(e: Event) {
		e.preventDefault();
		const r = buildOpen({ ...f, id: f.id || next });
		if ('error' in r) error = r.error;
		else (error = ''), onopen(r.open);
	}
</script>

<form class="open" onsubmit={submit}>
	<h2 class="label">Open a vote</h2>
	<label>Question (English)<input type="text" bind:value={f.question.en} /></label>
	<label>Question (German)<input type="text" bind:value={f.question.de} /></label>
	<div class="two">
		<label>Options, English – one per line<textarea rows="3" bind:value={f.options.en}></textarea></label>
		<label>Options, German – same order<textarea rows="3" bind:value={f.options.de}></textarea></label>
	</div>
	<fieldset class="rule" aria-label="Rule">
		<legend class="label">Rule</legend>
		<div class="seg">
			{#each RULES as [r, label] (r)}
				<button type="button" aria-pressed={f.rule === r} onclick={() => (f.rule = r)}>{label}</button>
			{/each}
		</div>
	</fieldset>
	<div class="row">
		{#if f.rule === 'first'}<label>n<input type="number" min="1" bind:value={f.n} /></label>{/if}
		<label>Seconds<input type="number" min="1" bind:value={f.seconds} /></label>
		<label>Fallback (option no.)<input type="number" min="1" max={count} value={f.fallback + 1} oninput={(e) => (f.fallback = Number(e.currentTarget.value) - 1)} /></label>
		<label>Groups (comma; empty = all)<input type="text" bind:value={f.groups} /></label>
	</div>
	<label class="check"><input type="checkbox" bind:checked={f.show} /> A voter sees the counts after voting</label>
	{#if error}<p class="error" role="alert">{error}</p>{/if}
	<div class="actions"><Button type="submit" variant="primary" size="sm">Open vote</Button></div>
</form>

<style>
	.open {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		padding: var(--space-15);
		border: var(--border-width-regular) solid var(--ink);
		font-family: var(--font-mono);
	}
	.label,
	label {
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		font-size: var(--type-caption-size);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	input[type='text'],
	input[type='number'],
	textarea {
		box-sizing: border-box;
		width: 100%;
		padding: 0 var(--space-05);
		border: var(--border-width-hairline) solid var(--border-muted);
		border-radius: var(--radius-none);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
		font-size: var(--type-body-size);
		text-transform: none;
		letter-spacing: normal;
	}
	input[type='text'],
	input[type='number'] {
		height: var(--control-height);
	}
	textarea {
		padding: var(--space-05);
	}
	input:focus,
	textarea:focus {
		border-color: var(--ink);
	}
	.two,
	.row {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
		gap: var(--space-1);
	}
	fieldset {
		border: 0;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
	}
	.seg {
		display: inline-flex;
		align-self: flex-start;
		border: var(--border-width-regular) solid var(--ink);
	}
	.seg button {
		box-sizing: border-box;
		min-height: var(--control-height);
		padding: 0 var(--space-1);
		background: var(--paper);
		color: var(--ink);
		border: 0;
		border-radius: var(--radius-none);
		font: inherit;
		line-height: var(--type-button-line);
		cursor: pointer;
	}
	.seg button + button {
		border-left: var(--border-width-regular) solid var(--ink);
	}
	.seg button[aria-pressed='true'] {
		background: var(--ink);
		color: var(--paper);
	}
	.check {
		flex-direction: row;
		align-items: center;
		gap: var(--space-05);
		text-transform: none;
		letter-spacing: normal;
		font-size: var(--type-body-size);
	}
	.check input {
		width: 1.25rem;
		height: 1.25rem; /* control-ok: a checkbox is exempt (DESIGN "One control height") */
		accent-color: var(--ink);
	}
	.error {
		margin: 0;
		padding-left: var(--space-05);
		border-left: 0.5rem solid var(--state-error);
	}
	.actions {
		display: flex;
		justify-content: flex-end;
	}
</style>
