<?php
/**
 * QR landing page.
 *
 *   /?c=CODE            membership/promo: shows the item(s) of the code (variant picker for edition options),
 *                       "Add to my inventory" → email → one-time code (same page) → claimed → redirect_to
 *                       ?code=CODE&inventory=open. "Skip" continues to the comic without claiming.
 *                       event/xr/page codes redirect right away (unchanged).
 *   /?c=CODE&skip=1     redirect without claiming
 *   /?login=TOKEN[&c=]  magic link from the login mail: signs in (and claims the code) in the browser
 *   /                   enter a code by hand
 *
 * The claim flow runs in the browser against /api/ (auth/request, auth/verify, inventory/claim); the session
 * token is kept in localStorage `scutoons_session`, so the next scan on this device needs no email.
 */
require_once __DIR__ . '/api/lib/bootstrap.php';

function h($value): string
{
    return htmlspecialchars((string) $value, ENT_QUOTES, 'UTF-8');
}

/** redirect_to + ?code=CODE (+ extra params) */
function continueUrl(string $redirectTo, string $code, string $extra = ''): string
{
    $separator = str_contains($redirectTo, '?') ? '&' : '?';
    return $redirectTo . $separator . 'code=' . rawurlencode($code) . $extra;
}

function pageLocale(): string
{
    $lang = $_GET['lang'] ?? null;
    if (is_string($lang) && in_array($lang, ['de', 'en'], true)) return $lang;
    $accept = strtolower((string) ($_SERVER['HTTP_ACCEPT_LANGUAGE'] ?? ''));
    return str_starts_with($accept, 'de') ? 'de' : 'en';
}

