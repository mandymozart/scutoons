<?php
declare(strict_types=1);

namespace Scutoons\Tests\Infrastructure;

use Scutoons\Infrastructure\Kas\KasApi;
use Scutoons\Infrastructure\Kas\KasFault;

/**
 * all-inkl's KAS API in memory (#201) – never the real zone: the DNS records of scutoons.com as `get_dns_settings`
 * lists them (`record_name` relative to the zone, like `x1.lan`), add / update change them, every call is recorded.
 * `fail` = the next call throws that fault string.
 */
final class FakeKas implements KasApi
{
    /** @var list<array{action: string, params: array<string, string|int>}> */
    public array $calls = [];
    /** @var list<array<string, string>> */
    public array $records = [
        ['record_zone' => 'scutoons.com', 'record_name' => '', 'record_type' => 'A', 'record_data' => '85.13.0.1', 'record_aux' => '0', 'record_id' => '1', 'record_changeable' => 'Y'],
        ['record_zone' => 'scutoons.com', 'record_name' => 'www', 'record_type' => 'CNAME', 'record_data' => 'scutoons.com.', 'record_aux' => '0', 'record_id' => '2', 'record_changeable' => 'Y'],
        ['record_zone' => 'scutoons.com', 'record_name' => 'old.lan', 'record_type' => 'A', 'record_data' => '192.168.1.20', 'record_aux' => '0', 'record_id' => '3', 'record_changeable' => 'Y'],
        ['record_zone' => 'scutoons.com', 'record_name' => '_acme-challenge.lan', 'record_type' => 'TXT', 'record_data' => 'token', 'record_aux' => '0', 'record_id' => '4', 'record_changeable' => 'Y'],
    ];
    public ?string $fail = null;
    private int $nextId = 100;

    public function call(string $action, array $params = []): mixed
    {
        $this->calls[] = ['action' => $action, 'params' => $params];
        if ($this->fail !== null) {
            $fault = $this->fail;
            $this->fail = null;
            throw new KasFault($fault);
        }
        switch ($action) {
            case 'get_dns_settings':
                return $this->records;
            case 'add_dns_settings':
                $id = (string) $this->nextId++;
                $this->records[] = ['record_zone' => 'scutoons.com', 'record_name' => (string) $params['record_name'], 'record_type' => (string) $params['record_type'],
                    'record_data' => (string) $params['record_data'], 'record_aux' => (string) $params['record_aux'], 'record_id' => $id, 'record_changeable' => 'Y'];
                return $id;
            case 'update_dns_settings':
                foreach ($this->records as $i => $record) {
                    if ($record['record_id'] === (string) $params['record_id']) {
                        $this->records[$i]['record_data'] = (string) $params['record_data'];
                        return 'TRUE';
                    }
                }
                throw new KasFault('record_id_not_found');
        }
        throw new KasFault('unknown_action');
    }
}
