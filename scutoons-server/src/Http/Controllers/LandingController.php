<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Access\Permissions;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Domain\Codes\CodeRules;
use Scutoons\Domain\Codes\CodeService;
use Scutoons\Domain\Events\EventLog;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Templates\Templates;
use Throwable;

/**
 * PUBLIC URLS – printed on every QR code, must keep working unchanged (agents/PLATFORM.md "RULE: printed / public
 * URLs never change"). The QR landing page (agents/ITEMS.md "Access model": viewing is device-level, owning needs an
 * account):
 *
 *   /?c=CODE            claimable types (permissions map – today membership): preview of the item(s) + Continue →
 *                       redirect_to?code=CODE (the viewer grants device access) and "Add to my account" →
 *                       <viewer origin>/account?code=CODE (sign in there, then claim). No email step here.
 *                       Signup-only types (not claimable, "signup": true – today promo): "Promo code – no item to
 *                       collect" + Continue and "Create account" → <viewer origin>/account?code=CODE.
 *                       Other types (page, event, xr, unknown) redirect right away.
 *   /?c=CODE&skip=1     redirect right away
 *   /?login=TOKEN       old magic links → forwarded to the viewer (APP_URL/?login=TOKEN)
 *   /                   enter a code by hand
 * Every other path outside /api/ and /generate/ shows this page too (as the old catch-all rewrite did).
 */
final class LandingController
{
    /** @var array<string, string> */
    private array $t = [];
    private string $locale = 'en';

    public function __construct(
        private readonly Templates $templates,
        private readonly Settings $settings,
        private readonly Clock $clock,
        private readonly CodeService $codes,
        private readonly Catalog $catalog,
        private readonly Permissions $permissions,
        private readonly EventLog $events,
    ) {
    }

    public function __invoke(Request $request, Response $response): Response
    {
        $query = $request->getQueryParams();
        $this->locale = self::pageLocale($query['lang'] ?? null, $request->getHeaderLine('Accept-Language'));
        /** @var array<string, array<string, string>> $texts */
        $texts = $this->templates->data('landing/texts');
        $this->t = $texts[$this->locale];
        $t = $this->t;

        // Old magic links (…/?login=TOKEN): login happens in the viewer now
        $loginToken = is_string($query['login'] ?? null) ? $query['login'] : null;
        if ($loginToken !== null) {
            return $response->withStatus(302)->withHeader('Location', rtrim($this->settings->get('APP_URL'), '/') . '/?login=' . rawurlencode($loginToken));
        }

        $codeParam = isset($query['c']) && is_string($query['c']) ? trim($query['c']) : null;
        if ($codeParam === null || $codeParam === '') {
            return $this->page($response, 200, $t['members'], $this->templates->render('landing/enter', ['t' => $t]));
        }

        try {
            $code = CodeRules::normalize($codeParam);
            $row = $code !== null ? $this->codes->find($code) : null;
        } catch (Throwable $error) {
            error_log('[qr] landing: ' . $error->getMessage());
            return $this->message($response, 500, $t['error']);
        }
        if (!$row) return $this->message($response, 404, $t['not_found']);
        // Statistics: every scan of an existing code (also expired/revoked ones)
        if (!isset($query['skip'])) $this->events->log('scan', (string) $row['code'], null, EventLog::toonFromUrl((string) $row['redirect_to']));
        if (CodeRules::isExpired($row, $this->clock->time())) return $this->message($response, 410, $t['expired']);
        if (CodeRules::isRevoked($row)) return $this->message($response, 410, $t['revoked']);

        $redirectTo = (string) $row['redirect_to'];
        $code = (string) $row['code'];
        $continueUrl = self::continueUrl($redirectTo, $code);
        // Skip, and code types that can neither be owned nor create an account (page/event/xr): redirect
        $rules = $this->permissions->forType((string) $row['type']);
        if (isset($query['skip']) || (!$rules['claimable'] && !$rules['signup'])) {
            return $response->withStatus(302)->withHeader('Location', $continueUrl);
        }
        $accountUrl = $this->viewerOrigin($redirectTo) . '/account?code=' . rawurlencode($code);

        // Promo code (signup, nothing to own): no item preview; Continue + Create account
        if (!$rules['claimable']) {
            return $this->page($response, 200, $t['promo'], $this->templates->render('landing/promo', compact('t', 'continueUrl', 'accountUrl')));
        }
        return $this->codePage($response, $row, $continueUrl, $accountUrl);
    }

