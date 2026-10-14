<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;

/**
 * Protected files – a score's MusicXML and PDF (agents/research/SCORES.md §5; album downloads later). They live on
 * their own bunny.net storage + pull zone with token authentication (PROTECTED_CDN_URL, PROTECTED_CDN_KEY); the API
 * hands out short-lived signed URLs, and the public catalog never names their paths (CatalogController::items).
 * Rules (owner 2026-10-14): the MusicXML for anyone opening the score in the view (a guest's inventory lives on the
 * device – the server can't check it), the PDF only to a signed-in account that owns the score.
 * Without a key (dev) the plain CDN URL is answered – the local media server has no tokens.
 */
final class ProtectedFiles
{
    /** how long a signed URL works */
    public const TTL = 600;

    public function __construct(
        private readonly Settings $settings,
        private readonly Clock $clock,
        private readonly Catalog $catalog,
        private readonly InventoryService $inventory,
    ) {
    }

    /**
     * GET /api/items/{id}/score/{file} → { url, expires }. 404 unknown_item / no_score / no_file, 401 (pdf, signed out),
     * 403 not_owner (pdf).
     * @param array<string, mixed>|null $user the signed-in account, if any
     * @return array{url: string, expires: int}
     */
    public function score(string $itemId, string $file, ?array $user): array
    {
        $item = $this->catalog->item($itemId) ?? throw new ApiError(404, 'unknown_item', 'Unknown item.');
        $score = is_array($item['score'] ?? null) ? $item['score'] : throw new ApiError(404, 'no_score', 'This item has no score.');
        if ($file !== 'musicxml' && $file !== 'pdf') throw new ApiError(404, 'no_file', 'musicxml or pdf.');
        $path = is_string($score[$file] ?? null) && $score[$file] !== '' ? $score[$file] : throw new ApiError(404, 'no_file', 'This score has no such file.');
        if ($file === 'pdf') {
            if ($user === null) throw new ApiError(401, 'unauthorized', 'Sign in to download the PDF.');
            if ($this->inventory->count((string) $user['id'], $itemId) < 1) throw new ApiError(403, 'not_owner', 'Only the owner of this score can download it.');
        }
        return $this->signed($path);
    }

    /**
     * A signed URL of a protected file (bunny.net token authentication, HMAC-SHA256: the URL path + expires, base64url,
     * prefix `HS256-` – BunnyWay/BunnyCDN.TokenAuthentication php/url_signing.php)
     * @return array{url: string, expires: int}
     */
    public function signed(string $path): array
    {
        $expires = $this->clock->time() + self::TTL;
        $urlPath = '/' . implode('/', array_map('rawurlencode', explode('/', ltrim($path, '/'))));
        $key = $this->settings->get('PROTECTED_CDN_KEY');
        if ($key === '') return ['url' => rtrim($this->settings->get('CDN_URL'), '/') . $urlPath, 'expires' => $expires];
        $token = 'HS256-' . rtrim(strtr(base64_encode(hash_hmac('sha256', $urlPath . $expires, $key, true)), '+/', '-_'), '=');
        return ['url' => rtrim($this->settings->get('PROTECTED_CDN_URL'), '/') . "$urlPath?token=$token&expires=$expires", 'expires' => $expires];
    }

    /**
     * The catalog as readers get it: a score names its language and whether it has a PDF, never its paths
     * @param array<string, mixed> $item
     * @return array<string, mixed>
     */
    public static function publicItem(array $item): array
    {
        if (!is_array($item['score'] ?? null)) return $item;
        $item['score'] = ['language' => $item['score']['language'] ?? null, 'pdf' => is_string($item['score']['pdf'] ?? null) && $item['score']['pdf'] !== ''];
        return $item;
    }
}
