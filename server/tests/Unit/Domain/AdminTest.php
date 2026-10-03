<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\Admin\Batches;
use Scutoons\Domain\Admin\CodeAdmin;
use Scutoons\Domain\Admin\Dashboard;
use Scutoons\Domain\Admin\EventsAdmin;
use Scutoons\Domain\Admin\ListQuery;
use Scutoons\Domain\Admin\UserAdmin;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Codes\CodeService;
use Scutoons\Domain\Events\EventLog;
use Scutoons\Infrastructure\Storage\FileStore;
use Scutoons\Tests\TestCase;

final class AdminTest extends TestCase
{
    public function testListQueryConvention(): void
    {
        $query = ListQuery::from(['page' => '3', 'page_size' => '9999', 'sort' => '-scans', 'q' => ' ab ', 'type' => 'promo', 'x' => 'y'], ['scans', 'code'], 'code', ['type']);
        self::assertSame([3, 500, 'scans', true, 'ab', ['type' => 'promo'], 1000], [$query->page, $query->pageSize, $query->sortField, $query->descending, $query->q, $query->filters, $query->offset()]);
        self::assertSame(['items' => [1], 'pagination' => ['total' => 7, 'page' => 3, 'pageSize' => 500]], $query->result([1], 7));
        $this->expectException(ApiError::class);
        ListQuery::from(['sort' => 'email'], ['code'], 'code');
    }

    public function testCodeListFiltersSortsAndHidesOwners(): void
    {
        $admin = $this->get(CodeAdmin::class);
        $a = $this->code('membership', ['edition_id' => '3_catpope_vinyl', 'edition_number' => 1]);
        $b = $this->code('membership', ['edition_id' => '3_catpope_vinyl', 'edition_number' => 2]);
        $this->code('promo');
        $this->get(CodeService::class)->claim($this->user('owner@example.com'), $b, null, null);
        $events = $this->get(EventLog::class);
        $events->log('scan', $a);
        $events->log('scan', $a);

        $list = $admin->list(ListQuery::from(['sort' => '-scans', 'edition_id' => '3_catpope_vinyl'], CodeAdmin::SORTABLE, 'code', CodeAdmin::FILTERS));
        self::assertSame(2, $list['pagination']['total']);
        self::assertSame([$a, 2], [$list['items'][0]['code'], $list['items'][0]['scans']]);
        self::assertStringNotContainsString('owner@', (string) json_encode($list));
        $claimed = $admin->list(ListQuery::from(['status' => 'claimed'], CodeAdmin::SORTABLE, 'code', CodeAdmin::FILTERS));
        self::assertSame([$b], array_column($claimed['items'], 'code'));
        self::assertCount(2, $admin->export(ListQuery::from(['edition_id' => '3_catpope_vinyl'], CodeAdmin::SORTABLE, 'code', CodeAdmin::FILTERS)));
        // the search box ignores case on SQLite and MariaDB alike (codes themselves are case-sensitive: utf8mb4_bin)
        $this->code('membership', ['code' => 'FindMe12']);
        $this->code('membership', ['code' => 'findme12']);
        $found = $admin->list(ListQuery::from(['q' => 'FINDME'], CodeAdmin::SORTABLE, 'code', CodeAdmin::FILTERS));
        self::assertSame(['FindMe12', 'findme12'], array_column($found['items'], 'code'));
        // a hand-made code that differs only in case from an existing one is another code
        self::assertSame('FINDME12', $admin->create(['redirect_to' => 'https://scutoons.com/catpope', 'type' => 'promo', 'code' => 'FINDME12'])['code']);
    }

    public function testCreateRevokeSignatureTimeline(): void
    {
        $admin = $this->get(CodeAdmin::class);
        $row = $admin->create(['redirect_to' => 'https://scutoons.com/catpope', 'type' => 'promo']);
        self::assertMatchesRegularExpression('/^[A-Za-z0-9_-]{8}$/', $row['code']);
        try {
            $admin->create(['redirect_to' => 'https://scutoons.com/catpope', 'type' => 'promo', 'code' => $row['code']]);
            self::fail('code_exists expected');
        } catch (ApiError $error) {
            self::assertSame(409, $error->status);
        }
        self::assertSame('gone', $admin->setRevoked($row['code'], true, ' gone ')['revoked_reason']);
        self::assertNull($admin->setRevoked($row['code'], false)['revoked_at']);
        self::assertSame(['unrevoke', 'revoke'], array_column($admin->timeline($row['code']), 'event'));
        $admin->saveSignature($row['code'], ['signature' => str_repeat('ab', 40), 'publicKey' => 'p', 'algorithm' => 'ML-DSA-65', 'signedData' => '{"code":"x"}']);
        self::assertSame('ABAB:ABAB', substr($admin->signature($row['code'])['fingerprint'], 0, 9));
        self::assertTrue($admin->detail($row['code'])['signed']);
    }

