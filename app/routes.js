import { HomePage } from './pages/home.js';
import { AttributionsPage } from './pages/attributions.js';
import { EventsPage } from './pages/events.js';

export const routes = [
	{
		path: '',
		component: HomePage
	},
	{
		path: 'attributions',
		component: AttributionsPage
	},
	{
		path: 'events',
		component: EventsPage
	}
];