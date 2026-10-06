<script lang="ts">
	// The "Ask" panel (CUTOVER 10.28 step 2): a chat with an agent that has the signed-in person's rights in this app
	// (the Ask service, tools/mcp: content tools in the creator, the partner network in the embassy, …). Docked on the
	// right like the editor's side panels (drags wider up to half the window; the page makes room), full screen on
	// phones. The agent's answer streams in; each tool call is a step, a step can link to its page (`hrefOf`) and an
	// app can render a step's result (`result`, e.g. the embassy's mail draft). Chats are saved by the Ask service in
	// the API (`/api/agent-chats`, per app; owner 2026-10-04: "I need to see what I did before if something stops or
	// breaks"): the clock button lists the earlier chats, one opens to read or go on; the open chat is remembered per
	// browser and comes back after a reload. Every message carries the page (`context`), the app language and what the
	// app adds (`extra`, e.g. the creator's text language).
	import { onMount, type Snippet } from 'svelte';
	import { Icon, CloseButton, Divider, panelWidth } from '@scutoons/ui';
	import { getSessionToken, type AgentApp, type AgentChatsApi, type AgentChatSummary, type AgentStep, type AgentTurn } from '@scutoons/api';
	import { ask } from './ask.svelte.ts';
	import { chatMarkdown } from './chat-markdown.ts';
	import type { AskLabels } from './labels.ts';

	let {
		app,
		url,
		chats,
		context,
		appLanguage,
		labels,
		extra = () => ({}),
		hrefOf = (step) => step.link ?? null,
		stepLabel = (step) => (step.target ? `${step.tool} · ${step.target}` : step.tool),
		result,
		onstep,
		onclose
	}: {
		app: AgentApp;
		/** the Ask service's endpoint ('' = not available in this build) */
		url: string;
		chats: AgentChatsApi;
		/** where the person is, sent with every message and shown above the field */
		context: string;
		appLanguage: string;
		labels: AskLabels;
		extra?: () => Record<string, unknown>;
		/** the page a step points to (a saved draft, an organisation; default the step's `link`) */
		hrefOf?: (step: AgentStep) => string | null;
		stepLabel?: (step: AgentStep) => string;
		/** an app-specific result of a step (`step.data`), shown below the answer */
		result?: Snippet<[AgentStep]>;
		/** a step came in (e.g. the agent changed something the page shows: reload it) */
		onstep?: (step: AgentStep) => void;
		onclose: () => void;
	} = $props();

	const size = panelWidth('sc-panel-width:ask', 24);

	// the page makes room for the panel (owner 2026-10-04: "the chat pushes the page"): from 768px the app's frame
	// ends at the panel's edge (`html.sc-ask-open`, `--ask-width`), so the page stays usable next to the chat
	$effect(() => {
		const root = document.documentElement;
		root.classList.add('sc-ask-open');
		root.style.setProperty('--ask-width', size.css);
		return () => {
			root.classList.remove('sc-ask-open');
			root.style.removeProperty('--ask-width');
		};
	});

	// one panel per app, the app never changes
	// svelte-ignore state_referenced_locally
	const CURRENT = `scutoons_${app}_ask_chat`;

	let turns = $state<AgentTurn[]>([]);
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
			const { chat: saved } = await chats.get(id);
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
			history = (await chats.list()).chats;
		} catch {
			historyError = labels.historyUnavailable;
		}
	}
	const when = (iso: string) => {
		const d = new Date(iso.endsWith('Z') || iso.includes('+') ? iso : `${iso}Z`);
		return Number.isNaN(d.getTime()) ? iso : new Intl.DateTimeFormat(appLanguage, { dateStyle: 'medium', timeStyle: 'short' }).format(d);
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
	// a question handed over by the page (a problem's "Ask"): into the field, the person sends it
	$effect(() => {
		if (!ask.prefill || !field) return;
		input = ask.prefill;
		ask.prefill = '';
		if (view === 'history') view = 'chat';
		field.focus();
	});
	let busy = $state(false);
	let controller: AbortController | null = null;
	let list = $state<HTMLElement>();

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
	function apply(turn: AgentTurn, block: string) {
		const name = /^event: (.*)$/m.exec(block)?.[1];
		const raw = /^data: (.*)$/m.exec(block)?.[1];
		if (!name || raw === undefined) return;
		const data = JSON.parse(raw);
		if (name === 'chat') remember(data.id);
		else if (name === 'text') turn.text += data.delta;
		else if (name === 'step') {
			turn.steps.push(data);
			onstep?.(data);
		}
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
		if (!url) {
			turn.error = labels.unavailable;
			return;
		}
		busy = true;
		controller = new AbortController();
		try {
			const token = getSessionToken();
			const res = await fetch(url, {
				method: 'POST',
				credentials: 'include',
				headers: { 'Content-Type': 'application/json', ...(token ? { Authorization: `Bearer ${token}` } : {}) },
				body: JSON.stringify({ ...extra(), app, chat, text, context, appLanguage }),
				signal: controller.signal
			});
			if (!res.ok || !res.body) throw new Error(res.status === 401 ? labels.signedOut : labels.unavailable);
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
			if (!(error instanceof DOMException && error.name === 'AbortError')) turn.error = error instanceof Error ? error.message : labels.unavailable;
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

<!-- the app's outermost region on the right: fixed, the app's frame ends at it (agents/DOCKING.md) -->
<div class="ask-edge" style:--sc-divider-top="var(--ask-top, var(--top, 0rem))" style:--sc-divider-bottom="var(--action-bar-height, 0rem)">
	<Divider {size} edge="left" at={size.css} label={labels.resize} controls="sc-ask-panel" fixed />
</div>
<aside class="ask" id="sc-ask-panel" style:--panel-width={size.css} aria-label={labels.title} data-testid="ask-panel">
	<header>
		<h2>{labels.title}</h2>
		<button type="button" class="sc-round" class:is-active={view === 'history'} title={labels.history} aria-label={labels.history} aria-pressed={view === 'history'} onclick={showHistory}><Icon name="clock" /></button>
		<button type="button" class="sc-round" title={labels.newChat} aria-label={labels.newChat} onclick={newChat} disabled={!turns.length && view === 'chat'}><Icon name="plus" /></button>
		<CloseButton label={labels.close} onclick={onclose} />
	</header>

	{#if view === 'history'}
		<ol class="history" data-testid="ask-history">
			{#if historyError}<li class="empty">{historyError}</li>
			{:else if !history}<li class="empty">…</li>
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
		{#if !turns.length}<li class="sc-empty">{labels.empty}</li>{/if}
		{#each turns as turn, i (i)}
			<li class="turn {turn.who}">
				<p class="who">{turn.who === 'you' ? labels.you : labels.agent}</p>
				{#if turn.steps.length}
					<ul class="steps">
						{#each turn.steps as step, j (j)}
							{@const href = hrefOf(step)}
							<li class:failed={!!step.error} title={step.error}>
								<Icon name={step.error ? 'x' : step.key || step.changed ? 'edit-3' : 'check'} size="1em" />
								<span class="what">{stepLabel(step)}</span>
								{#if step.key && step.version}
									{#if href}<a {href}>{labels.draft(String(step.version))}</a>{:else}<span>{labels.draft(String(step.version))}</span>{/if}
								{:else if href}
									<a {href} class="open" title={stepLabel(step)} aria-label={stepLabel(step)}><Icon name="arrow-right" size="1em" /></a>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
				{#if turn.text && turn.who === 'agent'}
					<!-- Markdown, HTML-escaped first (chat-markdown.ts) -->
					<div class="text md">{@html chatMarkdown(turn.text)}</div>
				{:else if turn.text}<p class="text">{turn.text}</p>{/if}
				{#if result}
					{#each turn.steps as step, j (j)}
						{#if step.data && !step.error}{@render result(step)}{/if}
					{/each}
				{/if}
				{#if turn.error}<p class="sc-error">{turn.error}</p>{/if}
				{#if busy && i === turns.length - 1}<p class="working">{labels.working}</p>{/if}
			</li>
		{/each}
	</ol>
	{/if}

	<form class="composer" onsubmit={send}>
		<p class="context">{labels.context(context)}</p>
		<div class="row">
			<textarea bind:this={field} bind:value={input} rows="1" placeholder={labels.placeholder} aria-label={labels.placeholder} {onkeydown}></textarea>
			{#if busy}
				<button type="button" class="sc-round" title={labels.stop} aria-label={labels.stop} onclick={() => controller?.abort()}><Icon name="square" /></button>
			{:else}
				<button type="submit" class="sc-round" title={labels.send} aria-label={labels.send} disabled={!input.trim()}><Icon name="arrow-up" /></button>
			{/if}
		</div>
	</form>
</aside>

<style>
	/* the app sets where the panel starts and ends: `--ask-top` (below its header; the creator's `--top`) and
	   `--action-bar-height` (an action bar at the bottom) */
	.ask {
		position: fixed;
		top: var(--ask-top, var(--top, 0rem));
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
	.ask-edge {
		display: contents;
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
		overflow-wrap: anywhere;
	}
	/* the agent's answer: rendered Markdown */
	.md {
		white-space: normal;
	}
	.md :global(:is(p, ul, ol, pre)) {
		margin: 0 0 var(--space-05);
	}
	.md :global(:is(p, ul, ol, pre):last-child) {
		margin-bottom: 0;
	}
	.md :global(:is(ul, ol)) {
		padding-left: var(--space-15);
	}
	.md :global(.heading) {
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		font-size: var(--type-caption-size);
		margin-top: var(--space-1);
	}
	.md :global(code) {
		background: var(--surface-panel);
		padding: 0 0.2em;
	}
	.md :global(pre) {
		overflow-x: auto;
		padding: var(--space-05);
		background: var(--surface-panel);
	}
	.md :global(pre code) {
		background: none;
		padding: 0;
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
	.steps .open {
		display: inline-flex;
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
		/* the shell (@scutoons/ui AppShell) ends at the panel; an app without it does the same with `--ask-inset` */
		:global(html.sc-ask-open .sc-shell-frame) {
			margin-right: var(--ask-width);
		}
	}
	@media (max-width: 767px) {
		.ask {
			left: 0;
			width: auto;
			border-left: 0;
			/* full screen: above an app's bottom navigation */
			z-index: 40;
		}
	}
</style>
