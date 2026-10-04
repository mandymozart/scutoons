<script lang="ts">
	// The "Ask" panel (CUTOVER 10.28 step 2): a chat with an agent that has the signed-in creator's rights (the SCUTOONS
	// tools, tools/mcp). Docked on the right like the editor's side panels (drags wider up to half the window; the page
	// makes room), full screen on phones. The agent's answer streams in; each tool call is a step, a saved draft links to
	// its page. Chats are saved by the Ask service in the API (`/api/agent-chats`, owner 2026-10-04: "I need to see what I
	// did before if something stops or breaks"): the clock button lists the creator's earlier chats, one opens to read or
	// go on; the open chat is remembered per browser and comes back after a reload. The context sent with every message:
	// the page, the header's text language and the app language.
	import { onMount } from 'svelte';
	import { Icon, CloseButton } from '@scutoons/ui';
	import { getSessionToken, type AgentChatSummary, type AgentTurn } from '@scutoons/api';
	import { parseEntityKey } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { panelWidth } from '$lib/editor/panel-width.svelte.ts';
	import { textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { agentChatsApi } from '$lib/api.ts';
	import { ASK_URL } from '$lib/config.ts';

	let { context, onclose }: { context: string; onclose: () => void } = $props();

	const size = panelWidth('sc-panel-width:ask', 24);

	// the page makes room for the panel (owner 2026-10-04: "the chat pushes the page"): from 768px the shell's frame
	// ends at the panel's edge, so the editor stays usable next to the chat
	$effect(() => {
		const root = document.documentElement;
		root.classList.add('sc-ask-open');
		root.style.setProperty('--ask-width', size.css);
		return () => {
			root.classList.remove('sc-ask-open');
			root.style.removeProperty('--ask-width');
		};
	});

	type Turn = AgentTurn;
	const CURRENT = 'scutoons_creator_ask_chat';

	let turns = $state<Turn[]>([]);
	let chat = $state<string | null>(null);
	let view = $state<'chat' | 'history'>('chat');
	let history = $state<AgentChatSummary[] | null>(null);
	let historyError = $state('');

	function remember(id: string | null) {
		chat = id;
		try {
			if (id) localStorage.setItem(CURRENT, id);
			else localStorage.removeItem(CURRENT);
		} catch {
			/* storage blocked: the chat is still in the history */
		}
	}

	async function openChat(id: string) {
		try {
			const { chat: saved } = await agentChatsApi.get(id);
			turns = saved.turns ?? [];
			remember(saved.id);
			view = 'chat';
			scrollDown();
		} catch {
			remember(null);
		}
	}

	async function showHistory() {
		if (view === 'history') return void (view = 'chat');
		view = 'history';
		historyError = '';
		history = null;
		try {
			history = (await agentChatsApi.list()).chats;
		} catch {
			historyError = m['creator.ask_history_unavailable']();
		}
	}
	const when = (iso: string) => {
		const d = new Date(iso.endsWith('Z') || iso.includes('+') ? iso : `${iso}Z`);
		return Number.isNaN(d.getTime()) ? iso : new Intl.DateTimeFormat(getLocale(), { dateStyle: 'medium', timeStyle: 'short' }).format(d);
	};

	// after a reload / a break: the chat that was open comes back
	onMount(() => {
		let id: string | null = null;
		try {
			id = localStorage.getItem(CURRENT);
		} catch {
			/* no storage */
		}
		if (id) openChat(id);
	});
	let input = $state('');
	let field = $state<HTMLTextAreaElement>();

	// the field grows with its text (one line when empty, up to half the window, then it scrolls) and is one line again
	// once the text is sent; sizes in rem like everywhere in the app
	$effect(() => {
		void input;
		if (!field) return;
		field.style.height = 'auto';
		const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
		field.style.height = `${(field.scrollHeight + field.offsetHeight - field.clientHeight) / rem}rem`;
	});
	let busy = $state(false);
	let controller: AbortController | null = null;
	let list = $state<HTMLElement>();

	/** the creator page of a saved document */
	function hrefOf(key: string): string | null {
		const ref = parseEntityKey(key);
		if (!ref?.toon) return null;
		switch (ref.kind) {
			case 'chapter':
				return `/${ref.toon}/chapter/${ref.number}`;
			case 'scene':
				return `/${ref.toon}/chapter/${ref.number}?group=${encodeURIComponent(ref.scene ?? '')}`;
			case 'season':
				return `/${ref.toon}/season/${ref.number}`;
			case 'script':
			case 'cast':
			case 'credits':
				return `/${ref.toon}/${ref.kind}`;
			case 'toon':
			case 'toon-settings':
				return `/${ref.toon}/toon`;
			default:
				return null;
		}
	}
	const keyLabel = (key: string) => key.split(':').slice(1).join(' · ');

	function scrollDown() {
		queueMicrotask(() => list?.scrollTo({ top: list.scrollHeight }));
	}

	function newChat() {
		controller?.abort();
		turns = [];
		remember(null);
		view = 'chat';
	}

	/** one Server-Sent Event of the service ("event: <name>\ndata: <json>") */
	function apply(turn: Turn, block: string) {
		const name = /^event: (.*)$/m.exec(block)?.[1];
		const raw = /^data: (.*)$/m.exec(block)?.[1];
		if (!name || raw === undefined) return;
		const data = JSON.parse(raw);
		if (name === 'chat') remember(data.id);
		else if (name === 'text') turn.text += data.delta;
		else if (name === 'step') turn.steps.push(data);
		else if (name === 'error') turn.error = data.message;
	}

	async function send(event?: Event) {
		event?.preventDefault();
		const text = input.trim();
		if (!text || busy) return;
		input = '';
		turns.push({ who: 'you', text, steps: [] });
		turns.push({ who: 'agent', text: '', steps: [] });
		const turn = turns[turns.length - 1];
		scrollDown();
		if (!ASK_URL) {
			turn.error = m['creator.ask_unavailable']();
			return;
		}
		busy = true;
		controller = new AbortController();
		try {
			const token = getSessionToken();
			const res = await fetch(ASK_URL, {
				method: 'POST',
				credentials: 'include',
				headers: { 'Content-Type': 'application/json', ...(token ? { Authorization: `Bearer ${token}` } : {}) },
				body: JSON.stringify({ chat, text, context, textLanguage: textLanguage.value, appLanguage: getLocale() }),
				signal: controller.signal
			});
			if (!res.ok || !res.body) throw new Error(res.status === 401 ? m['creator.ask_signed_out']() : m['creator.ask_unavailable']());
			const reader = res.body.pipeThrough(new TextDecoderStream()).getReader();
			let buffer = '';
			for (;;) {
				const { value, done } = await reader.read();
				if (done) break;
				buffer += value;
				let cut: number;
				while ((cut = buffer.indexOf('\n\n')) >= 0) {
					apply(turn, buffer.slice(0, cut));
					buffer = buffer.slice(cut + 2);
					scrollDown();
				}
			}
		} catch (error) {
			if (!(error instanceof DOMException && error.name === 'AbortError')) turn.error = error instanceof Error ? error.message : m['creator.ask_unavailable']();
		} finally {
			busy = false;
			controller = null;
		}
	}

	function onkeydown(event: KeyboardEvent) {
		// Enter sends, Shift+Enter is a new line
		if (event.key === 'Enter' && !event.shiftKey && !event.isComposing) send(event);
	}
</script>

<!-- a focusable separator is a widget (WAI-ARIA window splitter) -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div class="resize" role="separator" aria-orientation="vertical" aria-label={m['creator.resize_panel']()} title={m['creator.resize_panel']()} tabindex="0" style:--panel-width={size.css} onpointerdown={size.start} onkeydown={size.key} ondblclick={size.reset}></div>
<aside class="ask" style:--panel-width={size.css} aria-label={m['creator.ask']()} data-testid="ask-panel">
	<header>
		<h2>{m['creator.ask']()}</h2>
		<button type="button" class="sc-round" class:is-active={view === 'history'} title={m['creator.ask_history']()} aria-label={m['creator.ask_history']()} aria-pressed={view === 'history'} onclick={showHistory}><Icon name="clock" /></button>
		<button type="button" class="sc-round" title={m['creator.ask_new']()} aria-label={m['creator.ask_new']()} onclick={newChat} disabled={!turns.length && view === 'chat'}><Icon name="plus" /></button>
		<CloseButton label={m['creator.ask_close']()} onclick={onclose} />
	</header>

	{#if view === 'history'}
		<ol class="history" data-testid="ask-history">
			{#if historyError}<li class="empty">{historyError}</li>
			{:else if !history}<li class="empty">…</li>
			{:else if !history.length}<li class="empty">{m['creator.ask_history_empty']()}</li>
			{:else}
				{#each history as item (item.id)}
					<li>
						<button type="button" class="sc-nav-item" class:is-current={item.id === chat} onclick={() => openChat(item.id)}>
							<span class="title">{item.title || '–'}</span>
							<span class="meta">{when(item.updated_at)}{item.page ? ` · ${item.page}` : ''}</span>
						</button>
					</li>
				{/each}
			{/if}
		</ol>
	{:else}
	<ol class="chat" bind:this={list} aria-live="polite">
		{#if !turns.length}<li class="empty">{m['creator.ask_empty']()}</li>{/if}
		{#each turns as turn, i (i)}
			<li class="turn {turn.who}">
				<p class="who">{turn.who === 'you' ? m['creator.ask_you']() : m['creator.ask_agent']()}</p>
				{#if turn.steps.length}
					<ul class="steps">
						{#each turn.steps as step, j (j)}
							{@const href = step.key ? hrefOf(step.key) : null}
							<li class:failed={!!step.error} title={step.error}>
								<Icon name={step.error ? 'x' : step.key ? 'edit-3' : 'check'} size="1em" />
								<span class="what">{step.tool}{step.key ? ` · ${keyLabel(step.key)}` : step.target ? ` · ${step.target}` : ''}</span>
								{#if step.key && step.version}
									{#if href}<a {href}>{m['creator.ask_draft']({ id: String(step.version) })}</a>{:else}<span>{m['creator.ask_draft']({ id: String(step.version) })}</span>{/if}
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
				{#if turn.text}<p class="text">{turn.text}</p>{/if}
				{#if turn.error}<p class="sc-error">{turn.error}</p>{/if}
				{#if busy && i === turns.length - 1}<p class="working">{m['creator.ask_working']()}</p>{/if}
			</li>
		{/each}
	</ol>
	{/if}

	<form class="composer" onsubmit={send}>
		<p class="context">{m['creator.ask_context']({ page: context })} · {m['creator.ask_drafts_only']()}</p>
		<div class="row">
			<textarea bind:this={field} bind:value={input} rows="1" placeholder={m['creator.ask_placeholder']()} aria-label={m['creator.ask_placeholder']()} {onkeydown}></textarea>
			{#if busy}
				<button type="button" class="sc-round" title={m['creator.ask_stop']()} aria-label={m['creator.ask_stop']()} onclick={() => controller?.abort()}><Icon name="square" /></button>
			{:else}
				<button type="submit" class="sc-round" title={m['creator.ask_send']()} aria-label={m['creator.ask_send']()} disabled={!input.trim()}><Icon name="arrow-up" /></button>
			{/if}
		</div>
	</form>
</aside>

<style>
	.ask {
		position: fixed;
		top: var(--top);
		right: 0;
		bottom: var(--action-bar-height, 0rem);
		width: var(--panel-width);
		z-index: 30;
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		background: var(--paper);
		border-left: var(--border-width-regular) solid var(--ink);
	}
	.resize {
		position: fixed;
		z-index: 31;
		top: var(--top);
		bottom: var(--action-bar-height, 0rem);
		right: calc(var(--panel-width) - 0.375rem);
		width: 0.75rem;
		cursor: col-resize;
		touch-action: none;
	}
	header {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		/* the same strip as the editor's file row on the left (chapter-editor .files): one control row */
		padding: var(--space-05) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	h2 {
		margin: 0;
		flex: 1;
	}
	.chat {
		flex: 1;
		overflow: auto;
		list-style: none;
		margin: 0;
		padding: var(--space-1);
		display: flex;
		flex-direction: column;
		gap: var(--space-15);
	}
	.history {
		flex: 1;
		overflow: auto;
		list-style: none;
		margin: 0;
		padding: var(--space-05) 0;
	}
	.history button {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		width: 100%;
		text-align: left;
	}
	.history .title {
		overflow-wrap: anywhere;
	}
	.history .meta {
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
	.history .empty {
		padding: 0 var(--space-1);
	}
	.who {
		margin: 0 0 var(--space-05);
		font-size: var(--type-caption-size);
		letter-spacing: var(--tracking-label);
		text-transform: uppercase;
		color: var(--text-muted);
	}
	.turn.you .text {
		border-left: var(--border-width-regular) solid var(--ink);
		padding-left: var(--space-1);
	}
	.text {
		margin: 0;
		white-space: pre-wrap;
	}
	.empty,
	.working {
		margin: 0;
		color: var(--text-muted);
	}
	.steps {
		list-style: none;
		margin: 0 0 var(--space-1);
		padding: 0;
		font-size: var(--type-caption-size);
	}
	.steps li {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		padding: 0.15rem 0;
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
	}
	.steps .what {
		flex: 1;
		overflow-wrap: anywhere;
	}
	.steps .failed {
		color: var(--text-muted);
	}
	.composer {
		border-top: var(--border-width-regular) solid var(--ink);
		padding: var(--space-05) var(--space-1) var(--space-1);
	}
	.context {
		margin: 0 0 var(--space-05);
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
	.row {
		display: flex;
		align-items: flex-end;
		gap: var(--space-05);
	}
	textarea {
		flex: 1;
		resize: none;
		/* one line = exactly one control (the send button next to it); more lines grow from there */
		padding-block: calc((var(--control-height) - 1lh - 2 * var(--border-width-hairline)) / 2); /* control-ok: one line = --control-height */
		min-height: var(--control-height);
		max-height: 50vh;
		max-height: 50dvh;
		overflow-y: auto;
		box-sizing: border-box;
	}
	@media (min-width: 768px) {
		:global(html.sc-ask-open) {
			/* the page's own right-docked panels (Problems, History, help) sit left of the chat */
			--ask-inset: var(--ask-width);
		}
		:global(html.sc-ask-open .sc-shell-frame) {
			margin-right: var(--ask-width);
		}
	}
	@media (max-width: 767px) {
		.ask {
			left: 0;
			width: auto;
			border-left: 0;
		}
		.resize {
			display: none;
		}
	}
</style>
