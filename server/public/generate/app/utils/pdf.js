import { TemplateParser } from './template.js';
import { QR_DOMAIN } from '../config.js';

/**
 * Render template content to PDF (shared logic)
 * @param {jsPDF} doc - The PDF document
 * @param {string} renderedTemplate - Rendered template content
 * @param {SVGElement} svgElement - The QR code SVG
 * @param {Object} qrData - QR data (for crypto footer)
 * @param {number} qrCodeSize - Size of QR code in pixels
 * @returns {Promise<jsPDF>} The PDF document
 */
async function renderTemplateToPdf(doc, renderedTemplate, svgElement, qrData, qrCodeSize = 40) {
	return new Promise((resolve, reject) => {
		const margin = 15;
		const pageWidth = doc.internal.pageSize.getWidth();
		const pageHeight = doc.internal.pageSize.getHeight();
		let y = margin;

		// Split template into lines and track QR position
		const lines = renderedTemplate.split('\n');
		let qrCodeY = null;
		
		// Consistent typewriter style - single font size
		doc.setFontSize(9);
		doc.setFont('courier', 'normal');
		
		for (let i = 0; i < lines.length; i++) {
			const line = lines[i].trim();
			
			// Skip empty lines (but add small space)
			if (line === '') {
				y += 4;
				continue;
			}
			
			// Handle QR code placeholder - mark position for insertion
			if (line === '[QR_CODE_PLACEHOLDER]') {
				qrCodeY = y;
				y += qrCodeSize + 3; // Reserve space for QR code + margin
				continue;
			}
			
			// Handle signature line (underscores) - about 1/3 of page width
			if (line.startsWith('___')) {
				const lineWidth = (pageWidth - 2 * margin) / 3;
				doc.line(margin, y, margin + lineWidth, y);
				y += 5;
				continue;
			}
			
			// All text in consistent typewriter style
			const splitText = doc.splitTextToSize(line, pageWidth - 2 * margin);
			doc.text(splitText, margin, y);
			y += splitText.length * 4.5 + 1;
		}

		// Add QR code at marked position
		if (svgElement && qrCodeY !== null) {
			const canvas = document.createElement('canvas');
			canvas.width = 256;
			canvas.height = 256;
			const ctx = canvas.getContext('2d');
			const img = new Image();
			const svgString = new XMLSerializer().serializeToString(svgElement);
			const svgBlob = new Blob([svgString], { type: 'image/svg+xml;charset=utf-8' });
			const url = URL.createObjectURL(svgBlob);

			img.onload = () => {
				ctx.drawImage(img, 0, 0);
				const imgData = canvas.toDataURL('image/png');
				// Insert QR code at the marked position from template
				doc.addImage(imgData, 'PNG', margin, qrCodeY, qrCodeSize, qrCodeSize);
				URL.revokeObjectURL(url);

				// Post-Quantum Cryptographic Signature (bottom-aligned, typewriter style)
				if (qrData.signature) {
					const bottomMargin = 15;
					let yBottom = pageHeight - bottomMargin;
					
					// Consistent typewriter style
					doc.setFontSize(7);
					doc.setFont('courier', 'normal');
					
					// Calculate from bottom up
					yBottom -= 3; // Last line
					doc.text('This certificate is protected by post-quantum cryptography (ML-DSA-65/Dilithium).', margin, yBottom);
					
					yBottom -= 4; // Verify line
					doc.text('Verify at ' + QR_DOMAIN + '/verify.html with code: ' + qrData.code, margin, yBottom); // exact: codes are case-sensitive
					
					yBottom -= 4; // Fingerprint
					doc.text(`Fingerprint: ${qrData.fingerprint}`, margin, yBottom);
					
					yBottom -= 4; // Algorithm line
					doc.text(`Algorithm: ${qrData.signature.algorithm} | Timestamp: ${new Date(qrData.signature.timestamp).toLocaleString()}`, margin, yBottom);
					
					yBottom -= 5; // Title
					doc.text('Post-Quantum Cryptographic Verification', margin, yBottom);
				}

				resolve(doc);
			};

			img.onerror = (error) => {
				reject(new Error('Failed to load QR code image'));
			};

			img.src = url;
		} else {
			reject(new Error('No SVG element provided'));
		}
	});
}

/**
 * Generate PDF from template (shared logic for all PDF types)
 * @param {string} templateUrl - Path to template file
 * @param {Object} qrData - QR code data
 * @param {Object} templateData - Template field data
 * @param {SVGElement} svgElement - The QR code SVG element
 * @returns {Promise<jsPDF>} The generated PDF document
 */
async function generatePdfFromTemplate(templateUrl, qrData, templateData, svgElement) {
	// Fetch and render template
	const parser = new TemplateParser();
	
	// Format dates for display
	const formattedData = {
		...qrData,
		...templateData,
		created_at: new Date(qrData.created_at).toLocaleDateString(),
		expires_at: qrData.expires_at ? 'Expires: ' + new Date(qrData.expires_at).toLocaleDateString() : ''
	};
	
	const renderedTemplate = await parser.fetchAndRenderTemplate(templateUrl, formattedData);
	
	if (!renderedTemplate) {
		throw new Error('Failed to load or render template');
	}
	
	// Create PDF document
	const { jsPDF } = window.jspdf;
	const doc = new jsPDF();
	doc.setFont('courier', 'normal');
	
	// Use shared rendering logic (50px QR code)
	return renderTemplateToPdf(doc, renderedTemplate, svgElement, qrData, 50);
}

/**
 * Generate Certificate of Authenticity PDF
 */
export async function generateCoaPdf(qrData, templateData, svgElement) {
	return generatePdfFromTemplate('app/templates/cos.md', qrData, templateData, svgElement);
}

/**
 * Generate QR Print PDF
 */
export async function generateQrPrintPdf(qrData, templateData, svgElement) {
	return generatePdfFromTemplate('app/templates/qr.md', qrData, templateData, svgElement);
}
