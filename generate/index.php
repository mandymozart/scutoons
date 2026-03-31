<?php
require_once '../config.php';
?>
<!DOCTYPE html>
<html lang="en">

<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>SCUTOONS - QR</title>
    <script type="module">
        import {
            nanoid
        } from 'https://cdn.jsdelivr.net/npm/nanoid/nanoid.js';
        window.nanoid = nanoid;
    </script>
    <script src="lib/qrcode/qrcode.min.js"></script>
    <link rel="stylesheet" type="text/css" href="https://www.scutoons.com/css/styles.css">
    <style>
        #qrResult {
            text-align: center;
            padding: 1rem;
            background-color: #f8f9fa;
            display: none;
        }

        #qrCode {
            margin: 2rem 0;
        }

        .qr-info {
            padding: 1rem;
            word-break: break-all;
            margin: 1rem 0;
            text-align: left;
        }

        .error {
            color: #dc3545;
            margin: 1rem 0;
        }

        .success {
            color: #28a745;
            margin: 1rem 0;
        }

        .content {
            display: flex;
            gap: 1rem;
        }
    </style>
</head>

<body>
    <div class="container">
        <div class="content">
            <form id="qrForm">
                <div class="form-group">
                    <label for="redirectUrl">Redirect URL:</label>
                    <input type="url" id="redirectUrl" name="redirectUrl" required placeholder="https://scutoons.com">
                </div>
                <div class="form-group">
                    <label for="type">Type:</label>
                    <select id="type" name="type">
                        <option value="membership">Membership</option>
                        <option value="event">Event</option>
                        <option value="xr">XR</option>
                        <option value="promo">Promo</option>
                    </select>
                </div>
                <div class="form-group">
                    <label for="expiresAt">Expires At (optional):</label>
                    <input type="datetime-local" id="expiresAt" name="expiresAt">
                    <small>Leave empty for no expiration</small>
                </div>
                <button type="submit">Generate</button>
                <button id="downloadBtn" style="display:none; margin-top: 1rem;">Download</button>
                <div id="message"></div>
            </form>
            <div id="qrResult">
                <div id="qrCode"></div>
                <div class="qr-info">
                    <strong>QR URL:</strong><br>
                    <span id="qrUrl"></span>
                </div>
                <div class="qr-info">
                    <strong>Redirect to:</strong><br>
                    <span id="redirectTo"></span>
                </div>
                <div class="qr-info">
                    <strong>Code:</strong> <span id="code"></span>
                </div>
            </div>
        </div>
    </div>

    <script>
        let isSubmitting = false;
        document.getElementById('qrForm').addEventListener('submit', async function(e) {
            e.preventDefault();

            // Prevent duplicate submissions
            if (isSubmitting) {
                console.log('Form submission already in progress, ignoring duplicate request');
                return;
            }
            
            isSubmitting = true;
            const submitButton = this.querySelector('button[type="submit"]');
            const originalButtonText = submitButton.textContent;
            submitButton.disabled = true;
            submitButton.textContent = 'Generating...';

            // Generate nanoid code  
            const code = window.nanoid(8);
            const messageDiv = document.getElementById('message');
            messageDiv.innerHTML = '';

            try {
                const formData = new FormData(this);
                formData.append('code', code);

                const response = await fetch('generate.php', {
                    method: 'POST',
                    body: formData
                });

                const result = await response.json();

                if (result.success) {
                    // Generate QR code
                    const qrUrl = `https://qr.scutoons.com?c=${result.code}`;

                    // Clear previous QR code
                    document.getElementById('qrCode').innerHTML = '';

                    // Generate SVG QR code
                    const qrSvg = new QRCode({
                        msg: qrUrl,
                        dim: 256,
                        pad: 4,
                        ecl: 'H',
                        pal: ['#000000', '#FFFFFF']
                    });

                    document.getElementById('qrCode').appendChild(qrSvg);

                    // Show result
                    document.getElementById('qrUrl').textContent = qrUrl;
                    document.getElementById('redirectTo').textContent = result.redirect_to;
                    document.getElementById('code').textContent = result.code;
                    document.getElementById('qrResult').style.display = 'block';

                    // Setup download button
                    const downloadBtn = document.getElementById('downloadBtn');
                    downloadBtn.style.display = 'inline-block';
                    downloadBtn.onclick = function() {
                        // Get the SVG element
                        const svgElement = document.getElementById('qrCode').querySelector('svg');
                        const svgString = new XMLSerializer().serializeToString(svgElement);
                        const blob = new Blob([svgString], {
                            type: 'image/svg+xml'
                        });
                        const url = URL.createObjectURL(blob);

                        // Create download link
                        const a = document.createElement('a');
                        a.href = url;
                        a.download = `SCUTOONS-QR-${result.type.toUpperCase()}-${result.code}.svg`;
                        a.click();

                        // Clean up
                        URL.revokeObjectURL(url);
                    };

                    messageDiv.innerHTML = '<div class="success">QR Code generated successfully!</div>';

                } else {
                    messageDiv.innerHTML = '<div class="error">Error: ' + result.message + '</div>';
                }
            } catch (error) {
                messageDiv.innerHTML = '<div class="error">Error generating QR code: ' + error.message + '</div>';
            } finally {
                // Re-enable the submit button after request completes
                isSubmitting = false;
                submitButton.disabled = false;
                submitButton.textContent = originalButtonText;
            }
        });
    </script>
</body>

</html>