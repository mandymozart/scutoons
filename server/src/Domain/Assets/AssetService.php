<?php
declare(strict_types=1);

namespace Scutoons\Domain\Assets;

use RuntimeException;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Infrastructure\AssetStorage\AssetStorage;
use Scutoons\Infrastructure\AssetStorage\AssetStorages;
use Scutoons\Infrastructure\AssetStorage\StorageKey;
use Scutoons\Infrastructure\AssetStorage\StoredFile;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use stdClass;
use Throwable;

/**
 * The asset operations (agents/ASSETS.md "Operations"). Content keeps referencing stable CDN paths; these keep the
 * registry, the storage and the references consistent:
 *   upload   a new file (new asset, or a new rendition of an existing one) – never overwrites (409 exists)
 *   replace  new bytes at the SAME path (new version; the old bytes are kept under .versions/; CDN purge)
 *   relink   point every reference of one path at another (content: new draft versions; catalog: the JSON file)
 *   rename   copy an asset's files to a new name + relink (the old files stay until nothing references them)
 *   delete   refuses while anything references the asset (the sanity check), else removes its files
 */
final class AssetService
{
    public const UPLOAD_EXTENSIONS = ['jpg', 'jpeg', 'png', 'webp', 'gif', 'svg', 'avif', 'mp4', 'webm', 'mp3', 'm4a', 'ogg', 'wav', 'flac', 'zip', 'pdf'];
    public const ARCHIVE_PREFIX = '.versions/';

    public function __construct(
        private readonly AssetRegistry $registry,
        private readonly AssetUsages $usages,
        private readonly AssetStorages $storages,
        private readonly MediaProbe $probe,
        private readonly EntityStore $entities,
        private readonly Database $db,
        private readonly Settings $settings,
    ) {
    }

    public function storage(): AssetStorage
    {
        return $this->storages->primary();
    }

    public function maxUploadBytes(): int
    {
        return max(1, $this->settings->int('ASSET_UPLOAD_MAX_BYTES') ?: 209715200);
    }

    /** A clean, allowed target path or a 422 */
    public static function targetPath(string $path): string
    {
        try {
            $key = StorageKey::normalize($path);
        } catch (Throwable) {
            throw new ApiError(422, 'invalid_path', 'Invalid path.');
        }
        if (StorageKey::junk($key)) throw new ApiError(422, 'invalid_path', 'Hidden files and folders are not allowed.');
        if (!preg_match('#^[A-Za-z0-9_][A-Za-z0-9_./ -]*$#', $key) || str_contains($key, '//')) throw new ApiError(422, 'invalid_path', 'Use letters, digits, "-", "_", "." and "/" in paths.');
        if (!str_contains($key, '/')) throw new ApiError(422, 'invalid_path', 'Files go into a folder (e.g. catpope/images/…).');
        if (!in_array(AssetPaths::extension($key), self::UPLOAD_EXTENSIONS, true)) {
            throw new ApiError(422, 'invalid_type', 'This file type is not accepted.', ['allowed' => self::UPLOAD_EXTENSIONS]);
        }
        return $key;
    }

    /** The bytes must match the path's extension (a PNG named .webp is refused) */
    private static function checkContent(string $file, string $key): void
    {
        $ext = AssetPaths::extension($key);
        $head = (string) file_get_contents($file, false, null, 0, 64);
        $ok = match ($ext) {
            'jpg', 'jpeg' => str_starts_with($head, "\xFF\xD8\xFF"),
            'png' => str_starts_with($head, "\x89PNG"),
            'gif' => str_starts_with($head, 'GIF8'),
            'webp' => str_starts_with($head, 'RIFF') && substr($head, 8, 4) === 'WEBP',
            'svg' => (bool) preg_match('/<svg\b/i', (string) file_get_contents($file, false, null, 0, 4096)),
            'mp4', 'm4a' => substr($head, 4, 4) === 'ftyp',
            'webm' => str_starts_with($head, "\x1A\x45\xDF\xA3"),
            'mp3' => str_starts_with($head, 'ID3') || (strlen($head) > 1 && ord($head[0]) === 0xFF && (ord($head[1]) & 0xE0) === 0xE0),
            'zip' => str_starts_with($head, "PK\x03\x04") || str_starts_with($head, "PK\x05\x06"),
            'pdf' => str_starts_with($head, '%PDF'),
            default => true,
        };
        if (!$ok) throw new ApiError(422, 'content_mismatch', 'The file content does not match its extension.');
        if ($ext === 'svg' && preg_match('/<script\b|\bon[a-z]+\s*=|javascript:/i', (string) file_get_contents($file))) {
            throw new ApiError(422, 'unsafe_svg', 'SVGs with scripts or event handlers are not accepted.');
        }
    }

