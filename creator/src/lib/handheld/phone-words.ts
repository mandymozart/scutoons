// The phone page's few words (#126): the barebone page loads no paraglide bundle (its locale modules hold every creator
// message), so its dozen strings live here in de + en – the language the phone's browser prefers, as the creator's own
// paraglide strategy does without its cookie.
const EN = {
	title: 'Handheld camera',
	connecting: 'Connecting to the PC …',
	connected: 'Connected',
	noCode: 'This link has no valid code – scan the QR code in the Handheld panel again.',
	expired: 'This code is used or ran out – show a new QR code on the PC.',
	failed: 'No connection – the API or the relay cannot be reached.',
	closed: 'The PC left – show a new QR code there.',
	off: 'The handheld camera is off for this shot – switch it on in the PC’s Handheld panel.',
	start: 'Start',
	startAr: 'Start (AR)',
	rotationOnly: 'Rotation only',
	startHint: 'Hold the phone like the camera, then start. The PC shows the shot through it.',
	record: 'Record',
	stop: 'Stop',
	recenter: 'Recenter',
	recenterHint: 'The phone’s pose now = the shot’s camera',
	modeXr: 'position + rotation',
	modeOrientation: 'rotation · drag to move',
	trackingLost: 'tracking lost',
	noSensor: 'This browser gives the page no motion sensor.',
	dragHint: 'Drag: one finger moves, two fingers lift / lower.',
	stage: 'The camera – drag to move'
};
export type PhoneWords = typeof EN;
const DE: PhoneWords = {
	title: 'Handheld-Kamera',
	connecting: 'Verbinde mit dem PC …',
	connected: 'Verbunden',
	noCode: 'Dieser Link hat keinen gültigen Code – den QR-Code im Handheld-Panel noch einmal scannen.',
	expired: 'Dieser Code ist benutzt oder abgelaufen – am PC einen neuen QR-Code zeigen.',
	failed: 'Keine Verbindung – die API oder das Relay sind nicht erreichbar.',
	closed: 'Der PC ist weg – dort einen neuen QR-Code zeigen.',
	off: 'Die Handheld-Kamera ist für diesen Shot aus – im Handheld-Panel am PC einschalten.',
	start: 'Starten',
	startAr: 'Starten (AR)',
	rotationOnly: 'Nur Drehung',
	startHint: 'Das Handy wie die Kamera halten, dann starten. Der PC zeigt den Shot dadurch.',
	record: 'Aufnehmen',
	stop: 'Stopp',
	recenter: 'Neu ausrichten',
	recenterHint: 'Die Lage des Handys jetzt = die Kamera des Shots',
	modeXr: 'Position + Drehung',
	modeOrientation: 'Drehung · ziehen zum Bewegen',
	trackingLost: 'Tracking verloren',
	noSensor: 'Dieser Browser gibt der Seite keinen Lagesensor.',
	dragHint: 'Ziehen: ein Finger bewegt, zwei Finger heben / senken.',
	stage: 'Die Kamera – ziehen zum Bewegen'
};

/** the words in the browser's preferred language (de or en) */
export function phoneWords(languages: readonly string[] = typeof navigator === 'undefined' ? [] : navigator.languages ?? [navigator.language]): { words: PhoneWords; lang: 'de' | 'en' } {
	const de = languages.find((l) => /^(de|en)\b/i.test(l))?.toLowerCase().startsWith('de') ?? false;
	return de ? { words: DE, lang: 'de' } : { words: EN, lang: 'en' };
}
