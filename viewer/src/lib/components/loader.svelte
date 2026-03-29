<script lang="ts">
	import { m } from "$lib/paraglide/messages.js";

	let { visible = false, oncancel }: { visible?: boolean; oncancel?: () => void } = $props();
</script>
<div class="loading" class:hidden={!visible}>
	<img src="/SCUTOONS-logo-animated.svg" alt="Patience ..." /><br />
	{m['common.loading']()}<br />
	{#if oncancel}
		<button class="btn btn-secondary" onclick={oncancel}>{m['common.cancel']()}</button>
	{/if}
</div>


<style>
	.loading {
		position: fixed;
		inset: 0;
		width: 100dvw;
		height: 100dvh;
		z-index: 999;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-direction: column;
		background: rgba(255, 255, 255, 1);
		text-align: center;
	}

	.hidden {
		display: none;
	}
	.btn {
		position: absolute;
		bottom: 2rem;
		right: 2rem;
		left: 2rem;
	}
</style>
