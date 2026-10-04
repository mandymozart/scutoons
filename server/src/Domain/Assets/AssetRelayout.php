<?php
declare(strict_types=1);

namespace Scutoons\Domain\Assets;

use RuntimeException;
use Scutoons\Infrastructure\AssetStorage\LocalStorage;
use Scutoons\Infrastructure\AssetStorage\StorageKey;

/**
 * The folder relayout (agents/ASSETS.md "Folder convention", owner 2026-10-02: done before the bunny upload, bunny gets
 * the new structure once). From the reviewed AssetLayout mapping it
 *   a) COPIES each moving file to its new path in a local CDN folder (never moves; verifies size + SHA-256),
 *   b) rewrites every reference in the repository's tracked files: the content seed JSON (all toons + the platform-wide
 *      `scutoons/_`), the server's catalog JSON (items incl. albums, products, editions, toons), and exact CDN paths in
 *      code (e.g. the camera model path); paths are rewritten in the style they were written (toon-relative,
 *      `../`-relative, root-relative),
 *   c) reports everything (`report()`), incl. the planned deletions and lines that look like CDN paths but were not
 *      rewritten (manual review).
 * Deleting the old paths is a separate step (deleteOld) – only after the merge, so running dev stacks keep working.
 */
final class AssetRelayout
{
    /** tracked files whose JSON strings are content / catalog references */
    public const JSON_SCOPES = ['shared/content/seed/', 'server/config/'];
    /** tracked code / test / script files searched for exact CDN paths */
    public const CODE_SCOPES = ['viewer/src/', 'creator/src/', 'admin/src/', 'shop/src/', 'embassy/src/', 'shared/', 'server/src/', 'server/tests/', 'server/templates/', 'qr/', 'scripts/'];
    public const CODE_EXTENSIONS = ['ts', 'js', 'mjs', 'svelte', 'php', 'py', 'html', 'css'];

    /** @var array<string, string> old CDN-root path → new */
    private array $map = [];
    /** @var array<string, true> every old file (resolution: does a path exist from the root?) */
    private array $known = [];
    /** @var list<string> */
    private array $deletes = [];

    /**
     * @param list<array<string, mixed>> $planItems AssetLayout::plan()['items']
     * @param list<string> $publicBases URL prefixes that mean "the CDN"
     */
    public function __construct(array $planItems, private readonly array $publicBases = [])
    {
        foreach ($planItems as $item) {
            $this->known[(string) $item['from']] = true;
            if (($item['action'] ?? '') === 'move') $this->map[(string) $item['from']] = (string) $item['to'];
            if (($item['action'] ?? '') === 'delete') $this->deletes[] = (string) $item['from'];
        }
    }

    /** @return array<string, string> */
    public function map(): array
    {
        return $this->map;
    }

    /** @return list<string> */
    public function deletes(): array
    {
        return $this->deletes;
    }

    /**
     * a) copy every moving file to its new path (skips identical copies)
     * @return array{copied: int, identical: int, bytes: int, failed: array<string, string>}
     */
    public function copy(LocalStorage $storage, bool $apply): array
    {
        $out = ['copied' => 0, 'identical' => 0, 'bytes' => 0, 'failed' => []];
        foreach ($this->map as $from => $to) {
            try {
                $source = $storage->localPath($from);
                if (!is_file($source)) throw new RuntimeException('source missing');
                $hash = hash_file('sha256', $source);
                $target = $storage->localPath($to);
                if (is_file($target) && hash_file('sha256', $target) === $hash) {
                    $out['identical']++;
                    continue;
                }
                if (is_file($target)) throw new RuntimeException("$to exists with other bytes");
                if ($apply) {
                    $storage->copy($from, $to);
                    if (hash_file('sha256', $target) !== $hash) throw new RuntimeException('verification failed');
                }
                $out['copied']++;
                $out['bytes'] += (int) filesize($source);
            } catch (\Throwable $error) {
                $out['failed'][$from] = $error->getMessage();
            }
        }
        return $out;
    }

