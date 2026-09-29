// QR SVGs exactly like the legacy generate/ SPA: datalog/qrcode-svg, 256px, pad 4, ECL H, black on white.
import QRCode from './qrcode-svg.js';

export function qrSvgElement(url: string): SVGSVGElement {
	return QRCode({ msg: url, dim: 256, pad: 4, ecl: 'H', pal: ['#000000', '#FFFFFF'] });
}

export function serializeSvg(svg: SVGSVGElement): string {
	return new XMLSerializer().serializeToString(svg);
}

/** SVG markup of the QR code for a URL */
export const qrSvg = (url: string) => serializeSvg(qrSvgElement(url));

/** `SCUTOONS-QR-<TYPE>-<code>.svg` (archive + generate download) */
export const svgFilename = (type: string, code: string) => `SCUTOONS-QR-${type.toUpperCase()}-${code}.svg`;
export const coaFilename = (code: string) => `SCUTOONS-COA-${code}.pdf`;
export const qrPrintFilename = (code: string) => `SCUTOONS-QR-PRINT-${code}.pdf`;
export const signatureFilename = (code: string) => `SIGNATURE-${code}.json`;
