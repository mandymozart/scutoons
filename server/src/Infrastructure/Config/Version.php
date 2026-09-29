<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Config;

/**
 * The deployed version of this API: `version` from composer.json, git sha + build time from version.json (written into
 * the server folder by the deploy, deploy/deploy.mjs; absent locally → sha "dev").
 * version.json = { app, version, git_sha, built_at, shared } – the same shape every app build writes.
 */
final class Version
{
    /** @var array{app: string, version: string, git_sha: string, built_at: ?string}|null */
    private ?array $info = null;

    public function __construct(private readonly Settings $settings)
    {
    }

    /** @return array{app: string, version: string, git_sha: string, built_at: ?string} */
    public function info(): array
    {
        if ($this->info !== null) return $this->info;
        $root = $this->settings->root();
        $composer = json_decode((string) @file_get_contents($root . '/composer.json'), true);
        $built = json_decode((string) @file_get_contents($root . '/version.json'), true);
        $built = is_array($built) ? $built : [];
        return $this->info = [
            'app' => 'api',
            'version' => (string) ($composer['version'] ?? '0.0.0'),
            'git_sha' => is_string($built['git_sha'] ?? null) ? $built['git_sha'] : 'dev',
            'built_at' => is_string($built['built_at'] ?? null) ? $built['built_at'] : null,
        ];
    }
}
