<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import { page } from '$app/state';
	import { ActionBar, Badge, Button, confirmDialog, DataTable, Field, Section, Skeleton, Toggle, type Column } from '@scutoons/ui';
	import type { AdminReadingChapter, AdminReviewRule, AdminUserDetail, RoleInfo } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { day, errorText, minute } from '$lib/format.ts';
	import { tableLabels } from '$lib/labels.ts';
	import { getAdminSession } from '$lib/session.svelte.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';
	import ReadingBar from '$lib/components/reading-bar.svelte';
	import { chapterLabel, toonName } from '$lib/reading.ts';

	const toaster = getToaster();
	const session = getAdminSession();
	const id = $derived(page.params.id ?? '');

	let user = $state<AdminUserDetail | null>(null);
	let roles = $state<RoleInfo[]>([]);
	let toons = $state<{ id: string; name: string }[]>([]);
	let error = $state<string | null>(null);
	let working = $state<string | null>(null);

	$effect(() => {
		const current = id;
		user = null;
		error = null;
		Promise.all([admin.getUser(current), admin.roles(), admin.catalog()])
			.then(([u, r, c]) => {
				user = u.user;
				roles = r.items;
				toons = c.toons.map((t) => ({ id: t.id, name: t.name }));
			})
			.catch((e) => (error = errorText(e)));
	});

	const isSelf = $derived(!!user && user.email === session.user?.email);

	async function setRole(role: string, on: boolean) {
		if (!user) return;
		if (!on && role === 'admin' && isSelf && !(await confirmDialog({ title: m['users.revoke_own_admin_title'](), message: m['users.revoke_own_admin'](), confirmLabel: m['users.revoke_own_admin_label']() }))) {
			user = { ...user };
			return;
		}
		working = role;
		try {
			const result = on ? await admin.grantRole(user.id, role) : await admin.revokeRole(user.id, role);
			user = result.user;
			toaster.addToast({
				message: on ? m['users.granted']({ role, email: user.email }) : m['users.revoked']({ role, email: user.email }),
				type: 'success'
			});
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
			user = { ...user };
		} finally {
			working = null;
		}
	}

	// ── Review rules (docs/API.md "Reviews"): per toon or every toon – what the account does (owner 2026-10-12): two grants,
	// reviewer "Reviews publishing requests" and physical "Publishes physical items", one restriction, needs_review "Submits
	// for review"; the stored ids stay ──
	let ruleToon = $state('*');
	let ruleKind = $state<AdminReviewRule['rule']>('reviewer');
	const RULES: { id: AdminReviewRule['rule']; label: () => string; hint: () => string }[] = [
		{ id: 'reviewer', label: () => m['users.rule_reviewer'](), hint: () => m['users.rule_reviewer_hint']() },
		{ id: 'physical', label: () => m['users.rule_physical'](), hint: () => m['users.rule_physical_hint']() },
		{ id: 'needs_review', label: () => m['users.rule_needs_review'](), hint: () => m['users.rule_needs_review_hint']() }
	];
	const ruleOf = (rule: string) => RULES.find((r) => r.id === rule);
	const ruleLabel = (rule: string) => ruleOf(rule)?.label() ?? rule;
	const toonLabel = (toon: string) => (toon === '*' ? m['users.review_all_toons']() : (toons.find((t) => t.id === toon)?.name ?? toon));

	async function setReviewRule(toon: string, rule: AdminReviewRule['rule'], on: boolean) {
		if (!user) return;
		working = 'reviews';
		try {
			const result = await admin.setReviewRule(user.id, toon, rule, on);
			user = { ...user, review_rules: result.review_rules };
			const vars = { rule: ruleLabel(rule), toon: toonLabel(toon) };
			toaster.addToast({ message: on ? m['users.review_added'](vars) : m['users.review_removed'](vars), type: 'success' });
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		} finally {
			working = null;
		}
	}

	function addReviewRule(e: SubmitEvent) {
		e.preventDefault();
		void setReviewRule(ruleToon, ruleKind, true);
	}

	// ── Avatar-body unlocks (ITEMS.md "Reader avatar bodies & character unlocks") ──
	let unlockCharacter = $state('');
	let unlockLooks = $state('');

	async function unlock(e: SubmitEvent) {
		e.preventDefault();
		if (!user || !unlockCharacter.trim()) return;
		const character = unlockCharacter.trim();
		const looks = unlockLooks.split(',').map((l) => l.trim()).filter(Boolean);
		working = 'characters';
		try {
			const result = await admin.unlockCharacter(user.id, character, looks.length ? { looks } : {});
			user = { ...user, characters: result.characters };
			unlockCharacter = '';
			unlockLooks = '';
			toaster.addToast({ message: m['users.unlocked']({ character, email: user.email }), type: 'success' });
		} catch (err) {
			toaster.addToast({ message: errorText(err), type: 'error' });
		} finally {
			working = null;
		}
	}

	async function lock(character: string) {
		if (!user) return;
		working = 'characters';
		try {
			const result = await admin.lockCharacter(user.id, character);
			user = { ...user, characters: result.characters };
			toaster.addToast({ message: m['users.unlock_revoked']({ character, email: user.email }), type: 'success' });
		} catch (err) {
			toaster.addToast({ message: errorText(err), type: 'error' });
		} finally {
			working = null;
		}
	}

	const codeColumns = $derived<Column[]>([
		{ id: 'code', label: m['codes.col_code']() },
		{ id: 'type', label: m['codes.col_type']() },
		{ id: 'edition_id', label: m['common.edition']() },
		{ id: 'claimed_at', label: m['codes.col_claimed']() },
		{ id: 'revoked_at', label: m['codes.status_revoked']() }
	]);

	// where this member is in the toons (BACKLOG 10.69): their save game's chapter checkpoints, last read first
	const readingColumns = $derived<Column[]>([
		{ id: 'chapter', label: m['users.reading_col_chapter']() },
		{ id: 'position', label: m['users.reading_col_progress']() },
		{ id: 'spot', label: m['users.reading_col_spot']() },
		{ id: 'read_at', label: m['users.reading_col_read']() }
	]);
