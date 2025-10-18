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
	<script src="app/lib/qrcode/qrcode.min.js"></script>
	<link rel="stylesheet" type="text/css" href="https://www.scutoons.com/assets/styles.css">
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

	<script type="module" src="app/main.js"></script>
</body>

</html>