    /**
     * Upload a NEW file. Becomes a rendition of the asset with the same slug (e.g. `x.480p.mp4` next to `x.1080p.mp4`) or
     * a new asset. An existing path → 409 exists (replace it explicitly).
     * @param array<string, mixed> $user
     * @param list<string> $tags
     * @return array<string, mixed>
     */
    public function upload(array $user, string $localFile, string $path, string $title = '', array $tags = []): array
    {
        $this->registry->requireAvailable();
        $key = self::targetPath($path);
        $size = (int) filesize($localFile);
        if ($size <= 0) throw new ApiError(422, 'empty_file', 'The file is empty.');
        if ($size > $this->maxUploadBytes()) throw new ApiError(413, 'file_too_large', 'The file is larger than ' . $this->maxUploadBytes() . ' bytes.');
        self::checkContent($localFile, $key);
        $storage = $this->storage();
        $existing = $storage->stat($key);
        if ($existing) {
            $owner = $this->registry->byPath($key);
            throw new ApiError(409, 'exists', 'A file with this path exists – replace it or choose another name.', ['asset' => $owner['asset']['id'] ?? null, 'path' => $key]);
        }
        $stored = $storage->upload($key, $localFile, StorageKey::mime($key));
        $stored = new StoredFile($stored->key, $stored->size, $stored->modified !== '' ? $stored->modified : gmdate('c'), $stored->sha256 ?? (hash_file('sha256', $localFile) ?: null), $stored->mime);
        $parsed = AssetPaths::parse($key);
        $probe = $this->probe->probe($localFile, $parsed['kind']);
        $by = (string) $user['id'];
        $assetId = $this->db->transaction(function () use ($parsed, $stored, $storage, $probe, $by, $title, $tags) {
            $asset = $this->registry->bySlug($parsed['slug']);
            if ($asset && $asset['status'] === 'active' && ($version = $this->registry->currentVersion((string) $asset['id']))) {
                $label = AssetPaths::label($parsed, false);
                $this->registry->addRendition((string) $asset['id'], (int) $version['id'], $label, $parsed['quality'], $stored, $probe, 'uploaded');
                $this->registry->touch((string) $asset['id']);
                return (string) $asset['id'];
            }
            if ($asset) $this->db->run('UPDATE assets SET slug = ? WHERE id = ?', [$parsed['slug'] . '~deleted-' . substr((string) $asset['id'], 0, 8), $asset['id']]);
            $id = $this->registry->create($parsed['slug'], $parsed['kind'], $title, $tags, $by);
            $versionId = $this->registry->addVersion($id, $stored, $storage->name(), $probe, $by, 'uploaded');
            $this->registry->addRendition($id, $versionId, AssetPaths::label($parsed, true), $parsed['quality'], $stored, $probe, 'uploaded');
            return $id;
        });
        return $this->registry->detail($assetId) + ['uploaded' => $key];
    }

