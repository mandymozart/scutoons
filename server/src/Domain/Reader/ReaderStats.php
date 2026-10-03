<?php
declare(strict_types=1);

namespace Scutoons\Domain\Reader;

/**
 * The reader stats substances change (agents/ITEMS.md "Substances"): id → { label, baseline 0–100, half_life_days } –
 * code, not catalog data (owner 2026-10-04). Served as `stats` in GET /api/items/ (the place they had in
 * config/items.json). Mirror of @scutoons/content `READER_STATS` (substances.ts); CatalogTest keeps both equal.
 */
final class ReaderStats
{
    public const DEFINITIONS = [
        'testo' => ['label' => ['de' => 'Testo', 'en' => 'Testo'], 'baseline' => 50, 'half_life_days' => 3],
        'appetite' => ['label' => ['de' => 'Appetit', 'en' => 'Appetite'], 'baseline' => 50, 'half_life_days' => 3],
    ];

    public static function isStat(mixed $id): bool
    {
        return is_string($id) && isset(self::DEFINITIONS[$id]);
    }
}
