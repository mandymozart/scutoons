<?php
/**
 * QR landing page (ITEMS.md "Access model", final: viewing is device-level, owning needs an account).
 *
 *   /?c=CODE            claimable types (permissions map api/permissions.json – today membership):
 *                       preview of the item(s) + Continue → redirect_to?code=CODE (the viewer grants device access
 *                       via /api/access/) and "Add to my account" → <viewer origin>/account?code=CODE (sign in there,
 *                       then claim). No email step here.
 *                       Signup-only types (not claimable, "signup": true – today promo): "Promo code – no item to
 *                       collect" + Continue and "Create account" → <viewer origin>/account?code=CODE.
 *                       Other types (page, event, xr, unknown) redirect right away.
 *   /?c=CODE&skip=1     redirect right away
 *   /?login=TOKEN       old magic links → forwarded to the viewer (APP_URL/?login=TOKEN)
 *   /                   enter a code by hand
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

/** scheme://host[:port] of redirect_to (the viewer the code points to), APP_URL as fallback */
function viewerOrigin(string $redirectTo): string
{
    $parts = parse_url($redirectTo);
    if (!isset($parts['scheme'], $parts['host']) || !in_array($parts['scheme'], ['http', 'https'], true)) {
        return rtrim(Config::get('APP_URL'), '/');
    }
    return $parts['scheme'] . '://' . $parts['host'] . (isset($parts['port']) ? ':' . $parts['port'] : '');
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
        'found_many' => 'Your code',
        'found_many_hint' => 'This code is for one of these. You pick yours when you add it to your account.',
        'membership' => 'Membership',
        'membership_hint' => 'This code unlocks every SCUTOONS chapter.',
        'edition' => 'Edition',
        'continue' => 'Continue',
        'continue_hint' => 'Unlocks every chapter on this device – no sign-up needed.',
        'add_account' => 'Add to my account',
        'add_account_hint' => 'Keep the item in your inventory and read on every device (sign in with your email).',
        'not_found' => 'QR code not found. Please contact us!',
        'expired' => 'This QR code has expired. Please contact us!',
        'revoked' => 'This code is no longer valid.',
        'error' => 'Something went wrong. Please try again later.',
        'members' => 'Members area',
        'members_hint' => 'Scan a QR code to get started or type it here:',
        'access_code' => 'Access code',
        'enter' => 'Enter',
        'faq_q' => "I don't have an access code. Where can I get one?",
        'faq_a' => 'QR codes come with SCUTOONS limited editions:',
        'shop' => 'Shop now (Bandcamp)',
        'promo' => 'Promo code – no item to collect',
        'promo_hint' => 'This code unlocks every SCUTOONS chapter on your device. You can also create an account to save your progress across devices and get news about new releases.',
        'create_account' => 'Create account',
        'create_account_hint' => 'Sign up with your email – this code lets you create one.',
    ],
    'de' => [
        'found' => 'Gegenstand gefunden',
        'found_many' => 'Dein Code',
        'found_many_hint' => 'Dieser Code gehört zu einem davon. Welchen du hast, wählst du beim Hinzufügen zu deinem Konto.',
        'membership' => 'Mitgliedschaft',
        'membership_hint' => 'Dieser Code schaltet alle SCUTOONS-Kapitel frei.',
        'edition' => 'Edition',
        'continue' => 'Weiter',
        'continue_hint' => 'Schaltet alle Kapitel auf diesem Gerät frei – ohne Anmeldung.',
        'add_account' => 'Zu meinem Konto hinzufügen',
        'add_account_hint' => 'Behalte den Gegenstand in deinem Inventar und lies auf jedem Gerät (Anmeldung mit E-Mail).',
        'not_found' => 'QR-Code nicht gefunden. Bitte kontaktiere uns!',
        'expired' => 'Dieser QR-Code ist abgelaufen. Bitte kontaktiere uns!',
        'revoked' => 'Dieser Code ist nicht mehr gültig.',
        'error' => 'Etwas ist schiefgelaufen. Bitte versuche es später noch einmal.',
        'members' => 'Mitgliederbereich',
        'members_hint' => 'Scanne einen QR-Code oder gib ihn hier ein:',
        'access_code' => 'Zugangscode',
        'enter' => 'Weiter',
        'faq_q' => 'Ich habe keinen Zugangscode. Wo bekomme ich einen?',
        'faq_a' => 'QR-Codes gibt es zu den limitierten SCUTOONS-Editionen:',
        'shop' => 'Zum Shop (Bandcamp)',
        'promo' => 'Promo-Code – kein Gegenstand zum Sammeln',
        'promo_hint' => 'Dieser Code schaltet alle SCUTOONS-Kapitel auf deinem Gerät frei. Du kannst auch ein Konto erstellen, um deinen Fortschritt auf allen Geräten zu speichern und Neuigkeiten zu neuen Veröffentlichungen zu bekommen.',
        'create_account' => 'Konto erstellen',
        'create_account_hint' => 'Registriere dich mit deiner E-Mail – mit diesem Code geht das.',
    ],
];

$locale = pageLocale();
$t = TEXTS[$locale];