const TEXTS = [
    'en' => [
        'found' => 'Item found',
        'found_many' => 'Choose your item',
        'found_many_hint' => 'This code is for one of these. Pick the one you got.',
        'membership' => 'Membership',
        'membership_hint' => 'Add this code to your account to keep your membership on every device.',
        'edition' => 'Edition',
        'add' => 'Add to my inventory',
        'skip' => 'Skip',
        'skip_hint' => 'Continue to the comic without adding it.',
        'claimed' => 'This code is already in an inventory.',
        'claimed_hint' => 'Is it yours? Sign in to see it in your inventory.',
        'sign_in' => 'Sign in',
        'continue' => 'Continue to the comic',
        'email' => 'Your email',
        'email_hint' => 'We send you a code to sign in. No password.',
        'send' => 'Send code',
        'otp' => 'Code from the email',
        'otp_hint' => 'We sent a 6-digit code to',
        'confirm' => 'Confirm',
        'other_email' => 'Use another email',
        'working' => 'One moment …',
        'signing_in' => 'Signing you in …',
        'done' => 'Added to your inventory',
        'done_hint' => 'Taking you to the comic …',
        'signed_in' => 'You are signed in.',
        'open_app' => 'Open SCUTOONS',
        'choose_first' => 'Choose an item first.',
        'not_found' => 'QR code not found. Please contact us!',
        'expired' => 'This QR code has expired. Please contact us!',
        'error' => 'Something went wrong. Please try again later.',
        'members' => 'Members area',
        'members_hint' => 'Scan a QR code to get started or type it here:',
        'access_code' => 'Access code',
        'enter' => 'Enter',
        'faq_q' => 'I don\'t have an access code. Where can I get one?',
        'faq_a' => 'QR codes come with SCUTOONS limited editions:',
        'shop' => 'Shop now (Bandcamp)',
        'errors' => [
            'invalid_email' => 'This email address is not valid.',
            'invalid_otp' => 'This code is not valid.',
            'expired' => 'This code has expired. Request a new email.',
            'already_used' => 'This code was used already. Request a new email.',
            'too_many_attempts' => 'Too many wrong codes. Request a new email.',
            'rate_limited' => 'Too many emails requested. Try again later.',
            'mail_failed' => 'The email could not be sent. Try again later.',
            'already_claimed' => 'This code is already in someone else\'s inventory.',
            'choose_item' => 'Choose an item first.',
            'invalid_item' => 'Choose an item first.',
            'not_found' => 'QR code not found.',
            'invalid_token' => 'This link is not valid. Request a new email.',
        ],
    ],
    'de' => [
        'found' => 'Gegenstand gefunden',
        'found_many' => 'Wähle deinen Gegenstand',
        'found_many_hint' => 'Dieser Code gehört zu einem davon. Wähle den, den du bekommen hast.',
        'membership' => 'Mitgliedschaft',
        'membership_hint' => 'Füge den Code deinem Konto hinzu, dann hast du deine Mitgliedschaft auf jedem Gerät.',
        'edition' => 'Edition',
        'add' => 'Zu meinem Inventar hinzufügen',
        'skip' => 'Überspringen',
        'skip_hint' => 'Weiter zum Comic, ohne ihn hinzuzufügen.',
        'claimed' => 'Dieser Code ist schon in einem Inventar.',
        'claimed_hint' => 'Ist es deiner? Melde dich an, um ihn in deinem Inventar zu sehen.',
        'sign_in' => 'Anmelden',
        'continue' => 'Weiter zum Comic',
        'email' => 'Deine E-Mail',
        'email_hint' => 'Wir schicken dir einen Code zum Anmelden. Kein Passwort.',
        'send' => 'Code senden',
        'otp' => 'Code aus der E-Mail',
        'otp_hint' => 'Wir haben einen 6-stelligen Code geschickt an',
        'confirm' => 'Bestätigen',
        'other_email' => 'Andere E-Mail verwenden',
        'working' => 'Einen Moment …',
        'signing_in' => 'Du wirst angemeldet …',
        'done' => 'Zu deinem Inventar hinzugefügt',
        'done_hint' => 'Weiter zum Comic …',
        'signed_in' => 'Du bist angemeldet.',
        'open_app' => 'SCUTOONS öffnen',
        'choose_first' => 'Wähle zuerst einen Gegenstand.',
        'not_found' => 'QR-Code nicht gefunden. Bitte kontaktiere uns!',
        'expired' => 'Dieser QR-Code ist abgelaufen. Bitte kontaktiere uns!',
        'error' => 'Etwas ist schiefgelaufen. Bitte versuche es später noch einmal.',
        'members' => 'Mitgliederbereich',
        'members_hint' => 'Scanne einen QR-Code oder gib ihn hier ein:',
        'access_code' => 'Zugangscode',
        'enter' => 'Weiter',
        'faq_q' => 'Ich habe keinen Zugangscode. Wo bekomme ich einen?',
        'faq_a' => 'QR-Codes gibt es zu den limitierten SCUTOONS-Editionen:',
        'shop' => 'Zum Shop (Bandcamp)',
        'errors' => [
            'invalid_email' => 'Diese E-Mail-Adresse ist nicht gültig.',
            'invalid_otp' => 'Dieser Code ist nicht gültig.',
            'expired' => 'Dieser Code ist abgelaufen. Fordere eine neue E-Mail an.',
            'already_used' => 'Dieser Code wurde schon benutzt. Fordere eine neue E-Mail an.',
            'too_many_attempts' => 'Zu viele falsche Codes. Fordere eine neue E-Mail an.',
            'rate_limited' => 'Zu viele E-Mails angefordert. Versuche es später noch einmal.',
            'mail_failed' => 'Die E-Mail konnte nicht gesendet werden. Versuche es später noch einmal.',
            'already_claimed' => 'Dieser Code ist schon im Inventar von jemand anderem.',
            'choose_item' => 'Wähle zuerst einen Gegenstand.',
            'invalid_item' => 'Wähle zuerst einen Gegenstand.',
            'not_found' => 'QR-Code nicht gefunden.',
            'invalid_token' => 'Dieser Link ist nicht gültig. Fordere eine neue E-Mail an.',
        ],
    ],
];

$locale = pageLocale();
$t = TEXTS[$locale];

