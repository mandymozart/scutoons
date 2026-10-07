<script lang="ts">
	// The Problems panel (chapter editor, meta pages): errors and warnings of the open page, notes on request. A row
	// jumps to its place; "Looks fine" accepts a warning (it moves behind "accepted"), "Ask" hands it to the Ask panel.
	// An acceptance is an edit of the document the warning is about (`accepted_warnings` in that locale's file,
	// @scutoons/content accepted.ts): the page turns unsaved, Save keeps it, undo takes it back.
	import { CloseButton, DockFold, Icon, shellDock } from '@scutoons/ui';
	import { fileLabel } from '$lib/labels.ts';
	import { askAbout } from '$lib/ask.svelte.ts';
	import type { Problem } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { acceptStore, isAccepted as accepted, toggleAccepted } from '$lib/editor/accepted.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { exprTextAt } from '$lib/editor/expr-messages.ts';

	let {
		problems,
		docs = [],
		home,
		ongoto,
		onclose
	}: {
		problems: Problem[];
		/** the page's documents: a warning about one of them can be accepted (stored in it) */
		docs?: ContentDoc[];
		/** the page's own file: its name is left out of the places (other files, e.g. a scene group, keep theirs) */
		home?: string;
		ongoto?: (problem: Problem) => void;
		onclose?: () => void;
	} = $props();

	let showInfo = $state(false);
	let showAccepted = $state(false);

	const storeOf = (p: Problem) => acceptStore(docs, p);
	const isAccepted = (p: Problem) => accepted(docs, p);

	const counts = $derived({
		error: problems.filter((p) => p.level === 'error').length,
		warning: problems.filter((p) => p.level === 'warning' && !isAccepted(p)).length,
		accepted: problems.filter(isAccepted).length,
		info: problems.filter((p) => p.level === 'info').length
	});
	const visible = $derived(problems.filter((p) => (p.level === 'info' ? showInfo : isAccepted(p) ? showAccepted : true)));

	// a path as a place: groups.0.slices.42.dialog.1.text → Group 1 · Slice 43 · Line 2 (the page's own file and a
	// line's "text" field add nothing)
	const NUMBERED: Record<string, (n: number) => string> = {
		groups: (n) => m['creator.group_n']({ number: n }),
		slices: (n) => m['creator.slice_n']({ number: n }),
		dialog: (n) => m['creator.line_n']({ number: n }),
		events: (n) => m['creator.event_n']({ number: n })
	};
	/** an expression's problem in the creator's language (the validator's message is English), with its place */
	const messageOf = (p: Problem) => (p.expr && p.code?.startsWith('expr:') ? exprTextAt({ code: p.code, ...p.expr }) : p.message);

	function placeOf(p: Problem): string {
		const parts = p.file === home ? [] : [fileLabel(p.file)];
		const path = p.path.at(-1) === 'text' ? p.path.slice(0, -1) : p.path;
		for (let i = 0; i < path.length; i++) {
			const key = path[i];
			const next = path[i + 1];
			if (typeof key === 'string' && typeof next === 'number' && NUMBERED[key]) {
				parts.push(NUMBERED[key](next + 1));
				i++;
			} else parts.push(typeof key === 'number' ? `#${key + 1}` : String(key));
		}
		if (p.locale) parts.push(p.locale);
		return parts.join(' · ') || fileLabel(p.file);
	}
</script>

<div class="problems" data-testid="problems-panel">
	<header>
		<h2>{m['creator.problems']()}</h2>
		<DockFold label={m['creator.fold_panel']()} maximizeLabel={m['creator.maximize_panel']()} restoreLabel={m['creator.restore_panel']()} />
		{#if onclose}<CloseButton label={m['creator.close']()} onclick={onclose} />{/if}
	</header>
	<p class="summary">
		<span class:bad={counts.error > 0}>{m['creator.problems_errors']({ count: counts.error })}</span> · {m['creator.problems_warnings']({ count: counts.warning })}
	</p>
	<div class="filters">
		<label><input type="checkbox" bind:checked={showAccepted} disabled={!counts.accepted} /> {m['creator.problems_accepted']({ count: counts.accepted })}</label>
		<label><input type="checkbox" bind:checked={showInfo} disabled={!counts.info} /> {m['creator.problems_notes']({ count: counts.info })}</label>
	</div>
	<ul>
		{#each visible as p, i (i)}
			{@const place = placeOf(p)}
			{@const done = isAccepted(p)}
			<li class="level-{p.level}" class:is-accepted={done}>
				<button type="button" class="goto" onclick={() => (shellDock.restore(), shellDock.foldMain(false), ongoto?.(p))} title={m['creator.problems_goto']()}>
					<span class="where">{place}</span>
					<span class="msg">{messageOf(p)}</span>
				</button>
				<span class="actions">
					{#if storeOf(p)}
						<button type="button" class="sc-round" class:is-active={done} aria-pressed={done} title={done ? m['creator.problems_unaccept']() : m['creator.problems_accept']()} aria-label={done ? m['creator.problems_unaccept']() : m['creator.problems_accept']()} onclick={() => toggleAccepted(docs, p)}><Icon name="check" /></button>
					{/if}
					{#if p.level !== 'info' && !done}
						<button type="button" class="sc-round" title={m['creator.problems_ask']()} aria-label={m['creator.problems_ask']()} onclick={() => askAbout(m['creator.problems_ask_text']({ place, message: p.message }))}><Icon name="message-square" /></button>
					{/if}
				</span>
			</li>
		{/each}
	</ul>
</div>

<style>
	header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--space-05);
	}
	h2 {
		margin: 0;
		font-size: var(--type-body-size);
		line-height: var(--type-body-line);
		font-weight: normal;
	}
	.summary {
		margin: 0 0 var(--space-05);
		color: var(--muted);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.summary .bad {
		color: var(--error);
	}
	.filters {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05) var(--space-1);
		padding-bottom: var(--space-05);
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.filters label {
		display: flex;
		align-items: center;
		gap: var(--space-05);
	}
	.hint {
		color: var(--muted);
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	li {
		display: flex;
		align-items: flex-start;
		gap: var(--space-05);
		padding: var(--space-05) 0 var(--space-05) var(--space-075);
		border-left: var(--border-width-focus) solid var(--border-muted);
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
	}
	li.level-error {
		border-left-color: var(--error);
	}
	li.level-warning {
		border-left-color: var(--warn);
	}
	li.is-accepted {
		border-left-color: var(--border-muted);
	}
	li.is-accepted .msg {
		color: var(--muted);
	}
	.goto {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		padding: 0;
		border: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.goto:hover .msg,
	.goto:focus-visible .msg {
		text-decoration: underline;
	}
	.where {
		color: var(--muted);
		overflow-wrap: anywhere;
	}
	.actions {
		display: flex;
		gap: var(--space-025);
		flex: none;
	}
</style>
