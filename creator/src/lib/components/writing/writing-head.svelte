<script lang="ts">
	/** The head of a writing page's column (lap 2 C14): an optional language line, the title, the path under it – muted. */
	let { title, sub = '', lang = '', testid = 'writing-title' }: { title: string; sub?: string; lang?: string; testid?: string } = $props();
</script>

<header class="writing-head">
	{#if lang}<p class="lang">{lang}</p>{/if}
	<h1 data-testid={testid}>{title}</h1>
	{#if sub}<p class="sub">{sub}</p>{/if}
</header>

<style>
	.writing-head {
		margin-bottom: 1.4em;
	}
	.lang {
		margin: 0 0 0.5rem;
		font-size: var(--editor-small);
		letter-spacing: 0.12em;
		text-transform: uppercase;
		color: var(--tool-muted);
	}
	h1 {
		margin: 0 0 0.25rem;
		font-size: var(--wr-title, 1.8em);
		font-weight: normal;
		line-height: 1.2;
	}
	.sub {
		margin: 0;
		font-size: var(--editor-small);
		color: var(--tool-muted);
	}
</style>
