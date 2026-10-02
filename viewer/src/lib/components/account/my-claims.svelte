<script lang="ts">
	import { onMount } from 'svelte';
	import { addClaimFile, myClaims, withdrawClaim } from '$lib/api.ts';
	import type { OwnershipClaim } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { formatDate } from '$lib/utils/date.ts';

	/**
	 * "Meine Ansprüche" on the account page (agents/ITEMS.md "Ownership, claims, gifting"): the reader's ownership claims
	 * with their status; an open one can get more proof or be withdrawn. Hidden while there are none.
	 */
	let claims = $state<OwnershipClaim[]>([]);
	let error = $state<string | null>(null);
	let working = $state<string | null>(null);

	async function load() {
		try {
			claims = (await myClaims()).items;
		} catch {
			claims = [];
		}
	}

	function replace(claim: OwnershipClaim) {
		claims = claims.map((c) => (c.id === claim.id ? claim : c));
	}

	async function withdraw(claim: OwnershipClaim) {
		working = claim.id;
		error = null;
		try {
			replace((await withdrawClaim(claim.id)).claim);
		} catch {
			error = m['account.claim_failed']();
		} finally {
			working = null;
		}
	}

	async function addFiles(claim: OwnershipClaim, event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const files = Array.from(input.files ?? []);
		working = claim.id;
		error = null;
		for (const file of files) {
			try {
				replace((await addClaimFile(claim.id, file, file.name)).claim);
			} catch {
				error = m['claim.dispute_file_failed']({ name: file.name });
			}
		}
		input.value = '';
		working = null;
	}

	const statusLabel = (status: OwnershipClaim['status']) =>
		({
			open: m['account.claim_status_open'](),
			transferred: m['account.claim_status_transferred'](),
			rejected: m['account.claim_status_rejected'](),
			withdrawn: m['account.claim_status_withdrawn']()
		})[status] ?? status;

	onMount(load);
</script>

{#if claims.length}
	<section data-testid="my-claims">
		<h2>{m['account.claims']()}</h2>
		<ul>
			{#each claims as claim (claim.id)}
				<li>
					<div class="head">
						<strong>{claim.code}</strong>
						<span class="status" data-status={claim.status}>{statusLabel(claim.status)}</span>
					</div>
					<p class="meta">
						{formatDate(new Date(claim.created_at))}{claim.files.length ? ` · ${m['account.claim_files']({ count: claim.files.length })}` : ''}
					</p>
					{#if claim.status === 'open'}
						<div class="actions">
							<label class="btn btn-secondary file">
								{m['account.claim_add_file']()}
								<input type="file" multiple accept="image/jpeg,image/png,image/webp,application/pdf" disabled={working === claim.id} onchange={(e) => addFiles(claim, e)} />
							</label>
							<button class="btn-secondary" disabled={working === claim.id} onclick={() => withdraw(claim)}>{m['account.claim_withdraw']()}</button>
						</div>
					{/if}
				</li>
			{/each}
		</ul>
		{#if error}<p class="error" role="alert">{error}</p>{/if}
	</section>
{/if}

<style>
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	li {
		border: 2px solid black;
		padding: 0.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	.head {
		display: flex;
		justify-content: space-between;
		gap: 0.5rem;
	}
	.status {
		text-transform: uppercase;
		letter-spacing: 0.08em;
		font-size: 0.75rem;
	}
	.status[data-status='open'] {
		background: black;
		color: white;
		padding: 0 0.25rem;
	}
	.meta {
		margin: 0;
		font-size: 0.75rem;
	}
	.actions {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
	}
	.file {
		position: relative;
		cursor: pointer;
	}
	.file input {
		position: absolute;
		inset: 0;
		opacity: 0;
		cursor: pointer;
	}
	.error {
		border: 2px solid black;
		padding: 0.5rem;
	}
</style>
