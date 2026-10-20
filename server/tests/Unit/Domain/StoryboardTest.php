<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Console\ContentFiles;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Storyboard\Generations;
use Scutoons\Domain\Storyboard\WorkStorage;
use Scutoons\Infrastructure\Database\MigrationRunner;
use Scutoons\Tests\TestCase;

/**
 * The storyboard on the server (#120): the entity `storyboard:<toon>:<chapter>`, its import migration, the generations
 * (the ledger) and the shared work storage
 */
final class StoryboardTest extends TestCase
{
    public const PNG = "\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
    public const MP4 = "\0\0\0\x18ftypmp42\0\0\0\0mp42isom";

    protected function settings(): array
    {
        return ['STORYBOARD_WORK_PATH' => $this->dir . '/work', 'STORYBOARD_WORK_MAX_BYTES' => '1000'] + parent::settings();
    }

    private function tmp(string $bytes): string
    {
        $file = $this->dir . '/upload-' . bin2hex(random_bytes(3));
        file_put_contents($file, $bytes);
        return $file;
    }

    /** @param array<string, mixed> $error */
    private static function fails(callable $call, int $status, string $code, array $error = []): void
    {
        try {
            $call();
            self::fail("expected $status $code");
        } catch (ApiError $e) {
            self::assertSame([$status, $code], [$e->status, $e->errorCode], $e->getMessage());
            foreach ($error as $k => $v) self::assertSame($v, $e->extra[$k] ?? null);
        }
    }

    public function testTheEntityKeyIsAChapterNumberWithoutLocaleAndCreatorOnly(): void
    {
        $key = EntityKey::parse('storyboard:catpope:3');
        self::assertSame(['storyboard', 'catpope', 3, null, 'storyboard:catpope:3'], [$key->kind, $key->toon, $key->number, $key->locale, (string) $key]);
        self::assertTrue(EntityKey::isValid('storyboard:catpope:0'), 'a prologue');
        foreach (['storyboard:catpope', 'storyboard:catpope:3:de', 'storyboard:catpope:03', 'storyboard:scutoons:3', 'storyboard'] as $bad) {
            self::assertFalse(EntityKey::isValid($bad), $bad);
        }
        self::assertContains('storyboard', EntityKey::CREATOR_ONLY_KINDS);
        self::assertSame('catpope/_/storyboard3.json', ContentFiles::pathOf($key));
        self::assertSame('storyboard:catpope:3', ContentFiles::keyOf('catpope', '_', 'storyboard3.json'));
        self::assertNull(ContentFiles::keyOf('catpope', 'de', 'storyboard3.json'));

        // versioned with drafts like every entity
        $store = $this->get(EntityStore::class);
        $doc = (object) ['storyboard' => 2, 'toon' => 'catpope', 'chapter' => 3, 'scenes' => [(object) ['storyboard' => 2, 'toon' => 'catpope', 'source' => (object) ['scene' => 25], 'groups' => []]]];
        $v1 = $store->save($this->user(), $key, (object) ['content' => $doc]);
        self::assertSame(['draft', 'storyboard', 'catpope', null], [$v1['version']['status'], $v1['version']['kind'], $v1['version']['toon'], $v1['version']['locale']]);
        self::fails(fn() => $store->save($this->user(), $key, (object) ['content' => $doc, 'base' => $v1['version']['id'] - 1]), 409, 'conflict');
    }

    public function testTheImportMigrationStoresEachChapterOnceWhereTheToonExists(): void
    {
        $file = self::root() . '/migrations/mysql/2026-10-20-storyboard-import.mysql.sql';
        $runner = new MigrationRunner($this->db, self::root() . '/migrations');
        $runner->runFile($file);
        self::assertSame(0, (int) $this->db->value("SELECT COUNT(*) FROM entity_versions WHERE kind = 'storyboard'"), 'no toon catpope (the test database): nothing');

        $this->db->run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES ('toon:catpope', 'toon', 'catpope', NULL, '{}', NULL, '2026-10-01 00:00:00', 'published', NULL)");
        $this->get(EntityStore::class)->save($this->user(), EntityKey::parse('storyboard:catpope:5'), (object) ['content' => (object) ['storyboard' => 2, 'toon' => 'catpope', 'chapter' => 5, 'scenes' => []]]);
        $runner->runFile($file);
        $runner->runFile($file); // idempotent
        $rows = $this->db->all("SELECT entity_key, status, json FROM entity_versions WHERE kind = 'storyboard' ORDER BY id");
        self::assertSame(['storyboard:catpope:5', 'storyboard:catpope:3', 'storyboard:catpope:4'], array_column($rows, 'entity_key'), 'chapter 5 existed: kept');
        self::assertSame(['draft', 'published', 'published'], array_column($rows, 'status'));
        $three = json_decode((string) $rows[1]['json'], true);
        self::assertSame([2, 'catpope', 3], [$three['storyboard'], $three['toon'], $three['chapter']]);
        self::assertSame(range(25, 34), array_map(fn(array $s) => $s['source']['scene'], $three['scenes']), 'scenes in scene order');
        $file25 = json_decode((string) file_get_contents(dirname(self::root()) . '/storyboards/catpope/s1c3/scene-25.json'), true);
        self::assertSame($file25, $three['scenes'][0], 'the scene file as it is');
    }

