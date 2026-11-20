<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Console\AssetsTimemapsCommand;
use Scutoons\Domain\Assets\AssetImporter;
use Scutoons\Domain\Assets\AssetJobs;
use Scutoons\Domain\Assets\AssetRegistry;
use Scutoons\Domain\Assets\AssetTimeMaps;
use Scutoons\Domain\Assets\AssetUsages;
use Scutoons\Infrastructure\AssetStorage\AssetStorages;
use Scutoons\Tests\TestCase;
use Symfony\Component\Console\Tester\CommandTester;

/** The time map of each video derivative: stored by the job, and by the backfill; the binary is a stub that prints a fixed map */
final class AssetTimeMapsTest extends TestCase
{
    private const MP4 = "\0\0\0\x18ftypmp42\0\0\0\0mp42isom";

    protected function settings(): array
    {
        $bin = $this->dir . '/bin';
        if (!is_dir($bin)) {
            mkdir($bin, 0777, true);
            // ffmpeg: copies the input (-i) to the last argument; timemap: prints a map, or fails while `fail` exists
            file_put_contents("$bin/ffmpeg.php", '<?php $a = $argv; $i = array_search("-i", $a, true); copy($a[$i + 1], end($a));');
            file_put_contents("$bin/timemap.php", '<?php if (is_file(__DIR__ . "/fail")) { fwrite(STDERR, "no decoder for h264"); exit(1); } echo "{\"offset\":0.5,\"fps\":24}\n";');
        }
        return [
            'ASSET_STORAGE' => 'local',
            'ASSET_LOCAL_ROOT' => $this->dir . '/cdn',
            'CDN_URL' => 'http://cdn.test/',
            'FFMPEG_PATH' => $this->wrapper('ffmpeg'),
            'TIMEMAP_PATH' => $this->wrapper('timemap'),
        ] + parent::settings();
    }

    private function wrapper(string $name): string
    {
        $bin = $this->dir . '/bin';
        if (PHP_OS_FAMILY === 'Windows') {
            file_put_contents("$bin/$name.cmd", '@"' . PHP_BINARY . "\" \"$bin/$name.php\" %*\r\n");
            return "$bin/$name.cmd";
        }
        file_put_contents("$bin/$name.sh", "#!/bin/sh\nexec '" . PHP_BINARY . "' '$bin/$name.php' \"\$@\"\n");
        chmod("$bin/$name.sh", 0755);
        return "$bin/$name.sh";
    }

    private function fixture(): void
    {
        foreach (['480p', '720p', '1080p'] as $m) {
            $file = $this->dir . "/cdn/catpope/videos/clip.$m.mp4";
            @mkdir(dirname($file), 0777, true);
            file_put_contents($file, self::MP4 . $m);
        }
        $this->get(AssetImporter::class)->import($this->get(AssetStorages::class)->local());
        $this->get(AssetUsages::class)->rebuild();
    }

    /** @return array{0: ?float, 1: ?float} offset, fps of the rendition at $path */
    private function map(string $path): array
    {
        $row = $this->db->one('SELECT time_offset, frame_rate FROM asset_renditions WHERE path = ?', [$path]);
        self::assertNotNull($row, $path);
        return [$row['time_offset'] === null ? null : (float) $row['time_offset'], $row['frame_rate'] === null ? null : (float) $row['frame_rate']];
    }

    /** @return array<string, mixed> */
    private function runJob(string $level, bool $overwrite = true): array
    {
        $jobs = $this->get(AssetJobs::class);
        $asset = $this->get(AssetRegistry::class)->bySlug('catpope/videos/clip');
        self::assertNotNull($asset);
        $job = $jobs->enqueue((string) $asset['id'], 'video-renditions', ['levels' => [(int) $level], 'overwrite' => $overwrite], null);
        return $jobs->run($job['id']);
    }

