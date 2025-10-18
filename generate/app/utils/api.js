export async function generateQRCode(formData) {
	const response = await fetch('generate.php', {
		method: 'POST',
		body: formData
	});

	if (!response.ok) {
		throw new Error(`HTTP error! status: ${response.status}`);
	}

	return await response.json();
}
