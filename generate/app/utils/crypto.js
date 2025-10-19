/**
 * Post-Quantum Cryptography utilities using noble post-quantum
 * Implements ML-DSA (Dilithium) for digital signatures
 * Uses standalone library (noble-post-quantum.min.js)
 */

// Access ML-DSA from the global noblePostQuantum object
const getMlDsa = () => {
	if (typeof window.noblePostQuantum === 'undefined' || !window.noblePostQuantum.ml_dsa65) {
		throw new Error('noble-post-quantum library not loaded. Include noble-post-quantum.min.js before this script.');
	}
	return window.noblePostQuantum.ml_dsa65;
};

// Keypair loaded from server
let KEYPAIR = null;
let KEYPAIR_LOADING = null; // Promise to prevent multiple simultaneous loads

/**
 * Initialize or load keypair from server
 * Keys are stored securely on the server at /api/keys/keypair.json
 */
export async function initializeKeypair() {
	if (KEYPAIR) {
		return KEYPAIR;
	}
	
	// If already loading, wait for that to complete
	if (KEYPAIR_LOADING) {
		return await KEYPAIR_LOADING;
	}
	
	// Start loading
	KEYPAIR_LOADING = (async () => {
		try {
			console.log('🔐 Loading keypair from server...');
			
			const response = await fetch('/api/keys/keys.php');
			const data = await response.json();
			
			if (!data.success) {
				if (data.needsGeneration) {
					throw new Error('Keys not generated yet. Please visit /api/keys/generate-keys.php to generate keys first.');
				}
				throw new Error(data.error || 'Failed to load keys');
			}
			
			// Convert hex strings to Uint8Array
			const secretKey = hexToBytes(data.privateKey);
			const publicKey = hexToBytes(data.publicKey);
			
			// Validate key lengths
			if (secretKey.length !== 4032) {
				throw new Error(`Invalid secret key length: ${secretKey.length} (expected 4032)`);
			}
			if (publicKey.length !== 1952) {
				throw new Error(`Invalid public key length: ${publicKey.length} (expected 1952)`);
			}
			
			KEYPAIR = { secretKey, publicKey };
			
			console.log('✅ Keypair loaded successfully');
			console.log('   Algorithm: ML-DSA-65');
			console.log('   Public Key (first 32 chars):', data.publicKey.substring(0, 32) + '...');
			console.log('   Generated:', data.generated || 'Unknown');
			
			if (data.warning) {
				console.warn('⚠️', data.warning);
			}
			
			return KEYPAIR;
			
		} catch (error) {
			KEYPAIR_LOADING = null; // Reset so we can try again
			console.error('❌ Failed to load keypair:', error.message);
			throw error;
		}
	})();
	
	return await KEYPAIR_LOADING;
}

/**
 * Sign QR code data using ML-DSA (Dilithium)
 * @param {Object} qrData - The QR code data to sign
 * @returns {Promise<Object>} Signature data with metadata
 */
export async function signQRCode(qrData) {
	const ml_dsa65 = getMlDsa();
	const keypair = await initializeKeypair();
	
	// Create a canonical string representation of the QR data
	const dataToSign = JSON.stringify({
		code: qrData.code,
		type: qrData.type,
		redirect_to: qrData.redirect_to,
		url: qrData.url,
		created_at: qrData.created_at,
		expires_at: qrData.expires_at
	});
	
	// Convert to bytes
	const message = new TextEncoder().encode(dataToSign);
	
	// Sign using ML-DSA-65 (Dilithium)
	// Standalone library uses: sign(message, secretKey, ctx?)
	console.log('Attempting to sign with keypair...');
	console.log('- secretKey length:', keypair.secretKey.length);
	console.log('- message length:', message.length);
	
	const signature = ml_dsa65.sign(message, keypair.secretKey);
	console.log('- signature length:', signature.length);
	
	return {
		signature: bytesToHex(signature),
		publicKey: bytesToHex(keypair.publicKey),
		algorithm: 'ML-DSA-65',
		signedData: dataToSign,
		timestamp: new Date().toISOString()
	};
}

/**
 * Verify a signature
 * @param {string} signatureHex - Hex-encoded signature
 * @param {string} publicKeyHex - Hex-encoded public key
 * @param {string} signedData - The original signed data
 * @returns {boolean} True if signature is valid
 */
export function verifySignature(signatureHex, publicKeyHex, signedData) {
	try {
		const ml_dsa65 = getMlDsa();
		
		console.log('verifySignature called with:');
		console.log('- signatureHex length:', signatureHex.length, 'chars');
		console.log('- publicKeyHex length:', publicKeyHex.length, 'chars');
		console.log('- signedData length:', signedData.length, 'chars');
		
		const signature = hexToBytes(signatureHex);
		const publicKey = hexToBytes(publicKeyHex);
		const message = new TextEncoder().encode(signedData);
		
		console.log('After conversion:');
		console.log('- signature:', signature.length, 'bytes');
		console.log('- publicKey:', publicKey.length, 'bytes');
		console.log('- message:', message.length, 'bytes');
		
		// Validate lengths
		if (signature.length !== 3309) {
			console.warn('⚠️ Unexpected signature length! Expected 3309, got', signature.length);
		}
		if (publicKey.length !== 1952) {
			console.warn('⚠️ Unexpected public key length! Expected 1952, got', publicKey.length);
		}
		
		// Match sign(message, secretKey) pattern with verify(signature, message, publicKey)
		console.log('Attempting verification...');
		console.log('Testing verify(signature, message, publicKey)...');
		const result = ml_dsa65.verify(signature, message, publicKey);
		console.log('Verification result:', result);
		return result;
	} catch (error) {
		console.error('Signature verification failed:', error);
		return false;
	}
}

/**
 * Generate a human-readable certificate fingerprint
 * @param {Object} signatureData - The signature data
 * @returns {string} Formatted fingerprint
 */
export function generateFingerprint(signatureData) {
	// Take first 32 bytes of signature for fingerprint
	const fingerprintBytes = signatureData.signature.slice(0, 64); // 32 bytes in hex
	return fingerprintBytes.match(/.{1,4}/g).join(':').toUpperCase();
}

/**
 * Export signature data for storage/display
 * @param {Object} signatureData - The signature data
 * @returns {string} Base64-encoded signature package
 */
export function exportSignature(signatureData) {
	const json = JSON.stringify(signatureData);
	return btoa(json);
}

/**
 * Import signature data from storage
 * @param {string} base64Data - Base64-encoded signature package
 * @returns {Object} Signature data
 */
export function importSignature(base64Data) {
	const json = atob(base64Data);
	return JSON.parse(json);
}

// Utility functions
function bytesToHex(bytes) {
	return Array.from(bytes)
		.map(b => b.toString(16).padStart(2, '0'))
		.join('');
}

function hexToBytes(hex) {
	const bytes = new Uint8Array(hex.length / 2);
	for (let i = 0; i < bytes.length; i++) {
		bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
	}
	return bytes;
}