    /**
     * Replace the bytes of one file IN PLACE (same path – nothing in content changes): a new version, the old bytes are
     * copied to .versions/<asset>/<n>/<file> first, the CDN cache is purged. Other renditions of the old version move
     * along and are reported as stale (regenerate them with an optimization job).
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function replace(array $user, string $assetId, string $localFile, ?string $path = null, ?string $note = null): array
    {
        $this->registry->requireAvailable();
        $asset = $this->registry->asset($assetId) ?? throw new ApiError(404, 'not_found', 'Unknown asset.');
        if ($asset['status'] !== 'active') throw new ApiError(409, 'deleted', 'The asset was deleted.');
        $version = $this->registry->currentVersion($assetId) ?? throw new ApiError(409, 'no_version', 'The asset has no current version.');
        $renditions = $this->registry->renditions((int) $version['id']);
        $key = $path ?? (string) $version['path'];
        $target = null;
        foreach ($renditions as $r) if ($r['path'] === $key) $target = $r;
        if (!$target) throw new ApiError(422, 'invalid_path', 'That file is not part of the asset\'s current version.');
        if (AssetPaths::extension($key) !== AssetPaths::extension((string) $target['path'])) throw new ApiError(422, 'invalid_type', 'The replacement must have the same file type.');
        $size = (int) filesize($localFile);
        if ($size <= 0) throw new ApiError(422, 'empty_file', 'The file is empty.');
        if ($size > $this->maxUploadBytes()) throw new ApiError(413, 'file_too_large', 'The file is larger than ' . $this->maxUploadBytes() . ' bytes.');
        self::checkContent($localFile, $key);
        $hash = hash_file('sha256', $localFile) ?: null;
        if ($hash !== null && $target['sha256'] === $hash) throw new ApiError(409, 'unchanged', 'The file is identical to the current one.');

        $storage = $this->storage();
        $number = (int) $version['number'];
        $archive = self::ARCHIVE_PREFIX . $assetId . '/' . $number . '/' . StorageKey::basename($key);
        if ($storage->stat($key)) $storage->copy($key, $archive);
        $stored = $storage->upload($key, $localFile, StorageKey::mime($key));
        $stored = new StoredFile($stored->key, $stored->size, gmdate('c'), $hash, $stored->mime);
        $probe = $this->probe->probe($localFile, AssetPaths::kind($key));
        $purged = $storage->purge($key);
        $by = (string) $user['id'];
        $stale = [];
        $this->db->transaction(function () use ($assetId, $version, $renditions, $key, $stored, $storage, $probe, $by, $note, $archive, &$stale) {
            $isSource = $key === $version['path'];
            $newVersion = $this->registry->addVersion($assetId, $isSource ? $stored : new StoredFile((string) $version['path'], (int) $version['size'], '', $version['sha256'], $version['mime']),
                $storage->name(), $isSource ? $probe : ['width' => $version['width'], 'height' => $version['height'], 'duration' => $version['duration']], $by, $note ?? "replaced $key");
            $this->db->run('UPDATE asset_versions SET archive_path = ? WHERE id = ?', [$archive, $version['id']]);
            foreach ($renditions as $r) {
                $this->db->run('UPDATE asset_renditions SET version_id = ? WHERE id = ?', [$newVersion, $r['id']]);
                if ($r['path'] === $key) {
                    $this->registry->updateRendition((int) $r['id'], $stored, $probe);
                    $this->db->run("UPDATE asset_renditions SET origin = 'uploaded' WHERE id = ?", [$r['id']]);
                } elseif ($isSource) {
                    $stale[] = (string) $r['path'];
                }
            }
        });
        return $this->registry->detail($assetId) + ['replaced' => $key, 'archived_as' => $archive, 'purged' => $purged, 'stale_renditions' => $stale];
    }

    /**
     * What deleting the asset would break: every reference of its current files
     * @return array<string, mixed>
     */
    public function deleteCheck(string $assetId): array
    {
        $this->registry->requireAvailable();
        $asset = $this->registry->asset($assetId) ?? throw new ApiError(404, 'not_found', 'Unknown asset.');
        $paths = $this->registry->currentPaths($assetId);
        $usages = $this->registry->usagesOf($paths);
        $blocking = array_values(array_filter($usages, fn(array $u) => $u['state'] !== 'draft'));
        $archived = $this->db->all("SELECT path, archive_path FROM asset_versions WHERE asset_id = ? AND status = 'archived'", [$assetId]);
        return [
            'status' => 'success',
            'asset' => ['id' => $asset['id'], 'slug' => $asset['slug']],
            'files' => $paths,
            'usages' => $usages,
            'can_delete' => $usages === [],
            'blocking' => count($blocking),
            'drafts' => count($usages) - count($blocking),
            'archived_versions' => count($archived),
            'note' => 'History versions of content are not indexed: restoring an old version may bring a reference back.',
        ];
    }

