<?php
declare(strict_types=1);

namespace Scutoons\Domain\Assets;

use Scutoons\Domain\Admin\ListQuery;
use Scutoons\Domain\ApiError;

/**
 * The admin's Assets → Unused list as a server-side list (research/ASSET-LISTS.md): the rows of
 * AssetUsages::unusedFiles, a map tile set as ONE row (its unused tiles together – "painted v2 · 507 tiles"), then the
 * filters, the sort and a page. Pure: no storage, no DB.
 *   q        slug contains
 *   folder   the top-level folder, or a path prefix with a `/` (like GET /assets)
 *   kind     image | video | …
 *   what     asset (nothing of it is used: the whole asset goes) | files (leftovers of a used asset)
 *   tiles    all (default) | hide | only
 *   sort     slug | size | files (`-size` = biggest first)
 */
final class UnusedList
{
    public const FILTERS = ['folder', 'kind', 'what', 'tiles'];
    public const SORTS = ['slug', 'size', 'files'];
    /** the files a paged row carries (a tile set has thousands; the delete asks for all of them with `all=1`) */
    public const PREVIEW_FILES = 12;

    /**
     * One row per asset, every all-unused map tile of a set merged into one row (`id` = `tiles:<set>`)
     * @param list<array{id: string, slug: string, kind: string, all_unused: bool, files: list<array{path: string, what: string, size: int|null}>, size: int}> $items
     * @return list<array<string, mixed>>
     */
    public static function rows(array $items): array
    {
        $rows = [];
        $sets = [];
        foreach ($items as $item) {
            $set = $item['all_unused'] ? AssetPaths::tileSet($item['slug']) : null;
            if ($set === null) {
                $rows[] = $item + ['assets' => 1, 'tiles' => null, 'file_count' => count($item['files'])];
                continue;
            }
            if (!isset($sets[$set])) {
                $sets[$set] = count($rows);
                $rows[] = ['id' => "tiles:$set", 'slug' => $set, 'kind' => $item['kind'], 'all_unused' => true, 'files' => [], 'size' => 0, 'assets' => 0, 'tiles' => 0, 'file_count' => 0];
            }
            $row = &$rows[$sets[$set]];
            array_push($row['files'], ...$item['files']);
            $row['size'] += $item['size'];
            $row['assets']++;
            $row['tiles']++;
            $row['file_count'] += count($item['files']);
            unset($row);
        }
        return $rows;
    }

    /**
     * The rows the filters find, sorted
     * @param list<array<string, mixed>> $rows
     * @return list<array<string, mixed>>
     */
    public static function filter(array $rows, ListQuery $list): array
    {
        $q = mb_strtolower($list->q);
        $folder = $list->filter('folder');
        $kind = $list->filter('kind');
        if ($kind !== null && !in_array($kind, AssetPaths::KINDS, true)) throw new ApiError(400, 'invalid_kind', 'Unknown kind.', ['kinds' => AssetPaths::KINDS]);
        $what = $list->filter('what');
        if ($what !== null && !in_array($what, ['asset', 'files'], true)) throw new ApiError(400, 'invalid_what', 'what is asset | files.');
        $tiles = $list->filter('tiles') ?? 'all';
        if (!in_array($tiles, ['hide', 'only', 'all'], true)) throw new ApiError(400, 'invalid_tiles', 'tiles is hide | only | all.');
        $out = array_values(array_filter($rows, function (array $r) use ($q, $folder, $kind, $what, $tiles): bool {
            $slug = (string) $r['slug'];
            if ($q !== '' && !str_contains(mb_strtolower($slug), $q)) return false;
            if ($folder !== null && !(str_contains($folder, '/') ? str_starts_with($slug, $folder) : AssetPaths::folder($slug) === $folder)) return false;
            if ($kind !== null && $r['kind'] !== $kind) return false;
            if ($what !== null && (bool) $r['all_unused'] !== ($what === 'asset')) return false;
            if ($tiles !== 'all' && ($r['tiles'] !== null) !== ($tiles === 'only')) return false;
            return true;
        }));
        $field = $list->sortField;
        $sign = $list->descending ? -1 : 1;
        usort($out, function (array $a, array $b) use ($field, $sign): int {
            $by = match ($field) {
                'size' => $a['size'] <=> $b['size'],
                'files' => $a['file_count'] <=> $b['file_count'],
                default => 0,
            };
            return $sign * ($by !== 0 ? $by : strcmp((string) $a['slug'], (string) $b['slug']));
        });
        return $out;
    }

    /**
     * @param list<array<string, mixed>> $rows
     * @return array{rows: int, assets: int, files: int, bytes: int}
     */
    public static function totals(array $rows): array
    {
        return [
            'rows' => count($rows),
            'assets' => array_sum(array_map(fn(array $r) => (int) $r['assets'], $rows)),
            'files' => array_sum(array_map(fn(array $r) => (int) $r['file_count'], $rows)),
            'bytes' => array_sum(array_map(fn(array $r) => (int) $r['size'], $rows)),
        ];
    }

    /**
     * A row as a page shows it: at most PREVIEW_FILES files (`file_count` has them all)
     * @param array<string, mixed> $row
     * @return array<string, mixed>
     */
    public static function preview(array $row): array
    {
        return ['files' => array_slice($row['files'], 0, self::PREVIEW_FILES)] + $row;
    }
}
