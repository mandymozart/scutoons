<script lang="ts">
	import { CloseButton, DockFold, Segmented, panelWidth, useDock } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The newsletter's preview as a dock panel (#98, agents/DOCKING.md – like the creator's Reader panel): the mail as a
	 * recipient sees it, the unsaved text of the chosen language, in the frame of every platform mail. A phone (390 CSS
	 * px) or a desktop mail window (640) – a device width, not a UI size, so px there – scaled into the panel; the frame
	 * scrolls. Folds to a rail, maximizes, ✕ closes (the page's "Vorschau" button opens it again).
	 */
	let { open, subject, html, onclose }: { open: boolean; subject: string; html: string; onclose: () => void } = $props();

	// desktop = the mail's own column (600) with a little margin – a wider window only adds white
	const DEVICES = { phone: 390, desktop: 640 } as const;
	let device = $state<keyof typeof DEVICES>('desktop');
	let boxWidth = $state(0);
	let boxHeight = $state(0);
	const width = $derived(DEVICES[device]);
	const scale = $derived(boxWidth > 0 ? Math.min(1, boxWidth / width) : 1);

	const size = panelWidth('sc-panel-width:newsletter-preview', 30, undefined, { foldable: true });
	useDock(() => (open ? { kind: 'newsletter-preview', panel, size, title: m['newsletter.preview'](), icon: 'mail', close: onclose } : null));
</script>

{#snippet panel()}
	<section class="preview" aria-label={m['newsletter.preview']()} data-testid="newsletter-preview">
		<header>
			<h2>{m['newsletter.preview']()}</h2>
			<div class="actions">
				<Segmented
					label={m['newsletter.preview_device']()}
					options={[
						{ value: 'phone', label: m['newsletter.preview_phone']() },
						{ value: 'desktop', label: m['newsletter.preview_desktop']() }
					]}
					value={device}
					onchange={(v) => (device = v)}
				/>
				<DockFold label={m['shell.fold_panel']()} maximizeLabel={m['shell.maximize_panel']()} restoreLabel={m['shell.restore_panel']()} />
				<CloseButton label={m['common.close']()} onclick={onclose} />
			</div>
		</header>
		<p class="subject" title={subject}>{subject || '—'}</p>
		<div class="fit" bind:clientWidth={boxWidth} bind:clientHeight={boxHeight}>
			<iframe
				title={m['newsletter.preview']()}
				srcdoc={html}
				sandbox=""
				style:width="{width}px"
				style:height="{scale > 0 ? boxHeight / scale : boxHeight}px"
				style:transform="scale({scale})"
			></iframe>
		</div>
	</section>
{/snippet}

<style>
	.preview {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		background: var(--paper);
	}
	header {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		flex: none;
		padding: var(--space-05) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	h2 {
		flex: 1;
		min-width: 0;
		margin: 0;
		font-size: var(--type-label-size);
		line-height: var(--type-label-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.actions {
		display: flex;
		align-items: center;
		gap: var(--space-05);
	}
	/* the subject as the inbox shows it */
	.subject {
		flex: none;
		margin: 0;
		padding: var(--space-05) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.fit {
		flex: 1;
		min-height: 0;
		overflow: hidden;
		background: #ffffff;
	}
	iframe {
		display: block;
		border: 0;
		transform-origin: top left;
		background: #ffffff;
	}
</style>
