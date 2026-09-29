import { generateCoaPdf, generateQrPrintPdf } from '../utils/pdf.js';

/**
 * Shared Download Buttons Component
 * Handles download functionality for SVG, PDFs, and signature files
 */
export default class DownloadButtons {
	constructor(prefix = '', qrDisplayComponent) {
		this.prefix = prefix;
		this.qrDisplay = qrDisplayComponent;
		
		// DOM elements
		// Pattern: downloadBtn (no prefix) or regenDownloadBtn (with prefix - capital D)
		const d = prefix ? 'D' : 'd';
		this.downloadBtn = document.getElementById(`${prefix}${d}ownloadBtn`);
		this.downloadCoaBtn = document.getElementById(`${prefix}${d}ownloadCoaBtn`);
		this.downloadQrPrintBtn = document.getElementById(`${prefix}${d}ownloadQrPrintBtn`);
		this.downloadSigBtn = document.getElementById(`${prefix}${d}ownloadSigBtn`);
		
		// Debug: Log which buttons were found
		console.log('DownloadButtons initialized with prefix:', prefix);
		console.log('SVG button found:', !!this.downloadBtn);
		console.log('CoA button found:', !!this.downloadCoaBtn);
		console.log('QR Print button found:', !!this.downloadQrPrintBtn);
		console.log('Sig button found:', !!this.downloadSigBtn);
		
		// Current QR data
		this.qrData = null;
		this.templateData = null;
		
		// Bind event handlers
		this.init();
	}

	init() {
		if (this.downloadBtn) {
			this.downloadBtn.addEventListener('click', () => this.downloadSvg());
		}
		if (this.downloadCoaBtn) {
			this.downloadCoaBtn.addEventListener('click', () => this.downloadCoaPdf());
		}
		if (this.downloadQrPrintBtn) {
			this.downloadQrPrintBtn.addEventListener('click', () => this.downloadQrPrintPdf());
		}
		if (this.downloadSigBtn) {
			this.downloadSigBtn.addEventListener('click', () => this.downloadSignature());
		}
	}

	/**
	 * Set QR data for downloads
	 * @param {Object} qrData - QR data (code, type, url, signature, etc.)
	 * @param {Object} templateData - Template field data (receiver, productname, etc.)
	 */
	setData(qrData, templateData = {}) {
		this.qrData = qrData;
		this.templateData = templateData;
	}

	/**
	 * Show/hide buttons based on QR type and signature availability
	 * @param {string} type - QR type (membership, promo, event, xr)
	 * @param {boolean} hasSignature - Whether signature exists
	 */
	updateVisibility(type, hasSignature) {
		// Always show SVG download
		if (this.downloadBtn) {
			this.downloadBtn.style.display = 'inline-block';
		}
		
		// Show PDF buttons based on type
		if (this.downloadCoaBtn) {
			this.downloadCoaBtn.style.display = type === 'membership' ? 'inline-block' : 'none';
		}
		if (this.downloadQrPrintBtn) {
			this.downloadQrPrintBtn.style.display = type === 'promo' ? 'inline-block' : 'none';
		}
		
		// Show signature download if signature exists
		if (this.downloadSigBtn) {
			this.downloadSigBtn.style.display = hasSignature ? 'inline-block' : 'none';
		}
	}

	/**
	 * Hide all download buttons
	 */
	hideAll() {
		if (this.downloadBtn) this.downloadBtn.style.display = 'none';
		if (this.downloadCoaBtn) this.downloadCoaBtn.style.display = 'none';
		if (this.downloadQrPrintBtn) this.downloadQrPrintBtn.style.display = 'none';
		if (this.downloadSigBtn) this.downloadSigBtn.style.display = 'none';
	}

	/**
	 * Download SVG file
	 */
	downloadSvg() {
		const svgElement = this.qrDisplay.getSvgElement();
		if (!svgElement) {
			console.error('No SVG element available');
			return;
		}
		
		const svgString = new XMLSerializer().serializeToString(svgElement);
		const blob = new Blob([svgString], { type: 'image/svg+xml' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `qr-${this.qrData.code}.svg`;
		a.click();
		URL.revokeObjectURL(url);
	}

	/**
	 * Download Certificate of Authenticity PDF
	 */
	async downloadCoaPdf() {
		const svgElement = this.qrDisplay.getSvgElement();
		console.log('Download CoA clicked');
		console.log('QR Data:', this.qrData);
		console.log('Template Data:', this.templateData);
		console.log('SVG Element:', svgElement);
		
		if (!this.qrData || !svgElement) {
			console.error('Missing QR data or SVG element');
			console.error('Has qrData:', !!this.qrData);
			console.error('Has svgElement:', !!svgElement);
			return;
		}
		
		try {
			const pdf = await generateCoaPdf(this.qrData, this.templateData, svgElement);
			pdf.save(`certificate-${this.qrData.code}.pdf`);
		} catch (error) {
			console.error('Error generating CoA PDF:', error);
			throw error;
		}
	}

	/**
	 * Download QR Print PDF
	 */
	async downloadQrPrintPdf() {
		const svgElement = this.qrDisplay.getSvgElement();
		if (!this.qrData || !svgElement) {
			console.error('Missing QR data or SVG element');
			return;
		}
		
		try {
			const pdf = await generateQrPrintPdf(this.qrData, this.templateData, svgElement);
			pdf.save(`qr-print-${this.qrData.code}.pdf`);
		} catch (error) {
			console.error('Error generating QR print PDF:', error);
			throw error;
		}
	}

	/**
	 * Download signature JSON file
	 */
	downloadSignature() {
		if (!this.qrData || !this.qrData.signature) {
			console.error('No signature data available');
			return;
		}
		
		const signatureData = {
			...this.qrData.signature,
			code: this.qrData.code,
			type: this.qrData.type,
			created_at: this.qrData.created_at
		};
		
		const blob = new Blob([JSON.stringify(signatureData, null, 2)], { type: 'application/json' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `signature-${this.qrData.code}.json`;
		a.click();
		URL.revokeObjectURL(url);
	}
}