/** Renders a full page in the Scutoons design (IBM Plex Mono, ink on paper, square, 2px rules) */
function renderPage(string $title, string $body, ?array $clientConfig = null): never
{
    global $locale;
    ?>
<!DOCTYPE html>
<html lang="<?= h($locale) ?>">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="robots" content="noindex">
    <title><?= h($title) ?> · SCUTOONS</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;700&display=swap" rel="stylesheet">
    <style>
        :root {
            --ink: #000000;
            --paper: #ffffff;
            --primary-cyan: #00d1ff;
            --border-muted: #cccccc;
            --border-subtle: #dddddd;
            --border-hover: #333333;
            --surface-hover: #f0f0f0;
            --surface-panel: #f5f5f5;
            --surface-tint-cyan: #f0fcff;
            --text-muted: #6b6b6b;
            --acid-red: #ff210a;
        }
        html { font-size: 16px; }
        @media (max-width: 479px) { html { font-size: 14px; } }
        @media (min-width: 768px) { html { font-size: 24px; } }
        *, *::before, *::after { box-sizing: border-box; }
        body {
            margin: 0; background: var(--paper); color: var(--ink);
            font-family: 'IBM Plex Mono', monospace; font-size: 1rem; line-height: 1.4;
        }
        main { max-width: 26rem; margin: 0 auto; padding: 1.5rem 1rem 3rem; }
        .logo { display: block; margin: 0 auto 1.5rem; width: 10rem; max-width: 60%; }
        .logo img { display: block; width: 100%; height: auto; }
        .panel { border: 2px solid var(--ink); padding: 1rem; margin-bottom: 1rem; }
        h1 { font-size: 1rem; font-weight: 700; margin: 0 0 0.75rem; }
        p { margin: 0 0 0.75rem; }
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
        input.otp { font-size: 1.5rem; letter-spacing: 0.5rem; text-align: center; font-weight: 700; }
        .field-label { display: block; font-weight: 700; margin: 0 0 0.25rem; }
        .error { border: 2px solid var(--acid-red); padding: 0.5rem 0.75rem; margin: 0 0 0.75rem; }
        .notice { background: var(--surface-panel); padding: 0.5rem 0.75rem; margin: 0 0 1rem; }
        [hidden] { display: none !important; }
        .faq h2 { font-size: 1rem; margin: 0 0 0.5rem; }
        a { color: var(--ink); }
    </style>
</head>
<body>
<main>
    <a class="logo" href="https://scutoons.com"><img src="https://scutoons.com/SCUTOONS-logo.svg" alt="SCUTOONS"></a>
    <?= $body ?>
</main>
<?php if ($clientConfig !== null): ?>
<script>window.QR_PAGE = <?= json_encode($clientConfig, JSON_HEX_TAG | JSON_HEX_AMP | JSON_HEX_APOS | JSON_HEX_QUOT | JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES) ?>;</script>
<script src="/assets/js/claim.js" defer></script>
<?php endif; ?>
</body>
</html>
<?php
    exit;
}

function messagePage(int $status, string $message): never
{
    http_response_code($status);
    renderPage($message, '<div class="panel"><p>' . h($message) . '</p><a class="btn btn-secondary" href="/">SCUTOONS</a></div>');
}

/** The client flow texts (subset of TEXTS) */
function clientTexts(array $t): array
{
    $keys = ['working', 'signing_in', 'done', 'done_hint', 'signed_in', 'open_app', 'choose_first', 'error', 'otp_hint', 'errors'];
    return array_intersect_key($t, array_flip($keys));
}

