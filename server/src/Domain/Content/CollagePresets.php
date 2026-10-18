<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content;

use stdClass;

/**
 * Custom collage presets (“Save as preset”, agents/research/COLLAGE.md §12.3): the platform-wide document `colorations`
 * holds `{ presets: [{ id, name, removed?, …style }] }`. Readers never resolve a custom preset – the two answers they
 * already fetch arrive baked (Library::chapter, ProtectedFiles::score): a piece that names one gets `style` = its drawn
 * style and loses `preset`. Code looks (`ink` … `fade`) stay for the renderer. The PHP twin of @scutoons/content
 * `customPreset` / `bakePresets` (collage.ts); both suites read shared/content/test/fixtures/collage-bake.json
 * (CollagePresetsTest). Copies only – never the input: Snapshot::doc caches decoded documents and NeutralDoc passes
 * `pieces` through by reference.
 */
final class CollagePresets
{
    public const KEY = 'colorations';
    /**
     * = @scutoons/content PRESET_ID (CollagePresetsTest): the code looks' ids and 'none' never match. `D`: `$` is the end
     * only, as in JS (PCRE's `$` would also match before a trailing newline)
     */
    public const PRESET_ID = '/^(?!(?:ink|red|cyan|negative|burn|acid|sticker|glow|ghost|fade|none)$)[a-z][a-z0-9-]{0,39}$/D';
    /** = collage.ts PIECE_STYLE_KEYS (CollagePresetsTest): nothing else is baked */
    public const STYLE_KEYS = ['blend', 'opacity', 'fill', 'outline', 'shadow', 'blur'];
    private const JSON_FLAGS = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION | JSON_THROW_ON_ERROR;

    /**
     * id → entry of a `colorations` document (decoded with stdClass objects): the entries whose id matches PRESET_ID,
     * the first of a repeated id, deleted ones too (a tombstone still draws); anything else = []
     * @return array<string, stdClass>
     */
    public static function index(mixed $doc): array
    {
        $index = [];
        $presets = $doc instanceof stdClass && property_exists($doc, 'presets') && is_array($doc->presets) ? $doc->presets : [];
        foreach ($presets as $entry) {
            if (!$entry instanceof stdClass || !property_exists($entry, 'id')) continue;
            $id = $entry->id;
            if (!is_string($id) || preg_match(self::PRESET_ID, $id) !== 1 || array_key_exists($id, $index)) continue;
            $index[$id] = $entry;
        }
        return $index;
    }

    /**
     * bakePresets: pieces as stdClass (chapters) or assoc arrays (catalog rows), keys kept. A piece that names a preset
     * of the index gets a copy; every other piece is the same value – nothing baked = the same array back
     * @template K of array-key
     * @param array<K, mixed> $pieces
     * @param array<string, stdClass> $index
     * @return array<K, mixed>
     */
    public static function bake(array $pieces, array $index): array
    {
        if ($index === []) return $pieces;
        return array_map(fn(mixed $piece) => self::piece($piece, $index), $pieces);
    }

    /**
     * A copy of a chapter answer with groups[].slices[].pieces baked – an explicit walk (a pickup's `pieces` is a count,
     * content.ts); only what changes is copied, an empty index or nothing baked = the chapter itself
     * @param array<string, stdClass> $index
     */
    public static function chapter(stdClass $chapter, array $index): stdClass
    {
        if ($index === [] || !property_exists($chapter, 'groups') || !is_array($chapter->groups)) return $chapter;
        $groups = array_map(fn(mixed $group) => self::group($group, $index), $chapter->groups);
        if ($groups === $chapter->groups) return $chapter;
        $out = clone $chapter;
        $out->groups = $groups;
        return $out;
    }

    /** @param array<string, stdClass> $index */
    private static function group(mixed $group, array $index): mixed
    {
        if (!$group instanceof stdClass || !property_exists($group, 'slices') || !is_array($group->slices)) return $group;
        $slices = array_map(fn(mixed $slice) => self::slice($slice, $index), $group->slices);
        if ($slices === $group->slices) return $group;
        $out = clone $group;
        $out->slices = $slices;
        return $out;
    }

    /** @param array<string, stdClass> $index */
    private static function slice(mixed $slice, array $index): mixed
    {
        if (!$slice instanceof stdClass || !property_exists($slice, 'pieces') || !is_array($slice->pieces)) return $slice;
        $pieces = self::bake($slice->pieces, $index);
        if ($pieces === $slice->pieces) return $slice;
        $out = clone $slice;
        $out->pieces = $pieces;
        return $out;
    }

    /**
     * One piece: when it names a preset of the index, a copy with `style` = the drawn style per key (an own key – even
     * null = off – over the preset's; nulls dropped; a JSON copy in the piece's own form) and without `preset`; `style`
     * keeps its place, a new one goes last, an empty one is no key (an empty assoc `[]` would encode as a list)
     * @param array<string, stdClass> $index
     */
    private static function piece(mixed $piece, array $index): mixed
    {
        if (is_array($piece)) {
            $id = array_key_exists('preset', $piece) ? $piece['preset'] : null;
            if (!is_string($id) || !array_key_exists($id, $index)) return $piece;
            $style = self::drawn(array_key_exists('style', $piece) && is_array($piece['style']) ? $piece['style'] : null, $index[$id], true);
            unset($piece['preset']);
            if ($style !== null) $piece['style'] = $style;
            else unset($piece['style']);
            return $piece;
        }
        if (!$piece instanceof stdClass) return $piece;
        $id = property_exists($piece, 'preset') ? $piece->preset : null;
        if (!is_string($id) || !array_key_exists($id, $index)) return $piece;
        $style = self::drawn(property_exists($piece, 'style') && $piece->style instanceof stdClass ? $piece->style : null, $index[$id], false);
        $out = clone $piece;
        unset($out->preset);
        if ($style !== null) $out->style = $style;
        else unset($out->style);
        return $out;
    }

    /**
     * pieceStyle: the six keys, the piece's own (property_exists / array_key_exists – never isset: opacity 0 is a value, an
     * own null switches the effect off) over the preset's, nulls dropped; a JSON copy, null when nothing is left
     * @param array<mixed>|stdClass|null $own
     * @return array<mixed>|stdClass|null
     */
    private static function drawn(array|stdClass|null $own, stdClass $preset, bool $assoc): array|stdClass|null
    {
        $style = [];
        foreach (self::STYLE_KEYS as $key) {
            if (is_array($own) && array_key_exists($key, $own)) $value = $own[$key];
            elseif ($own instanceof stdClass && property_exists($own, $key)) $value = $own->{$key};
            elseif (property_exists($preset, $key)) $value = $preset->{$key};
            else continue;
            if ($value !== null) $style[$key] = $value;
        }
        if ($style === []) return null;
        $copy = json_decode(json_encode($style, self::JSON_FLAGS), $assoc, 512, JSON_THROW_ON_ERROR);
        return is_array($copy) || $copy instanceof stdClass ? $copy : null;
    }
}
