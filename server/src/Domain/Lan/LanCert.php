<?php
declare(strict_types=1);

namespace Scutoons\Domain\Lan;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;

/**
 * The venue certificate (#201): the wildcard `*.lan.scutoons.com` from Let's Encrypt (DNS-01 at bunny DNS, where the
 * zone lan.scutoons.com lives), issued and renewed by the workflow lan-cert.yml, which hands it to the API
 * (`PUT /api/deploy/lan-cert`, the deploy key); an approved deck fetches it with its own token (`GET /api/decks/lan-cert`,
 * `deck-tls`, #180). The names need no registration: bunny's script answers `192-168-1-116.lan.scutoons.com` with
 * 192.168.1.116 (deploy/bunny/lan-dns.js) – the API never touches DNS.
 *
 * Kept as files in `LAN_CERT_PATH` (storage/lan-cert – outside the docroot, never deployed over): fullchain.pem,
 * privkey.pem (0600), meta.json (names, expiry, when stored). Why files and not the database: it is one pair the
 * workflow replaces whole, a dump of the DB (the dev seed!) must never carry the private key, and the relay wants files.
 */
final class LanCert
{
    /** the venue names' zone (bunny DNS, delegated from all-inkl by an NS record) */
    public const DOMAIN = 'lan.scutoons.com';
    /** the venue certificate's name */
    public const VENUE = '*.lan.scutoons.com';
    /** #441: the hosted relay's certificate, the same path to a different reader (GET /api/relay/live-cert) */
    public const LIVE = 'live.scutoons.com';

    public function __construct(private readonly Settings $settings, private readonly Clock $clock)
    {
    }

    /**
     * Check and store a new pair. With ext-openssl (production, dev): it must be a certificate for
     * `$name` (self::VENUE or self::LIVE), not expired, and the key must be its key.
     * @return array{names: list<string>, expires: ?string, stored_at: string}
     */
    public function store(string $fullchain, string $privkey, string $name = self::VENUE): array
    {
        $fullchain = trim(str_replace("\r\n", "\n", $fullchain)) . "\n";
        $privkey = trim(str_replace("\r\n", "\n", $privkey)) . "\n";
        if (!str_contains($fullchain, '-----BEGIN CERTIFICATE-----')) throw new ApiError(422, 'invalid_certificate', 'fullchain is not a PEM certificate chain.');
        if (preg_match('/-----BEGIN [A-Z ]*PRIVATE KEY-----/', $privkey) !== 1) throw new ApiError(422, 'invalid_key', 'privkey is not a PEM private key.');
        $names = [];
        $expires = null;
        if (function_exists('openssl_x509_parse')) {
            $info = @openssl_x509_parse($fullchain);
            if (!is_array($info)) throw new ApiError(422, 'invalid_certificate', 'fullchain does not parse as a certificate.');
            $names = self::names($info);
            if (!in_array($name, $names, true)) throw new ApiError(422, 'wrong_certificate', "The certificate is not for $name (it names " . implode(', ', $names) . ').');
            $until = (int) ($info['validTo_time_t'] ?? 0);
            if ($until <= $this->clock->time()) throw new ApiError(422, 'expired_certificate', 'The certificate has expired.');
            $expires = gmdate('Y-m-d\TH:i:s\Z', $until);
            if (!@openssl_x509_check_private_key($fullchain, $privkey)) throw new ApiError(422, 'key_mismatch', 'privkey is not the certificate\'s key.');
        }
        $meta = ['names' => $names, 'expires' => $expires, 'stored_at' => gmdate('Y-m-d\TH:i:s\Z', $this->clock->time())];
        $dir = $this->dir($name);
        if (!is_dir($dir) && !@mkdir($dir, 0700, true) && !is_dir($dir)) throw new ApiError(500, 'storage', 'Cannot create the certificate folder.');
        self::write("$dir/fullchain.pem", $fullchain, 0644);
        self::write("$dir/privkey.pem", $privkey, 0600);
        self::write("$dir/meta.json", (string) json_encode($meta, JSON_UNESCAPED_SLASHES | JSON_PRETTY_PRINT), 0644);
        return $meta;
    }

    /**
     * The stored pair and what it is.
     * @return array{names: list<string>, expires: ?string, stored_at: ?string, fullchain: string, privkey: string}
     */
    public function get(string $name = self::VENUE): array
    {
        $dir = $this->dir($name);
        $fullchain = is_file("$dir/fullchain.pem") ? (string) file_get_contents("$dir/fullchain.pem") : '';
        $privkey = is_file("$dir/privkey.pem") ? (string) file_get_contents("$dir/privkey.pem") : '';
        if ($fullchain === '' || $privkey === '') throw new ApiError(404, 'no_certificate', "No certificate for $name yet – run the workflow lan-cert (server/docs/API.md \"The venue certificate\").");
        $meta = is_file("$dir/meta.json") ? json_decode((string) file_get_contents("$dir/meta.json"), true) : null;
        $meta = is_array($meta) ? $meta : [];
        $names = array_values(array_filter(is_array($meta['names'] ?? null) ? $meta['names'] : [], 'is_string'));
        return [
            'names' => $names,
            'expires' => is_string($meta['expires'] ?? null) ? $meta['expires'] : null,
            'stored_at' => is_string($meta['stored_at'] ?? null) ? $meta['stored_at'] : null,
            'fullchain' => $fullchain,
            'privkey' => $privkey,
        ];
    }

    private function dir(string $name): string
    {
        $setting = $name === self::LIVE ? 'LIVE_CERT_PATH' : 'LAN_CERT_PATH';
        $dir = $this->settings->path($setting);
        if ($dir === '') throw new ApiError(503, 'not_configured', "$setting is empty.");
        return rtrim($dir, '/');
    }

    /**
     * subject CN + the DNS names of subjectAltName
     * @param array<string, mixed> $info openssl_x509_parse
     * @return list<string>
     */
    private static function names(array $info): array
    {
        $names = [];
        $cn = $info['subject']['CN'] ?? null;
        if (is_string($cn)) $names[] = $cn;
        $san = $info['extensions']['subjectAltName'] ?? '';
        foreach (explode(',', is_string($san) ? $san : '') as $part) {
            $part = trim($part);
            if (str_starts_with($part, 'DNS:')) $names[] = substr($part, 4);
        }
        return array_values(array_unique($names));
    }

    /** a temp file next to it, then a rename: a reader never sees half a file */
    private static function write(string $path, string $content, int $mode): void
    {
        $tmp = $path . '.' . bin2hex(random_bytes(4)) . '.tmp';
        if (@file_put_contents($tmp, $content) === false) throw new ApiError(500, 'storage', 'Cannot write the certificate.');
        @chmod($tmp, $mode);
        if (!@rename($tmp, $path)) {
            @unlink($tmp);
            throw new ApiError(500, 'storage', 'Cannot write the certificate.');
        }
    }
}