    public function testRecordListAndReviewAGeneration(): void
    {
        $generations = $this->get(Generations::class);
        $user = $this->user('maker@scutoons.test');
        $item = $generations->record($user, null, [
            'toon' => 'catpope', 'chapter' => 3, 'shot' => 'sb-25-1', 'kind' => 'image', 'generator' => 'comfy-local', 'model' => 'flux-dev',
            'prompt' => 'Ein Bergdorf im Schnee', 'seed' => 42, 'ratio' => '9:16', 'ms' => 5300, 'parents' => [], 'inputs' => [['role' => 'pose', 'file' => 'take-1.png']],
            'params' => ['steps' => 30], 'file' => 'generated/sb-25-1-a.png', 'file_hash' => str_repeat('ab', 32), 'machine' => 'Tilman PC',
        ]);
        self::assertSame(['catpope', 3, 'sb-25-1', 'image', 'succeeded', 'comfy-local', 42, 'Tilman PC', null, false, null, 'maker@scutoons.test', null, '2026-09-21T14:13:20+00:00'],
            [$item['toon'], $item['chapter'], $item['shot'], $item['kind'], $item['status'], $item['generator'], $item['seed'], $item['machine'], $item['mark'],
                $item['approved'], $item['work_url'], $item['author'], $item['via'], $item['created_at']]);
        self::assertSame([[['role' => 'pose', 'file' => 'take-1.png']], ['steps' => 30], []], [$item['inputs'], $item['params'], $item['parents']]);
        $other = $generations->record($user, null, ['toon' => 'catpope', 'chapter' => 3, 'shot' => 'sb-25-2', 'kind' => 'video', 'generator' => 'upload',
            'file' => 'clip.mp4', 'machine' => 'laptop', 'parents' => [$item['id']], 'duration' => 5, 'cost' => 0.4, 'currency' => 'USD', 'mark' => 'review']);
        self::assertSame([5.0, 0.4, 'review', '2026-09-21T14:13:20+00:00', [$item['id']]], [$other['duration'], $other['cost'], $other['mark'], $other['marked_at'], $other['parents']]);
        $generations->record($user, null, ['toon' => 'catpope', 'chapter' => 4, 'shot' => 'sb-35-0', 'kind' => 'image', 'generator' => 'luma', 'status' => 'failed', 'error' => 'timeout']);

        self::assertSame([$item['id'], $other['id']], array_column($generations->list('catpope', '3'), 'id'));
        self::assertSame([$other['id']], array_column($generations->list('catpope', 3, 'sb-25-2'), 'id'));
        self::assertSame(['failed'], array_column($generations->list('catpope', 4), 'status'));

        $this->clock->advance(60);
        $marked = $generations->update($item['id'], ['mark' => 'rejected', 'note' => 'zu dunkel']);
        self::assertSame(['rejected', '2026-09-21T14:14:20+00:00', 'zu dunkel', false], [$marked['mark'], $marked['marked_at'], $marked['note'], $marked['approved']]);
        $approved = $generations->update($item['id'], ['mark' => null, 'approved' => true, 'dataset' => true]);
        self::assertSame([null, null, true, true, 'zu dunkel'], [$approved['mark'], $approved['marked_at'], $approved['approved'], $approved['dataset'], $approved['note']]);
    }

    public function testRefusesWhatDoesNotFitAndProviderJobs(): void
    {
        $generations = $this->get(Generations::class);
        $user = $this->user();
        $base = ['toon' => 'catpope', 'chapter' => 3, 'shot' => 'sb-25-1', 'kind' => 'image', 'generator' => 'comfy-local', 'file' => 'a.png', 'machine' => 'pc'];
        self::fails(fn() => $generations->record($user, null, ['job' => 'luma-123'] + $base), 422, 'provider_jobs_later');
        self::fails(fn() => $generations->record($user, null, ['status' => 'queued'] + $base), 422, 'provider_jobs_later');
        self::fails(fn() => $generations->record($user, null, ['toon' => 'Cat Pope'] + $base), 400, 'invalid_toon');
        self::fails(fn() => $generations->record($user, null, ['chapter' => -1] + $base), 400, 'invalid_chapter');
        foreach ([
            'kind' => ['kind' => 'audio'], 'shot' => ['shot' => '../x'], 'generator' => ['generator' => 'Luma AI'], 'file' => ['file' => '../secret.png'],
            'machine' => ['machine' => null], 'mark' => ['mark' => 'great'], 'approved' => ['approved' => 'yes'], 'parents' => ['parents' => ['x']],
            'inputs' => ['inputs' => ['a' => 1]], 'params' => ['params' => [1, 2]], 'ratio' => ['ratio' => 'wide'], 'file_hash' => ['file_hash' => 'abc'],
            'cost' => ['cost' => -1], 'colour' => ['colour' => 'red'], 'work_path' => ['work_path' => 'missing.png'],
        ] as $field => $change) {
            self::fails(fn() => $generations->record($user, null, $change + $base), 422, 'invalid_generation', ['field' => $field]);
        }
        self::fails(fn() => $generations->record($user, null, ['machine' => Generations::WORK] + $base), 422, 'invalid_generation', ['field' => 'work_path']);
        $item = $generations->record($user, null, $base);
        self::fails(fn() => $generations->update($item['id'], ['status' => 'failed']), 422, 'invalid_generation', ['field' => 'status']);
        self::fails(fn() => $generations->update($item['id'], ['prompt' => 'x']), 422, 'invalid_generation', ['field' => 'prompt']);
        self::fails(fn() => $generations->update($item['id'] + 99, ['note' => 'x']), 404, 'not_found');
    }

