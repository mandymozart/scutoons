<?php
declare(strict_types=1);

namespace Scutoons\Domain\Admin;

use RuntimeException;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Editions;
use Scutoons\Domain\Catalog\Toons;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Infrastructure\Storage\FileStore;
use Scutoons\Support\Ids;
use ZipArchive;

/**
 * Batch runs of an edition (the generate SPA's "Batch" mode, the admin app's "Batches"): a manifest file
 * `BATCH-<timestamp>-<edition id>.json` lists the codes of the batch with their number and progress
 * (`status`: pending|running|done|error, `completedSteps`: db, svg, pdf, sig). Signing, QR SVGs and certificate PDFs
 * are made client-side; the server stores the manifest and the files (FileStore) and packs a ZIP.
 */
final class Batches
{
    public const FILENAME_PATTERN = '/^BATCH-[\w.-]+\.json$/';

    public function __construct(
        private readonly FileStore $files,
        private readonly Editions $editions,
        private readonly Toons $toons,
        private readonly Database $db,
        private readonly Clock $clock,
    ) {
    }

    public static function validFilename(string $filename): bool
    {
        return preg_match(self::FILENAME_PATTERN, $filename) === 1 && basename($filename) === $filename;
    }

    /**
     * Summary of every manifest, newest first (get-batches.php)
     * @return list<array{filename: string, edition: mixed, editionId: mixed, total: mixed, done: int, errors: int, created_at: mixed, complete: bool}>
     */
    public function summaries(): array
    {
        $batches = [];
        foreach ($this->files->list('batch', 'BATCH-*.json') as $filename) {
            $manifest = json_decode((string) $this->files->read('batch', $filename), true);
            if (!is_array($manifest) || !isset($manifest['codes']) || !is_array($manifest['codes'])) continue;
            $done = count(array_filter($manifest['codes'], fn($c) => ($c['status'] ?? null) === 'done'));
            $errors = count(array_filter($manifest['codes'], fn($c) => ($c['status'] ?? null) === 'error'));
            $batches[] = [
                'filename' => $filename,
                'edition' => $manifest['edition'] ?? '',
                'editionId' => $manifest['editionId'] ?? '',
                'total' => $manifest['total'] ?? 0,
                'done' => $done,
                'errors' => $errors,
                'created_at' => $manifest['created_at'] ?? '',
                'complete' => $done === ($manifest['total'] ?? -1),
            ];
        }
        usort($batches, fn($a, $b) => $b['created_at'] <=> $a['created_at']);
        return $batches;
    }

    /** The manifest as stored (raw JSON), or null */
    public function raw(string $filename): ?string
    {
        if (!self::validFilename($filename)) return null;
        return $this->files->read('batch', $filename);
    }

    /** @return array<string, mixed> the decoded manifest, 404 not_found */
    public function get(string $filename): array
    {
        $manifest = json_decode((string) $this->raw($filename), true);
        if (!is_array($manifest)) throw new ApiError(404, 'not_found', 'Batch not found.');
        return $manifest;
    }

    /**
     * Creates a new batch manifest for an edition, like the generate SPA does: codes are fresh nanoid(8) codes that
     * exist neither in the database nor in another manifest; numbers start at $startNumber. The codes are NOT inserted
     * into the database here – each one is registered (POST /api/admin/codes) when its row is processed.
     * 409 batch_complete when a complete batch of this edition exists (unless $force).
     * @return array<string, mixed> the manifest
     */
    public function create(string $editionId, ?int $count, int $startNumber, bool $force = false): array
    {
        $edition = $this->editions->find($editionId) ?? throw new ApiError(404, 'unknown_edition', 'Unknown edition.');
        $count ??= (int) ($edition['edition_count'] ?? 0);
        if ($count < 1 || $count > 2000) throw new ApiError(400, 'invalid_count', 'count must be between 1 and 2000.');
        if ($startNumber < 1) throw new ApiError(400, 'invalid_start_number', 'start_number must be 1 or more.');
        if (!$force) {
            foreach ($this->summaries() as $batch) {
                if ($batch['editionId'] === $editionId && $batch['complete']) {
                    throw new ApiError(409, 'batch_complete', 'A complete batch of this edition exists already. Add a new edition (admin → Catalog → Editions) to make more codes.', ['batch' => $batch]);
                }
            }
        }
        $taken = array_flip($this->manifestCodes());
        $codes = [];
        for ($i = 0; $i < $count; $i++) {
            do {
                $code = Ids::nanoid(8);
            } while (isset($taken[$code]) || $this->db->value('SELECT 1 FROM qr WHERE code = ?', [$code]) !== null);
            $taken[$code] = true;
            $codes[] = ['n' => $startNumber + $i, 'code' => $code, 'status' => 'pending', 'completedSteps' => []];
        }
        $now = $this->clock->time();
        $filename = 'BATCH-' . gmdate('Y-m-d\THis', $now) . '-' . $editionId . '.json';
        if ($this->files->exists('batch', $filename)) throw new ApiError(409, 'batch_exists', 'A batch with this name exists – try again in a second.');
        $toon = $this->toons->find((string) ($edition['toon'] ?? ''));
        $manifest = [
            'filename' => $filename,
            'edition' => $edition['name'] ?? $editionId,
            'editionId' => $editionId,
            'productname' => $edition['name'] ?? $editionId,
            'edition_number' => $edition['edition_number'] ?? 1,
            'edition_count' => $edition['edition_count'] ?? $count,
            'excluded_number' => $edition['excluded_number'] ?? 0,
            'creators' => $edition['creators'] ?? '',
            'redirectUrl' => $toon['redirectUrl'] ?? '',
            'total' => $count,
            'created_at' => gmdate('Y-m-d\TH:i:s.000\Z', $now),
            'codes' => $codes,
        ];
        $this->save($filename, $manifest);
        return $manifest;
    }

