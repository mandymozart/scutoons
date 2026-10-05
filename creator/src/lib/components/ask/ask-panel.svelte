<script lang="ts">
	// The creator's Ask panel (@scutoons/ask): the agent with the signed-in creator's content rights (tools/mcp). What
	// the creator adds: its labels, the header's text language with every message, and links from a saved draft to its
	// page.
	import { AskPanel, type AskLabels } from '@scutoons/ask';
	import type { AgentStep } from '@scutoons/api';
	import { parseEntityKey } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { agentChatsApi } from '$lib/api.ts';
	import { ASK_URL } from '$lib/config.ts';

	let { context, onclose }: { context: string; onclose: () => void } = $props();

	const labels: AskLabels = {
		title: m['creator.ask'](),
		history: m['creator.ask_history'](),
		historyEmpty: m['creator.ask_history_empty'](),
		historyUnavailable: m['creator.ask_history_unavailable'](),
		newChat: m['creator.ask_new'](),
		close: m['creator.ask_close'](),
		empty: m['creator.ask_empty'](),
		you: m['creator.ask_you'](),
		agent: m['creator.ask_agent'](),
		working: m['creator.ask_working'](),
		context: (page) => `${m['creator.ask_context']({ page })} · ${m['creator.ask_drafts_only']()}`,
		placeholder: m['creator.ask_placeholder'](),
		send: m['creator.ask_send'](),
		stop: m['creator.ask_stop'](),
		resize: m['creator.resize_panel'](),
		unavailable: m['creator.ask_unavailable'](),
		signedOut: m['creator.ask_signed_out'](),
		draft: (id) => m['creator.ask_draft']({ id })
	};

	/** the creator page of a saved document */
	function hrefOf(step: AgentStep): string | null {
		const ref = step.key ? parseEntityKey(step.key) : null;
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
	const stepLabel = (step: AgentStep) => `${step.tool}${step.key ? ` · ${keyLabel(step.key)}` : step.target ? ` · ${step.target}` : ''}`;
</script>

<AskPanel
	app="creator"
	url={ASK_URL}
	chats={agentChatsApi}
	{context}
	appLanguage={getLocale()}
	{labels}
	extra={() => ({ textLanguage: textLanguage.value })}
	{hrefOf}
	{stepLabel}
	{onclose}
/>