    /**
     * Deletes the asset's files (all versions) – refused while anything references them (409 in_use + the list).
     * `force` lets an admin delete when only DRAFT working copies reference it; published content and the catalog
     * always block.
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function delete(array $user, string $assetId, bool $force = false): array
    {
        $check = $this->deleteCheck($assetId);
        if (!$check['can_delete'] && ($check['blocking'] > 0 || !$force)) {
            throw new ApiError(409, 'in_use', 'The asset is still referenced – relink or remove the references first.', ['usages' => $check['usages'], 'blocking' => $check['blocking'], 'drafts' => $check['drafts']]);
        }
        $storage = $this->storage();
        $files = [];
        foreach ($this->db->all('SELECT path FROM asset_renditions WHERE asset_id = ?', [$assetId]) as $r) $files[(string) $r['path']] = true;
        foreach ($this->db->all('SELECT path, archive_path FROM asset_versions WHERE asset_id = ?', [$assetId]) as $v) {
            $files[(string) $v['path']] = true;
            if ($v['archive_path']) $files[(string) $v['archive_path']] = true;
        }
        $deleted = [];
        $failed = [];
        foreach (array_keys($files) as $key) {
            try {
                if ($storage->stat($key)) {
                    $storage->delete($key);
                    $storage->purge($key);
                    $deleted[] = $key;
                }
            } catch (Throwable $error) {
                $failed[$key] = $error->getMessage();
            }
        }
        if ($failed) throw new ApiError(502, 'storage_failed', 'Some files could not be deleted.', ['deleted' => $deleted, 'failed' => $failed]);
        $this->db->run("UPDATE assets SET status = 'deleted', slug = ?, updated_at = ? WHERE id = ?", [$check['asset']['slug'] . '~deleted-' . substr($assetId, 0, 8), gmdate('Y-m-d H:i:s'), $assetId]);
        $this->db->run("UPDATE asset_versions SET status = 'purged' WHERE asset_id = ?", [$assetId]);
        $this->db->run('DELETE FROM asset_renditions WHERE asset_id = ?', [$assetId]);
        return ['status' => 'success', 'deleted' => $deleted, 'by' => $user['id'] ?? null];
    }

    /**
     * Points every reference of $from at $to (the file must exist on the storage). Content: one new DRAFT version per
     * touched entity (author = $user; published content changes when the creator publishes – or right away with
     * $publish when the entity's newest version was the published one); catalog files are rewritten in place when the
     * server may write them (else listed as `manual`).
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function relink(array $user, string $from, string $to, bool $publish = false): array
    {
        $this->registry->requireAvailable();
        $from = StorageKey::normalize($from);
        $to = self::targetPath($to);
        if ($from === $to) throw new ApiError(422, 'same_path', 'Nothing to relink.');
        if (!$this->storage()->stat($to)) throw new ApiError(422, 'missing_target', "The new file $to is not on the storage.");
        return $this->relinkMany($user, [$from => $to], $publish);
    }

    /**
     * @param array<string, mixed> $user
     * @param array<string, string> $map old path → new path
     * @return array<string, mixed>
     */
    private function relinkMany(array $user, array $map, bool $publish): array
    {
        $usages = $this->registry->usagesOf(array_keys($map));
        $byRef = [];
        foreach ($usages as $u) $byRef[$u['source'] . '|' . $u['ref']][] = $u;
        $content = [];
        $catalog = [];
        $manual = [];
        $this->db->transaction(function () use ($byRef, $map, $user, $publish, &$content, &$catalog, &$manual) {
            foreach ($byRef as $refKey => $list) {
                [$source, $ref] = explode('|', $refKey, 2);
                if ($source === 'content') {
                    $content[] = $this->rewriteEntity($user, $ref, $list, $map, $publish);
                } else {
                    $result = $this->rewriteCatalog($ref, $list, $map);
                    if ($result['written']) $catalog[] = $result;
                    else $manual[] = $result;
                }
            }
        });
        foreach ($content as $c) $this->usages->reindexEntity($c['ref']);
        if ($catalog) $this->usages->reindexCatalog();
        return ['status' => 'success', 'map' => $map, 'content' => $content, 'catalog' => $catalog, 'manual' => $manual, 'usages' => count($usages)];
    }

