<?php
declare(strict_types=1);

namespace Scutoons\Domain\Catalog;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;

/**
 * Protected files – a score's MusicXML and PDF (agents/research/SCORES.md §5; album downloads later). They live on
 * their own bunny.net storage + pull zone with token authentication (PROTECTED_CDN_URL, BUNNY_PROTECTED_CDN_KEY); the API
 * hands out short-lived signed URLs, and the public catalog never names their paths (CatalogController::items).
 * Rules (owner 2026-10-14): the MusicXML for anyone opening the score in the view (a guest's inventory lives on the
 * device – the server can't check it), the PDF only to a signed-in account that owns the score.
 * Without a key (dev) the plain CDN URL is answered – the local media server has no tokens.
 */
final class ProtectedFiles
{
    /** how long a signed URL works */
    public const TTL = 600;

    /** the media quality levels, lowest first (@scutoons/content MEDIA_QUALITIES) */
    public const QUALITIES = ['low', 'medium', 'high'];

    public function __construct(
        private readonly Settings $settings,
        private readonly Clock $clock,
        private readonly Catalog $catalog,
        private readonly InventoryService $inventory,
    ) {
    }

    /**
     * GET /api/items/{id}/score/{file}[?part=<name>] → { url, expires } – the full score, or one of its named parts
     * (`score.parts.<name>`: the choir …). 404 unknown_item / no_score / no_part / no_file, 401 (pdf, signed out),
     * 403 not_owner (pdf).
     * @param array<string, mixed>|null $user the signed-in account, if any
     * @return array{url: string, expires: int}
     */
    public function score(string $itemId, string $file, ?array $user, ?string $part = null): array
    {
        $item = $this->catalog->item($itemId) ?? throw new ApiError(404, 'unknown_item', 'Unknown item.');
        $score = is_array($item['score'] ?? null) ? $item['score'] : throw new ApiError(404, 'no_score', 'This item has no score.');
        if ($part !== null && $part !== '') {
            $score = is_array($score['parts'][$part] ?? null) ? $score['parts'][$part] : throw new ApiError(404, 'no_part', 'This score has no such part.');
        }
        if ($file !== 'musicxml' && $file !== 'pdf') throw new ApiError(404, 'no_file', 'musicxml or pdf.');
        $path = is_string($score[$file] ?? null) && $score[$file] !== '' ? $score[$file] : throw new ApiError(404, 'no_file', 'This score has no such file.');
        if ($file === 'pdf') {
            if ($user === null) throw new ApiError(401, 'unauthorized', 'Sign in to download the PDF.');
            if ($this->inventory->count((string) $user['id'], $itemId) < 1) throw new ApiError(403, 'not_owner', 'Only the owner of this score can download it.');
        }
        return $this->signed($path);
    }

    /** paths in the vault (the protected storage): scores' files, physical releases' audio */
    public static function inVault(string $path): bool
    {
        return str_starts_with($path, 'scores/') || str_starts_with($path, 'albums/');
    }

    /**
     * GET /api/albums/{id}/download | /api/albums/{id}/tracks/{track} → { url, expires } – a physical release's zip / one
     * track (owner 2026-10-14) by the album's `access` (agents/ITEMS.md "Players, albums"): `everyone`; `members` = a
     * signed-in account; `owners` = an account owning a carrier of the album (an item with `album` = it). Defaults:
     * download members, listen owners (the reader also needs a player – the app's rule). A file not in the vault (an older
     * album) is answered with its public CDN URL. A track's file follows the reader's media quality `?quality=low|medium|high`
     * (owner 2026-10-16; `trackFile`); without one its `src`. 404 unknown_album / unknown_track / no_file, 401, 403 not_owner.
     * @param array<string, mixed>|null $user
     * @return array{url: string, expires: int}
     */
    public function album(string $albumId, ?string $trackId, ?array $user, ?string $quality = null): array
    {
        $album = null;
        foreach ((array) ($this->catalog->data()['albums'] ?? []) as $a) if (is_array($a) && ($a['id'] ?? null) === $albumId) $album = $a;
        if ($album === null) throw new ApiError(404, 'unknown_album', 'Unknown album.');
        if ($trackId === null) {
            $path = is_string($album['download'] ?? null) && $album['download'] !== '' ? $album['download'] : throw new ApiError(404, 'no_file', 'This album has no download.');
            $rule = $album['access']['download'] ?? 'members';
        } else {
            $track = null;
            foreach ((array) ($album['tracks'] ?? []) as $t) if (is_array($t) && ($t['id'] ?? null) === $trackId) $track = $t;
            $path = is_string($track['src'] ?? null) && $track['src'] !== '' ? self::trackFile($track, $quality) : throw new ApiError(404, 'unknown_track', 'Unknown track.');
            $rule = $album['access']['listen'] ?? 'owners';
        }
        if ($rule !== 'everyone') {
            if ($user === null) throw new ApiError(401, 'unauthorized', 'Sign in for this album.');
            if ($rule === 'owners') {
                $carriers = array_keys(array_filter($this->catalog->items(), fn($i) => ($i['album'] ?? null) === $albumId));
                $owns = array_filter($carriers, fn($id) => $this->inventory->count((string) $user['id'], (string) $id) > 0) !== [];
                if (!$owns) throw new ApiError(403, 'not_owner', 'Only owners of this release.');
            }
        }
        if (!self::inVault($path)) return ['url' => rtrim($this->settings->get('CDN_URL'), '/') . '/' . ltrim($path, '/'), 'expires' => $this->clock->time() + self::TTL];
        return $this->signed($path);
    }

    /**
     * A track's file at a quality level: the level's variant, else the nearest lower one, else `src` (@scutoons/content
     * pickQualityVariant – a reader on "low" never gets a bigger file because a variant is missing); no / an unknown level → `src`
     * @param array<string, mixed> $track
     */
    public static function trackFile(array $track, ?string $quality): string
    {
        $src = (string) $track['src'];
        $at = $quality === null ? false : array_search($quality, self::QUALITIES, true);
        if ($at === false || !is_array($track['variants'] ?? null)) return $src;
        for ($i = (int) $at; $i >= 0; $i--) {
            $file = $track['variants'][self::QUALITIES[$i]] ?? null;
            if (is_string($file) && $file !== '') return $file;
        }
        return $src;
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
        $key = $this->settings->get('BUNNY_PROTECTED_CDN_KEY');
        if ($key === '') return ['url' => rtrim($this->settings->get('CDN_URL'), '/') . $urlPath, 'expires' => $expires];
        $token = 'HS256-' . rtrim(strtr(base64_encode(hash_hmac('sha256', $urlPath . $expires, $key, true)), '+/', '-_'), '=');
        return ['url' => rtrim($this->settings->get('PROTECTED_CDN_URL'), '/') . "$urlPath?token=$token&expires=$expires", 'expires' => $expires];
    }

    /**
     * The catalog as readers get it: a score names its language, whether it has a PDF and its parts, never its paths
     * @param array<string, mixed> $item
     * @return array<string, mixed>
     */
    public static function publicItem(array $item): array
    {
        if (!is_array($item['score'] ?? null)) return $item;
        $hasPdf = fn(mixed $files) => is_array($files) && is_string($files['pdf'] ?? null) && $files['pdf'] !== '';
        $parts = is_array($item['score']['parts'] ?? null) ? $item['score']['parts'] : [];
        $item['score'] = ['language' => $item['score']['language'] ?? null, 'pdf' => $hasPdf($item['score'])]
            + ($parts !== [] ? ['parts' => array_map(fn($p) => ['pdf' => $hasPdf($p)], $parts)] : []);
        return $item;
    }
}