    public function testDashboardCountsPerDayAndEdition(): void
    {
        $code = $this->code('membership', ['edition_id' => '2_catpope_beetles']);
        $events = $this->get(EventLog::class);
        $events->log('scan', $code);
        $this->clock->advance(-86400);
        $events->log('scan', $code);
        $events->log('access', $code);
        $this->clock->advance(86400);
        $dashboard = $this->get(Dashboard::class)->build(3);
        self::assertSame(['2026-09-19', '2026-09-20', '2026-09-21'], array_column($dashboard['daily'], 'day'));
        self::assertSame([0, 1, 1], array_column($dashboard['daily'], 'scan'));
        self::assertSame(2, $dashboard['period']['scan']);
        $beetles = array_values(array_filter($dashboard['editions'], fn($e) => $e['id'] === '2_catpope_beetles'))[0];
        self::assertSame([1, 0, 20, 2, 1], [$beetles['codes'], $beetles['claimed'], $beetles['edition_count'], $beetles['scan'], $beetles['access']]);
        self::assertSame(1, $this->get(EventsAdmin::class)->list(ListQuery::from(['event' => 'access'], EventsAdmin::SORTABLE, '-created_at', EventsAdmin::FILTERS))['pagination']['total']);
    }

    public function testBatchesReserveCodesAndPackAZip(): void
    {
        $batches = $this->get(Batches::class);
        $manifest = $batches->create('2_catpope_beetles', 3, 5);
        self::assertSame([5, 6, 7], array_column($manifest['codes'], 'n'));
        self::assertCount(3, array_unique(array_column($manifest['codes'], 'code')));
        self::assertSame($manifest['filename'], $batches->summaries()[0]['filename']);
        foreach ($manifest['codes'] as $i => $entry) $manifest['codes'][$i]['status'] = 'done';
        $batches->save($manifest['filename'], $manifest);
        try {
            $batches->create('2_catpope_beetles', 3, 1);
            self::fail('batch_complete expected');
        } catch (ApiError $error) {
            self::assertSame('batch_complete', $error->errorCode);
        }
        $code = $manifest['codes'][0]['code'];
        $batches->writeFile('certificates', "SCUTOONS-COA-$code.pdf", 'data:application/pdf;base64,' . base64_encode('%PDF'));
        $batches->writeFile('signatures', "SIGNATURE-$code.json", '{"a":"ä/"}');
        self::assertSame("{\n    \"a\": \"ä/\"\n}", $this->get(FileStore::class)->read('signatures', "SIGNATURE-$code.json"));
        $zip = new \ZipArchive();
        self::assertTrue($zip->open($batches->zip($manifest['filename'])));
        self::assertSame(3, $zip->numFiles, 'manifest + pdf + signature');
        $zip->close();
        self::assertSame(3, count($batches->manifestIndex()));
        $batches->writeFile('svgs', '../x.svg', '<svg/>');
        self::assertFileExists($this->dir . '/files/svgs/x.svg', 'path parts are dropped (as files.php did)');
        self::assertFileDoesNotExist($this->dir . '/files/x.svg');
        try {
            $batches->writeFile('svgs', '.hidden.svg', '<svg/>');
            self::fail('invalid_filename expected');
        } catch (ApiError $error) {
            self::assertSame('invalid_filename', $error->errorCode);
        }
    }

    public function testUserAdminListsAndChangesRoles(): void
    {
        $users = $this->get(UserAdmin::class);
        $user = $this->user('someone@example.com');
        $this->user('other@example.com');
        $list = $users->list(ListQuery::from(['q' => 'SOMEONE', 'sort' => 'email'], UserAdmin::SORTABLE, '-created_at', UserAdmin::FILTERS));
        self::assertSame(['someone@example.com'], array_column($list['items'], 'email'));
        self::assertTrue($users->setRole((string) $user['id'], 'creator', true, 'admin:test')['changed']);
        self::assertSame(1, $users->list(ListQuery::from(['role' => 'creator'], UserAdmin::SORTABLE, '-created_at', UserAdmin::FILTERS))['pagination']['total']);
        self::assertSame(['member', 'creator'], $users->detail((string) $user['id'])['roles']);
        self::assertSame(2, $users->search('', 50)['total']);
    }
}
