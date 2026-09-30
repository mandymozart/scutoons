<?php
declare(strict_types=1);

namespace Scutoons\Domain\Library;

/** A composed library answer: the JSON body plus what the HTTP layer turns into headers */
final class Answer
{
    /** @param list<string> $missing documents a reference pointed to that don't exist (X-Content-Missing) */
    public function __construct(
        public readonly string $body,
        public readonly string $etag,
        public readonly bool $draft,
        public readonly ?string $locale = null,
        public readonly bool $fallback = false,
        public readonly array $missing = [],
    ) {
    }

    /** @return array<string, mixed> the cached form (without the draft flag: drafts are never cached) */
    public function toCache(): array
    {
        return ['body' => $this->body, 'locale' => $this->locale, 'fallback' => $this->fallback, 'missing' => $this->missing];
    }

    public static function fromCache(string $raw, string $etag): ?self
    {
        $data = json_decode($raw, true);
        if (!is_array($data) || !is_string($data['body'] ?? null)) return null;
        $missing = is_array($data['missing'] ?? null) ? array_values(array_map('strval', $data['missing'])) : [];
        return new self($data['body'], $etag, false, is_string($data['locale'] ?? null) ? $data['locale'] : null, (bool) ($data['fallback'] ?? false), $missing);
    }
}
