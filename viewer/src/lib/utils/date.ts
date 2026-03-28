export function formatDate(date: Date): string {
	const options = {
		year: 'numeric',
		month: '2-digit',
		day: '2-digit',
		hour: '2-digit',
		minute: '2-digit'
	} as const;
	return date.toLocaleDateString('de-DE', options).replace(',', ' -');
}
