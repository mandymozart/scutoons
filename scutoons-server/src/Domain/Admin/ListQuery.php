<?php
declare(strict_types=1);

namespace Scutoons\Domain\Admin;

use Scutoons\Domain\ApiError;

/**
 * The URL-driven list convention of the admin app (agents/PLATFORM.md "Admin UI patterns"):
 * `?page=&page_size=&sort=&q=&<filter>=` → `{ items, pagination: { total, page, pageSize } }`.
 * page starts at 1; page_size is clamped to [1, max]; sort = a field name, `-field` = descending; unknown sort
 * fields → 400 invalid_sort.
 */
final class ListQuery
{
    /** @param array<string, string> $filters */
    public function __construct(
        public readonly int $page,
        public readonly int $pageSize,
        public readonly string $sortField,
        public readonly bool $descending,
        public readonly string $q,
        public readonly array $filters,
    ) {
    }

    /**
     * @param array<mixed> $query          the request's query parameters
     * @param list<string> $sortable       allowed sort fields
     * @param list<string> $filterNames    query parameters kept as filters (strings only)
     */
    public static function from(array $query, array $sortable, string $defaultSort, array $filterNames = [], int $defaultSize = 50, int $maxSize = 500): self
    {
        $page = max(1, (int) ($query['page'] ?? 1));
        $size = isset($query['page_size']) ? (int) $query['page_size'] : $defaultSize;
        $size = max(1, min($maxSize, $size));
        $sort = is_string($query['sort'] ?? null) && $query['sort'] !== '' ? $query['sort'] : $defaultSort;
        $descending = str_starts_with($sort, '-');
        $field = ltrim($sort, '-');
        if (!in_array($field, $sortable, true)) {
            throw new ApiError(400, 'invalid_sort', "Unknown sort field $field.", ['sortable' => $sortable]);
        }
        $q = is_string($query['q'] ?? null) ? trim(mb_substr($query['q'], 0, 254)) : '';
        $filters = [];
        foreach ($filterNames as $name) {
            $value = $query[$name] ?? null;
            if (is_string($value) && $value !== '') $filters[$name] = $value;
        }
        return new self($page, $size, $field, $descending, $q, $filters);
    }

    public function offset(): int
    {
        return ($this->page - 1) * $this->pageSize;
    }

    public function filter(string $name): ?string
    {
        return $this->filters[$name] ?? null;
    }

    /** ORDER BY clause for a whitelisted column expression (plus a stable tiebreaker) */
    public function orderBy(string $column, string $tiebreaker): string
    {
        $direction = $this->descending ? 'DESC' : 'ASC';
        return "ORDER BY $column $direction, $tiebreaker $direction";
    }

    /**
     * @param list<mixed> $items
     * @return array{items: list<mixed>, pagination: array{total: int, page: int, pageSize: int}}
     */
    public function result(array $items, int $total): array
    {
        return ['items' => $items, 'pagination' => ['total' => $total, 'page' => $this->page, 'pageSize' => $this->pageSize]];
    }
}
