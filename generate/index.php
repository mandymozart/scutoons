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
	<script src="https://cdnjs.cloudflare.com/ajax/libs/jspdf/3.0.3/jspdf.umd.min.js"></script>
	<script src="app/lib/qrcode/qrcode.min.js"></script>
	<script src="../assets/js/noble-post-quantum.min.js"></script>
	<link rel="stylesheet" type="text/css" href="https://www.scutoons.com/css/styles.css">
	<style>
		html {
			font-size: 16px;
		}
		
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

		.container {
			margin: 1rem 2rem;
		}

		.app-layout {
			max-width: 100%;
		}

		.mode-selector {
			display: flex;
			gap: 1rem;
			align-items: baseline;
			margin-bottom: 1rem;
		}

		.mode-option {
			cursor: pointer;
		}

		.mode-option.active {
			font-weight: bold;
			color: #000;
			text-decoration: none;
		}

		.content {
			display: grid;
			grid-template-columns: 1fr 1fr;
			gap: 1rem;
		}

		.mode-panel {
			display: none;
		}

		.mode-panel.active {
			display: block;
		}

		textarea {
			height: 10rem;
		}

		@media screen and (max-width: 768px) {
			.mode-selector {
				margin-bottom: 1rem;
			}

			.content {
				grid-template-columns: 1fr;
			}
		}

		/* ── Batch view ─────────────────────────────── */
		/* .batch-table {
			border-collapse: collapse;
			font-size: .85rem;
			font-family: 'Courier New', monospace;
			width: 100%;
		}
		.batch-table th, .batch-table td {
			padding: .25rem .6rem;
			border: 1px solid #ddd;
			white-space: nowrap;
		}
		.batch-table thead th {
			background: #f0f0f0;
			text-align: left;
		}
		.progress-wrap {
			max-height: 62vh;
			overflow-y: auto;
			border: 1px solid #ddd;
			margin-top: .5rem;
		}
		.progress-wrap .batch-table th,
		.progress-wrap .batch-table td {
			border-left: none;
			border-right: none;
		} */
		.step-done    { color: #28a745; font-weight: bold; }
		.step-error   { color: #dc3545; font-weight: bold; }
		.step-pending { color: #aaa; }
		/* .badge {
			display: inline-block;
			padding: .1rem .4rem;
			border-radius: 3px;
			font-size: .75rem;
			font-weight: bold;
		}
		.badge-done { background: #d4edda; color: #155724; }
		.badge-warn { background: #fff3cd; color: #856404; } */
		/* .mono  { font-family: 'Courier New', monospace; letter-spacing: .05em; }
		.muted { color: #666; font-size: .9rem; } */
		.err   { color: #dc3545; }
		/* .btn-download {
			display: inline-block;
			margin-left: .35rem;
			padding: .15rem .5rem;
			font-size: .8rem;
			font-family: inherit;
			border: 1px solid #ccc;
			border-radius: 3px;
			text-decoration: none;
			color: inherit;
			background: #f8f9fa;
		}
		.btn-download:hover { background: #e9ecef; } */
		.batch-block-warning {
			background: #f8d7da;
			border: 1px solid #f5c2c7;
			color: #842029;
			padding: .5rem .75rem;
			border-radius: 4px;
			margin: .5rem 0;
			font-size: .85rem;
		}
	</style>
</head>

<body>
	<div class="container">
		<!-- Mode Selector -->
		<div class="mode-selector">
			<span>Mode:</span>
			<span class="mode-option active" data-mode="generate">Generate</span>
			<span class="mode-option" data-mode="regenerate">Regenerate</span>
			<span class="mode-option" data-mode="batch">Batch</span>
		</div>
		<div class="app-layout view">

			<!-- Main Content Area -->
			<div>
				<!-- Generate Mode -->
				<div class="mode-panel active" id="generateMode">
					<div class="content">
						<form id="qrForm">
							<h2>Generate QR Code</h2>
							<div class="form-group">
								<label for="toon">Toon:</label>
								<select id="toon" name="toon">
									<option value="">none</option>
								</select>
							</div>
							<div class="form-group">
								<label for="type">Type:</label>
								<select id="type" name="type">
									<option value="membership">Membership</option>
									<option value="event">Event</option>
									<option value="xr">XR</option>
									<option value="promo">Promo</option>
									<option value="page">Page</option>
								</select>
							</div>
							<div class="form-group">
								<label for="redirectUrl">Redirect URL:</label>
								<input type="url" id="redirectUrl" name="redirectUrl" required
									placeholder="https://scutoons.com">
							</div>

							<div class="form-group">
								<label for="expiresAt">Expires At (optional):</label>
								<input type="datetime-local" id="expiresAt" name="expiresAt">
								<small>Leave empty for no expiration</small>
							</div>
							<section class="template-variables">

							</section>
							<button type="submit">Generate</button>
						</form>
						<div class="downloads">
							<div id="message"></div>
							<div style="margin-top: 1rem;" class="download-actions">
								<button type="button" id="downloadBtn"
									style="display:none; margin-top: 0.5rem;">SVG</button>
								<button type="button" id="downloadCoaBtn"
									style="display:none; margin-top: 0.5rem;">CoS</button>
								<button type="button" id="downloadQrPrintBtn"
									style="display:none; margin-top: 0.5rem;">Printable QR</button>
								<button type="button" id="downloadSigBtn"
									style="display:none; margin-top: 0.5rem;">Signature File</button>
							</div>
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
								<div class="qr-info" id="signatureInfo" style="display:none;">
									<strong>🔐 Post-Quantum Signature:</strong><br>
									<small>
										Algorithm: <span id="sigAlgorithm"></span><br>
										Fingerprint: <span id="sigFingerprint"></span><br>
										<a href="../verify.html" target="_blank">Verify Certificate →</a>
									</small>
								</div>
							</div>
						</div>

					</div>
				</div>
				<!-- Batch Mode -->
				<div class="mode-panel" id="batchMode"></div>
				<!-- Regenerate Mode -->
				<div class="mode-panel" id="regenerateMode">
					<div class="content">
						<form id="regenerateForm">
							<h2>Regenerate from Existing Code</h2>
							<div class="form-group">
								<label for="existingCode">Code:</label>
								<input type="text" id="existingCode" name="existingCode" required
									placeholder="Enter existing QR code">
								<small>Enter the code from a previously generated QR</small>
							</div>
							<button type="submit">Load & Regenerate</button>
						</form>
						<div>
							<div id="regenerateMessage"></div>
							<div style="margin-top: 1rem;">
								<button type="button" id="regenDownloadBtn"
									style="display:none; margin-top: 0.5rem;">SVG</button>
								<button type="button" id="regenDownloadCoaBtn"
									style="display:none; margin-top: 0.5rem;">CoS</button>
								<button type="button" id="regenDownloadQrPrintBtn"
									style="display:none; margin-top: 0.5rem;">Printable QR</button>
								<button type="button" id="regenDownloadSigBtn"
									style="display:none; margin-top: 0.5rem;">Signature File</button>
							</div>
							<div id="regenQrResult" style="display:none;">
								<div id="regenQrCode"></div>
								<div class="qr-info">
									<strong>QR URL:</strong><br>
									<span id="regenQrUrl"></span>
								</div>
								<div class="qr-info">
									<strong>Redirect to:</strong><br>
									<span id="regenRedirectTo"></span>
								</div>
								<div class="qr-info">
									<strong>Code:</strong> <span id="regenCode"></span>
								</div>
								<div class="qr-info" id="regenSignatureInfo" style="display:none;">
									<strong>🔐 Post-Quantum Signature:</strong><br>
									<small>
										Algorithm: <span id="regenSigAlgorithm"></span><br>
										Fingerprint: <span id="regenSigFingerprint"></span><br>
										<a href="../verify.html" target="_blank">Verify Certificate →</a>
									</small>
								</div>
							</div>
						</div>
					</div>
				</div>
			</div>
		</div>
	</div>

	<script type="module" src="app/main.js"></script>
</body>

</html>