    /**
     * @param array<string, mixed> $user
     * @param list<array<string, mixed>> $usages
     * @param array<string, string> $map
     * @return array<string, mixed>
     */
    private function rewriteEntity(array $user, string $key, array $usages, array $map, bool $publish): array
    {
        $latest = $this->entities->latest($key) ?? throw new RuntimeException("Entity $key vanished.");
        $published = $this->entities->published($key);
        $doc = json_decode((string) $latest['json']);
        $changed = 0;
        $seen = [];
        foreach ($usages as $u) {
            if (isset($seen[$u['field']])) continue;
            $seen[$u['field']] = true;
            $new = AssetPaths::relativeTo($map[$u['path']], (string) ($u['toon'] ?? ''), (string) $u['raw']);
            $doc = self::setPointer($doc, (string) $u['field'], (string) $u['raw'], $new, $changed);
        }
        if ($changed === 0) return ['ref' => $key, 'changed' => 0, 'version' => null, 'published' => false, 'note' => 'the newest version no longer has these references'];
        $json = $this->entities->encode($doc);
        $id = $this->entities->insert(EntityKey::parse($key), $json, (string) $user['id'], 'draft', 'asset relink: ' . implode(', ', array_map(fn($a, $b) => "$a → $b", array_keys($map), $map)));
        $didPublish = false;
        if ($publish && $published && (int) $published['id'] === (int) $latest['id']) {
            $this->entities->setPublished(['id' => $id, 'entity_key' => $key]);
            $didPublish = true;
        }
        return ['ref' => $key, 'changed' => $changed, 'version' => $id, 'published' => $didPublish,
            'note' => $didPublish ? '' : ($published ? 'new draft – the published version keeps the old path until it is published' : 'new draft')];
    }

    /**
     * Sets the string at a JSON pointer when it still equals $old (objects stay objects)
     */
    public static function setPointer(mixed $node, string $pointer, string $old, string $new, int &$changed): mixed
    {
        if ($pointer === '' || $pointer === '/') {
            if ($node === $old) {
                $changed++;
                return $new;
            }
            return $node;
        }
        $parts = explode('/', ltrim($pointer, '/'));
        $head = strtr(array_shift($parts), ['~1' => '/', '~0' => '~']);
        $rest = $parts ? '/' . implode('/', $parts) : '';
        if ($node instanceof stdClass && property_exists($node, $head)) {
            $node->{$head} = self::setPointer($node->{$head}, $rest, $old, $new, $changed);
        } elseif (is_array($node) && array_key_exists((int) $head, $node) && ctype_digit($head)) {
            $node[(int) $head] = self::setPointer($node[(int) $head], $rest, $old, $new, $changed);
        }
        return $node;
    }

    /**
     * @param list<array<string, mixed>> $usages
     * @param array<string, string> $map
     * @return array{ref: string, written: bool, replacements: int, reason: string}
     */
    private function rewriteCatalog(string $file, array $usages, array $map): array
    {
        $path = null;
        foreach (AssetUsages::CATALOG_FILES as $setting) {
            $candidate = $this->settings->path($setting);
            if ($candidate !== '' && basename($candidate) === $file) $path = $candidate;
        }
        if ($path === null || !is_file($path)) return ['ref' => $file, 'written' => false, 'replacements' => 0, 'reason' => 'unknown catalog file'];
        $text = (string) file_get_contents($path);
        $count = 0;
        foreach ($usages as $u) {
            $raw = (string) $u['raw'];
            $new = AssetPaths::relativeTo($map[$u['path']], '', $raw);
            $text = str_replace((string) json_encode($raw, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE), (string) json_encode($new, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE), $text, $n);
            $count += $n;
        }
        if (!is_writable($path)) return ['ref' => $file, 'written' => false, 'replacements' => 0, 'reason' => 'the file is not writable here – change it in the repository (server/config) and deploy'];
        if (json_decode($text) === null) return ['ref' => $file, 'written' => false, 'replacements' => 0, 'reason' => 'the rewrite would break the JSON'];
        file_put_contents($path, $text);
        return ['ref' => $file, 'written' => true, 'replacements' => $count, 'reason' => 'server/config changed – commit it (production: deploy)'];
    }

