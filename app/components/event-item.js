import { formatDate } from '../utils/date.js';

export function renderEventItem(event, isPast = false) {
		const dateStr = formatDate(event.dateObj);
		const pastClass = isPast ? 'past-event' : '';
		
		let locationHtml = event.city;
		if (event.location) {
			locationHtml += ` - ${event.location}`;
		}
		
		let cardHtml = `
			<div class="event-item ${pastClass}">
				<span class="event-item-date">${dateStr}</span>
				<span class="event-item-location">&mdash; ${locationHtml}</span>
				<span class="event-item-title">&mdash; ${event.title}</span>
                `;
		
		if (event.link) {
			cardHtml += `<a href="${event.link}" target="_blank" class="event-item-link">Link →</a>`;
		}
		
		cardHtml += `</div>`;
		
		return cardHtml;
	}