    /**
     * b) rewrite the references in tracked files under $repo
     * @param list<string> $tracked repository-relative paths (git ls-files)
     * @return array{files: array<string, list<array{from: string, to: string, count: int}>>, conflicts: list<string>, review: list<string>}
     */
    public function rewrite(string $repo, array $tracked, bool $apply): array
    {
        $result = ['files' => [], 'conflicts' => [], 'review' => []];
        foreach ($tracked as $rel) {
            $rel = str_replace('\\', '/', $rel);
            $ext = strtolower(pathinfo($rel, PATHINFO_EXTENSION));
            $isJson = $ext === 'json' && $this->inScope($rel, self::JSON_SCOPES);
            $isCode = in_array($ext, self::CODE_EXTENSIONS, true) && $this->inScope($rel, self::CODE_SCOPES);
            if (!$isJson && !$isCode) continue;
            $path = "$repo/$rel";
            $text = (string) file_get_contents($path);
            $changes = $isJson ? $this->jsonChanges($rel, $text, $result['conflicts']) : $this->codeChanges($text);
            if ($isCode) {
                foreach (preg_split('/\r?\n/', $text) ?: [] as $n => $line) {
                    if (preg_match('#[\'"`/](images|videos|audio|models)/[^\'"`\s]*\.(webp|png|jpe?g|svg|mp4|mp3|zip|json)#', $line) && !$this->lineRewritten($line, $changes)) {
                        $result['review'][] = "$rel:" . ($n + 1) . ': ' . trim(mb_substr($line, 0, 200));
                    }
                }
            }
            if (!$changes) continue;
            $new = $text;
            $list = [];
            foreach ($changes as $from => $to) {
                $count = 0;
                $new = $isJson
                    ? str_replace(self::token($from), self::token($to), $new, $count)
                    : (string) preg_replace(self::codePattern($from), $to, $new, -1, $count);
                if ($count) $list[] = ['from' => $from, 'to' => $to, 'count' => $count];
            }
            if (!$list) continue;
            if ($isJson && json_decode($new) === null) {
                $result['conflicts'][] = "$rel: the rewrite would break the JSON – skipped";
                continue;
            }
            if ($apply) file_put_contents($path, $new);
            $result['files'][$rel] = $list;
        }
        return $result;
    }

    /** @param list<string> $scopes */
    private function inScope(string $rel, array $scopes): bool
    {
        foreach ($scopes as $scope) if (str_starts_with($rel, $scope)) return !str_contains($rel, '/node_modules/') && !str_contains($rel, '/vendor/');
        return false;
    }

