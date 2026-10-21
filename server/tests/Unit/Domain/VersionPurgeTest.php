<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\Admin\Dashboard;
use Scutoons\Domain\Admin\VersionPurge;
use Scutoons\Domain\ApiError;
use Scutoons\Tests\TestCase;

/** Purge old document versions (issue 154): what goes, what always stays, the dry run's counts as the guard */
final class VersionPurgeTest extends TestCase
{
    private function version(string $key, string $createdAt, string $status = 'draft'): int
    {
        $this->db->run('INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES (?, ?, ?, NULL, ?, NULL, ?, ?, NULL)',
            [$key, explode(':', $key)[0], explode(':', $key)[1] ?? null, '{"n":1}', $createdAt, $status]);
        return $this->db->lastInsertId();
    }

    /** @return list<int> the ids left of a key */
    private function ids(string $key): array
    {
        return array_map('intval', array_column($this->db->all('SELECT id FROM entity_versions WHERE entity_key = ? ORDER BY id', [$key]), 'id'));
    }

    public function testThePlanKeepsPublishedNewestAndReviewedVersions(): void
    {
        $old = [];
        for ($i = 1; $i <= 3; $i++) $old[] = $this->version('chapter:catpope:1', "2026-07-0$i 10:00:00");
        $published = $this->version('chapter:catpope:1', '2026-07-05 10:00:00', 'published');
        $reviewed = $this->version('chapter:catpope:1', '2026-07-06 10:00:00');
        $this->db->run("INSERT INTO content_reviews (entity_key, toon, version_id, state, created_at) VALUES ('chapter:catpope:1', 'catpope', ?, 'returned', '2026-07-06 11:00:00')", [$reviewed]);
        $recent = $this->version('chapter:catpope:1', '2026-09-01 10:00:00');
        $newest = $this->version('chapter:catpope:1', '2026-09-02 10:00:00');
        // a document whose only (old) version is its newest: nothing goes
        $lonely = $this->version('season:catpope:1', '2026-01-01 00:00:00');

        $plan = $this->get(VersionPurge::class)->plan('2026-08-01');
        self::assertSame('2026-08-01 00:00:00', $plan['cutoff']);
        self::assertSame([1, 3], [$plan['documents'], $plan['versions']]);
        self::assertSame(['key' => 'chapter:catpope:1', 'kind' => 'chapter', 'toon' => 'catpope', 'remove' => 3, 'keep' => 4, 'bytes' => 21,
            'oldest' => '2026-07-01 10:00:00', 'newest' => '2026-07-03 10:00:00'], $plan['items'][0]);

        $done = $this->get(VersionPurge::class)->purge('2026-08-01', 1, 3);
        self::assertSame([1, 3, 21], [$done['documents'], $done['deleted'], $done['bytes']]);
        self::assertSame([$published, $reviewed, $recent, $newest], $this->ids('chapter:catpope:1'));
        self::assertSame([$lonely], $this->ids('season:catpope:1'));
        self::assertSame(0, $this->get(VersionPurge::class)->plan('2026-08-01')['versions'], 'a second run finds nothing');
        self::assertNotContains($old[0], $this->ids('chapter:catpope:1'));
    }

    public function testThePurgeRefusesWhenTheDryRunsCountsChanged(): void
    {
        $this->version('toon:catpope', '2026-07-01 00:00:00');
        $this->version('toon:catpope', '2026-07-02 00:00:00');
        $purge = $this->get(VersionPurge::class);
        self::assertSame(1, $purge->plan('2026-08-01')['versions']);
        // a save after the dry run turns the kept newest version into history
        $this->version('toon:catpope', '2026-09-20 00:00:00');
        try {
            $purge->purge('2026-08-01', 1, 1);
            self::fail('expected plan_changed');
        } catch (ApiError $e) {
            self::assertSame([409, 'plan_changed', ['documents' => 1, 'versions' => 2]], [$e->status, $e->errorCode, $e->extra]);
        }
        self::assertCount(3, $this->ids('toon:catpope'), 'nothing deleted');
        self::assertSame(2, $purge->purge('2026-08-01', 1, 2)['deleted']);
    }

    public function testTheDayMustBeAValidPastDay(): void
    {
        $purge = $this->get(VersionPurge::class);
        foreach (['', '2026-02-30', '21.09.2026', '2026-09-22', null] as $bad) {
            try {
                $purge->plan($bad);
                self::fail('expected invalid_date for ' . var_export($bad, true));
            } catch (ApiError $e) {
                self::assertSame([422, 'invalid_date'], [$e->status, $e->errorCode]);
            }
        }
        self::assertSame(0, $purge->plan('2026-09-21')['versions'], 'today is allowed');
        $this->expectExceptionMessage('dry run');
        $purge->purge('2026-09-21', '1', 1);
    }

    public function testTheDashboardCountsWhatAPurgeOfThirtyDaysWouldRemove(): void
    {
        $this->version('toon:catpope', '2026-08-01 00:00:00');
        $this->version('toon:catpope', '2026-08-20 00:00:00'); // the cutoff: 2026-08-22
        $this->version('toon:catpope', '2026-09-20 00:00:00');
        self::assertSame(2, $this->get(VersionPurge::class)->countOlderThan(30));
        self::assertSame(2, $this->get(Dashboard::class)->build(7)['totals']['old_versions']);
    }
}
