import { QRCODE_API_SERVER } from '$lib/config.ts';
import type { Membership, ValidationResult } from '$lib/models/membership.ts';

/**
 * @param {string} code
 * @returns {Promise<ValidationResult>}
 */
export const validateQRCode = async (code: string): Promise<ValidationResult> => {
	if (!code) {
		return { success: false, message: 'No QR code provided' };
	}

	try {
		const response = await fetch(QRCODE_API_SERVER + 'validate/', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json'
			},
			body: JSON.stringify({ code })
		});

		const data = await response.json();

		if (data.status === 'success' && data.qr) {
			const membershipData: Membership = {
				...data.qr,
				code,
				validated_at: new Date().toISOString(),
				expires_at: data.qr.expires_at
			};

			return { success: true, membership: membershipData };
		} else {
			return { success: false, message: data.message || 'Validation failed' };
		}
	} catch (error) {
		console.error('Error validating QR code:', error);
		return { success: false, message: 'Network error', error };
	}
};