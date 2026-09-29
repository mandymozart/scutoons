<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\Admin\Batches;
use Scutoons\Domain\Catalog\Editions;
use Scutoons\Infrastructure\Database\Database;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\ConsoleOutputInterface;
use Symfony\Component\Console\Output\OutputInterface;

#[AsCommand(name: 'report-orphan-codes', description: 'READ-ONLY: edition codes that are in no batch manifest')]
final class ReportOrphanCodesCommand extends Command
{
    public function __construct(private readonly Database $db, private readonly Batches $batches, private readonly Editions $editions)
    {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addOption('csv', null, InputOption::VALUE_NONE, 'CSV (code, edition_id, signed edition number, created_at, claimed)')
            ->addOption('manifests', null, InputOption::VALUE_REQUIRED, 'Another manifest folder; default FILES_PATH/batch')
            ->setHelp('Never changes or deletes anything. Background: earlier batch runs inserted codes whose manifest was '
                . 'overwritten or never written (locally 37 Dago and 20 beetles duplicates). Decide per code – they may be printed.');
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $errors = $output instanceof ConsoleOutputInterface ? $output->getErrorOutput() : $output;
        $dir = $input->getOption('manifests');
        $manifests = $this->batches->manifestIndex(is_string($dir) ? $dir : null, fn(string $line) => $errors->writeln($line));
        $rows = $this->db->all("SELECT code, edition_id, created_at, email, claimed_by, signature FROM qr
                                WHERE edition_id IS NOT NULL AND edition_id <> '' ORDER BY edition_id, created_at, code");
        $counts = [];
        foreach ($this->editions->all() as $edition) $counts[(string) $edition['id']] = (int) ($edition['edition_count'] ?? 0);

        $summary = [];
        $orphans = [];
        foreach ($rows as $row) {
            $edition = (string) $row['edition_id'];
            $summary[$edition] ??= ['in_db' => 0, 'in_manifest' => 0, 'orphans' => 0, 'edition_count' => $counts[$edition] ?? null];
            $summary[$edition]['in_db']++;
            if (isset($manifests[$row['code']])) {
                $summary[$edition]['in_manifest']++;
                continue;
            }
            $summary[$edition]['orphans']++;
            // The signed payload carries the number printed on the certificate
            $signed = null;
            $signature = json_decode((string) $row['signature'], true);
            if (is_array($signature) && isset($signature['signedData'])) {
                $data = json_decode((string) $signature['signedData'], true);
                if (is_array($data) && isset($data['edition_number'])) $signed = (int) $data['edition_number'];
            }
            $orphans[] = ['code' => $row['code'], 'edition_id' => $edition, 'signed_number' => $signed, 'created_at' => $row['created_at'],
                'claimed' => ($row['email'] !== '' && $row['email'] !== null) || $row['claimed_by'] !== null];
        }

        if ($input->getOption('csv')) {
            $out = fopen('php://memory', 'w+');
            if ($out === false) return self::FAILURE;
            fputcsv($out, ['code', 'edition_id', 'signed_edition_number', 'created_at', 'claimed'], ',', '"', '\\');
            foreach ($orphans as $o) fputcsv($out, [$o['code'], $o['edition_id'], $o['signed_number'], $o['created_at'], $o['claimed'] ? 'yes' : 'no'], ',', '"', '\\');
            rewind($out);
            $output->write((string) stream_get_contents($out));
            return self::SUCCESS;
        }

        $output->writeln('Codes per edition (DB vs. batch manifests):');
        foreach ($summary as $edition => $s) {
            $output->writeln(sprintf('  %-20s db=%4d  in manifest=%4d  orphans=%4d  edition_count=%s',
                $edition, $s['in_db'], $s['in_manifest'], $s['orphans'], $s['edition_count'] ?? '?'));
        }
        $output->writeln("\nOrphan codes (in no manifest) – nothing was changed:");
        foreach ($orphans as $o) {
            $output->writeln(sprintf('  %-8s  %-20s  signed #%-4s  created %s%s',
                $o['code'], $o['edition_id'], $o['signed_number'] ?? '?', $o['created_at'], $o['claimed'] ? '  CLAIMED' : ''));
        }
        $output->writeln("\n" . count($orphans) . ' orphan code(s).');
        return self::SUCCESS;
    }
}
