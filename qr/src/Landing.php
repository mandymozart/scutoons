<?php
declare(strict_types=1);

namespace Qr;

/**
 * PUBLIC URLS – printed on every QR code, must keep working unchanged (agents/PLATFORM.md "RULE: printed / public
 * URLs never change"). The QR landing page, rendered here on the server from the API's answers
 * (agents/ITEMS.md "Access model": viewing is device-level, owning needs an account):
 *
 *   /?c=CODE            claimable types (today membership): preview of the item(s). Not registered yet: "Add to my
 *                       account" first → <viewer origin>/account?code=CODE (sign in there, then claim) with "Register it
 *                       now – otherwise someone else can", then Continue → redirect_to?code=CODE (the viewer grants
 *                       device access). Registered to an account: Read (= Continue) + "This is mine? File a claim" →
 *                       redirect_to?code=CODE&claim=1 (the viewer's claim form) – agents/ITEMS.md "Ownership, claims,
 *                       gifting". Never says whose it is. No email step here.
 *                       Signup-only types (not claimable, "signup": true – today promo): "Promo code – no item to
 *                       collect" + Continue and "Create account" → <viewer origin>/account?code=CODE.
 *                       Other types (page, event, xr) redirect right away. Every lookup is a `scan` (API, &scan=1).
 *   /?c=CODE&skip=1     redirect right away (a fresh cached lookup is enough; no scan is counted)
 *   /?login=TOKEN       old magic links → forwarded to the viewer (WWW_URL/?login=TOKEN)
 *   /                   enter a code by hand
 * Unknown code 404, expired / revoked 410, too many lookups from this IP 429 (the API's per-IP limit), API down 503
 * with "Scanning works, try again in a minute" (+ Continue when the code is cached).
 * Every other path outside /api/ and /generate/ shows this page too (as the old catch-all rewrite did).
 */
final class Landing
{
    /** @var array<string, string> */
    private array $t = [];
    private string $locale = 'en';

    public function __construct(private readonly Config $config, private readonly Api $api, private readonly View $view)
    {
    }

    /** @param array<int|string, mixed> $query */
    public function handle(array $query, string $acceptLanguage): Response
    {
        $this->locale = self::pageLocale($query['lang'] ?? null, $acceptLanguage);
        /** @var array<string, array<string, string>> $texts */
        $texts = $this->view->data('landing/texts');
        $this->t = $texts[$this->locale];
        $t = $this->t;

        // Old magic links (…/?login=TOKEN): login happens in the viewer now
        $loginToken = is_string($query['login'] ?? null) ? $query['login'] : null;
        if ($loginToken !== null) {
            return Response::redirect(rtrim($this->config->get('WWW_URL'), '/') . '/?login=' . rawurlencode($loginToken));
        }

        $codeParam = isset($query['c']) && is_string($query['c']) ? trim($query['c']) : null;
        if ($codeParam === null || $codeParam === '') {
            return $this->page(200, $t['members'], $this->view->render('landing/enter', ['t' => $t]));
        }
        $code = self::normalize($codeParam);
        if ($code === null) return $this->message(404, $t['not_found']);

        $skip = isset($query['skip']);
        $lookup = $this->api->code($code, !$skip, $skip);
        if ($lookup === null) return $this->unavailable($code);
        $status = $lookup['status'];
        $info = $lookup['body'];
        if ($status === 429) {
            $response = $this->message(429, $t['rate_limited']);
            return $lookup['retry_after'] !== null ? $response->withHeader('Retry-After', $lookup['retry_after']) : $response;
        }
        if ($status === 410) return $this->message(410, ($info['error'] ?? '') === 'revoked' ? $t['revoked'] : $t['expired']);
        if ($status === 404 || $status === 400) return $this->message(404, $t['not_found']);
        if ($status !== 200 || !is_string($info['redirect_to'] ?? null) || !is_string($info['code'] ?? null)) return $this->unavailable($code);

        $redirectTo = $info['redirect_to'];
        $code = $info['code'];
        $continueUrl = self::continueUrl($redirectTo, $code);
        $claimable = ($info['claimable'] ?? false) === true;
        $signup = ($info['signup'] ?? false) === true;
        // Skip, and code types that can neither be owned nor create an account (page/event/xr): redirect
        if ($skip || (!$claimable && !$signup)) return Response::redirect($continueUrl);
        $accountUrl = $this->viewerOrigin($redirectTo) . '/account?code=' . rawurlencode($code);

        // Promo code (signup, nothing to own): no item preview; Continue + Create account
        if (!$claimable) {
            return $this->page(200, $t['promo'], $this->view->render('landing/promo', compact('t', 'continueUrl', 'accountUrl')));
        }
        $items = $this->api->items();
        if ($items === null) return $this->unavailable($code);
        return $this->codePage($info, $items, $continueUrl, $accountUrl);
    }

