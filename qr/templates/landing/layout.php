<?php
/**
 * Page frame of the QR landing page in the Scutoons design (IBM Plex Mono, ink on paper, square, 2px rules).
 * Tokens + self-hosted fonts: /assets/tokens/scutoons-tokens.css, qr's generated copy of @scutoons/tokens (bin/sync-tokens.mjs).
 * @var callable $e  @var string $locale  @var string $title  @var string $body (already escaped HTML)
 */
?>
<!DOCTYPE html>
<html lang="<?= $e($locale) ?>">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="robots" content="noindex">
    <title><?= $e($title) ?> · SCUTOONS</title>
    <link rel="stylesheet" href="/assets/tokens/scutoons-tokens.css">
    <style>
        html { font-size: 16px; }
        @media (max-width: 479px) { html { font-size: 14px; } }
        @media (min-width: 768px) { html { font-size: 24px; } }
        *, *::before, *::after { box-sizing: border-box; }
        body {
            margin: 0; background: var(--paper); color: var(--ink);
            font-family: var(--font-mono); font-size: 1rem; line-height: 1.4;
        }
        main { max-width: 26rem; margin: 0 auto; padding: 1.5rem 1rem 3rem; }
        .logo { display: block; margin: 0 auto 1.5rem; width: 10rem; max-width: 60%; }
        .logo img { display: block; width: 100%; height: auto; }
        .panel { border: 2px solid var(--ink); padding: 1rem; margin-bottom: 1rem; }
        h1 { font-size: 1rem; font-weight: 700; margin: 0 0 0.75rem; }
        p { margin: 0 0 0.75rem; line-height: inherit; }
        .caption { font-size: 0.75rem; line-height: 1.3; color: var(--text-muted); }
        .items { display: grid; gap: 0.75rem; margin: 0 0 1rem; }
        .item { display: grid; grid-template-columns: 5rem 1fr; gap: 0.75rem; align-items: center;
                border: 2px solid var(--border-muted); padding: 0.5rem; background: var(--paper); }
        label.item { cursor: pointer; transition: border-color 0.1s ease; }
        label.item:hover { border-color: var(--border-hover); }
        label.item:has(input:checked) { border-color: var(--ink); }
        label.item input { position: absolute; opacity: 0; pointer-events: none; }
        .item.single { border-color: var(--ink); }
        .icon { width: 5rem; height: 5rem; display: flex; align-items: center; justify-content: center;
                background: var(--surface-tint-cyan); }
        label.item:has(input:checked) .icon, .item.single .icon { background: var(--primary-cyan); }
        .icon img { width: 100%; height: 100%; object-fit: contain; display: block; }
        .icon .fallback { font-weight: 700; font-size: 1.5rem; }
        .item-name { font-weight: 700; display: block; }
        .item-meta { display: block; }
        .item-desc { margin: 0 0 1rem; }
        .actions { display: grid; gap: 0.75rem; }
        button, .btn {
            display: block; width: 100%; font: inherit; line-height: 1; text-align: center; text-decoration: none;
            padding: 1rem 1.5rem; border: 0; border-radius: 0; cursor: pointer; touch-action: manipulation;
            background: var(--ink); color: var(--paper);
        }
        .btn-secondary { background: var(--paper); color: var(--ink); border: 2px solid var(--border-muted);
                         padding: calc(1rem - 2px) 1.5rem; transition: border-color 0.1s ease; }
        .btn-secondary:hover { border-color: var(--ink); }
        .btn-clear { background: none; color: var(--ink); padding: 0.5rem 0; text-decoration: underline; }
        button:disabled, .disabled { opacity: 0.5; cursor: not-allowed; }
        input[type=email], input[type=text] {
            display: block; width: 100%; font: inherit; padding: 0.75rem; margin: 0 0 0.75rem;
            border: 1px solid var(--border-subtle); border-radius: 0; background: var(--paper); color: var(--ink);
        }
        input:focus-visible, button:focus-visible, .btn:focus-visible, label.item:has(input:focus-visible) {
            outline: 2px solid var(--ink); outline-offset: 2px;
        }
        .field-label { display: block; font-weight: 700; margin: 0 0 0.25rem; }
        .actions .caption { margin: -0.25rem 0 0.5rem; }
        .warn { border-left: 0.375rem solid var(--ink); padding: 0.25rem 0.5rem; font-weight: 700; margin: 0 0 0.75rem; }
        #registered { margin: 0 0 0.75rem; }
        .faq h2 { font-size: 1rem; font-weight: 700; margin: 0 0 0.5rem; }
        a { color: var(--ink); }
    </style>
</head>
<body>
<main>
    <a class="logo" href="https://scutoons.com"><img src="https://scutoons.com/SCUTOONS-logo.svg" alt="SCUTOONS"></a>
    <?= $body ?>
</main>
</body>
</html>
