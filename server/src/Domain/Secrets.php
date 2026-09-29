<?php
declare(strict_types=1);

namespace Scutoons\Domain;

use RuntimeException;
use Scutoons\Infrastructure\Config\Settings;

/**
 * Keys derived from AUTH_SECRET. Each purpose has its own label, so one key never serves two purposes.
 */
final class Secrets
{
    public function __construct(private readonly Settings $settings)
    {
    }

    public function authSecret(): string
    {
        $secret = $this->settings->get('AUTH_SECRET');
        if (strlen($secret) < 32) throw new RuntimeException('AUTH_SECRET is missing or shorter than 32 characters');
        return $secret;
    }

    /** HMAC-SHA256 (hex) of $value with AUTH_SECRET itself (login codes, IP hashes of login requests, lookup buckets) */
    public function hmac(string $value): string
    {
        return hash_hmac('sha256', $value, $this->authSecret());
    }

    /** A binary key for one purpose: HMAC(label, AUTH_SECRET) */
    public function derivedKey(string $label): string
    {
        return hash_hmac('sha256', $label, $this->authSecret(), true);
    }
}