/** Entry form (email → one-time code → done), shared by the code page and the magic-link page */
function authSteps(array $t): string
{
    return '
    <div class="error" id="error" role="alert" hidden></div>
    <form class="panel" id="step-email" hidden novalidate>
        <label class="field-label" for="email">' . h($t['email']) . '</label>
        <input type="email" id="email" name="email" autocomplete="email" inputmode="email" required placeholder="you@example.com">
        <p class="caption">' . h($t['email_hint']) . '</p>
        <button type="submit">' . h($t['send']) . '</button>
    </form>
    <form class="panel" id="step-otp" hidden novalidate>
        <label class="field-label" for="otp">' . h($t['otp']) . '</label>
        <p class="caption"><span id="otp-hint"></span></p>
        <input type="text" id="otp" name="otp" class="otp" inputmode="numeric" autocomplete="one-time-code" maxlength="6" pattern="[0-9]{6}" required placeholder="000000">
        <button type="submit">' . h($t['confirm']) . '</button>
        <button type="button" class="btn-clear" id="other-email">' . h($t['other_email']) . '</button>
    </form>
    <div class="panel" id="step-status" hidden aria-live="polite">
        <h1 id="status-title"></h1>
        <p class="caption" id="status-hint"></p>
        <a class="btn" id="status-link" hidden></a>
    </div>';
}

/** One item row: icon + name + edition / variant */
function itemRow(array $item, ?string $variantId, ?string $meta, string $locale, string $tag = 'div', string $extra = ''): string
{
    $variant = Catalog::variant($item, $variantId);
    $name = Catalog::text($item['name'], $locale);
    $icon = Catalog::cdnUrl($variant['icon'] ?? $item['icon'] ?? null);
    $variantName = $variant ? Catalog::text($variant['name'], $locale) : null;
    $lines = array_filter([$variantName, $meta]);
    return '<' . $tag . ' class="item' . ($tag === 'div' ? ' single' : '') . '"' . $extra . '>'
        . '<span class="icon">' . ($icon ? '<img src="' . h($icon) . '" alt="" loading="lazy" onerror="this.replaceWith(Object.assign(document.createElement(\'span\'),{className:\'fallback\',textContent:' . h(json_encode(mb_substr($name, 0, 1))) . '}))">' : '') . '</span>'
        . '<span><span class="item-name">' . h($name) . '</span>'
        . implode('', array_map(fn($line) => '<span class="item-meta caption">' . h($line) . '</span>', $lines))
        . '</span></' . $tag . '>';
}

// ── Magic link without a code: sign in, then offer the app ─────────────────────────────────────────────────
$loginToken = is_string($_GET['login'] ?? null) ? $_GET['login'] : null;
$codeParam = isset($_GET['c']) && is_string($_GET['c']) ? trim($_GET['c']) : null;

if ($loginToken !== null && ($codeParam === null || $codeParam === '')) {
    renderPage($t['signing_in'], authSteps($t), [
        'mode' => 'login',
        'api' => '/api',
        'loginToken' => $loginToken,
        'appUrl' => Config::get('APP_URL'),
        'texts' => clientTexts($t),
    ]);
}

