<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { fxSettings } from '$lib/fx-settings.svelte.ts';
	import { mediaQuality } from '$lib/media-quality.svelte.ts';
	import { readerName } from '$lib/reader-name.svelte.ts';
	import { autoSave } from '$lib/offline/auto-save.svelte.ts';
	import { savedChapters } from '$lib/offline/saved-chapters.svelte.ts';
	import { draftPreview } from '$lib/draft-preview.svelte.ts';
	import { push } from '$lib/push.svelte.ts';
	import { install } from '$lib/install.svelte.ts';
	import { roundMb } from '$lib/sw/saved-rules.ts';
	import { GROUP_ICON, groupScope, type AccountGroup, type AccountReader } from '$lib/account/groups.ts';
	import { groupTitle } from './group-title.ts';

	/**
	 * The account page's index (owner 2026-10-16, concept B): one row per group – its icon, title, where it is kept
	 * (a "Konto" / "Gerät" badge where the whole group is one or the other) and a summary of its values, so most readers
	 * never open it. A row is a link to the group's page `/account/<id>`; `current` marks the one shown beside it
	 * (desktop).
	 */
	let { groups, reader, current = null }: { groups: AccountGroup[]; reader: AccountReader; current?: AccountGroup | null } = $props();

	const session = getSessionContext();
	const inventory = getInventoryContext();
	const onOff = (what: string, on: boolean) => (on ? m['account.sum_on']({ what }) : m['account.sum_off']({ what }));

	function summary(group: AccountGroup): string {
		const user = session.isLoggedIn ? session.user : null;
		switch (group) {
			case 'details':
				return readerName.value ?? user?.email ?? '';
			case 'reading': {
				const quality = m[`media_quality.${mediaQuality.setting ?? 'auto'}`]();
				if (!user) return quality;
				const hints = onOff(m['account.sum_hints'](), user.show_hints === true);
				const sensitive = user.hide_sensitive !== false ? m['chapter.sensitive_hidden']() : m['account.sum_sensitive_shown']();
				return `${quality} · ${hints} · ${sensitive}`;
			}
			case 'sound':
				return `${Math.round(fxSettings.volume * 100)} % · ${onOff(m['fx.haptics'](), fxSettings.haptics)}`;
			case 'offline': {
				const count = savedChapters.list.length;
				const parts = install.installed ? [m['pwa.sum_installed']()] : [];
				if (savedChapters.supported) parts.push(m[`saved.auto_${autoSave.value}`]());
				if (count) parts.push(m['saved.summary']({ count, size: roundMb(savedChapters.bytes) }));
				return parts.join(' · ');
			}
			case 'notifications': {
				const parts = push.state === 'hidden' || push.state === 'install-first' ? [] : [onOff(m['account.sum_push'](), push.state === 'on')];
				if (user) parts.push(onOff(m['account.sum_news'](), !!user.marketing_opt_in));
				return parts.join(' · ');
			}
			case 'collection':
				return m['account.items_count']({ count: inventory.entries.reduce((sum, e) => sum + e.qty, 0) });
			case 'connections':
				return m['account.ai_title']();
			case 'decks':
				return m['account.decks_summary']();
			case 'creator':
				return onOff(m['account.draft_preview'](), draftPreview.on);
		}
	}
</script>

<nav aria-label={m['account.settings']()}>
	<ul>
		{#each groups as group (group)}
			{@const scope = groupScope(group, reader)}
			<li>
				<a href={localizeHref(`/account/${group}`)} aria-current={current === group ? 'page' : undefined} data-testid="account-group-{group}">
					<Icon name={GROUP_ICON[group]} size="1rem" />
					<span class="text">
						<span class="title">
							{groupTitle(group)}
							{#if scope}<span class="sc-chip">{scope === 'device' ? m['account.scope_device']() : m['account.scope_account']()}</span>{/if}
						</span>
						<small>{summary(group)}</small>
					</span>
					<Icon name="chevron-right" size="1rem" />
				</a>
			</li>
		{/each}
	</ul>
</nav>

<style>
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		border-top: 2px solid var(--ink);
	}
	li {
		border-bottom: 2px solid var(--ink);
	}
	a {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		align-items: center;
		gap: 0.75rem;
		min-height: calc(var(--control-height) + 1rem);
		padding: 0.5rem 0.25rem;
		color: var(--ink);
		text-decoration: none;
	}
	a:hover {
		background: var(--surface-hover);
	}
	a:focus-visible {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: -4px;
	}
	/* the group shown beside the index (desktop): inverted, like a chosen segment */
	a[aria-current='page'] {
		background: var(--ink);
		color: var(--paper);
	}
	a[aria-current='page'] small {
		color: var(--border-muted);
	}
	a[aria-current='page'] .sc-chip {
		background: var(--paper);
		color: var(--ink);
	}
	.text {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
		min-width: 0;
	}
	.title {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-weight: 700;
	}
	small {
		font-size: 0.75rem;
		line-height: 1.3;
		color: var(--text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	@media (prefers-reduced-motion: no-preference) {
		a {
			transition: background-color 0.1s ease;
		}
	}
</style>
