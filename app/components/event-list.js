import { API_BASE } from '../config.js';
import { renderEventItem } from './event-item.js';

export class EventListComponent {
	scutoonPages;
	
	constructor(scutoonPages) {
		this.scutoonPages = scutoonPages;
	}

	init() {
		this.initList();
	}

	initList() {
		fetch(`${API_BASE}/events.json`)
			.then((response) => response.json())
			.then((data) => {
				const now = new Date();
				const events = data.events.map(event => ({
					...event,
					dateObj: new Date(event.date)
				}));
				
				const upcomingEvents = events.filter((event) => event.dateObj > now);
				const pastEvents = events.filter((event) => event.dateObj <= now);
				upcomingEvents.sort((a, b) => a.dateObj - b.dateObj);
				pastEvents.sort((a, b) => b.dateObj - a.dateObj);
				this.renderEvents(upcomingEvents, pastEvents);
			})
			.catch((error) => {
				console.error("Error loading events:", error);
			});
	}

	renderEvents(upcomingEvents, pastEvents) {
		const container = document.getElementById("events-container");
		if (!container) {
			console.error("Events container not found");
			return;
		}
		
		let html = "";
		
		if (upcomingEvents.length > 0) {
			html += '<div class="events-section">';
			upcomingEvents.forEach(event => {
				html += renderEventItem(event);
			});
			html += '</div>';
		}
		
		if (pastEvents.length > 0) {
			html += '<div class="events-section">';
			pastEvents.forEach(event => {
				html += renderEventItem(event, true);
			});
			html += '</div>';
		}
		
		container.innerHTML = html;
	}
}