    private static function token(string $value): string
    {
        return (string) json_encode($value, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
    }

    /** @param array<string, string> $changes */
    private function lineRewritten(string $line, array $changes): bool
    {
        foreach (array_keys($changes) as $from) if (str_contains($line, $from)) return true;
        return false;
    }

    /** The toon folder a seed file's paths are relative to ('' = the CDN root) */
    public static function baseOf(string $rel): string
    {
        if (preg_match('#^shared/content/seed/([a-z0-9_-]+)/#', $rel, $m)) return $m[1] === 'scutoons' ? '' : $m[1];
        return '';
    }

    /**
     * raw string → new raw string for every media reference of a JSON file whose target moves
     * @param list<string> $conflicts
     * @return array<string, string>
     */
    private function jsonChanges(string $rel, string $text, array &$conflicts): array
    {
        $doc = json_decode($text, true);
        if (!is_array($doc)) return [];
        $changes = [];
        $walk = function (mixed $node, string $base) use (&$walk, &$changes, &$conflicts, $rel): void {
            if (is_array($node)) {
                if (isset($node['toon']) && is_string($node['toon']) && preg_match('/^[a-z0-9_-]{1,64}$/', $node['toon']) && !str_starts_with($rel, 'server/config/')) $base = $node['toon'];
                // the toon index (seed toons.json): one entry per toon, its paths are relative to that toon's folder
                if (str_ends_with($rel, 'seed/toons.json') && isset($node['id'], $node['uri']) && is_string($node['id']) && preg_match('/^[a-z0-9_-]{1,64}$/', $node['id'])) $base = $node['id'];
                foreach ($node as $v) $walk($v, $base);
                return;
            }
            if (!is_string($node) || !preg_match(AssetPaths::MEDIA_EXTENSION, trim((string) preg_replace('/[?#].*$/', '', $node)))) return;
            [$path, $usedBase] = $this->resolve($node, $base);
            if ($path === null || !isset($this->map[$path])) return;
            $new = AssetPaths::relativeTo($this->map[$path], $usedBase, $node);
            if (isset($changes[$node]) && $changes[$node] !== $new) {
                $conflicts[] = "$rel: \"$node\" means different files in different places – rewrite by hand";
                return;
            }
            $changes[$node] = $new;
        };
        $walk($doc, self::baseOf($rel));
        return $changes;
    }

    /** @return array{0: ?string, 1: string} */
    private function resolve(string $raw, string $base): array
    {
        $primary = AssetPaths::resolve($raw, $base, $this->publicBases);
        if ($base === '' || $primary === null) return [$primary, ''];
        if (isset($this->known[$primary])) return [$primary, $base];
        $root = AssetPaths::resolve($raw, '', $this->publicBases);
        if ($root !== null && isset($this->known[$root]) && !str_starts_with(trim($raw), '../')) return [$root, ''];
        return [$primary, $base];
    }

    /**
     * exact root-relative CDN paths in code (quoted, or after a "/" of a URL)
     * @return array<string, string>
     */
    private function codeChanges(string $text): array
    {
        $changes = [];
        foreach ($this->map as $from => $to) {
            if (str_contains($text, $from) && preg_match(self::codePattern($from), $text)) $changes[$from] = $to;
        }
        return $changes;
    }

    /** an exact CDN path as a string literal or URL tail in code */
    public static function codePattern(string $path): string
    {
        return '~(?<=[\'"`/])' . preg_quote($path, '~') . '(?=[\'"`?#])~';
    }

    /**
     * App code (not tests) that contains exact CDN paths – these count as uses when the layout is planned
     * @param list<string> $tracked
     * @param list<string> $keys
     * @return array<string, list<array<string, mixed>>> path → uses { source: code, ref: file, state: code }
     */
    public static function codeReferences(string $repo, array $tracked, array $keys): array
    {
        $out = [];
        foreach ($tracked as $rel) {
            $rel = str_replace('\\', '/', $rel);
            if (!in_array(strtolower(pathinfo($rel, PATHINFO_EXTENSION)), self::CODE_EXTENSIONS, true)) continue;
            if (preg_match('~(^|/)(tests?|test)/|Test\.php$|\.test\.ts$~', $rel) || str_contains($rel, '/node_modules/')) continue;
            $inScope = false;
            foreach (self::CODE_SCOPES as $scope) if (str_starts_with($rel, $scope)) $inScope = true;
            if (!$inScope || !is_file("$repo/$rel")) continue;
            $text = (string) file_get_contents("$repo/$rel");
            foreach ($keys as $key) {
                if (str_contains($text, $key) && preg_match(self::codePattern($key), $text)) $out[$key][] = ['source' => 'code', 'ref' => $rel, 'state' => 'code'];
            }
        }
        return $out;
    }

    /**
     * Deletes the old paths (moved files whose copy is verified, and the planned deletions) from a local folder
     * @return array{deleted: int, kept: array<string, string>}
     */
    public function deleteOld(LocalStorage $storage, bool $apply): array
    {
        $out = ['deleted' => 0, 'kept' => []];
        foreach ($this->map as $from => $to) {
            $old = $storage->localPath($from);
            $new = $storage->localPath($to);
            if (!is_file($old)) continue;
            if (!is_file($new) || hash_file('sha256', $old) !== hash_file('sha256', $new)) {
                $out['kept'][$from] = 'the copy at the new path is missing or differs';
                continue;
            }
            if ($apply) $storage->delete($from);
            $out['deleted']++;
        }
        foreach ($this->deletes as $key) {
            if (!StorageKey::valid($key) || !is_file($storage->localPath($key))) continue;
            if ($apply) $storage->delete($key);
            $out['deleted']++;
        }
        return $out;
    }
}
