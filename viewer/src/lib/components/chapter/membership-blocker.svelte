<script lang="ts">
	import { goto } from '$app/navigation';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import type { Snippet } from 'svelte';

	let {
		isPreview = false,
		children
	}: {
		isPreview?: boolean;
		children: Snippet;
	} = $props();

	const membership = getMembershipContext();

	const shouldShowContent = $derived(isPreview || membership.isValid);

	const handleBuyAccess = () => {
		goto(localizeHref('/shop'));
	};
</script>

{#if shouldShowContent}
	{@render children()}
{:else}
	<div class="view membership-required-view">
		<div class="membership-required">
			<div class="membership-card">
				<h2>🔒 {m['membership_blocker.title']()}</h2>
				<p>{m['membership_blocker.description']()}</p>

				<div class="membership-info">
					<h3>{m['membership_blocker.how_to_access']()}</h3>
					<ol>
						<li>{m['membership_blocker.step_1']()}</li>
						<li>{m['membership_blocker.step_2']()}</li>
						<li>{m['membership_blocker.step_3']()}</li>
					</ol>
				</div>

				<div class="membership-actions">
					<button onclick={handleBuyAccess} class="btn-buy-access">
						{m['account.buy_access']()}
					</button>
				</div>

				{#if membership.membership && !membership.isValid}
					<div class="membership-error">
						<p>{m['membership_blocker.invalid_code']()}</p>
					</div>
				{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	.membership-required {
		border: 4px solid #000;
		max-width: 800px;
		margin: 0 auto;
		padding: 2rem;
	}
</style>