    /** @param array<string, mixed> $row */
    private function codePage(Response $response, array $row, string $continueUrl, string $accountUrl): Response
    {
        $t = $this->t;
        $fixedItems = $this->codes->items((string) $row['code']);
        $options = $this->codes->options($row, $fixedItems);
        $edition = $this->codes->edition($row);
        $editionLine = $edition && $edition['number'] !== null
            ? $t['edition'] . ' ' . $edition['number'] . ($edition['count'] ? ' / ' . $edition['count'] : '')
            : null;
        $choices = [];
        foreach ($options as $option) {
            foreach ($option['variants'] ?: [null] as $variantId) $choices[] = [$option['item_id'], $variantId];
        }

        $heading = $t['membership'];
        $hint = null;
        $hintCaption = false;
        $rows = [];
        $description = null;
        if ($fixedItems) {
            $heading = $t['found'];
            foreach ($fixedItems as $entry) {
                $item = $this->catalog->item($entry['item_id']);
                if (!$item) continue;
                $meta = trim(($entry['qty'] > 1 ? $entry['qty'] . ' × ' : '') . ($editionLine ?? ''));
                $rows[] = $this->itemRow($item, $entry['variant'], $meta !== '' ? $meta : null);
            }
            $item = count($fixedItems) === 1 ? $this->catalog->item($fixedItems[0]['item_id']) : null;
            if ($item) $description = Catalog::text($item['description'], $this->locale);
        } elseif (count($choices) === 1) {
            $heading = $t['found'];
            $item = $this->catalog->item((string) $choices[0][0]) ?? [];
            $rows[] = $this->itemRow($item, $choices[0][1], $editionLine);
            $description = Catalog::text($item['description'] ?? '', $this->locale);
        } elseif ($choices) {
            // Preview only – the choice is made in the viewer
            $heading = $t['found_many'];
            $hint = $t['found_many_hint'] . ($editionLine ? ' ' . $editionLine . '.' : '');
            $hintCaption = true;
            foreach ($choices as [$itemId, $variantId]) {
                $rows[] = $this->itemRow($this->catalog->item((string) $itemId) ?? [], $variantId, null, false);
            }
        } else {
            $hint = $t['membership_hint'];
        }
        $body = $this->templates->render('landing/code', compact('t', 'heading', 'hint', 'hintCaption', 'rows', 'description', 'continueUrl', 'accountUrl'));
        return $this->page($response, 200, $fixedItems || $choices ? $t['found'] : $t['membership'], $body);
    }

    /**
     * Variables of the item partial: icon + name + variant / edition lines
     * @param array<string, mixed> $item
     * @return array<string, mixed>
     */
    private function itemRow(array $item, ?string $variantId, ?string $meta, bool $highlight = true): array
    {
        $variant = $this->catalog->variant($item, $variantId);
        $name = Catalog::text($item['name'] ?? '', $this->locale);
        $icon = $this->catalog->cdnUrl($variant['icon'] ?? $item['icon'] ?? null);
        $variantName = $variant ? Catalog::text($variant['name'], $this->locale) : null;
        return ['name' => $name, 'icon' => $icon, 'lines' => array_values(array_filter([$variantName, $meta])), 'highlight' => $highlight];
    }

    private function message(Response $response, int $status, string $message): Response
    {
        return $this->page($response, $status, $message, $this->templates->render('landing/message', ['message' => $message]));
    }

    private function page(Response $response, int $status, string $title, string $body): Response
    {
        $response->getBody()->write($this->templates->render('landing/layout', ['locale' => $this->locale, 'title' => $title, 'body' => $body]));
        return $response->withStatus($status)->withHeader('Content-Type', 'text/html; charset=UTF-8');
    }

    /** redirect_to + ?code=CODE */
    public static function continueUrl(string $redirectTo, string $code): string
    {
        return $redirectTo . (str_contains($redirectTo, '?') ? '&' : '?') . 'code=' . rawurlencode($code);
    }

    /** scheme://host[:port] of redirect_to (the viewer the code points to), APP_URL as fallback */
    private function viewerOrigin(string $redirectTo): string
    {
        $parts = parse_url($redirectTo);
        if (!is_array($parts) || !isset($parts['scheme'], $parts['host']) || !in_array($parts['scheme'], ['http', 'https'], true)) {
            return rtrim($this->settings->get('APP_URL'), '/');
        }
        return $parts['scheme'] . '://' . $parts['host'] . (isset($parts['port']) ? ':' . $parts['port'] : '');
    }

    public static function pageLocale(mixed $lang, string $acceptLanguage): string
    {
        if (is_string($lang) && in_array($lang, ['de', 'en'], true)) return $lang;
        return str_starts_with(strtolower($acceptLanguage), 'de') ? 'de' : 'en';
    }
}
