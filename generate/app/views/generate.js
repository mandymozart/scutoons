import { generateQRCode } from '../utils/api.js';
import { generateCoaPdf, generateQrPrintPdf } from '../utils/pdf.js';
import { signQRCode, generateFingerprint, exportSignature } from '../utils/crypto.js';
import { QR_BASE_URL, QR_DOMAIN } from '../config.js';

export default class GenerateView {
	// Fields to exclude from template data (QR-specific system fields)
	static EXCLUDED_TEMPLATE_FIELDS = ['redirectUrl', 'type', 'expiresAt', 'code', 'toon', 'edition_select', 'edition_id'];

	constructor() {
		// State
		this.isSubmitting = false;
		this.currentQR = null; // { code, type, redirect_to, url, svg }
		this.templateData = {}; // Store user input for PDF generation

		// DOM elements
		this.formEl = document.getElementById('qrForm');
		this.downloadBtnEl = document.getElementById('downloadBtn');
		this.downloadCoaBtnEl = document.getElementById('downloadCoaBtn');
		this.downloadQrPrintBtnEl = document.getElementById('downloadQrPrintBtn');
		this.downloadSigBtnEl = document.getElementById('downloadSigBtn');
		this.messageEl = document.getElementById('message');
		this.qrCodeEl = document.getElementById('qrCode');
		this.qrResultEl = document.getElementById('qrResult');
		this.qrUrlEl = document.getElementById('qrUrl');
		this.redirectToEl = document.getElementById('redirectTo');
		this.codeEl = document.getElementById('code');

		// Attach event listeners
		this.attachEventListeners();
	}

	attachEventListeners() {
		if (this.formEl) {
			this.formEl.addEventListener('submit', (e) => this.handleFormSubmit(e));
		}

		if (this.downloadBtnEl) {
			this.downloadBtnEl.addEventListener('click', () => this.handleDownload());
		}

		if (this.downloadCoaBtnEl) {
			this.downloadCoaBtnEl.addEventListener('click', () => this.generateCoaPdf());
		}

		if (this.downloadQrPrintBtnEl) {
			this.downloadQrPrintBtnEl.addEventListener('click', () => this.generateQrPrintPdf());
		}

		if (this.downloadSigBtnEl) {
			this.downloadSigBtnEl.addEventListener('click', () => this.downloadSignature());
		}
	}

	async saveFile(content, filename, type) {
		try {
			const formData = new FormData();
			formData.append('doc', content);
			formData.append('filename', filename);
			formData.append('type', type);

			const response = await fetch('files.php', {
				method: 'POST',
				body: formData
			});

			const result = await response.json();
			if (result.success) {
				console.log(`File saved: ${type}/${filename}`);
			} else {
				console.error('File save error:', result.error);
			}
		} catch (error) {
			console.error('Error saving file:', error);
		}
	}

