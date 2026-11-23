<script lang="ts" module>
	import type { LogKind } from '@scutoons/content';

	/** every event type's icon: on its lines, its Type pill and in the legend */
	export const KIND_ICONS: Record<LogKind, string> = {
		logic: 'git-branch',
		save: 'save',
		publish: 'upload-cloud',
		history: 'clock',
		generation: 'zap',
		library: 'image',
		show: 'play',
		sync: 'refresh-cw',
		technical: 'terminal'
	};
</script>

<script lang="ts">
	// The Log tab (research/redesign/LOG.md, log.html A): one list – the open items' problems as standing lines on top,
	// the session's events below – with the scope pills, Show (All · Problems · Events), the event types with their icons
	// and legend, find and Copy all. The lines, filters, counts and pills are the core's (lib/log/log.svelte.ts); a
	// problem's accept and jump are its page's.
	import { Icon } from '@scutoons/ui';
	import { Button, Chip, IconButton, PillTabs } from '@scutoons/ui/editor';
	import type { LogScope, LogShow, LogShown } from '@scutoons/content';
	import { log } from '$lib/log/log.svelte.ts';
	import { toast } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';

	let scope = $state<LogScope>('item');
	let show = $state<LogShow>('all');
	let kinds = $state<LogKind[]>([]);
	let find = $state('');
	let notes = $state(false);
	let now = $state(Date.now());
	$effect(() => {
		const timer = setInterval(() => (now = Date.now()), 15_000);
		return () => clearInterval(timer);
	});

	$effect(() => void log.load());
	const pills = $derived(log.scopes());
	const legend = $derived(log.legend());
	/** the picked scope's prefix; a scope the open item lacks falls back to Item, then All */
	const prefix = $derived(pills.find((p) => p.scope === scope)?.prefix ?? pills.find((p) => p.scope === 'item')?.prefix ?? '');
	const filter = $derived({ show, kinds, scope: prefix, notes });
	const view = $derived.by(() => {
		void now;
		return log.view(filter, find, Date.now());
	});
	const typed = $derived(show !== 'problems');

	function toggleKind(kind: LogKind) {
		kinds = kinds.includes(kind) ? kinds.filter((k) => k !== kind) : [...kinds, kind];
	}
	async function copyAll() {
		await navigator.clipboard.writeText(log.text(filter, find));
		toast(m['shell.log_copied'](), { type: 'success' });
	}
	const label = (kind: LogKind) => legend?.kinds.find((k) => k.kind === kind)?.label ?? kind;
	const words = (s: LogShow) => legend?.shows.find((x) => x.show === s)?.words ?? s;
	const press = (line: LogShown) => line.jump && log.sourceOf(line.jump)?.jump(line.jump);
</script>

