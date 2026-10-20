<?php
declare(strict_types=1);

namespace Scutoons\Domain\Storyboard;

use Scutoons\Domain\Secrets;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;

/**
 * Signed links to work-storage files for the providers (#121): the work storage is private (content editors only), but
 * Luma / Runway fetch their reference images by URL. So the server hands the provider a short-lived link to exactly one
 * file: `<API_URL>/api/storyboard/ref/<toon>/<chapter>/<name>?expires=<unix>&sig=<hmac>` – GET only, valid
 * STORYBOARD_REF_MINUTES (15), the HMAC (a key derived from AUTH_SECRET for this purpose only) covers toon, chapter,
 * name and the expiry. CDN assets are public already and go to the provider as they are.
 */
final class RefLinks
{
    public function __construct(private readonly Settings $settings, private readonly Secrets $secrets, private readonly Clock $clock)
    {
    }

    private function minutes(): int
    {
        return max(1, min(24 * 60, $this->settings->int('STORYBOARD_REF_MINUTES') ?: 15));
    }

    private function sig(string $toon, int $chapter, string $name, int $expires): string
    {
        return substr(hash_hmac('sha256', "$toon/$chapter/$name/$expires", $this->secrets->derivedKey('storyboard-ref-link')), 0, 40);
    }

    /** The provider's link to one work file */
    public function link(string $toon, int $chapter, string $name): string
    {
        $expires = $this->clock->time() + $this->minutes() * 60;
        return rtrim($this->settings->get('API_URL'), '/') . "/api/storyboard/ref/$toon/$chapter/" . rawurlencode($name)
            . "?expires=$expires&sig=" . $this->sig($toon, $chapter, $name, $expires);
    }

    /** The link's signature fits and it hasn't run out (nor reaches further than one validity ahead) */
    public function valid(string $toon, int $chapter, string $name, mixed $expires, mixed $sig): bool
    {
        if (!is_string($expires) || !preg_match('/^\d{1,12}$/', $expires) || !is_string($sig)) return false;
        $at = (int) $expires;
        $now = $this->clock->time();
        if ($at < $now || $at > $now + $this->minutes() * 60) return false;
        return hash_equals($this->sig($toon, $chapter, $name, $at), $sig);
    }
}
