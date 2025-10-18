import { generateQRCode } from './utils/api.js';

class GenerateView {
	constructor() {
		// State
		this.isSubmitting = false;
		this.currentQR = null; // { code, type, redirect_to, url, svg }

		// DOM elements
		this.formEl = document.getElementById('qrForm');
		this.downloadBtnEl = document.getElementById('downloadBtn');
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
	}

	renderQRInfo(qrData) {
		this.qrUrlEl.textContent = qrData.url;
		this.redirectToEl.textContent = qrData.redirect_to;
		this.codeEl.textContent = qrData.code;
		this.qrResultEl.style.display = 'block';
		this.downloadBtnEl.style.display = 'inline-block';
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

			// Call API
			const result = await generateQRCode(formData);

			if (result.success) {
				// Update state with new QR data
				const qrUrl = `https://qr.scutoons.com?c=${result.code}`;
				this.currentQR = {
					code: result.code,
					type: result.type,
					redirect_to: result.redirect_to,
					url: qrUrl,
					svg: null
				};

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

		const svgElement = this.qrCodeEl.querySelector('svg');

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
}

// App Initialization
const App = {
	init() {
		new GenerateView();
		console.log('QR Generator App initialized');
	}
};

// Start app when DOM is ready
if (document.readyState === 'loading') {
	document.addEventListener('DOMContentLoaded', () => App.init());
} else {
	App.init();
}