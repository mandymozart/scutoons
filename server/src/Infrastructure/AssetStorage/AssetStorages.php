<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\AssetStorage;

use RuntimeException;
use Scutoons\Infrastructure\Config\Settings;

/**
 * The configured media storages (agents/ASSETS.md "Configuration"):
 *   ASSET_STORAGE        local | bunny – the one the asset layer writes to and scans (default local)
 *   ASSET_LOCAL_ROOT     the local media folder (default ../media)
 *   CDN_PUBLIC_URL       the public base readers load media from (default CDN_URL)
 *   BUNNY_STORAGE_ZONE, BUNNY_STORAGE_ENDPOINT, BUNNY_STORAGE_PASSWORD, BUNNY_STORAGE_READONLY_PASSWORD,
 *   BUNNY_PULLZONE_ID, BUNNY_API_KEY, BUNNY_WRITE_PREFIX (BunnyConfig)
 * Both can exist at once: `assets:check-storage` compares the zone with the local folder, a migration copies one into
 * the other (the in-between layer is what makes switching storages a configuration change).
 */
final class AssetStorages
{
    private ?LocalStorage $local = null;
    private ?BunnyStorage $bunny = null;

    public function __construct(private readonly Settings $settings, private readonly ?HttpClient $http = null)
    {
    }

    public function publicBase(): string
    {
        return $this->settings->get('CDN_PUBLIC_URL') ?: $this->settings->get('CDN_URL');
    }

    /** The public base of the storage the asset layer works on – where its files are loaded from (thumbnails, previews) */
    public function primaryPublicBase(): string
    {
        return $this->primaryName() === 'local' ? $this->settings->get('CDN_URL') : $this->publicBase();
    }

    /** The storage the asset layer works on (ASSET_STORAGE) */
    public function primary(): AssetStorage
    {
        return $this->byName($this->primaryName());
    }

    /**
     * ASSET_STORAGE resolved. `auto` = the local CDN folder when it exists (every dev machine: credentials in the
     * root .env must never turn a dev server into a writer on the production zone), else bunny when configured.
     */
    public function primaryName(): string
    {
        $name = $this->settings->get('ASSET_STORAGE') ?: 'auto';
        if ($name !== 'auto') return $name;
        if (is_dir($this->local()->root())) return 'local';
        return $this->bunny() !== null ? 'bunny' : 'local';
    }

    public function byName(string $name): AssetStorage
    {
        return match ($name) {
            'local' => $this->local(),
            'bunny' => $this->bunny() ?? throw new RuntimeException('bunny.net is not configured (BUNNY_STORAGE_ZONE + a zone password in the repository-root .env / config.php).'),
            default => throw new RuntimeException("Unknown ASSET_STORAGE: $name (local | bunny)"),
        };
    }

    public function local(): LocalStorage
    {
        $root = $this->settings->path('ASSET_LOCAL_ROOT') ?: $this->settings->root() . '/../media';
        $real = realpath($root);
        // the local folder is what CDN_URL serves (dev: the CDN on :8001); bunny's public base is CDN_PUBLIC_URL
        return $this->local ??= new LocalStorage($real !== false ? $real : $root, $this->settings->get('CDN_URL'));
    }

    public function bunnyConfig(): BunnyConfig
    {
        return BunnyConfig::fromSettings($this->settings, $this->settings->get('CDN_URL'));
    }

    /**
     * The protected storage (agents/research/SCORES.md §5 – scores' MusicXML / PDF, served only through signed URLs):
     * its own bunny.net zone (BUNNY_PROTECTED_STORAGE_ZONE + _PASSWORD, the public zone's endpoint), else on a dev
     * machine the local media folder (the dev CDN serves it, like ProtectedFiles' fallback), else null – never the
     * public zone.
     */
    public function protectedStorage(): ?AssetStorage
    {
        $zone = trim($this->settings->get('BUNNY_PROTECTED_STORAGE_ZONE'));
        $password = trim($this->settings->get('BUNNY_PROTECTED_STORAGE_PASSWORD'));
        if ($zone !== '' && $password !== '') {
            $public = $this->bunnyConfig();
            return new BunnyStorage($this->http ?? new CurlHttpClient(), new BunnyConfig($zone, $public->endpoint, $password, '', '', '', rtrim($this->settings->get('PROTECTED_CDN_URL'), '/')));
        }
        return $this->primaryName() === 'local' ? $this->local() : null;
    }

    /** null when the zone or both of its passwords are not configured */
    public function bunny(): ?BunnyStorage
    {
        if ($this->bunny) return $this->bunny;
        $config = $this->bunnyConfig();
        if (!$config->usable()) return null;
        return $this->bunny = new BunnyStorage($this->http ?? new CurlHttpClient(), $config);
    }

    /**
     * Per bunny key: set / missing / invalid, the value only for non-secrets (zone, endpoint, pull zone id, public URL)
     * @return array<string, array{status: string, value: ?string, note: string}>
     */
    public function configured(): array
    {
        return $this->bunnyConfig()->status;
    }
}
