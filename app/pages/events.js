import { EventListComponent } from '../components/event-list.js';

export class EventsPage {
  scutoonPages;
  constructor(scutoonPages) {
    this.scutoonPages = scutoonPages;
  }

  init(params = {}) {
    console.log('EventsPage initialized', params);
    const eventListComponent = new EventListComponent(this.scutoonPages);
    eventListComponent.init(params);
  }
}