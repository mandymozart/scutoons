<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\AssetStorage;

use Scutoons\Infrastructure\Config\Settings;

/**
 * The bunny.net settings, normalised and checked (agents/ASSETS.md "Connecting bunny.net"). Tolerant input – an
 * endpoint or public URL with or without `https://` and trailing slash – and a clear status per key for
 * `assets:check-storage` (set / missing / invalid + why). Password values never leave this object except into the
 * AccessKey header of BunnyStorage.
 */
final class BunnyConfig
{
    public const DEFAULT_ENDPOINT = 'storage.bunnycdn.com';
    /** the documented region endpoints (bunny.net/docs/storage/http "Storage endpoints") */
    public const KNOWN_ENDPOINTS = ['storage.bunnycdn.com', 'uk.storage.bunnycdn.com', 'ny.storage.bunnycdn.com', 'la.storage.bunnycdn.com',
        'sg.storage.bunnycdn.com', 'se.storage.bunnycdn.com', 'br.storage.bunnycdn.com', 'jh.storage.bunnycdn.com', 'syd.storage.bunnycdn.com'];

    public function __construct(
        public readonly string $zone,
        public readonly string $endpoint,
        public readonly string $password,
        public readonly string $readonlyPassword,
        public readonly string $pullZoneId,
        public readonly string $apiKey,
        public readonly string $publicBase,
        public readonly string $writePrefix = '',
        /** @var array<string, array{status: string, value: ?string, note: string}> */
        public readonly array $status = [],
    ) {
    }

    public static function fromSettings(Settings $settings, string $fallbackPublicBase = ''): self
    {
        $zone = trim($settings->get('BUNNY_STORAGE_ZONE'));
        $rawEndpoint = trim($settings->get('BUNNY_STORAGE_ENDPOINT'));
        $endpoint = self::host($rawEndpoint) ?: self::DEFAULT_ENDPOINT;
        $password = trim($settings->get('BUNNY_STORAGE_PASSWORD'));
        $readonly = trim($settings->get('BUNNY_STORAGE_READONLY_PASSWORD'));
        $rawPullZone = trim($settings->get('BUNNY_PULLZONE_ID'));
        $pullZone = preg_match('/^\d{1,12}$/', $rawPullZone) ? $rawPullZone : '';
        $apiKey = trim($settings->get('BUNNY_API_KEY'));
        $rawPublic = trim($settings->get('CDN_PUBLIC_URL'));
        $public = self::baseUrl($rawPublic !== '' ? $rawPublic : $fallbackPublicBase);

        $status = [];
        $status['BUNNY_STORAGE_ZONE'] = $zone === ''
            ? ['status' => 'missing', 'value' => null, 'note' => 'the storage zone name (bunny.net → Storage)']
            : (preg_match('/^[A-Za-z0-9-]{1,64}$/', $zone) ? ['status' => 'set', 'value' => $zone, 'note' => '']
                : ['status' => 'invalid', 'value' => $zone, 'note' => 'a zone name has only letters, digits and dashes']);
        $status['BUNNY_STORAGE_ENDPOINT'] = $rawEndpoint === ''
            ? ['status' => 'missing', 'value' => self::DEFAULT_ENDPOINT, 'note' => 'using the default (Falkenstein/Frankfurt); the zone → FTP & API Access → Hostname says which one']
            : (in_array($endpoint, self::KNOWN_ENDPOINTS, true) ? ['status' => 'set', 'value' => $endpoint, 'note' => '']
                : ['status' => (str_ends_with($endpoint, '.bunnycdn.com') ? 'set' : 'invalid'), 'value' => $endpoint, 'note' => 'not one of the documented region endpoints']);
        $status['BUNNY_STORAGE_PASSWORD'] = $password === ''
            ? ['status' => 'missing', 'value' => null, 'note' => 'read/write password: without it the asset layer cannot upload / replace / delete on the zone']
            : ['status' => 'set', 'value' => null, 'note' => ''];
        $status['BUNNY_STORAGE_READONLY_PASSWORD'] = $readonly === ''
            ? ['status' => 'missing', 'value' => null, 'note' => $password === '' ? 'no password at all: the zone cannot be read' : 'reads use the read/write password']
            : ['status' => 'set', 'value' => null, 'note' => ''];
        $status['BUNNY_PULLZONE_ID'] = $rawPullZone === ''
            ? ['status' => 'missing', 'value' => null, 'note' => 'purge, statistics and request logs need the numeric pull zone id']
            : ($pullZone !== '' ? ['status' => 'set', 'value' => $pullZone, 'note' => '']
                : ['status' => 'invalid', 'value' => $rawPullZone, 'note' => 'must be the NUMERIC pull zone id (bunny.net → CDN → the zone: the number in its URL), not its name – purge / statistics off']);
        $status['BUNNY_API_KEY'] = $apiKey === ''
            ? ['status' => 'missing', 'value' => null, 'note' => 'account API key: without it purge after a replace and the bandwidth statistics are off']
            : ['status' => 'set', 'value' => null, 'note' => ''];
        $status['CDN_PUBLIC_URL'] = $rawPublic === ''
            ? ['status' => 'missing', 'value' => $public ?: null, 'note' => $public !== '' ? 'using CDN_URL' : 'no public base: no purge, no links']
            : ($public !== '' ? ['status' => 'set', 'value' => $public, 'note' => $public !== rtrim($rawPublic, '/') ? 'normalised' : '']
                : ['status' => 'invalid', 'value' => $rawPublic, 'note' => 'not a URL / host name']);

        return new self($zone, $endpoint, $password, $readonly, $pullZone, $apiKey, $public, trim($settings->get('BUNNY_WRITE_PREFIX')), $status);
    }

    /** a zone name and at least one of its passwords */
    public function usable(): bool
    {
        return $this->zone !== '' && ($this->password !== '' || $this->readonlyPassword !== '');
    }

    /** "https://storage.bunnycdn.com/" / "storage.bunnycdn.com" → "storage.bunnycdn.com" ('' when it is not a host) */
    public static function host(string $value): string
    {
        $value = trim($value);
        if ($value === '') return '';
        if (!preg_match('#^[a-z]+://#i', $value)) $value = 'https://' . $value;
        $host = parse_url($value, PHP_URL_HOST);
        return is_string($host) && preg_match('/^[A-Za-z0-9.-]+$/', $host) ? strtolower($host) : '';
    }

    /** "scutoons.b-cdn.net" / "https://cdn.scutoons.com/" → "https://scutoons.b-cdn.net" / "https://cdn.scutoons.com" */
    public static function baseUrl(string $value): string
    {
        $value = trim($value);
        if ($value === '') return '';
        if (!preg_match('#^[a-z]+://#i', $value)) $value = 'https://' . $value;
        $parts = parse_url($value);
        if (!is_array($parts) || empty($parts['host']) || !preg_match('/^[A-Za-z0-9.-]+$/', $parts['host'])) return '';
        $scheme = strtolower($parts['scheme'] ?? 'https');
        $port = isset($parts['port']) ? ':' . $parts['port'] : '';
        return $scheme . '://' . strtolower($parts['host']) . $port . rtrim($parts['path'] ?? '', '/');
    }
}