    /**
     * Rename: copies the asset's current files to the new slug (same markers / extensions) as a new version, then
     * relinks every reference. The old files stay on the storage (old builds, unpublished content) and belong to the
     * archived version; the orphan list shows them once nothing references them.
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function rename(array $user, string $assetId, string $newSlug, bool $publish = false): array
    {
        $this->registry->requireAvailable();
        $asset = $this->registry->asset($assetId) ?? throw new ApiError(404, 'not_found', 'Unknown asset.');
        $version = $this->registry->currentVersion($assetId) ?? throw new ApiError(409, 'no_version', 'The asset has no current version.');
        $newSlug = trim(str_replace('\\', '/', $newSlug), '/');
        if ($newSlug === $asset['slug']) throw new ApiError(422, 'same_path', 'That is the current name.');
        if (!preg_match('#^[A-Za-z0-9_][A-Za-z0-9_/ -]*[A-Za-z0-9_-]$#', $newSlug) || !str_contains($newSlug, '/') || str_contains($newSlug, '..')) throw new ApiError(422, 'invalid_path', 'A new name like catpope/videos/new-name (no extension).');
        if ($this->registry->bySlug($newSlug)) throw new ApiError(409, 'exists', 'Another asset has this name.');
        $map = [];
        foreach ($this->registry->renditions((int) $version['id']) as $r) {
            $map[(string) $r['path']] = self::renamedPath((string) $r['path'], (string) $asset['slug'], $newSlug);
        }
        $storage = $this->storage();
        foreach ($map as $to) {
            self::targetPath($to);
            if ($storage->stat($to)) throw new ApiError(409, 'exists', "$to exists already.");
        }
        $copied = [];
        foreach ($map as $from => $to) {
            $copied[$to] = $storage->copy($from, $to);
        }
        $by = (string) $user['id'];
        $this->db->transaction(function () use ($assetId, $version, $map, $copied, $storage, $by, $newSlug, $asset) {
            $source = $copied[$map[(string) $version['path']]];
            $newVersion = $this->registry->addVersion($assetId, $source, $storage->name(), ['width' => $version['width'], 'height' => $version['height'], 'duration' => $version['duration']], $by, 'renamed from ' . $asset['slug']);
            foreach ($this->registry->renditions((int) $version['id']) as $r) {
                $this->db->run('INSERT INTO asset_renditions (asset_id, version_id, label, quality, path, size, sha256, mime, width, height, duration, bitrate, origin, modified_at, created_at)
                    SELECT asset_id, ?, label, quality, ?, size, sha256, mime, width, height, duration, bitrate, origin, modified_at, created_at FROM asset_renditions WHERE id = ?', [$newVersion, $map[(string) $r['path']], $r['id']]);
            }
            $this->db->run('UPDATE assets SET slug = ?, folder = ?, updated_at = ? WHERE id = ?', [$newSlug, AssetPaths::folder($newSlug), gmdate('Y-m-d H:i:s'), $assetId]);
        });
        $relink = $this->relinkMany($user, $map, $publish);
        return $this->registry->detail($assetId) + ['renamed' => $map, 'relink' => $relink];
    }

    /** "catpope/images/1x/a.jpg" with slug catpope/images/a → new slug catpope/art/b → "catpope/art/1x/b.jpg" */
    public static function renamedPath(string $path, string $oldSlug, string $newSlug): string
    {
        $oldDir = StorageKey::dirname($oldSlug);
        $oldStem = StorageKey::basename($oldSlug);
        $newDir = StorageKey::dirname($newSlug);
        $newStem = StorageKey::basename($newSlug);
        $rest = $oldDir === '' ? $path : substr($path, strlen($oldDir) + 1);
        $restDir = StorageKey::dirname($rest);
        $name = StorageKey::basename($rest);
        if (str_starts_with($name, $oldStem)) $name = $newStem . substr($name, strlen($oldStem));
        return ($newDir === '' ? '' : $newDir . '/') . ($restDir === '' ? '' : $restDir . '/') . $name;
    }
}