    /**
     * Stores a manifest (progress updates from the client). Needs `codes` (a list); the file name must match.
     * @param array<string, mixed> $manifest
     */
    public function save(string $filename, array $manifest): void
    {
        if (!self::validFilename($filename)) throw new ApiError(400, 'invalid_filename', 'Invalid batch file name.');
        if (!is_array($manifest['codes'] ?? null) || !array_is_list($manifest['codes'])) {
            throw new ApiError(422, 'invalid_manifest', 'A manifest needs a codes list.');
        }
        if (isset($manifest['filename']) && $manifest['filename'] !== $filename) {
            throw new ApiError(422, 'invalid_manifest', 'The manifest filename does not match.');
        }
        $this->files->write('batch', $filename, (string) json_encode($manifest, JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES | JSON_PRETTY_PRINT));
    }

    /**
     * Packs the manifest and the files of every finished code into files/zips/<name>.zip; returns its path.
     * 404 not_found, 500 when the ZIP cannot be written.
     */
    public function zip(string $filename): string
    {
        $manifest = $this->get($filename);
        $dir = $this->files->ensureDir('zips');
        $zipPath = $dir . '/' . str_replace('.json', '.zip', $filename);
        $zip = new ZipArchive();
        if ($zip->open($zipPath, ZipArchive::CREATE | ZipArchive::OVERWRITE) !== true) throw new RuntimeException('Could not create ZIP archive');
        $zip->addFile($this->files->path('batch', $filename), 'manifest.json');
        foreach ($manifest['codes'] as $item) {
            if (($item['status'] ?? null) !== 'done' || !is_string($item['code'] ?? null)) continue;
            $code = $item['code'];
            foreach ([
                ['svgs', "SCUTOONS-QR-MEMBERSHIP-$code.svg"],
                ['certificates', "SCUTOONS-COA-$code.pdf"],
                ['signatures', "SIGNATURE-$code.json"],
            ] as [$type, $name]) {
                if ($this->files->exists($type, $name)) $zip->addFile($this->files->path($type, $name), "$type/$name");
            }
        }
        $zip->close();
        return $zipPath;
    }

    /**
     * Stores one generated file (files.php): SVG markup, a PDF as data URI or raw bytes, JSON (re-encoded pretty,
     * UTF-8). 400 invalid_type / invalid_filename / invalid_file.
     */
    public function writeFile(string $type, string $filename, string $doc): void
    {
        if (!in_array($type, FileStore::TYPES, true)) throw new ApiError(400, 'invalid_type', 'Invalid file type');
        $filename = basename($filename);
        if (!FileStore::validName($type, $filename)) throw new ApiError(400, 'invalid_filename', 'Invalid filename');
        if ($doc === '') throw new ApiError(400, 'invalid_file', 'Missing doc');
        $ext = pathinfo($filename, PATHINFO_EXTENSION);
        if ($ext === 'pdf') {
            if (str_starts_with($doc, 'data:')) $doc = (string) base64_decode(explode(',', $doc, 2)[1] ?? '');
            if ($doc === '') throw new ApiError(400, 'invalid_file', 'Invalid PDF data');
        }
        if ($ext === 'json') {
            $decoded = json_decode($doc);
            if (json_last_error() !== JSON_ERROR_NONE) throw new ApiError(400, 'invalid_file', 'Invalid JSON: ' . json_last_error_msg());
            $doc = (string) json_encode($decoded, JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES | JSON_PRETTY_PRINT);
        }
        $this->files->write($type, $filename, $doc);
    }

    /**
     * Every code of the batch manifests in a folder (default: the stored ones) → edition, number, manifest file.
     * A code listed twice keeps its first manifest ($warn gets a line).
     * @param callable(string): void|null $warn
     * @return array<string, array{edition_id: string, n: int, file: string, status: string}>
     */
    public function manifestIndex(?string $dir = null, ?callable $warn = null): array
    {
        $dir = rtrim(str_replace('\\', '/', $dir ?? $this->files->root() . '/batch'), '/');
        $codes = [];
        $files = glob($dir . '/BATCH-*.json') ?: [];
        sort($files);
        foreach ($files as $file) {
            $manifest = json_decode((string) file_get_contents($file), true);
            if (!is_array($manifest) || !is_array($manifest['codes'] ?? null)) {
                if ($warn) $warn('skipping invalid manifest ' . basename($file));
                continue;
            }
            foreach ($manifest['codes'] as $entry) {
                if (!isset($entry['code'], $entry['n'])) continue;
                if (isset($codes[$entry['code']])) {
                    if ($warn) $warn("code {$entry['code']} appears in more than one manifest; keeping " . $codes[$entry['code']]['file']);
                    continue;
                }
                $codes[(string) $entry['code']] = [
                    'edition_id' => (string) ($manifest['editionId'] ?? ''),
                    'n' => (int) $entry['n'],
                    'file' => basename($file),
                    'status' => (string) ($entry['status'] ?? ''),
                ];
            }
        }
        return $codes;
    }

    /** @return list<string> every code listed in a manifest */
    private function manifestCodes(): array
    {
        $codes = [];
        foreach ($this->files->list('batch', 'BATCH-*.json') as $filename) {
            $manifest = json_decode((string) $this->files->read('batch', $filename), true);
            foreach (is_array($manifest['codes'] ?? null) ? $manifest['codes'] : [] as $entry) {
                if (is_string($entry['code'] ?? null)) $codes[] = $entry['code'];
            }
        }
        return $codes;
    }
}
