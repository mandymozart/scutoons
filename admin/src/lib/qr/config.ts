// PRINTED URLS – agents/PLATFORM.md "RULE: printed / public URLs never change". Every QR code and certificate carries
// these exact strings; they are constants on purpose (never from env) and must not change.
export const QR_DOMAIN = 'https://qr.scutoons.com';
export const QR_BASE_URL = QR_DOMAIN + '?c=';

/** `https://qr.scutoons.com?c=<CODE>` */
export const qrUrl = (code: string) => QR_BASE_URL + code;
