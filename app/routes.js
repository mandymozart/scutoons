import { HomePage } from './pages/home.js';
import { AttributionsPage } from './pages/attributions.js';
import { EventsPage } from './pages/events.js';
/**
 * Routes for the app
 * 
 * Supports:
 * - Exact paths: 'events'
 * - Wildcards: 'events/*' (matches any path starting with events/)
 * - Parameters: 'events/:category/:id' (params available in component init)
 * 
 * Routes are matched in order, so put more specific routes before general ones.
 * 
 * @type {Array<{path: string, component: any}>}
 */
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
		path: 'events/:category/:id',
		component: EventsPage
	},
	{
		path: 'events/:category',
		component: EventsPage
	},
	{
		path: 'events',
		component: EventsPage
	}
];