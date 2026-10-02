<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\AssetStorage;

/**
 * One file on a media storage (agents/ASSETS.md "Storage drivers"). `key` is the path from the CDN root, the same
 * string content references (after the toon folder is prefixed): `catpope/s1/s1-trailer-loop.720p.mp4`.
 */
final class StoredFile
{
    public function __construct(
        public readonly string $key,
        public readonly int $size,
        /** ISO 8601 UTC, '' when the driver doesn't know */
        public readonly string $modified = '',
        /** lower-case hex SHA-256 when the driver has it cheaply (bunny's listing does; local computes on demand) */
        public readonly ?string $sha256 = null,
        public readonly ?string $mime = null,
    ) {
    }

    /** @return array{key: string, size: int, modified: string, sha256: ?string, mime: ?string} */
    public function toArray(): array
    {
        return ['key' => $this->key, 'size' => $this->size, 'modified' => $this->modified, 'sha256' => $this->sha256, 'mime' => $this->mime];
    }
}
