/**
 * Shared QR Display Component
 * Handles rendering QR codes and displaying results for both Generate and Regenerate views
 */
export default class QRDisplay {
	constructor(containerPrefix = '') {
		this.prefix = containerPrefix;
		
		// DOM elements (with optional prefix for multiple instances)
		// Pattern: qrCode (no prefix) or regenQrCode (with prefix - capital Q)
		const q = containerPrefix ? 'Q' : 'q';
		const r = containerPrefix ? 'R' : 'r';
		const c = containerPrefix ? 'C' : 'c';
		const s = containerPrefix ? 'S' : 's';
		const a = containerPrefix ? 'A' : 'a';
		const f = containerPrefix ? 'F' : 'f';
		
		this.qrCodeEl = document.getElementById(`${containerPrefix}${q}rCode`);
		this.qrResultEl = document.getElementById(`${containerPrefix}${q}rResult`);
		this.qrUrlEl = document.getElementById(`${containerPrefix}${q}rUrl`);
		this.redirectToEl = document.getElementById(`${containerPrefix}${r}edirectTo`);
		this.codeEl = document.getElementById(`${containerPrefix}${c}ode`);
		this.signatureInfoEl = document.getElementById(`${containerPrefix}${s}ignatureInfo`);
		this.sigAlgorithmEl = document.getElementById(`${containerPrefix}${s}ig${a}lgorithm`);
		this.sigFingerprintEl = document.getElementById(`${containerPrefix}${s}ig${f}ingerprint`);
		
		// Store rendered SVG element
		this.svgElement = null;
	}

	/**
	 * Generate and render QR code
	 * @param {string} url - URL to encode in QR
	 * @returns {SVGElement} The generated SVG element
	 */
	renderQRCode(url) {
		console.log('QRDisplay.renderQRCode called with URL:', url);
		console.log('qrCodeEl exists:', !!this.qrCodeEl);
		
		if (!this.qrCodeEl) return null;
		
		// Clear previous QR code
		while (this.qrCodeEl.firstChild) {
			this.qrCodeEl.removeChild(this.qrCodeEl.firstChild);
		}

		// Generate QR SVG using QRCode library (same as GenerateView)
		const qrSvg = new QRCode({
			msg: url,
			dim: 256,
			pad: 4,
			ecl: 'H',
			pal: ['#000000', '#FFFFFF']
		});

		this.qrCodeEl.appendChild(qrSvg);
		this.svgElement = qrSvg;
		
		console.log('QR SVG generated:', qrSvg);
		console.log('Stored as svgElement:', this.svgElement);
		
		return qrSvg;
	}

	/**
	 * Display QR information in the UI
	 * @param {Object} qrData - QR data (url, code, redirect_to, signature, fingerprint)
	 */
	displayInfo(qrData) {
		if (this.qrUrlEl) this.qrUrlEl.textContent = qrData.url;
		if (this.redirectToEl) this.redirectToEl.textContent = qrData.redirect_to;
		if (this.codeEl) this.codeEl.textContent = qrData.code;
		
		// Show signature info if available
		if (qrData.signature && this.signatureInfoEl) {
			if (this.sigAlgorithmEl) {
				this.sigAlgorithmEl.textContent = qrData.signature.algorithm;
			}
			if (this.sigFingerprintEl) {
				this.sigFingerprintEl.textContent = qrData.fingerprint;
			}
			this.signatureInfoEl.style.display = 'block';
		} else if (this.signatureInfoEl) {
			this.signatureInfoEl.style.display = 'none';
		}
		
		// Show result container
		if (this.qrResultEl) {
			this.qrResultEl.style.display = 'block';
		}
	}

	/**
	 * Get the current SVG element
	 * @returns {SVGElement|null}
	 */
	getSvgElement() {
		console.log('getSvgElement called, returning:', this.svgElement);
		return this.svgElement;
	}

	/**
	 * Clear the QR display
	 */
	clear() {
		if (this.qrCodeEl) this.qrCodeEl.innerHTML = '';
		if (this.qrResultEl) this.qrResultEl.style.display = 'none';
		this.svgElement = null;
	}
}
