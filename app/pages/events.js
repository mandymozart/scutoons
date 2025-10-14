import { EventListComponent } from '../components/event-list.js';

export class EventsPage {
  scutoonPages;
  constructor(scutoonPages) {
    this.scutoonPages = scutoonPages;
  }

  init() {
    console.log('EventsPage initialized');
    const eventListComponent = new EventListComponent(this.scutoonPages);
    eventListComponent.init();
  }
}