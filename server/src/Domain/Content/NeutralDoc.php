<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content;

use stdClass;

/**
 * Language-neutral chapter / scene / season documents (agents/NEUTRAL.md): one stored document per entity
 * (`chapter:<toon>:<n>` …, EntityKey::NEUTRAL_DOC_KINDS) whose texts – and `release`, season `sequence[].status` – are
 * locale maps `{ "de": …, "en": … }`. Resolved here into the reader's per-locale shape, exactly like @scutoons/content
 * `resolveNeutral` (neutral.ts); both are tested against shared/content/test/fixtures/neutral/.
 *
 * - Served locale: the asked one when the document has it (chapter / season: a `title` in it; scene: any text in it; a
 *   document without any text has every locale), else the source language, else the toon's other languages, else the
 *   document's first locale.
 * - A text: the served locale → the source → the other languages → the first text there is (blank = missing); none → ''.
 * - Chapter: `language` = the served locale, `status` = the season's status of the chapter when given. Season:
 *   `language` = the served locale, `sequence[].status` resolved (none at all → no status). Toon entry (`toon`): its
 *   per-language settings (`toon()`).
 * Documents are never changed: changed levels are copies.
 */
final class NeutralDoc
{
    /** Whether a locale map's value is a text: not blank (JS `trim()`), a list when one line isn't blank */
    public static function isText(mixed $value): bool
    {
        if (is_string($value)) return preg_match('/^[\s\p{Z}\x{FEFF}]*$/u', $value) !== 1;
        if (!is_array($value)) return false;
        foreach ($value as $line) {
            if (is_string($line) && self::isText($line)) return true;
        }
        return false;
    }

    /**
     * The locales to try: the served one, the source, the toon's other languages
     * @param list<string> $languages
     * @return list<string>
     */
    public static function chain(string $locale, string $source, array $languages): array
    {
        return array_values(array_unique([$locale, $source, ...$languages]));
    }

    /**
     * A locale map's value in the first locale of the chain that has one, else the first there is, null when none;
     * anything that is no locale map is returned as it is
     * @param list<string> $chain
     */
    public static function text(mixed $value, array $chain): mixed
    {
        if (!$value instanceof stdClass) return $value;
        foreach ($chain as $locale) {
            if (property_exists($value, $locale) && self::isText($value->{$locale})) return $value->{$locale};
        }
        foreach (get_object_vars($value) as $text) {
            if (self::isText($text)) return $text;
        }
        return null;
    }

    /**
     * The locales a document has (chapter / season: its title's; scene: any text's), in its own order; [] = no text at all
     * @return list<string>
     */
    public static function locales(string $kind, mixed $doc): array
    {
        $out = [];
        if (!$doc instanceof stdClass) return $out;
        if ($kind === 'scene') self::groupLocales($doc, $out);
        else self::addLocales($doc->title ?? null, $out);
        return $out;
    }

    /**
     * The served locale for an asked one + whether it is a fallback
     * @param list<string> $languages
     * @return array{0: string, 1: bool}
     */
    public static function served(string $kind, mixed $doc, string $locale, string $source, array $languages): array
    {
        $has = self::locales($kind, $doc);
        if ($has === []) return [$locale, false];
        $served = $has[0];
        foreach (self::chain($locale, $source, $languages) as $candidate) {
            if (in_array($candidate, $has, true)) {
                $served = $candidate;
                break;
            }
        }
        return [$served, $served !== $locale];
    }

    /**
     * A neutral document for an asked locale: [served locale, fallback, the document in the reader's shape]
     * @param list<string> $languages
     * @return array{0: string, 1: bool, 2: mixed}
     */
    public static function resolve(string $kind, mixed $doc, string $locale, string $source, array $languages, ?string $chapterStatus = null): array
    {
        [$served, $fallback] = self::served($kind, $doc, $locale, $source, $languages);
        $chain = self::chain($served, $source, $languages);
        $resolved = match ($kind) {
            'chapter' => $doc instanceof stdClass ? self::chapter($doc, $served, $chain, $chapterStatus) : $doc,
            'season' => $doc instanceof stdClass ? self::season($doc, $served, $chain) : $doc,
            'toon' => $doc instanceof stdClass ? self::toon($doc, $served, $chain) : $doc,
            default => self::group($doc, $chain),
        };
        return [$served, $fallback, $resolved];
    }

    /** @param list<string> $chain */
    public static function chapter(stdClass $doc, string $locale, array $chain, ?string $status = null): stdClass
    {
        $out = clone $doc;
        foreach (['title', 'description', 'release'] as $field) self::textField($out, $field, $chain);
        $out->language = $locale;
        if ($status !== null && $status !== '') $out->status = $status;
        if (is_array($doc->groups ?? null)) $out->groups = array_map(fn(mixed $g) => self::group($g, $chain), $doc->groups);
        return $out;
    }

