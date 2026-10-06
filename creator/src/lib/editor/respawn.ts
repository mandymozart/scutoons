import { intervalMinutes, type RecurrenceInterval } from '@scutoons/content';
import { m } from '$lib/paraglide/messages.js';

/** "every visit", "every 6 h", "every 2 days" … – the largest whole unit */
export function intervalLabel(interval: RecurrenceInterval): string {
	const minutes = intervalMinutes(interval) ?? 0;
	if (minutes === 0) return m['creator.respawn_every_visit']();
	if (minutes % 1440 === 0) return m['creator.respawn_every_days']({ n: minutes / 1440 });
	if (minutes % 60 === 0) return m['creator.respawn_every_hours']({ n: minutes / 60 });
	return m['creator.respawn_every_minutes']({ n: minutes });
}