/** Renders a full page in the Scutoons design (IBM Plex Mono, ink on paper, square, 2px rules) */
function renderPage(string $title, string $body): never
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
        .field-label { display: block; font-weight: 700; margin: 0 0 0.25rem; }
        .actions .caption { margin: -0.25rem 0 0.5rem; }
        .faq h2 { font-size: 1rem; margin: 0 0 0.5rem; }
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
<?php
    exit;
}

function messagePage(int $status, string $message): never
{
    http_response_code($status);
    renderPage($message, '<div class="panel"><p>' . h($message) . '</p><a class="btn btn-secondary" href="/">SCUTOONS</a></div>');
}

/** One item row: icon + name + edition / variant */
function itemRow(array $item, ?string $variantId, ?string $meta, string $locale, string $tag = 'div', string $extra = '', bool $highlight = true): string
{
    $variant = Catalog::variant($item, $variantId);
    $name = Catalog::text($item['name'], $locale);
    $icon = Catalog::cdnUrl($variant['icon'] ?? $item['icon'] ?? null);
    $variantName = $variant ? Catalog::text($variant['name'], $locale) : null;
    $lines = array_filter([$variantName, $meta]);
    return '<' . $tag . ' class="item' . ($tag === 'div' && $highlight ? ' single' : '') . '"' . $extra . '>'
        . '<span class="icon">' . ($icon ? '<img src="' . h($icon) . '" alt="" loading="lazy" onerror="this.replaceWith(Object.assign(document.createElement(\'span\'),{className:\'fallback\',textContent:' . h(json_encode(mb_substr($name, 0, 1))) . '}))">' : '') . '</span>'
        . '<span><span class="item-name">' . h($name) . '</span>'
        . implode('', array_map(fn($line) => '<span class="item-meta caption">' . h($line) . '</span>', $lines))
        . '</span></' . $tag . '>';
}

// ── Old magic links (…/?login=TOKEN): login happens in the viewer now ────────────────────────────────────
$loginToken = is_string($_GET['login'] ?? null) ? $_GET['login'] : null;
$codeParam = isset($_GET['c']) && is_string($_GET['c']) ? trim($_GET['c']) : null;
if ($loginToken !== null) {
    header('Location: ' . rtrim(Config::get('APP_URL'), '/') . '/?login=' . rawurlencode($loginToken));
    exit;
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
// Statistics: every scan of an existing code (also expired/revoked ones)
if (!isset($_GET['skip'])) Events::log('scan', $row['code'], null, Events::toonFromUrl($row['redirect_to']));
if (qrIsExpired($row)) messagePage(410, $t['expired']);
if (qrIsRevoked($row)) messagePage(410, $t['revoked']);

// Skip, and code types that can neither be owned nor create an account (permissions map: page/event/xr): redirect
$typeRules = Permissions::forType((string) $row['type']);
if (isset($_GET['skip']) || (!$typeRules['claimable'] && !$typeRules['signup'])) {
    header('Location: ' . continueUrl($row['redirect_to'], $row['code']));
    exit;
}
$accountUrl = viewerOrigin($row['redirect_to']) . '/account?code=' . rawurlencode($row['code']);

// Promo code (signup, nothing to own): no item preview; Continue + Create account
if (!$typeRules['claimable']) {
    renderPage($t['promo'], '<div class="panel" id="promo"><h1>' . h($t['promo']) . '</h1><p>' . h($t['promo_hint']) . '</p>'
        . '<div class="actions"><a class="btn" id="continue" href="' . h(continueUrl($row['redirect_to'], $row['code'])) . '">'
        . h($t['continue']) . '</a><p class="caption">' . h($t['continue_hint']) . '</p>'
        . '<a class="btn btn-secondary" id="create-account" href="' . h($accountUrl) . '">' . h($t['create_account']) . '</a>'
        . '<p class="caption">' . h($t['create_account_hint']) . '</p></div></div>');
}

// Claimable code: preview of what it contains + Continue + Add to my account. The viewer claims it, asks for the
// item choice when needed and decides what to do when the code already belongs to someone.
$fixedItems = Codes::items($row['code']);
$options = Codes::options($row, $fixedItems);
$edition = Codes::edition($row);
$editionLine = $edition && $edition['number'] !== null
    ? $t['edition'] . ' ' . $edition['number'] . ($edition['count'] ? ' / ' . $edition['count'] : '')
    : null;

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
    // Preview only – the choice is made in the viewer
    $body .= '<h1>' . h($t['found_many']) . '</h1><p class="caption">' . h($t['found_many_hint'])
        . ($editionLine ? ' ' . h($editionLine) . '.' : '') . '</p><div class="items">';
    foreach ($choices as [$itemId, $variantId]) {
        $body .= itemRow(Catalog::item($itemId), $variantId, null, $locale, 'div', '', false);
    }
    $body .= '</div>';
} else {
    $body .= '<h1>' . h($t['membership']) . '</h1><p>' . h($t['membership_hint']) . '</p>';
}
$body .= '<div class="actions"><a class="btn" id="continue" href="' . h(continueUrl($row['redirect_to'], $row['code'])) . '">'
    . h($t['continue']) . '</a><p class="caption">' . h($t['continue_hint']) . '</p>'
    . '<a class="btn btn-secondary" id="add-account" href="' . h($accountUrl) . '">'
    . h($t['add_account']) . '</a><p class="caption">' . h($t['add_account_hint']) . '</p></div></div>';

renderPage($fixedItems || $choices ? $t['found'] : $t['membership'], $body);
