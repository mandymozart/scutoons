<script lang="ts">
	import type { Event } from '$lib/models/content.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { formatDate } from '$lib/utils/date.ts';
	import type { PageProps } from './$types.js';

	let { data }: PageProps = $props();
	let events = data.events ?? [];

	// Separate upcoming and past events
	const now = new Date();
	const upcomingEvents = events
		.filter((event: Event) => new Date(event.date) > now)
		.sort((a: Event, b: Event) => new Date(a.date).getTime() - new Date(b.date).getTime());

	const pastEvents = events
		.filter((event: Event) => new Date(event.date) <= now)
		.sort((a: Event, b: Event) => new Date(b.date).getTime() - new Date(a.date).getTime());
</script>

<article class="events">
	<header>
		<img src="/SCUTOONS-logo.svg" alt="SCUTOONS" />
	</header>
	<section>
		{#if upcomingEvents.length > 0}
			<div class="events-section">
				<h2>{m.upcoming_events()}</h2>
				<div class="event-list">
					{#each upcomingEvents as event}
						<div class="event-item">
							<span class="event-item-date">{formatDate(new Date(event.date))}</span>
							<span class="event-item-location">
								&mdash; {event.city}{#if event.location}, {event.location}{/if}
							</span>
							<span class="event-item-title">&mdash; {event.title}</span>
							{#if event.link}
								<a href={event.link} target="_blank" class="event-item-link">Link →</a>
							{/if}
						</div>
					{/each}
				</div>
			</div>
		{/if}

		{#if pastEvents.length > 0}
			<div class="events-section">
				<h2>{m.past_events()}</h2>
				<div class="event-list">
					{#each pastEvents as event}
						<div class="event-item past-event">
							<span class="event-item-date">{formatDate(new Date(event.date))}</span>
							<span class="event-item-location">
								&mdash; {event.city}{#if event.location}, {event.location}{/if}
							</span>
							<span class="event-item-title">&mdash; {event.title}</span>
							{#if event.link}
								<a href={event.link} target="_blank" class="event-item-link">Link →</a>
							{/if}
						</div>
					{/each}
				</div>
			</div>
		{/if}

		{#if upcomingEvents.length === 0 && pastEvents.length === 0}
			<p>No events at this time. Please check back later!</p>
		{/if}
	</section>
</article>

<style>
	/* Events Styling */
img {
	width: 100%;
}
.events {
	
}
	.events-section {
		font-family: 'IBM Plex Mono', monospace;
		font-size: 1rem;
		margin-bottom: 1rem;
		padding: 0 1rem 0.5rem 1rem;
		text-transform: uppercase;
	}
	.event-item.past-event {
		text-decoration: line-through;
	}
	.event-item:hover {
		font-style: italic;
	}
</style>
