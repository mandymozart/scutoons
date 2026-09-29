<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Signing;

/**
 * The ML-DSA-65 keypair of the certificate signatures (KEYPAIR_PATH, default storage/keys/keypair.json – private,
 * never served). Signing happens client-side in the admin tools (noble-post-quantum); the server stores the keypair,
 * hands the public key to everyone and the full keypair to admins only.
 */
final class KeyStore
{
    public function __construct(private readonly string $path)
    {
    }

    public function path(): string
    {
        return $this->path;
    }

    /** @return array<string, mixed>|null the stored keypair { privateKey, publicKey, algorithm, generated }, or null */
    public function keypair(): ?array
    {
        if (!is_file($this->path)) return null;
        $keys = json_decode((string) file_get_contents($this->path), true);
        return is_array($keys) ? $keys : null;
    }

    public function exists(): bool
    {
        return is_file($this->path);
    }
}