// ── No code: enter one by hand ───────────────────────────────────────────────────────────────────────────
if ($codeParam === null || $codeParam === '') {
    renderPage($t['members'], '
    <div class="panel">
        <h1>' . h($t['members']) . '</h1>
        <p>' . h($t['members_hint']) . '</p>
        <form method="GET">
            <label class="field-label" for="c">' . h($t['access_code']) . '</label>
            <input type="text" id="c" name="c" required maxlength="8" autocomplete="off" placeholder="' . h($t['access_code']) . '">
            <button type="submit">' . h($t['enter']) . '</button>
        </form>
    </div>
    <div class="faq">
        <h2>' . h($t['faq_q']) . '</h2>
        <p>' . h($t['faq_a']) . ' <a href="https://mandymozart.bandcamp.com/merch" target="_blank" rel="noopener">' . h($t['shop']) . '</a></p>
    </div>');
}

// ── A code ───────────────────────────────────────────────────────────────────────────────────────────────
try {
    $code = Codes::normalize($codeParam);
    $row = $code !== null ? Codes::find($code) : null;
} catch (Throwable $e) {
    error_log('[qr] landing: ' . $e->getMessage());
    messagePage(500, $t['error']);
}
if (!$row) messagePage(404, $t['not_found']);
if (qrIsExpired($row)) messagePage(410, $t['expired']);

// Skip, and codes that only point somewhere: redirect without claiming (unchanged behaviour)
if (isset($_GET['skip']) || !in_array($row['type'], Codes::CLAIMABLE_TYPES, true)) {
    header('Location: ' . continueUrl($row['redirect_to'], $row['code']));
    exit;
}

$fixedItems = Codes::items($row['code']);
$options = Codes::options($row, $fixedItems);
$edition = Codes::edition($row);
$editionLine = $edition && $edition['number'] !== null
    ? $t['edition'] . ' ' . $edition['number'] . ($edition['count'] ? ' / ' . $edition['count'] : '')
    : null;
$claimed = Codes::isClaimed($row);

// Choices: every item × variant of the options
$choices = [];
foreach ($options as $option) {
    foreach ($option['variants'] ?: [null] as $variantId) $choices[] = [$option['item_id'], $variantId];
}

$body = '<div class="panel">';
if ($fixedItems) {
    $body .= '<h1>' . h($t['found']) . '</h1><div class="items">';
    foreach ($fixedItems as $entry) {
        $item = Catalog::item($entry['item_id']);
        if (!$item) continue;
        $meta = trim(($entry['qty'] > 1 ? $entry['qty'] . ' × ' : '') . ($editionLine ?? ''));
        $body .= itemRow($item, $entry['variant'], $meta !== '' ? $meta : null, $locale);
    }
    $body .= '</div>';
    if (count($fixedItems) === 1 && ($item = Catalog::item($fixedItems[0]['item_id']))) {
        $body .= '<p class="item-desc">' . h(Catalog::text($item['description'], $locale)) . '</p>';
    }
} elseif (count($choices) === 1) {
    $item = Catalog::item($choices[0][0]);
    $body .= '<h1>' . h($t['found']) . '</h1><div class="items">' . itemRow($item, $choices[0][1], $editionLine, $locale) . '</div>'
        . '<p class="item-desc">' . h(Catalog::text($item['description'], $locale)) . '</p>';
} elseif ($choices) {
    $body .= '<h1>' . h($t['found_many']) . '</h1><p class="caption">' . h($t['found_many_hint'])
        . ($editionLine ? ' ' . h($editionLine) . '.' : '') . '</p>'
        . '<div class="items" role="radiogroup" id="choices">';
    foreach ($choices as $i => [$itemId, $variantId]) {
        $item = Catalog::item($itemId);
        $input = '<input type="radio" name="choice" value="' . h($itemId . '|' . ($variantId ?? '')) . '" data-item="' . h($itemId) . '" data-variant="' . h($variantId ?? '') . '">';
        $body .= str_replace('<span class="icon">', $input . '<span class="icon">', itemRow($item, $variantId, null, $locale, 'label'));
    }
    $body .= '</div>';
} else {
    $body .= '<h1>' . h($t['membership']) . '</h1><p>' . h($t['membership_hint']) . '</p>';
}

if ($claimed) {
    $body .= '<p class="notice">' . h($t['claimed']) . ' ' . h($t['claimed_hint']) . '</p>';
}
$body .= '<div class="actions" id="step-start">'
    . '<button type="button" id="add">' . h($claimed ? $t['sign_in'] : $t['add']) . '</button>'
    . '<a class="btn btn-secondary" id="skip" href="' . h('/?c=' . rawurlencode($row['code']) . '&skip=1') . '">' . h($claimed ? $t['continue'] : $t['skip']) . '</a>'
    . ($claimed ? '' : '<p class="caption">' . h($t['skip_hint']) . '</p>')
    . '</div></div>';
$body .= authSteps($t);

renderPage($fixedItems || $choices ? $t['found'] : $t['membership'], $body, [
    'mode' => 'claim',
    'api' => '/api',
    'code' => $row['code'],
    'locale' => $locale,
    'needsChoice' => !$fixedItems && count($choices) > 1,
    'loginToken' => $loginToken,
    'inventoryUrl' => continueUrl($row['redirect_to'], $row['code'], '&inventory=open'),
    'texts' => clientTexts($t),
]);