</script>

{#if error}
	<PageHeader type={m['users.type_label']()} title={id} />
	<p class="error-box" role="alert">{error}</p>
{:else if !user}
	<PageHeader type={m['users.type_label']()} title={id} />
	<Skeleton lines={8} label={m['shell.loading']()} />
{:else}
	<PageHeader type={m['users.type_label']()} title={user.email} meta={m['users.meta']({ created: day(user.created_at), id: user.id })} />

	<ActionBar label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={user.email}>
		{#snippet info()}<PageHelp page="user" />{/snippet}
		{#snippet start()}
			<a href="/users">{m['users.back']()}</a>
		{/snippet}
	</ActionBar>

	<div class="grid-2">
		<Section title={m['users.section_account']()}>
			<dl class="facts">
				<dt>{m['users.col_signup_code']()}</dt>
				<dd>{#if user.signup_code}<a class="mono-code" href="/codes/{encodeURIComponent(user.signup_code)}">{user.signup_code}</a>{:else}—{/if}</dd>
				<dt>{m['users.col_opt_in']()}</dt>
				<dd>{user.marketing_opt_in ? `${m['common.yes']()} · ${minute(user.marketing_opt_in_at)}` : m['common.no']()}</dd>
				<dt>{m['users.locale']()}</dt>
				<dd>{user.locale ?? '—'}</dd>
				<dt>{m['users.inventory']()}</dt>
				<dd>{user.inventory_entries}</dd>
				<dt>{m['users.sessions']()}</dt>
				<dd>{user.sessions}</dd>
				<dt>{m['users.last_seen']()}</dt>
				<dd>{minute(user.last_seen_at)}</dd>
			</dl>
		</Section>
		<Section title={m['users.section_roles']()}>
			<div class="roles">
				{#each roles as role (role.id)}
					<Toggle
						label={role.name}
						hint={role.description}
						checked={user.roles.includes(role.name)}
						disabled={working !== null}
						onchange={(on) => setRole(role.name, on)}
					/>
				{/each}
			</div>
			<p class="current">{#each user.roles as r (r)}<Badge tone={r === 'admin' ? 'ink' : 'neutral'}>{r}</Badge> {/each}</p>
		</Section>
	</div>

	<Section title={m['users.section_reading']()}>
		{#if !user.reading}
			<p class="sc-caption" data-testid="reading-none">{m['users.reading_none']()}</p>
		{:else}
			<p class="sc-caption">{m['users.reading_summary']({ variables: String(user.reading.variables), places: String(user.reading.places), at: minute(user.reading.updated_at) })}</p>
			{#if user.reading.chapters.length}
				<DataTable columns={readingColumns} rows={user.reading.chapters} rowKey={(c) => `${c.toon}/${c.chapter}`} labels={tableLabels()}>
					{#snippet cell(c: AdminReadingChapter, column)}
						{#if column.id === 'chapter'}{chapterLabel(toonName(toons, c.toon), c.chapter)}
						{:else if column.id === 'position'}
							{#if c.finished && c.position >= 1}<Badge tone="ink">{m['users.reading_finished']()}</Badge>
							{:else}<ReadingBar position={c.position} />{#if c.finished} <span class="meta">{m['users.reading_again']()}</span>{/if}{/if}
						{:else if column.id === 'spot'}{c.group !== null && c.slice !== null ? m['users.reading_spot']({ group: String(c.group + 1), slice: String(c.slice + 1) }) : '—'}
						{:else}{minute(c.read_at)}{/if}
					{/snippet}
				</DataTable>
			{/if}
		{/if}
	</Section>

	<Section title={m['users.section_reviews']()} hint={m['users.section_reviews_hint']()}>
		{#if user.review_rules?.length}
			<ul class="rules" data-testid="review-rules">
				{#each user.review_rules as r (r.toon + r.rule)}
					<li>
						<span>{toonLabel(r.toon)}</span>
						<span title={ruleOf(r.rule)?.hint()}><Badge tone={r.rule === 'needs_review' ? 'neutral' : 'ink'}>{ruleLabel(r.rule)}</Badge></span>
						<Button variant="clear" size="sm" disabled={working !== null} onclick={() => setReviewRule(r.toon, r.rule, false)}>{m['users.review_remove']()}</Button>
					</li>
				{/each}
			</ul>
		{/if}
		<form class="unlock" onsubmit={addReviewRule}>
			<Field label={m['users.review_toon']()}>
				{#snippet children({ id, describedby, invalid })}
					<select {id} aria-describedby={describedby} aria-invalid={invalid} bind:value={ruleToon} data-testid="review-toon">
						<option value="*">{m['users.review_all_toons']()}</option>
						{#each toons as t (t.id)}<option value={t.id}>{t.name}</option>{/each}
					</select>
				{/snippet}
			</Field>
			<Field label={m['users.review_rule']()} hint={ruleOf(ruleKind)?.hint()}>
				{#snippet children({ id, describedby, invalid })}
					<select {id} aria-describedby={describedby} aria-invalid={invalid} bind:value={ruleKind} data-testid="review-rule">
						{#each RULES as r (r.id)}<option value={r.id}>{r.label()}</option>{/each}
					</select>
				{/snippet}
			</Field>
			<Button type="submit" disabled={working !== null}>{m['users.review_add']()}</Button>
		</form>
	</Section>

	<Section title={m['users.section_characters']()}>
		<ul class="unlocks" data-testid="character-unlocks">
			{#each user.characters ?? [] as u (u.character)}
				<li>
					<span>{u.character}</span>
					{#each u.looks as look (look)}<Badge tone="neutral">{look}</Badge> {/each}
					<span class="meta">{u.source}{u.ref ? ` · ${u.ref}` : ''}{u.unlocked_at ? ` · ${minute(u.unlocked_at)}` : ''}</span>
					{#if u.source !== 'default'}
						<Button variant="clear" size="sm" disabled={working !== null} onclick={() => lock(u.character)}>{m['users.unlock_revoke']()}</Button>
					{/if}
				</li>
			{/each}
		</ul>
		<form class="unlock" onsubmit={unlock}>
			<Field label={m['users.character_id']()}>
				{#snippet children({ id, describedby, invalid })}
					<input {id} aria-describedby={describedby} aria-invalid={invalid} bind:value={unlockCharacter} placeholder="yellowmouse" data-testid="unlock-character" />
				{/snippet}
			</Field>
			<Field label={m['users.character_looks']()}>
				{#snippet children({ id, describedby, invalid })}
					<input {id} aria-describedby={describedby} aria-invalid={invalid} bind:value={unlockLooks} placeholder="biker/helmet" />
				{/snippet}
			</Field>
			<Button type="submit" disabled={working !== null || !unlockCharacter.trim()}>{m['users.unlock']()}</Button>
		</form>
	</Section>

	<Section title={m['users.section_codes']()}>
		<DataTable columns={codeColumns} rows={user.codes} rowKey={(c) => c.code} labels={tableLabels()}>
			{#snippet cell(c, column)}
				{#if column.id === 'code'}<a class="mono-code" href="/codes/{encodeURIComponent(c.code)}">{c.code}</a>
				{:else if column.id === 'edition_id'}{c.edition_id ?? '—'}{c.edition_number ? ` #${c.edition_number}` : ''}
				{:else if column.id === 'claimed_at'}{minute(c.claimed_at)}
				{:else if column.id === 'revoked_at'}{#if c.revoked_at}<Badge tone="error">{day(c.revoked_at)}</Badge>{:else}—{/if}
				{:else}{c.type}{/if}
			{/snippet}
		</DataTable>
	</Section>
{/if}

<style>
	.roles {
		display: flex;
		flex-direction: column;
		gap: var(--space-075);
	}
	.current {
		margin: var(--space-1) 0 0;
	}
	.unlocks {
		list-style: none;
		margin: 0 0 var(--space-1);
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	.unlocks li {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05);
	}
	.rules {
		list-style: none;
		margin: 0 0 var(--space-1);
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	.rules li {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05);
	}
	.unlocks .meta,
	.meta {
		font-size: var(--type-caption-size);
		opacity: 0.7;
	}
	.unlock {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		gap: var(--space-1);
	}
</style>