    /** @param list<string> $chain */
    public static function season(stdClass $doc, string $locale, array $chain): stdClass
    {
        $out = clone $doc;
        foreach (['title', 'description', 'release'] as $field) self::textField($out, $field, $chain);
        $out->language = $locale;
        if (is_array($doc->sequence ?? null)) {
            $out->sequence = array_map(function (mixed $item) use ($chain) {
                if (!$item instanceof stdClass || !property_exists($item, 'status')) return $item;
                $copy = clone $item;
                $status = self::text($item->status, $chain);
                if ($status === null) unset($copy->status);
                else $copy->status = $status;
                return $copy;
            }, $doc->sequence);
        }
        return $out;
    }

    /**
     * The toon entry `toon:<toon>` in a locale (agents/NEUTRAL.md step 4): the per-language toon settings it replaces –
     * `id`, `title`, `language` = the locale, `release` / `description` when the entry has them, `status` = the
     * language's `languageStatus` (none at all → no status); @scutoons/content `resolveToon`
     * @param list<string> $chain
     */
    public static function toon(stdClass $entry, string $locale, array $chain): stdClass
    {
        $out = new stdClass();
        $out->id = $entry->id ?? null;
        $out->title = self::text($entry->title ?? null, $chain) ?? '';
        $out->language = $locale;
        if (property_exists($entry, 'release')) $out->release = self::text($entry->release, $chain) ?? '';
        $status = self::text($entry->languageStatus ?? null, $chain);
        if ($status !== null) $out->status = $status;
        if (property_exists($entry, 'description')) $out->description = self::text($entry->description, $chain) ?? '';
        return $out;
    }

    /** @param list<string> $chain */
    public static function group(mixed $group, array $chain): mixed
    {
        if (!$group instanceof stdClass) return $group;
        $out = clone $group;
        if (is_array($group->slices ?? null)) $out->slices = array_map(fn(mixed $s) => self::slice($s, $chain), $group->slices);
        if (property_exists($group, 'events')) $out->events = self::events($group->events, $chain);
        return $out;
    }

    /** @param list<string> $chain */
    private static function slice(mixed $slice, array $chain): mixed
    {
        if (!$slice instanceof stdClass) return $slice;
        $out = clone $slice;
        if (is_array($slice->dialog ?? null)) {
            $out->dialog = array_map(function (mixed $dialog) use ($chain) {
                if (!$dialog instanceof stdClass) return $dialog;
                $copy = clone $dialog;
                self::textField($copy, 'text', $chain);
                return $copy;
            }, $slice->dialog);
        }
        if (property_exists($slice, 'events')) $out->events = self::events($slice->events, $chain);
        if (is_array($slice->items ?? null)) {
            $out->items = array_map(function (mixed $item) use ($chain) {
                if (!$item instanceof stdClass || !property_exists($item, 'events')) return $item;
                $copy = clone $item;
                $copy->events = self::events($item->events, $chain);
                return $copy;
            }, $slice->items);
        }
        return $out;
    }

    /** @param list<string> $chain */
    private static function events(mixed $events, array $chain): mixed
    {
        if (!is_array($events)) return $events;
        return array_map(function (mixed $event) use ($chain) {
            if (!$event instanceof stdClass || !($event->params ?? null) instanceof stdClass) return $event;
            $field = match ($event->type ?? null) {
                'showMessage' => 'text',
                'addChoice' => 'label',
                default => null,
            };
            if ($field === null || !property_exists($event->params, $field)) return $event;
            $copy = clone $event;
            $copy->params = clone $event->params;
            self::textField($copy->params, $field, $chain);
            return $copy;
        }, $events);
    }

    /** @param list<string> $chain */
    private static function textField(stdClass $target, string $field, array $chain): void
    {
        if (property_exists($target, $field)) $target->{$field} = self::text($target->{$field}, $chain) ?? '';
    }

    /** @param list<string> $out */
    private static function addLocales(mixed $value, array &$out): void
    {
        if (!$value instanceof stdClass) return;
        foreach (get_object_vars($value) as $locale => $text) {
            if (self::isText($text) && !in_array((string) $locale, $out, true)) $out[] = (string) $locale;
        }
    }

    /** @param list<string> $out */
    private static function eventLocales(mixed $event, array &$out): void
    {
        if (!$event instanceof stdClass || !($event->params ?? null) instanceof stdClass) return;
        if (($event->type ?? null) === 'showMessage') self::addLocales($event->params->text ?? null, $out);
        elseif (($event->type ?? null) === 'addChoice') self::addLocales($event->params->label ?? null, $out);
    }

    /** @param list<string> $out */
    private static function groupLocales(stdClass $group, array &$out): void
    {
        foreach (is_array($group->slices ?? null) ? $group->slices : [] as $slice) {
            if (!$slice instanceof stdClass) continue;
            foreach (is_array($slice->dialog ?? null) ? $slice->dialog : [] as $dialog) {
                if ($dialog instanceof stdClass) self::addLocales($dialog->text ?? null, $out);
            }
            foreach (is_array($slice->events ?? null) ? $slice->events : [] as $event) self::eventLocales($event, $out);
            foreach (is_array($slice->items ?? null) ? $slice->items : [] as $item) {
                if ($item instanceof stdClass) {
                    foreach (is_array($item->events ?? null) ? $item->events : [] as $event) self::eventLocales($event, $out);
                }
            }
        }
        foreach (is_array($group->events ?? null) ? $group->events : [] as $event) self::eventLocales($event, $out);
    }
}
