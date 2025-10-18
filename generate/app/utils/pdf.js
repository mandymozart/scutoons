/**
 * Generate Certificate of Authenticity PDF
 * @param {Object} qrData - QR code data (code, type, url, etc.)
 * @param {Object} templateData - Template field data (receiver, productname, etc.)
 * @param {SVGElement} svgElement - The QR code SVG element
 * @returns {Promise<jsPDF>} The generated PDF document
 */
export async function generateCoaPdf(qrData, templateData, svgElement) {
	return new Promise((resolve, reject) => {
		const { jsPDF } = window.jspdf;
		const doc = new jsPDF();

		doc.setFont('courier', 'normal');
		
		const margin = 20;
		const pageWidth = doc.internal.pageSize.getWidth();
		let y = margin;

		// Title
		doc.setFontSize(16);
		doc.text('Certificate of Authenticity', margin, y);
		y += 15;

		// Product name
		doc.setFontSize(14);
		const productName = templateData.productname || 'SCUTOONS Product';
		doc.text(`"${productName}"`, margin, y);
		y += 10;

		// Edition
		if (templateData.edition_number && templateData.edition_count) {
			doc.setFontSize(12);
			doc.text('Edition', margin, y);
			y += 6;
			doc.setFontSize(10);
			doc.text(`${templateData.edition_number}/${templateData.edition_count}`, margin, y);
			y += 12;
		} else {
			y += 8;
		}

		// Issuer section
		doc.setFontSize(12);
		doc.text('Issuer', margin, y);
		y += 6;
		doc.setFontSize(10);
		doc.text('Mandy Mozart aka Tilman Porschutz', margin, y);
		y += 10;

		// Original buyer
		doc.setFontSize(12);
		doc.text('Original buyer', margin, y);
		y += 6;
		doc.setFontSize(10);
		doc.text(templateData.receiver || 'N/A', margin, y);
		y += 10;

		// Main text
		doc.setFontSize(9);
		const editionCount = templateData.edition_count || '12';
		const excludedNumber = templateData.excluded_number || '3';
		const mainText = `The issuer guarantees that the artwork is limited to an edition of ${editionCount} with qr code with another ${excludedNumber} unnumbered in the possession of the artists. This work is authentic. The buyer initially purchased the artwork "${productName}" from the artist or an authorized dealer.`;
		const splitText = doc.splitTextToSize(mainText, pageWidth - 2 * margin);
		doc.text(splitText, margin, y);
		y += splitText.length * 5 + 5;

		// Code section
		doc.setFontSize(11);
		doc.text(`The artwork's code is ${qrData.code} and guarantees absolute uniqueness!`, margin, y);
		y += 10;

		// Add QR code - use stored SVG from state
		if (svgElement) {
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
				doc.addImage(imgData, 'PNG', margin, y, 50, 50);
				URL.revokeObjectURL(url);

				y += 55;

				// URL text
				doc.setFontSize(9);
				const urlText = `The code can be entered at https://qr.scutoons.com and the complete webtoon and any future releases of SCUTOONS can be accessed with this code. The issuer guarantees persistence of the code even if said domain will cease to exist.`;
				const splitUrlText = doc.splitTextToSize(urlText, pageWidth - 2 * margin);
				doc.text(splitUrlText, margin, y);
				y += splitUrlText.length * 5 + 10;

				// Collaborative piece notice
				const collabText = `The artwork is a collaborative piece. The piece's creators are mentioned here in the credits appendix or available online upon request of the webtoon release page.`;
				const splitCollabText = doc.splitTextToSize(collabText, pageWidth - 2 * margin);
				doc.text(splitCollabText, margin, y);
				y += splitCollabText.length * 5 + 15;

				// Signature section
				doc.setFontSize(11);
				doc.text('Signature, Date and Place of Purchase', margin, y);
				y += 10;
				doc.line(margin, y, pageWidth - margin, y);
				y += 5;
				doc.setFontSize(9);
				doc.text('Mandy Mozart aka Tilman Porschutz', margin, y);
				y += 10;

				// Creators
				if (templateData.creators) {
					doc.setFontSize(11);
					doc.text(`Creators of "${productName}"`, margin, y);
					y += 6;
					doc.setFontSize(9);
					const creatorText = doc.splitTextToSize(templateData.creators, pageWidth - 2 * margin);
					doc.text(creatorText, margin, y);
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
 * Generate QR Print PDF
 * @param {Object} qrData - QR code data (code, type, url, created_at, expires_at)
 * @param {Object} templateData - Template field data (receiver, etc.)
 * @param {SVGElement} svgElement - The QR code SVG element
 * @returns {Promise<jsPDF>} The generated PDF document
 */
export async function generateQrPrintPdf(qrData, templateData, svgElement) {
	return new Promise((resolve, reject) => {
		const { jsPDF } = window.jspdf;
		const doc = new jsPDF();

		doc.setFont('courier', 'normal');
		
		const margin = 20;
		const pageWidth = doc.internal.pageSize.getWidth();
		let y = margin;

		// Title
		doc.setFontSize(16);
		doc.text('SCUTOONS Access Certificate', margin, y);
		y += 15;

		// Date
		doc.setFontSize(12);
		doc.text('Date', margin, y);
		y += 6;
		doc.setFontSize(10);
		const date = new Date(qrData.created_at).toLocaleDateString();
		doc.text(date, margin, y);
		y += 10;

		// Issuer
		doc.setFontSize(12);
		doc.text('Issuer', margin, y);
		y += 6;
		doc.setFontSize(10);
		doc.text('Mandy Mozart aka Tilman Porschutz', margin, y);
		y += 10;

		// Receiver
		doc.setFontSize(12);
		doc.text('Receiver', margin, y);
		y += 6;
		doc.setFontSize(10);
		doc.text(templateData.receiver || 'N/A', margin, y);
		y += 10;

		// Access message
		doc.setFontSize(11);
		doc.text(`You have been granted access to SCUTOONS with the code ${qrData.code}.`, margin, y);
		y += 15;

		// Add QR code - use stored SVG from state
		if (svgElement) {
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
				doc.addImage(imgData, 'PNG', margin, y, 60, 60);
				URL.revokeObjectURL(url);

				y += 65;

				// URL
				doc.setFontSize(10);
				doc.text(qrData.url, margin, y);
				y += 10;

				// Instructions
				doc.setFontSize(9);
				const instructions = `The code can be entered manually at https://qr.scutoons.com and the complete webtoon and any future releases of SCUTOONS can be accessed with this code. The issuer guarantees persistence of the code even if said domain will cease to exist.`;
				const splitInstructions = doc.splitTextToSize(instructions, pageWidth - 2 * margin);
				doc.text(splitInstructions, margin, y);
				y += splitInstructions.length * 5 + 10;

				// Expiration
				if (qrData.expires_at) {
					doc.setFontSize(10);
					doc.text(`Expires: ${new Date(qrData.expires_at).toLocaleDateString()}`, margin, y);
					y += 10;
				}

				// Collaborative piece notice
				doc.setFontSize(9);
				const collabText = `The artwork is a collaborative piece. The piece's creators are mentioned here in the credits appendix or available online upon request of the webtoon release page.`;
				const splitCollabText = doc.splitTextToSize(collabText, pageWidth - 2 * margin);
				doc.text(splitCollabText, margin, y);
				y += splitCollabText.length * 5 + 10;

				// Bootleg notice
				doc.setFontSize(10);
				doc.text('Feel free to bootleg this qr code. We love to spread the gospel.', margin, y);

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