    public function testSharingIntoTheWorkStorage(): void
    {
        $work = $this->get(WorkStorage::class);
        $tmp = $this->tmp(self::PNG . 'pixels');
        $stored = $work->put('catpope', '3', 'sb-25-1-a.png', $tmp, 'image/png');
        self::assertSame(['sb-25-1-a.png', 'image/png', 22, hash('sha256', self::PNG . 'pixels'), '/api/storyboard/work/catpope/3/sb-25-1-a.png', true],
            [$stored['name'], $stored['type'], $stored['size'], $stored['sha256'], $stored['url'], $stored['created']]);
        self::assertFileExists($this->dir . '/work/catpope/3/sb-25-1-a.png');
        self::assertFalse($work->put('catpope', 3, 'sb-25-1-a.png', $tmp, '')['created'], 'the same bytes again: nothing to do');
        self::fails(fn() => $work->put('catpope', 3, 'sb-25-1-a.png', $this->tmp(self::PNG . 'other'), 'image/png'), 409, 'exists');
        self::fails(fn() => $work->put('catpope', 3, 'clip.mp4', $tmp, 'video/mp4'), 422, 'content_mismatch');
        self::fails(fn() => $work->put('catpope', 3, 'b.png', $tmp, 'image/webp'), 422, 'invalid_type');
        self::fails(fn() => $work->put('catpope', 3, 'b.gif', $tmp, ''), 400, 'invalid_name');
        self::fails(fn() => $work->put('catpope', 3, '../b.png', $tmp, ''), 400, 'invalid_name');
        self::fails(fn() => $work->put('catpope', 3, '.b.png', $tmp, ''), 400, 'invalid_name');
        self::assertTrue($work->put('catpope', 3, 'clip.mp4', $this->tmp(self::MP4), 'video/mp4')['created']);
        self::assertSame(['clip.mp4', 'sb-25-1-a.png'], array_column($work->list('catpope', 3), 'name'));
        self::assertSame([], $work->list('catpope', 4));

        // a generation points at its shared file
        $generations = $this->get(Generations::class);
        $item = $generations->record($this->user(), null, ['toon' => 'catpope', 'chapter' => 3, 'shot' => 'sb-25-1', 'kind' => 'image', 'generator' => 'comfy-local', 'file' => 'sb-25-1-a.png', 'machine' => 'pc']);
        self::fails(fn() => $generations->update($item['id'], ['work_path' => 'nope.png']), 422, 'invalid_generation', ['field' => 'work_path']);
        $shared = $generations->update($item['id'], ['machine' => 'work', 'work_path' => 'sb-25-1-a.png', 'mark' => 'review']);
        self::assertSame(['work', 'sb-25-1-a.png', '/api/storyboard/work/catpope/3/sb-25-1-a.png', 'review'], [$shared['machine'], $shared['work_path'], $shared['work_url'], $shared['mark']]);
        self::fails(fn() => $generations->update($item['id'], ['work_path' => null]), 422, 'invalid_generation', ['field' => 'work_path']);

        self::assertSame(['deleted' => 2, 'bytes' => 22 + strlen(self::MP4)], $work->deleteChapter('catpope', 3));
        self::assertDirectoryDoesNotExist($this->dir . '/work/catpope/3');
        self::assertSame(['deleted' => 0, 'bytes' => 0], $work->deleteChapter('catpope', 3));
    }

    public function testSniffsTheTypes(): void
    {
        self::assertSame('image/png', WorkStorage::sniff($this->tmp(self::PNG)));
        self::assertSame('image/jpeg', WorkStorage::sniff($this->tmp("\xFF\xD8\xFF\xE0\0\x10JFIF")));
        self::assertSame('image/webp', WorkStorage::sniff($this->tmp('RIFF' . "\x10\0\0\0" . 'WEBPVP8 ')));
        self::assertSame('video/mp4', WorkStorage::sniff($this->tmp(self::MP4)));
        self::assertSame('video/webm', WorkStorage::sniff($this->tmp("\x1A\x45\xDF\xA3\x9F\x42\x86\x81")));
        self::assertNull(WorkStorage::sniff($this->tmp('<svg onload="x">')));
    }
}