    /**
     * @param array<string, mixed> $info the API's /code/ answer
     * @param array<string, array<string, mixed>> $catalog
     */
    private function codePage(array $info, array $catalog, string $continueUrl, string $accountUrl): Response
    {
        $t = $this->t;
        /** @var list<array{item_id: string, variant: ?string, qty: int}> $fixedItems */
        $fixedItems = is_array($info['items'] ?? null) ? $info['items'] : [];
        /** @var list<array{item_id: string, variants: list<string>}> $options */
        $options = is_array($info['options'] ?? null) ? $info['options'] : [];
        $edition = is_array($info['edition'] ?? null) ? $info['edition'] : null;
        $editionLine = $edition && ($edition['number'] ?? null) !== null
            ? $t['edition'] . ' ' . $edition['number'] . (!empty($edition['count']) ? ' / ' . $edition['count'] : '')
            : null;
        $choices = [];
        foreach ($options as $option) {
            foreach ($option['variants'] ?: [null] as $variantId) $choices[] = [(string) $option['item_id'], $variantId];
        }

        $heading = $t['membership'];
        $hint = null;
        $hintCaption = false;
        $rows = [];
        $description = null;
        if ($fixedItems) {
            $heading = $t['found'];
            foreach ($fixedItems as $entry) {
                $item = $catalog[$entry['item_id']] ?? null;
                if (!$item) continue;
                $qty = (int) ($entry['qty'] ?? 1);
                $meta = trim(($qty > 1 ? $qty . ' × ' : '') . ($editionLine ?? ''));
                $rows[] = $this->itemRow($item, $entry['variant'] ?? null, $meta !== '' ? $meta : null);
            }
            $item = count($fixedItems) === 1 ? ($catalog[$fixedItems[0]['item_id']] ?? null) : null;
            if ($item) $description = self::text($item['description'] ?? '', $this->locale);
        } elseif (count($choices) === 1) {
            $heading = $t['found'];
            $item = $catalog[$choices[0][0]] ?? [];
            $rows[] = $this->itemRow($item, $choices[0][1], $editionLine);
            $description = self::text($item['description'] ?? '', $this->locale);
        } elseif ($choices) {
            // Preview only – the choice is made in the viewer
            $heading = $t['found_many'];
            $hint = $t['found_many_hint'] . ($editionLine ? ' ' . $editionLine . '.' : '');
            $hintCaption = true;
            foreach ($choices as [$itemId, $variantId]) {
                $rows[] = $this->itemRow($catalog[$itemId] ?? [], $variantId, null, false);
            }
        } else {
            $hint = $t['membership_hint'];
        }
        // registered (an account or the old email form): read + "This is mine? File a claim"; else register first
        $claimed = ($info['claimed'] ?? false) === true;
        $claimUrl = $continueUrl . '&claim=1';
        $body = $this->view->render('landing/code', compact('t', 'heading', 'hint', 'hintCaption', 'rows', 'description', 'continueUrl', 'accountUrl', 'claimed', 'claimUrl'));
        return $this->page(200, $fixedItems || $choices ? $t['found'] : $t['membership'], $body);
    }

