export interface QRCodeOptions {
	/** the message (URL) */
	msg: string;
	/** width/height in px (default 256) */
	dim?: number;
	/** quiet zone in modules (default 4) */
	pad?: number;
	/** mask pattern 0–7, -1 = auto */
	mtx?: number;
	/** error correction level L | M | Q | H */
	ecl?: 'L' | 'M' | 'Q' | 'H';
	/** error correction boost (default 1) */
	ecb?: number;
	/** [color, background] */
	pal?: string[];
	vrb?: number;
}

/** datalog/qrcode-svg: returns an <svg> element */
declare function QRCode(options: QRCodeOptions | string): SVGSVGElement;
export default QRCode;
