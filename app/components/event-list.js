import { API_BASE } from '../config.js';
import { renderEventItem } from './event-item.js';

export class EventListComponent {
	scutoonPages;
	params;
	
	constructor(scutoonPages) {
		this.scutoonPages = scutoonPages;
	}

	init(params = {}) {
		this.params = params;
		this.initList();
	}

	initList() {
		// Load events data and display them
		fetch(`${API_BASE}/events.json`)
			.then((response) => response.json())
			.then((data) => {
				const now = new Date();
				const events = data.events.map(event => ({
					...event,
					dateObj: new Date(event.date)
				}));
				
				// Filter upcoming and past events
				const upcomingEvents = events.filter((event) => event.dateObj > now);
				const pastEvents = events.filter((event) => event.dateObj <= now);
				
				// Sort upcoming events by date (earliest first)
				upcomingEvents.sort((a, b) => a.dateObj - b.dateObj);
				
				// Sort past events by date (most recent first)
				pastEvents.sort((a, b) => b.dateObj - a.dateObj);
				
				// Render the events
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
		
		// Render upcoming events
		if (upcomingEvents.length > 0) {
			html += '<div class="events-section">';
			upcomingEvents.forEach(event => {
				html += renderEventItem(event);
			});
			html += '</div>';
		}
		
		// Render past events
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
