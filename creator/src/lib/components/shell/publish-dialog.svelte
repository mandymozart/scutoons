<script lang="ts">
	import { CloseIcon } from '@scutoons/ui';
	import { Button, Chip, IconButton, Overlay } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { publishFlow as flow } from '$lib/editor/publish-flow.svelte.ts';
	import PublishLine from '$lib/components/shell/publish-line.svelte';

	/**
	 * The one Publish dialog (lap 2 A10, RULES H18) on the overlay level: what readers do not see yet as a ticked list,
	 * each line with what its draft does and its kind (the rest is its hover text); All · None; one primary button with the
	 * count; the confirm where one is needed; the note when something goes to review; then the report in the same box.
	 */
	const title = m['creator.publish_tree_title_all']();
	const primary = $derived(flow.kind === 'submit' ? m['creator.publish_tree_submit']({ count: flow.count }) : m['creator.publish_tree_publish']({ count: flow.count }));
	const listing = $derived(flow.step === 'list' || flow.step === 'sending');
	const sending = $derived(flow.step === 'sending');
	const authors = $derived([...new Set(flow.decided.map((l) => l.type))].join(' · '));
</script>

<Overlay place="center" label={title} onclose={() => flow.close()} data-testid="publish-dialog">
	<section class="box" aria-busy={sending || flow.step === 'loading'}>
		<header class="head">
			<h2>{title}</h2>
			<IconButton label={m['creator.close']()} disabled={sending} onclick={() => flow.close()} data-testid="publish-close"><CloseIcon size="0.8rem" stroke={1} /></IconButton>
		</header>

		<div class="body">
			{#if flow.step === 'loading'}
				<p class="quiet">{m['shell.publish_loading']()}</p>
			{:else if listing}
				<div class="choose">
					<span class="count" data-testid="publish-count">{m['shell.publish_chosen']({ count: flow.count, total: flow.lines.length })}</span>
					<Button variant="quiet" disabled={sending} onclick={() => flow.all()} data-testid="publish-all">{m['creator.publish_tree_all']()}</Button>
					<Button variant="quiet" disabled={sending} onclick={() => flow.none()} data-testid="publish-none">{m['creator.publish_tree_none']()}</Button>
				</div>
				<ul class="lines">
					{#each flow.lines as line (line.id)}
						{@const on = flow.ticked.has(line.id)}
						<li>
							<PublishLine {line} {on} disabled={sending} title={flow.held.get(line.id) ?? line.hint} ontick={(next) => flow.tick(line.id, next)} />
						</li>
					{/each}
				</ul>
			{:else if flow.step === 'confirm'}
				<div class="ask" data-testid="publish-confirm">
					<p>{m['shell.publish_confirm']({ count: flow.count })}</p>
					{#if flow.errors}<p class="warn">{m['creator.publish_errors']({ count: flow.errors })}</p>{/if}
					{#if flow.decided.length}
						<label class="field">
							<span>{m['creator.review_note_label']({ author: authors })}</span>
							<textarea bind:value={flow.decisionNote} rows="3" data-testid="publish-decision-note"></textarea>
						</label>
					{/if}
				</div>
			{:else if flow.step === 'note'}
				<div class="ask" data-testid="publish-note">
					<p>{m['creator.submit_title']()}</p>
					<label class="field">
						<span>{m['creator.submit_note_label']()}</span>
						<!-- svelte-ignore a11y_autofocus -->
						<textarea bind:value={flow.note} rows="3" placeholder={m['creator.submit_note_placeholder']()} autofocus></textarea>
					</label>
				</div>
			{:else}
				<ul class="report" data-testid="publish-report">
					{#each flow.report as n, i (i)}
						<li title={n.detail}>
							{#if n.type === 'error'}<Chip tone="error" label={m['shell.publish_refused']()} />{/if}
							<span>{n.text}</span>
						</li>
					{/each}
				</ul>
			{/if}
		</div>

		<footer class="foot">
			{#if listing}
				<Button disabled={sending} onclick={() => flow.close()} data-testid="publish-cancel">{m['creator.cancel']()}</Button>
				<Button variant="primary" disabled={sending || !flow.count} onclick={() => flow.go()} data-testid="publish-go">{primary}</Button>
			{:else if flow.step === 'confirm'}
				<Button onclick={() => flow.back()} data-testid="publish-back">{m['shell.publish_back']()}</Button>
				<Button variant="primary" onclick={() => flow.go()} data-testid="publish-go">{primary}</Button>
			{:else if flow.step === 'note'}
				<Button onclick={() => flow.answer(false)}>{m['creator.cancel']()}</Button>
				<Button variant="primary" onclick={() => flow.answer(true)} data-testid="publish-submit">{m['creator.submit_send']()}</Button>
			{:else}
				<span></span>
				<Button variant="primary" disabled={flow.step === 'loading'} onclick={() => flow.close()} data-testid="publish-done">{m['creator.close']()}</Button>
			{/if}
		</footer>
	</section>
</Overlay>

<style>
	/* a box of the overlay level (kit/tokens.toml [component.overlay.box]): see-through, its line, the halo on its words */
	.box {
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		width: min(32.5rem, calc(100vw - 2 * var(--kit-margin)));
		height: min(28rem, calc(100vh - 2 * var(--kit-margin)));
		border: var(--editor-edge) solid var(--kit-overlay-box-border);
		border-radius: var(--kit-radius-box);
		background: var(--kit-overlay-box-ground);
		text-shadow: var(--tool-halo);
	}
	.head {
		display: flex;
		align-items: center;
		gap: var(--kit-gap);
		min-height: var(--kit-bar);
		padding: 0 var(--kit-inset) 0 var(--editor-pad);
	}
	/* the box's name, as a panel's tab says it (not the tools' label heading) */
	.head h2 {
		flex: 1;
		margin: 0;
		font-size: inherit;
		font-weight: inherit;
		letter-spacing: normal;
		text-transform: none;
	}
	.body {
		display: flex;
		flex: 1;
		flex-direction: column;
		min-height: 0;
		padding: 0 var(--editor-pad);
	}
	.choose {
		display: flex;
		align-items: center;
		gap: var(--kit-seam);
		padding-bottom: calc(var(--editor-pad) / 2);
	}
	.count {
		flex: 1;
		color: var(--tool-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	.lines,
	.report {
		flex: 1;
		min-height: 0;
		margin: 0;
		padding: 0;
		overflow: auto;
		list-style: none;
	}
	.report li {
		display: flex;
		align-items: baseline;
		gap: var(--kit-gap);
		padding: calc(var(--editor-pad) / 2) 0;
		overflow-wrap: anywhere;
	}
	.ask {
		display: flex;
		flex-direction: column;
		gap: var(--editor-gap);
	}
	.ask p {
		margin: 0;
	}
	.warn {
		color: var(--tool-warn);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: calc(var(--editor-pad) / 2);
		color: var(--tool-muted);
	}
	.field textarea {
		box-sizing: border-box;
		width: 100%;
		padding: calc(var(--editor-pad) / 2) var(--editor-pad);
		border: var(--editor-edge) solid var(--kit-field-rest-border);
		border-radius: var(--editor-radius);
		background: var(--kit-field-rest-ground);
		color: var(--tool-text);
		font: inherit;
		resize: vertical;
	}
	.quiet {
		margin: 0;
		color: var(--tool-muted);
	}
	.foot {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--kit-gap);
		padding: var(--editor-pad);
	}
</style>
