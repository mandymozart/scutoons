<script lang="ts">
	import { problemFor, type CatalogFormProps } from '../kinds.ts';
	import { m } from '../messages.ts';
	import CheckField from './check-field.svelte';

	/**
	 * A code type (security-relevant: the editor asks before saving): may a code of this type be claimed, does it allow
	 * a signup, which permissions it grants. `default` = every type that is not listed.
	 */
	let { data = $bindable(), lists, problems }: CatalogFormProps = $props();

	const grants = $derived<string[]>(Array.isArray(data.grants) ? data.grants : []);
	const permissions = $derived(lists.permissions ?? []);
	const err = $derived(problemFor(problems, 'grants', 'permission', 'claimable', 'signup'));

	function grant(id: string, on: boolean) {
		data.grants = on ? [...grants.filter((g) => g !== id), id] : grants.filter((g) => g !== id);
	}
</script>

<fieldset class="group">
	<legend>{m['catalog.code_type_may']()}</legend>
	<CheckField label={m['catalog.claimable']()} checked={!!data.claimable} onchange={(v) => (data.claimable = v)} testid="code-type-claimable" />
	<CheckField label={m['catalog.signup']()} checked={!!data.signup} onchange={(v) => (data.signup = v)} testid="code-type-signup" />
</fieldset>
<fieldset class="group">
	<legend>{m['catalog.grants']()}</legend>
	{#each permissions as p (p.id)}
		<CheckField label="{p.id} – {p.data}" checked={grants.includes(p.id)} onchange={(on) => grant(p.id, on)} />
	{/each}
	{#each grants.filter((g) => !permissions.some((p) => p.id === g)) as unknown (unknown)}
		<CheckField label="{unknown} (?)" checked onchange={(on) => grant(unknown, on)} />
	{/each}
</fieldset>
{#if err}<p class="sc-error">{err}</p>{/if}

<style>
	.group {
		display: flex;
		flex-direction: column;
		margin-bottom: var(--space-15);
	}
	p {
		margin: 0;
	}
</style>
