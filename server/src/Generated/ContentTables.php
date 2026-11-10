<?php
// GENERATED from server/schema/tables/*.json by server/schema/generate.ts – edit the source, run `npm run generate`.
declare(strict_types=1);

namespace Scutoons\Generated;

/** Content tables, the same in Rust, TS and PHP (#477) */
final class ContentTables
{
    /**
     * Former characters that are personas now (owner 2026-09-30): an old VN / deck character id → the registry character and the persona its art went to (production data: catpope's `human` persona carries plf's art, `cat` cat's, `mountaintiger` mountain-tiger's). Generated into TS (shared/content/src/tables.ts) and PHP (server/src/Generated/ContentTables.php) by server/schema/generate.ts (#477).
     * @var array<string, array{character: string, persona: string}>
     */
    public const LEGACY_CHARACTER_IDS = [
        'plf' => ['character' => 'catpope', 'persona' => 'human'],
        'cat' => ['character' => 'catpope', 'persona' => 'cat'],
        'mountain-tiger' => ['character' => 'catpope', 'persona' => 'mountaintiger'],
    ];

    /**
     * The one locale order for content text (#474, #477): the asked locale › the document's source language › English › any. `asked`, `source` and `any` are steps, anything else a locale code. A text counts when it is a string with more than white space; `any` = the first such text in the map's own order. Generated into Rust, TS and PHP with the one pick function each by server/schema/generate.ts; locale-order.vectors.json is the cases every language runs.
     * @var list<string>
     */
    public const LOCALE_ORDER = ['asked', 'source', 'en', 'any'];

    /** Content text in the one locale order (LOCALE_ORDER): the first step with a text, null when none has one */
    public static function pickText(mixed $text, string $asked, ?string $source = null): ?string
    {
        if (is_object($text)) $text = get_object_vars($text);
        if (!is_array($text)) return null;
        foreach (self::LOCALE_ORDER as $step) {
            if ($step === 'any') {
                foreach ($text as $value) {
                    if (self::present($value)) return $value;
                }
                continue;
            }
            $locale = match ($step) { 'asked' => $asked, 'source' => $source, default => $step };
            $value = $locale !== null && $locale !== '' ? ($text[$locale] ?? null) : null;
            if (self::present($value)) return $value;
        }
        return null;
    }

    /** @phpstan-assert-if-true string $value a string with more than white space */
    private static function present(mixed $value): bool
    {
        return is_string($value) && preg_match('/^[\s\p{Z}\x{FEFF}]*$/u', $value) !== 1;
    }
}
