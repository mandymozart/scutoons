<script lang="ts">
	import { API_URL } from '$lib/config.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * "Set up your machine" under the AI connections (#127, research/STUDIO-IN-CREATOR.md §2.4 "the local kit"; owner
	 * 2026-10-20: "they can just install the stuff and connect their claude and go"): the same three steps for every
	 * member – the MCP (the connection above), the Figma plugin (a download built with the MCP – the MCP carries the
	 * relay), optionally ComfyUI – and the check: whoami. Folded by default; members only (ai-connections shows it).
	 */
	// the short link redirects to the API (static/.htaccess); the dev viewer links its dev API directly
	const plugin = import.meta.env.DEV ? `${API_URL}/mcp/download?file=figma-plugin` : '/figma-plugin';
</script>

<details class="setup" data-testid="ai-setup">
	<summary>{m['account.ai_setup']()}</summary>
	<ol>
		<li>{m['account.ai_setup_1']()}</li>
		<li>{m['account.ai_setup_2']()} <a href={plugin} download data-sveltekit-reload>{m['account.ai_setup_2_link']()}</a></li>
		<li>{m['account.ai_setup_3']()}</li>
	</ol>
	<p>{m['account.ai_setup_check']()}</p>
</details>

<style>
	.setup {
		font-size: 0.875rem;
	}
	summary {
		padding: 0.5rem 0;
		cursor: pointer;
		color: var(--text-muted);
	}
	summary:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: var(--focus-offset);
	}
	ol {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		margin: 0;
		padding-left: 1.25rem;
	}
	p {
		margin: 0.5rem 0 0;
		color: var(--text-muted);
	}
	a {
		color: var(--ink);
	}
</style>