<div class="log" data-testid="log-panel">
	<div class="row" data-testid="log-scopes">
		<span class="lab">{m['shell.log_scope']()}</span>
		<PillTabs small label={m['shell.log_scope']()} on={scope} onselect={(id) => (scope = id as typeof scope)} items={pills.map((p) => ({ id: p.scope, label: p.words, disabled: p.prefix === null, testid: `log-scope-${p.scope}` }))} />
	</div>
	<div class="row">
		<span class="lab">{m['shell.log_show']()}</span>
		<PillTabs small label={m['shell.log_show']()} on={show} onselect={(id) => (show = id as LogShow)} items={(legend?.shows ?? []).map((s) => ({ id: s.show, label: s.words, testid: `log-show-${s.show}` }))} />
		{#if view.notes && show !== 'events'}
			<Chip label={m['shell.log_notes']({ count: view.notes })} on={notes} onclick={() => (notes = !notes)} />
		{/if}
		<input class="find" type="search" placeholder={m['shell.log_find']()} aria-label={m['shell.log_find']()} bind:value={find} />
		<Button variant="quiet" icon="copy" onclick={copyAll}>{m['shell.log_copy']()}</Button>
		{#if typed && legend}
			<!-- the types: small icon chips, their name and meaning on hover (the Info view) – no legend in the panel (RULES H14) -->
			<span class="types" role="group" aria-label={m['shell.log_type']()} data-testid="log-types">
				{#each legend.kinds as k (k.kind)}
					<Chip label="" icon={KIND_ICONS[k.kind]} on={kinds.includes(k.kind)} onclick={() => toggleKind(k.kind)} title="{k.label}: {k.legend}" testid="log-type-{k.kind}" />
				{/each}
			</span>
		{/if}
	</div>
	<div class="lines">
		{#if show !== 'events' && view.problems.length}
			<p class="group"><span>{words('problems')}</span><span>{view.problems.length}</span></p>
			{#each view.problems as line (line.key)}
				{@const source = line.key ? log.sourceOf(line.key) : undefined}
				<div class="line {line.level}" class:accepted={line.accepted} data-testid="log-problem">
					<span class="dot" aria-hidden="true"></span>
					<button type="button" class="body" onclick={() => press(line)} title={m['shell.log_jump']()}>
						<span class="where"><span class="lvl">{line.level === 'ok' ? '' : line.level}</span> {line.source}</span>
						<span class="text">{line.text}</span>
					</button>
					{#if line.key && source?.acceptable(line.key)}
						<IconButton label={line.accepted ? m['creator.problems_unaccept']() : m['creator.problems_accept']()} icon="check" pressed={line.accepted} onclick={() => line.key && source.accept(line.key)} />
					{/if}
				</div>
			{/each}
		{/if}
		{#if show !== 'problems' && view.events.length}
			<p class="group" data-testid="log-group-events"><span>{words('events')}</span><span>{view.events.length}{kinds.length ? ` · ${kinds.map(label).join(', ').toLowerCase()}` : ''}</span></p>
			{#each view.events as line, i (i)}
				{@const kind = line.kind}
				<div class="line {line.level}" data-testid="log-event">
					<span class="dot" aria-hidden="true"></span>
					{#snippet said()}
						<span class="where">{#if kind}<span class="kind" title={label(kind)}><Icon name={KIND_ICONS[kind]} size="0.95em" strokeWidth={2} /></span>{/if}<span class="lvl">{line.level === 'ok' || line.level === 'info' ? '' : line.level}</span> {line.source}{line.ref ? ` · ${line.ref}` : ''}</span>
						<span class="text">{line.text}{line.times > 1 ? ` ×${line.times}` : ''}</span>
					{/snippet}
					{#if line.jump}
						<button type="button" class="body" onclick={() => press(line)} title={m['shell.log_jump']()}>{@render said()}</button>
					{:else}
						<div class="body">{@render said()}</div>
					{/if}
					<span class="age" title={line.clock}>{line.age}</span>
				</div>
			{/each}
		{/if}
		{#if !(show !== 'events' && view.problems.length) && !(show !== 'problems' && view.events.length)}
			<p class="sc-empty">{m['shell.log_empty']()}</p>
		{/if}
	</div>
</div>

<style>
	.log {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		font-size: var(--editor-small);
	}
	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.25rem;
		padding: 0.15rem var(--editor-pad) 0.35rem;
	}
	.lab,
	.group {
		margin: 0 0.3rem 0 0;
		color: var(--tool-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	.find {
		width: 8rem;
		margin-left: auto;
		font: inherit;
	}
	.lines {
		flex: 1;
		min-height: 0;
		overflow: auto;
		padding: 0 var(--editor-pad) var(--editor-pad);
	}
	.group {
		display: flex;
		gap: 0.5rem;
		padding: 0.6rem 0 0.25rem;
	}
	.line {
		display: grid;
		grid-template-columns: 0.6rem 1fr auto;
		align-items: start;
		gap: 0.5rem;
		padding: 0.3rem 0.2rem;
		border-bottom: var(--editor-edge) solid var(--tool-line);
	}
	.line:hover {
		background: var(--tool-hover);
	}
	.dot {
		width: 0.5rem;
		height: 0.5rem;
		margin-top: 0.4em;
		border-radius: 50%;
		background: var(--kit-status-none-mark);
	}
	.error .dot {
		background: var(--kit-status-error-mark);
	}
	.warn .dot {
		background: var(--kit-status-warning-mark);
	}
	.ok .dot {
		background: var(--kit-status-ok-mark);
	}
	.info .dot {
		background: var(--kit-status-info-mark);
	}
	.body {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		min-width: 0;
		min-height: 0; /* control-ok: a line of the list, not a control row */
		padding: 0;
		border: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-align: left;
		text-shadow: inherit;
	}
	button.body:hover .text {
		text-decoration: underline;
	}
	.where,
	.age {
		color: var(--tool-muted);
		font-size: var(--editor-label);
		white-space: nowrap;
	}
	.where {
		overflow: hidden;
		max-width: 100%;
		text-overflow: ellipsis;
	}
	.lvl {
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	.error .lvl {
		color: var(--tool-stop);
	}
	.warn .lvl {
		color: var(--tool-warn);
	}
	.kind {
		margin-right: 0.35rem;
		color: var(--tool-text);
	}
	.text {
		overflow-wrap: anywhere;
	}
	.accepted {
		opacity: 0.5;
	}
	.types {
		display: inline-flex;
		gap: 0.125rem;
	}
</style>
