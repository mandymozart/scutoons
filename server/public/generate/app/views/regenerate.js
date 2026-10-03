import QRDisplay from '../components/QRDisplay.js';
import DownloadButtons from '../components/DownloadButtons.js';

export default class RegenerateView {
	constructor() {
		this.form = document.getElementById('regenerateForm');
		this.messageDiv = document.getElementById('regenerateMessage');
		
		// Initialize components with 'regen' prefix
		this.qrDisplay = new QRDisplay('regen');
		this.downloadButtons = new DownloadButtons('regen', this.qrDisplay);
		
		// Stored data
		this.qrData = null;
		
		this.init();
	}

	init() {
		this.form.addEventListener('submit', (e) => this.handleSubmit(e));
	}

	async handleSubmit(e) {
		e.preventDefault();
		
		const code = document.getElementById('existingCode').value.trim(); // exact: codes are case-sensitive
		
		if (!code) {
			this.showMessage('Please enter a code', 'error');
			return;
		}
		
		try {
			// Fetch QR data from get-signature endpoint (includes everything we need)
			const response = await fetch(`get-signature.php?code=${encodeURIComponent(code)}`);
			
			const result = await response.json();
			
			if (!result.success) {
				this.showMessage(result.message || 'Failed to load QR code', 'error');
				return;
			}
			
			// All data is in qr_data (parsed from signedData)
			// Includes: code, type, url, dates, AND template fields (receiver, productname, etc.)
			this.qrData = {
				...result.qr_data,  // Everything including template fields
				signature: result.signature,
				fingerprint: result.fingerprint,
				redirect_to: result.redirect_to
			};
			
			// Generate QR code
			await this.generateQR();
			
			this.showMessage('✅ QR code loaded successfully! Downloads ready.', 'success');
			
		} catch (error) {
			console.error('Error loading QR code:', error);
			this.showMessage('Error loading QR code: ' + error.message, 'error');
		}
	}

	async generateQR() {
		// Render QR code using shared component
		this.qrDisplay.renderQRCode(this.qrData.url);
		
		// Display QR information
		this.qrDisplay.displayInfo(this.qrData);
		
		// Debug logging
		console.log('QR Data:', this.qrData);
		console.log('Type:', this.qrData.type);
		console.log('Has Signature:', !!this.qrData.signature);
		
		// Set data for downloads - qrData contains everything (template fields already in it)
		this.downloadButtons.setData(this.qrData, this.qrData);
		this.downloadButtons.updateVisibility(this.qrData.type, !!this.qrData.signature);
		
		console.log('Download buttons should now be visible');
	}

	showMessage(msg, type = 'info') {
		this.messageDiv.innerHTML = `<div class="${type}">${msg}</div>`;
		setTimeout(() => {
			this.messageDiv.innerHTML = '';
		}, 5000);
	}
}