	async saveSignatureToDb(code, signatureData) {
		try {
			const response = await fetch('save-signature.php', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({
					code: code,
					signature: signatureData
				})
			});

			const result = await response.json();
			if (result.success) {
				console.log('✅ Signature saved to database for code:', code);
			} else {
				console.error('❌ Database signature save error:', result.message);
			}
		} catch (error) {
			console.error('❌ Error saving signature to database:', error);
		}
	}

	renderQRCode(qrData) {
		// Clear previous QR code
		while (this.qrCodeEl.firstChild) {
			this.qrCodeEl.removeChild(this.qrCodeEl.firstChild);
		}

		// Generate QR SVG
		const qrSvg = new QRCode({
			msg: qrData.url,
			dim: 256,
			pad: 4,
			ecl: 'H',
			pal: ['#000000', '#FFFFFF']
		});

		this.qrCodeEl.appendChild(qrSvg);

		// Update state with the generated SVG
		if (this.currentQR) {
			this.currentQR.svg = qrSvg;
		}

		// Archive SVG to server
		const svgString = new XMLSerializer().serializeToString(qrSvg);
		const filename = `SCUTOONS-QR-${this.currentQR.type.toUpperCase()}-${this.currentQR.code}.svg`;
		this.saveFile(svgString, filename, 'svgs');
	}

	renderQRInfo(qrData) {
		this.qrUrlEl.textContent = qrData.url;
		this.redirectToEl.textContent = qrData.redirect_to;
		this.codeEl.textContent = qrData.code;
		this.qrResultEl.style.display = 'block';
		
		// Show post-quantum signature info if available
		if (qrData.signature) {
			const sigInfoEl = document.getElementById('signatureInfo');
			const sigAlgorithmEl = document.getElementById('sigAlgorithm');
			const sigFingerprintEl = document.getElementById('sigFingerprint');
			
			if (sigInfoEl && sigAlgorithmEl && sigFingerprintEl) {
				sigAlgorithmEl.textContent = qrData.signature.algorithm;
				sigFingerprintEl.textContent = qrData.fingerprint;
				sigInfoEl.style.display = 'block';
			}
		}
		
		// Always show SVG download
		this.downloadBtnEl.style.display = 'inline-block';

		// Show PDF download buttons based on type
		// Membership: Certificate of Authenticity only
		// Promo: QR Print PDF only
		// Event/XR: No PDFs
		if (this.downloadCoaBtnEl) {
			this.downloadCoaBtnEl.style.display = qrData.type === 'membership' ? 'inline-block' : 'none';
		}
		
		if (this.downloadQrPrintBtnEl) {
			this.downloadQrPrintBtnEl.style.display = qrData.type === 'promo' ? 'inline-block' : 'none';
		}
		
		// Show signature download if signature exists
		if (this.downloadSigBtnEl) {
			this.downloadSigBtnEl.style.display = qrData.signature ? 'inline-block' : 'none';
		}
	}

	renderMessage(message, type = 'success') {
		this.messageEl.innerHTML = message ? `<div class="${type}">${message}</div>` : '';
	}

	// Event Handlers
	async handleFormSubmit(e) {
		e.preventDefault();

		// Prevent duplicate submissions
		if (this.isSubmitting) {
			console.log('Form submission already in progress');
			return;
		}

		this.isSubmitting = true;

		const submitButton = e.target.querySelector('button[type="submit"]');
		const originalButtonText = submitButton.textContent;
		submitButton.disabled = true;
		submitButton.textContent = 'Generating...';
		this.renderMessage('');

		try {
			// Generate unique code
			const code = window.nanoid(8);

			// Prepare form data
			const formData = new FormData(e.target);
			formData.append('code', code);

			// Store template data for PDF generation - collect all form fields dynamically
			this.templateData = {};
			for (const [key, value] of formData.entries()) {
				// Skip QR-specific fields that aren't template variables
				if (!GenerateView.EXCLUDED_TEMPLATE_FIELDS.includes(key)) {
					this.templateData[key] = value || '';
				}
			}

			// Call API
			const result = await generateQRCode(formData);

			if (result.success) {
				// Update state with new QR data including ALL template fields
				const qrUrl = QR_BASE_URL + result.code;
				this.currentQR = {
					code: result.code,
					type: result.type,
					redirect_to: result.redirect_to,
					url: qrUrl,
					created_at: new Date().toISOString(),
					expires_at: result.expires_at || null,
					...this.templateData,  // Include ALL template fields for signing
					svg: null
				};

				// Generate post-quantum cryptographic signature
				// Signs ALL fields including template data (receiver, edition, artists, etc.)
				try {
					const signatureData = await signQRCode(this.currentQR);
					this.currentQR.signature = signatureData;
					this.currentQR.fingerprint = generateFingerprint(signatureData);
					
					// Export signature for storage and verification
					const signatureExport = exportSignature(signatureData);
					
					// Save full signature to server (for audit trail and verification)
					const sigFilename = `SIGNATURE-${this.currentQR.code}.json`;
					const sigData = JSON.stringify({
						code: this.currentQR.code,
						type: this.currentQR.type,
						signature: signatureData,
						fingerprint: this.currentQR.fingerprint,
						created_at: this.currentQR.created_at
					}, null, 2);
					await this.saveFile(sigData, sigFilename, 'signatures');
					
					// Save signature to database (template data already in signedData)
					await this.saveSignatureToDb(this.currentQR.code, signatureData);
					
					// Log for user to copy (can be used for manual verification)
					console.log('═══════════════════════════════════════════════');
					console.log('POST-QUANTUM SIGNATURE GENERATED');
					console.log('═══════════════════════════════════════════════');
					console.log('QR Code:', this.currentQR.code);
					console.log('Fingerprint:', this.currentQR.fingerprint);
					console.log('\nTo verify this certificate:');
					console.log('1. Visit: ' + QR_DOMAIN + '/generate/verify.html');
					console.log('2. Enter code:', this.currentQR.code);
					console.log('\nFull signature export (base64):');
					console.log(signatureExport);
					console.log('═══════════════════════════════════════════════');
					
					// Store signature export in QR data for PDF generation
					this.currentQR.signatureExport = signatureExport;
				} catch (error) {
					console.error('Failed to sign QR code:', error);
					// Continue without signature - non-critical
				}

				// Render views from state
				this.renderQRCode(this.currentQR);
				this.renderQRInfo(this.currentQR);
				this.renderMessage('QR Code generated successfully!', 'success');

			} else {
				this.renderMessage('Error: ' + result.message, 'error');
			}
		} catch (error) {
			console.error('Generation error:', error);
			this.renderMessage('Error generating QR code: ' + error.message, 'error');
		} finally {
			this.isSubmitting = false;
			submitButton.disabled = false;
			submitButton.textContent = originalButtonText;
		}
	}

	handleDownload() {
		if (!this.currentQR || !this.currentQR.svg) {
			console.error('No QR code in state');
			return;
		}

		// Use the stored SVG element from state
		const svgElement = this.currentQR.svg;

		if (!svgElement) {
			console.error('No SVG element found');
			return;
		}

		// Serialize and download
		const svgString = new XMLSerializer().serializeToString(svgElement);
		const blob = new Blob([svgString], { type: 'image/svg+xml' });
		const url = URL.createObjectURL(blob);

		const a = document.createElement('a');
		a.href = url;
		a.download = `SCUTOONS-QR-${this.currentQR.type.toUpperCase()}-${this.currentQR.code}.svg`;
		document.body.appendChild(a);
		a.click();
		document.body.removeChild(a);

		URL.revokeObjectURL(url);
	}

	async generateCoaPdf() {
		if (!this.currentQR) {
			this.renderMessage('Please generate a QR code first', 'error');
			return;
		}

		try {
			const doc = await generateCoaPdf(this.currentQR, this.templateData, this.currentQR.svg);
			
			// Save PDF locally
			const filename = `SCUTOONS-COA-${this.currentQR.code}.pdf`;
			doc.save(filename);
			
			// Archive PDF to server
			const pdfData = doc.output('datauristring');
			await this.saveFile(pdfData, filename, 'certificates');
			
			this.renderMessage('Certificate of Authenticity generated!', 'success');
		} catch (error) {
			console.error('Error generating CoA PDF:', error);
			this.renderMessage('Error generating Certificate of Authenticity', 'error');
		}
	}

	async generateQrPrintPdf() {
		if (!this.currentQR) {
			this.renderMessage('Please generate a QR code first', 'error');
			return;
		}

		try {
			const doc = await generateQrPrintPdf(this.currentQR, this.templateData, this.currentQR.svg);
			
			// Save PDF locally
			const filename = `SCUTOONS-QR-PRINT-${this.currentQR.code}.pdf`;
			doc.save(filename);
			
			// Archive PDF to server
			const pdfData = doc.output('datauristring');
			await this.saveFile(pdfData, filename, 'certificates');
			
			this.renderMessage('Printable QR certificate generated!', 'success');
		} catch (error) {
			console.error('Error generating QR Print PDF:', error);
			this.renderMessage('Error generating Printable QR certificate', 'error');
		}
	}

	downloadSignature() {
		if (!this.currentQR || !this.currentQR.signature) {
			this.renderMessage('No signature available to download', 'error');
			return;
		}

		try {
			// Create signature data object
			const sigData = {
				code: this.currentQR.code,
				type: this.currentQR.type,
				signature: this.currentQR.signature,
				fingerprint: this.currentQR.fingerprint,
				created_at: this.currentQR.created_at
			};

			// Convert to JSON and create blob
			const json = JSON.stringify(sigData, null, 2);
			const blob = new Blob([json], { type: 'application/json' });
			const url = URL.createObjectURL(blob);
			
			// Create download link and trigger
			const a = document.createElement('a');
			a.href = url;
			a.download = `SIGNATURE-${this.currentQR.code}.json`;
			document.body.appendChild(a);
			a.click();
			document.body.removeChild(a);
			URL.revokeObjectURL(url);
			
			this.renderMessage('Signature file downloaded!', 'success');
		} catch (error) {
			console.error('Error downloading signature:', error);
			this.renderMessage('Error downloading signature file', 'error');
		}
	}
}
