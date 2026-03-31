#!/usr/bin/env node
/**
 * Server-Side Signing Service
 * Signs data with ML-DSA-65 (Dilithium) using noble-post-quantum
 * 
 * SECURITY: Private key never leaves the server
 * Protected by htaccess in generate/ folder
 */

const fs = require('fs');
const path = require('path');

// Load noble-post-quantum
const noblePostQuantum = require('../assets/js/noble-post-quantum.min.js');
const ml_dsa65 = noblePostQuantum.ml_dsa65;

/**
 * Load keypair from secure storage
 */
function loadKeypair() {
    const keysFile = path.join(__dirname, '../api/keys/keypair.json');
    
    if (!fs.existsSync(keysFile)) {
        throw new Error('Keypair not found. Please generate keys first.');
    }
    
    const keys = JSON.parse(fs.readFileSync(keysFile, 'utf8'));
    
    if (!keys.privateKey || !keys.publicKey) {
        throw new Error('Invalid keypair format');
    }
    
    // Convert hex to Uint8Array
    const secretKey = hexToBytes(keys.privateKey);
    const publicKey = hexToBytes(keys.publicKey);
    
    return { secretKey, publicKey };
}

/**
 * Sign data using ML-DSA-65
 */
function signData(dataToSign, keypair) {
    // Convert string to bytes
    const message = new TextEncoder().encode(dataToSign);
    
    // Sign using ML-DSA-65
    const signature = ml_dsa65.sign(message, keypair.secretKey);
    
    return {
        signature: bytesToHex(signature),
        publicKey: bytesToHex(keypair.publicKey),
        algorithm: 'ML-DSA-65',
        signedData: dataToSign,
        timestamp: new Date().toISOString()
    };
}

// Utility functions
function hexToBytes(hex) {
    const bytes = new Uint8Array(hex.length / 2);
    for (let i = 0; i < bytes.length; i++) {
        bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
    }
    return bytes;
}

function bytesToHex(bytes) {
    return Array.from(bytes)
        .map(b => b.toString(16).padStart(2, '0'))
        .join('');
}

// Main execution
if (require.main === module) {
    // Read from stdin
    let inputData = '';
    
    process.stdin.on('data', (chunk) => {
        inputData += chunk;
    });
    
    process.stdin.on('end', () => {
        try {
            const data = JSON.parse(inputData);
            
            if (!data.dataToSign) {
                throw new Error('Missing dataToSign parameter');
            }
            
            const keypair = loadKeypair();
            const signatureData = signData(data.dataToSign, keypair);
            
            process.stdout.write(JSON.stringify({
                success: true,
                signature: signatureData
            }));
            
        } catch (error) {
            process.stderr.write(JSON.stringify({
                success: false,
                error: error.message
            }));
            process.exit(1);
        }
    });
}

module.exports = { signData, loadKeypair };