    /**
     * Variables of the item partial: icon + name + variant / edition lines
     * @param array<string, mixed> $item
     * @return array<string, mixed>
     */
    private function itemRow(array $item, ?string $variantId, ?string $meta, bool $highlight = true): array
    {
        $variant = null;
        foreach (is_array($item['variants'] ?? null) ? $item['variants'] : [] as $candidate) {
            if (($candidate['id'] ?? null) === $variantId) $variant = $candidate;
        }
        $name = self::text($item['name'] ?? '', $this->locale);
        $icon = $this->cdnUrl($variant['icon'] ?? $item['icon'] ?? null);
        $variantName = $variant ? self::text($variant['name'] ?? '', $this->locale) : null;
        return ['name' => $name, 'icon' => $icon, 'lines' => array_values(array_filter([$variantName, $meta])), 'highlight' => $highlight];
    }

    /** API down (or answering nonsense): friendly page, with Continue when this code was looked up recently */
    private function unavailable(string $code): Response
    {
        $cached = $this->api->cachedCode($code, $this->config->int('CODE_CACHE_KEEP'));
        $continueUrl = is_string($cached['redirect_to'] ?? null) && is_string($cached['code'] ?? null)
            ? self::continueUrl($cached['redirect_to'], $cached['code']) : null;
        $body = $this->view->render('landing/unavailable', ['t' => $this->t, 'continueUrl' => $continueUrl]);
        return $this->page(503, $this->t['unavailable'], $body)->withHeader('Retry-After', '60');
    }

    private function message(int $status, string $message): Response
    {
        return $this->page($status, $message, $this->view->render('landing/message', ['message' => $message]));
    }

    private function page(int $status, string $title, string $body): Response
    {
        return Response::html($status, $this->view->render('landing/layout', ['locale' => $this->locale, 'title' => $title, 'body' => $body]));
    }

    /** redirect_to + ?code=CODE */
    public static function continueUrl(string $redirectTo, string $code): string
    {
        return $redirectTo . (str_contains($redirectTo, '?') ? '&' : '?') . 'code=' . rawurlencode($code);
    }

    /** scheme://host[:port] of redirect_to (the viewer the code points to), WWW_URL as fallback */
    private function viewerOrigin(string $redirectTo): string
    {
        $parts = parse_url($redirectTo);
        if (!is_array($parts) || !isset($parts['scheme'], $parts['host']) || !in_array($parts['scheme'], ['http', 'https'], true)) {
            return rtrim($this->config->get('WWW_URL'), '/');
        }
        return $parts['scheme'] . '://' . $parts['host'] . (isset($parts['port']) ? ':' . $parts['port'] : '');
    }

    private function cdnUrl(mixed $path): ?string
    {
        if (!is_string($path) || $path === '') return null;
        return rtrim($this->config->get('CDN_URL'), '/') . '/' . ltrim($path, '/');
    }

    /** The code as the API accepts it (server: Domain\Codes\CodeRules::normalize), else null */
    public static function normalize(string $value): ?string
    {
        $code = trim($value);
        return preg_match('/^[A-Za-z0-9_-]{1,32}$/', $code) ? $code : null;
    }

    /** Localised text of a { de, en } map */
    public static function text(mixed $map, string $locale): string
    {
        if (!is_array($map)) return is_scalar($map) ? (string) $map : '';
        $first = reset($map);
        return (string) ($map[$locale] ?? $map['en'] ?? ($first === false ? '' : $first));
    }

    public static function pageLocale(mixed $lang, string $acceptLanguage): string
    {
        if (is_string($lang) && in_array($lang, ['de', 'en'], true)) return $lang;
        return str_starts_with(strtolower($acceptLanguage), 'de') ? 'de' : 'en';
    }
}
