<?php
declare(strict_types=1);

namespace Scutoons\Domain\Library;

use stdClass;

/**
 * One composition: the snapshot, the toon's languages, whether drafts are visible, and what the answer's headers
 * report (served locale, fallback, missing references).
 */
final class Context
{
    /** @var list<string> */
    public array $missing = [];
    public bool $fallback = false;
    public ?string $servedLocale = null;

    /** @param list<string> $languages source first */
    public function __construct(
        public readonly Snapshot $snapshot,
        public readonly string $toon,
        public readonly ?stdClass $entry,
        public readonly string $source,
        public readonly array $languages,
        /** creator draft mode: newest versions, no status gates */
        public readonly bool $draft,
        /** draft statuses count as readable (draft mode or CONTENT_SHOW_DRAFTS) */
        public readonly bool $includeDrafts,
    ) {
    }

    /**
     * A localized document of the toon: the asked locale, else the source language, else any other language of the
     * toon. Records a fallback when the served locale differs from the asked one.
     * @return array{0: string, 1: mixed}|null [served locale, document]
     */
    public function pick(string $path, string $locale): ?array
    {
        foreach (array_values(array_unique([$locale, $this->source, ...$this->languages])) as $candidate) {
            if (!$this->snapshot->has($this->toon, $candidate, $path)) continue;
            if ($candidate !== $locale) $this->fallback = true;
            return [$candidate, $this->snapshot->doc($this->toon, $candidate, $path)];
        }
        return null;
    }

    /** isItemPublic() of @scutoons/content: no status = published; `draft` only while drafts are included */
    public function isPublic(mixed $status): bool
    {
        $status = is_string($status) && $status !== '' ? $status : Library::PUBLISHED;
        return $status === Library::PUBLISHED || ($status === Library::DRAFT && $this->includeDrafts);
    }

    /** isItemListed(): readable, or `coming-soon` (listed, not clickable) */
    public function isListed(mixed $status): bool
    {
        return $this->isPublic($status) || $status === Library::COMING_SOON;
    }
}
