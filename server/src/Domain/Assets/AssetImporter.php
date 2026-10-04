<?php
declare(strict_types=1);

namespace Scutoons\Domain\Assets;

use Scutoons\Infrastructure\AssetStorage\AssetStorage;
use Scutoons\Infrastructure\AssetStorage\LocalStorage;
use Scutoons\Infrastructure\AssetStorage\StorageKey;
use Scutoons\Infrastructure\AssetStorage\StoredFile;
use Scutoons\Infrastructure\Database\Database;

/**
 * Registers what is on a storage (agents/ASSETS.md "Import"): every file becomes a rendition of an asset, files of one
 * slug (AssetPaths::group) one asset. Idempotent – a re-scan only adds new files to the CURRENT version, updates
 * sizes / checksums that changed on the storage (an out-of-band upload, e.g. the bunny dashboard) and drops renditions whose
 * file is gone. Deleted assets are left alone. Never writes to the storage.
 */
final class AssetImporter
{
    public function __construct(
        private readonly AssetRegistry $registry,
        private readonly MediaProbe $probe,
        private readonly Database $db,
    ) {
    }

    /**
     * @param bool $probe read sizes / durations (local files: getimagesize, ffprobe)
     * @param bool $hash SHA-256 of local files (the storage's checksum is used when it reports one)
     * @return array{files: int, attached: int, assets_created: int, renditions_added: int, renditions_updated: int, renditions_removed: int, assets: int}
     */
    public function import(AssetStorage $storage, string $prefix = '', bool $probe = true, bool $hash = false): array
    {
        $files = [];
        foreach ($storage->list($prefix) as $file) {
            if (StorageKey::junk($file->key)) continue;
            $files[$file->key] = $file;
        }
        $groups = AssetPaths::group(array_keys($files));
        $stats = ['files' => count($files), 'attached' => 0, 'assets_created' => 0, 'renditions_added' => 0, 'renditions_updated' => 0, 'renditions_removed' => 0, 'assets' => count($groups)];
        $local = $storage instanceof LocalStorage ? $storage : null;

        foreach ($groups as $slug => $group) {
            $this->db->transaction(function () use ($slug, $group, $files, $storage, $local, $probe, $hash, $prefix, &$stats) {
                $asset = $this->registry->bySlug($slug);
                if ($asset && $asset['status'] !== 'active') return;
                if (!$asset) {
                    // files of an archived version (a renamed / replaced asset keeps its old files) belong to that asset
                    $marks = implode(',', array_fill(0, count($group['files']), '?'));
                    if ($this->db->value("SELECT COUNT(*) FROM asset_renditions WHERE path IN ($marks)", array_keys($group['files'])) > 0) {
                        $stats['attached']++;
                        return;
                    }
                    $id = $this->registry->create($slug, $group['kind']);
                    $source = $this->withHash($files[$group['source']], $local, $hash);
                    $earliest = $this->earliest(array_map(fn(string $k) => $files[$k], array_keys($group['files'])));
                    $versionId = $this->registry->addVersion($id, $source, $storage->name(), $this->probeOf($source, $local, $probe), null, 'imported from ' . $storage->name(), $earliest);
                    foreach ($group['files'] as $key => $meta) {
                        $file = $key === $group['source'] ? $source : $this->withHash($files[$key], $local, $hash);
                        $this->registry->addRendition($id, $versionId, $meta['label'], $meta['quality'], $file, $this->probeOf($file, $local, $probe));
                        $stats['renditions_added']++;
                    }
                    $stats['assets_created']++;
                    return;
                }
                $id = (string) $asset['id'];
                $version = $this->registry->currentVersion($id);
                if (!$version) return;
                $existing = [];
                foreach ($this->registry->renditions((int) $version['id']) as $r) $existing[(string) $r['path']] = $r;
                foreach ($group['files'] as $key => $meta) {
                    $file = $files[$key];
                    if (!isset($existing[$key])) {
                        $file = $this->withHash($file, $local, $hash);
                        $this->registry->addRendition($id, (int) $version['id'], $meta['label'], $meta['quality'], $file, $this->probeOf($file, $local, $probe));
                        $stats['renditions_added']++;
                        continue;
                    }
                    $row = $existing[$key];
                    $changed = (int) $row['size'] !== $file->size || ($file->sha256 !== null && $row['sha256'] !== null && $row['sha256'] !== $file->sha256);
                    if ($changed || ($hash && $row['sha256'] === null)) {
                        $file = $this->withHash($file, $local, $hash);
                        $this->registry->updateRendition((int) $row['id'], $file, $changed ? $this->probeOf($file, $local, $probe) : []);
                        if ($key === $version['path']) {
                            $this->db->run('UPDATE asset_versions SET size = ?, sha256 = COALESCE(?, sha256) WHERE id = ?', [$file->size, $file->sha256, $version['id']]);
                        }
                        $stats['renditions_updated']++;
                    }
                }
                // renditions whose file is gone (only within the scanned prefix)
                foreach ($existing as $path => $row) {
                    if (isset($files[$path]) || !str_starts_with($path, $prefix)) continue;
                    $this->registry->removeRendition((int) $row['id']);
                    $stats['renditions_removed']++;
                }
            });
        }
        return $stats;
    }

    private function withHash(StoredFile $file, ?LocalStorage $local, bool $hash): StoredFile
    {
        if (!$hash || $file->sha256 !== null || !$local) return $file;
        $sha = hash_file('sha256', $local->localPath($file->key));
        return $sha === false ? $file : new StoredFile($file->key, $file->size, $file->modified, $sha, $file->mime);
    }

    /** @return array{width: ?int, height: ?int, duration: ?float, bitrate: ?int} */
    private function probeOf(StoredFile $file, ?LocalStorage $local, bool $probe): array
    {
        if (!$probe || !$local) return ['width' => null, 'height' => null, 'duration' => null, 'bitrate' => null];
        return $this->probe->probe($local->localPath($file->key), AssetPaths::kind($file->key));
    }

    /** @param list<StoredFile> $files the oldest modification time as the version's date (UTC Y-m-d H:i:s) */
    private function earliest(array $files): ?string
    {
        $times = array_filter(array_map(fn(StoredFile $f) => $f->modified !== '' ? (int) strtotime($f->modified) : 0, $files));
        return $times ? gmdate('Y-m-d H:i:s', min($times)) : null;
    }
}