    public function testTheJobStoresTheMeasuredMap(): void
    {
        $this->fixture();
        self::assertSame([null, null], $this->map('catpope/videos/clip.480p.mp4'), 'imported files have no map');
        $job = $this->runJob('480');
        self::assertSame('done', $job['status'], (string) $job['log']);
        self::assertStringContainsString('time map: offset 0.5 s, 24 fps', (string) $job['log']);
        self::assertSame([0.5, 24.0], $this->map('catpope/videos/clip.480p.mp4'));
        self::assertSame([null, null], $this->map('catpope/videos/clip.720p.mp4'), 'only the derivative the job made');
    }

    public function testAFailedMeasurementLeavesNullAndNeverFailsTheJob(): void
    {
        $this->fixture();
        self::assertSame('done', $this->runJob('480')['status']);
        file_put_contents($this->dir . '/bin/fail', '1');
        $job = $this->runJob('480');
        self::assertSame('done', $job['status'], (string) $job['log']);
        self::assertStringContainsString('time map not measured: timemap failed: no decoder for h264', (string) $job['log']);
        self::assertSame([null, null], $this->map('catpope/videos/clip.480p.mp4'), 'the replaced file\'s old map is stale');
    }

    public function testNotInstalledIsAFailureToo(): void
    {
        $this->fixture();
        $maps = new AssetTimeMaps($this->get(AssetRegistry::class), $this->get(AssetStorages::class), \Scutoons\Infrastructure\Config\Settings::fixed(self::root(), ['TIMEMAP_PATH' => $this->dir . '/nowhere/deck-asset-timemap']));
        $notes = [];
        self::assertFalse($maps->store((int) $this->db->value("SELECT id FROM asset_renditions WHERE path = 'catpope/videos/clip.480p.mp4'"), 'a', 'b', function (string $line) use (&$notes): void {
            $notes[] = $line;
        }));
        self::assertStringContainsString('time map not measured', $notes[0]);
    }

    public function testTheBackfillMeasuresInBatchesAndResumes(): void
    {
        $this->fixture();
        $maps = $this->get(AssetTimeMaps::class);
        $lines = [];
        $say = function (string $line) use (&$lines): void {
            $lines[] = $line;
        };

        $tester = new CommandTester($this->get(AssetsTimemapsCommand::class));
        $tester->execute(['--dry-run' => true]);
        self::assertStringContainsString('3 video renditions without a time map', $tester->getDisplay());

        $first = $maps->backfill(1, 0, 2, $say);
        self::assertSame([2, 0, 1], [$first['measured'], $first['failed'], $first['left']]);
        self::assertSame(1, $this->get(AssetRegistry::class)->countWithoutTimeMap());

        $rest = $maps->backfill(5, $first['last'], null, $say);
        self::assertSame([1, 0, 0], [$rest['measured'], $rest['failed'], $rest['left']]);
        foreach (['480p', '720p', '1080p'] as $m) self::assertSame([0.5, 24.0], $this->map("catpope/videos/clip.$m.mp4"), $m);

        // a failure is counted, left NULL, and a later run takes it up again
        $this->db->run('UPDATE asset_renditions SET time_offset = NULL, frame_rate = NULL WHERE path = ?', ['catpope/videos/clip.720p.mp4']);
        file_put_contents($this->dir . '/bin/fail', '1');
        $failed = $maps->backfill(5, 0, null, $say);
        self::assertSame([0, 1, 0], [$failed['measured'], $failed['failed'], $failed['left'] === 1 ? 0 : $failed['left']]);
        self::assertSame([null, null], $this->map('catpope/videos/clip.720p.mp4'));
        unlink($this->dir . '/bin/fail');
        $again = $maps->backfill(5, 0, null, $say);
        self::assertSame(1, $again['measured']);
        $tester->execute(['--dry-run' => true]);
        self::assertStringContainsString('0 video renditions without a time map', $tester->getDisplay());
    }
}
