<?php
declare(strict_types=1);

namespace Scutoons\Domain\Lan;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Kas\KasApi;
use Scutoons\Infrastructure\Kas\KasFault;

/**
 * The decks' LAN names (#201, agents/research/LIVE.md B.4): `<name>.lan.scutoons.com` is an A record at all-inkl that
 * points to a deck's private IP, so a phone on https://scutoons.com can open `wss://<name>.lan.scutoons.com:7700` (the
 * wildcard certificate, LanCert). Set through the KAS API (KasApi) with the server's credentials (`KAS_LOGIN`,
 * `KAS_AUTH_DATA`); without them every call is 503 `not_configured`.
 *
 * Only private IPv4 addresses (RFC 1918: 10/8, 172.16/12, 192.168/16): the names never point to the internet, so
 * they cannot be used to put a page of ours in front of someone else's server. Names: `[a-z0-9-]{1,32}`, no dash at
 * either end (a DNS label).
 */
final class LanNames
{
    public const ZONE = 'scutoons.com';
    public const SUB = 'lan';

    public function __construct(private readonly Settings $settings, private readonly KasApi $kas)
    {
    }

    public static function domain(): string
    {
        return self::SUB . '.' . self::ZONE;
    }

    public function configured(): bool
    {
        return trim($this->settings->get('KAS_LOGIN')) !== '' && trim($this->settings->get('KAS_AUTH_DATA')) !== '';
    }

    /**
     * Every `<name>.lan.scutoons.com` A record, by name.
     * @return list<array{name: string, host: string, ip: string, record_id: string}>
     */
    public function list(): array
    {
        $this->requireConfigured();
        $records = $this->kas('get_dns_settings', ['zone_host' => self::ZONE . '.']);
        $names = [];
        foreach (is_array($records) ? $records : [] as $record) {
            if (!is_array($record) || strtoupper((string) ($record['record_type'] ?? '')) !== 'A') continue;
            $name = self::lanName((string) ($record['record_name'] ?? ''));
            if ($name === null) continue;
            $names[] = ['name' => $name, 'host' => "$name." . self::domain(), 'ip' => (string) ($record['record_data'] ?? ''), 'record_id' => (string) ($record['record_id'] ?? '')];
        }
        usort($names, static fn(array $a, array $b): int => strcmp($a['name'], $b['name']));
        return $names;
    }

    /**
     * Point `<name>.lan.scutoons.com` to `$ip`: added, updated, or left alone when it already does.
     * @return array{name: string, host: string, ip: string, change: string}
     */
    public function set(string $name, string $ip): array
    {
        $name = strtolower(trim($name));
        $ip = trim($ip);
        if (!self::validName($name)) throw new ApiError(422, 'invalid_name', 'A LAN name is 1–32 of a-z, 0-9 and -, not starting or ending with -.');
        if (!self::privateIpv4($ip)) throw new ApiError(422, 'invalid_ip', 'Only a private IPv4 address (10.x, 172.16–31.x, 192.168.x).');
        $this->requireConfigured();
        $existing = null;
        foreach ($this->list() as $record) {
            if ($record['name'] === $name) {
                $existing = $record;
                break;
            }
        }
        $item = ['name' => $name, 'host' => "$name." . self::domain(), 'ip' => $ip];
        $recordName = $name . '.' . self::SUB;
        if ($existing === null) {
            $this->kas('add_dns_settings', ['zone_host' => self::ZONE . '.', 'record_type' => 'A', 'record_name' => $recordName, 'record_data' => $ip, 'record_aux' => 0]);
            return $item + ['change' => 'added'];
        }
        if ($existing['ip'] === $ip) return $item + ['change' => 'unchanged'];
        $this->kas('update_dns_settings', ['record_id' => $existing['record_id'], 'record_name' => $recordName, 'record_data' => $ip, 'record_aux' => 0]);
        return $item + ['change' => 'updated'];
    }

    public static function validName(string $name): bool
    {
        return preg_match('/^[a-z0-9](?:[a-z0-9-]{0,30}[a-z0-9])?$/', $name) === 1;
    }

    /** RFC 1918 only – 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16 */
    public static function privateIpv4(string $ip): bool
    {
        if (filter_var($ip, FILTER_VALIDATE_IP, FILTER_FLAG_IPV4) === false) return false;
        $n = (int) ip2long($ip);
        return ($n & 0xFF000000) === 0x0A000000 || ($n & 0xFFF00000) === 0xAC100000 || ($n & 0xFFFF0000) === 0xC0A80000;
    }

    /** `x1.lan` (or `x1.lan.scutoons.com.`) → `x1`; anything outside lan.scutoons.com → null */
    private static function lanName(string $recordName): ?string
    {
        $name = strtolower(rtrim(trim($recordName), '.'));
        if (str_ends_with($name, '.' . self::ZONE)) $name = substr($name, 0, -strlen('.' . self::ZONE));
        $suffix = '.' . self::SUB;
        if (!str_ends_with($name, $suffix)) return null;
        $name = substr($name, 0, -strlen($suffix));
        return self::validName($name) ? $name : null;
    }

    private function requireConfigured(): void
    {
        if (!$this->configured()) throw new ApiError(503, 'not_configured', 'LAN names are not configured on this server (KAS_LOGIN, KAS_AUTH_DATA).');
    }

    /** @param array<string, string|int> $params */
    private function kas(string $action, array $params): mixed
    {
        try {
            return $this->kas->call($action, $params);
        } catch (KasFault $e) {
            throw new ApiError(502, 'kas_error', "all-inkl's KAS API refused $action: " . $e->getMessage());
        }
    }
}
