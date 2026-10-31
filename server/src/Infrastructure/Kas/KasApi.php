<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Kas;

/**
 * all-inkl's KAS API (the hosting's own API: DNS records of scutoons.com – #201, Domain\Lan\LanNames). An interface so
 * the tests run against a fake – never against the real zone.
 */
interface KasApi
{
    /**
     * One KAS action (`get_dns_settings`, `add_dns_settings`, `update_dns_settings` …) with its `KasRequestParams`;
     * returns the answer's `Response.ReturnInfo` (a list of records, a record id, `TRUE` …).
     * @param array<string, string|int> $params
     * @throws KasFault the API refused (its fault string, e.g. `record_id_not_found`, `flood_protection`)
     */
    public function call(string $action, array $params = []): mixed;
}
