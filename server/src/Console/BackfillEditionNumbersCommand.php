<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\Admin\Batches;
use Scutoons\Infrastructure\Database\Database;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\ConsoleOutputInterface;
use Symfony\Component\Console\Output\OutputInterface;

#[AsCommand(name: 'backfill-edition-numbers', description: 'Fill qr.edition_number from the batch manifests (codes[].n)')]
final class BackfillEditionNumbersCommand extends Command
{
    public function __construct(private readonly Database $db, private readonly Batches $batches)
    {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addOption('dry-run', null, InputOption::VALUE_NONE, 'Only print what would change')
            ->addOption('sql', null, InputOption::VALUE_NONE, 'Print UPDATE statements (production: run them in phpMyAdmin); no database needed')
            ->addOption('force', null, InputOption::VALUE_NONE, 'Also overwrite existing numbers')
            ->addOption('manifests', null, InputOption::VALUE_REQUIRED, 'Another manifest folder (e.g. a copy from the server); default FILES_PATH/batch')
            ->setHelp('Only codes listed in a manifest get a number; codes in no manifest stay NULL (see report-orphan-codes). '
                . 'Existing non-NULL numbers are left alone unless --force.');
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $errors = $output instanceof ConsoleOutputInterface ? $output->getErrorOutput() : $output;
        $dir = $input->getOption('manifests');
        $manifests = $this->batches->manifestIndex(is_string($dir) ? $dir : null, fn(string $line) => $errors->writeln($line));
        $force = (bool) $input->getOption('force');

        if ($input->getOption('sql')) {
            // Statements for production (codes come from the manifests, never user input)
            $output->writeln('-- qr.edition_number backfill from ' . count($manifests) . ' manifest codes, generated ' . gmdate('c'));
            foreach ($manifests as $code => $info) {
                $quoted = "'" . str_replace("'", "''", (string) $code) . "'";
                $edition = "'" . str_replace("'", "''", $info['edition_id']) . "'";
                $output->writeln("UPDATE `qr` SET `edition_number` = {$info['n']} WHERE `code` = $quoted AND `edition_id` = $edition"
                    . ($force ? '' : ' AND `edition_number` IS NULL') . ';');
            }
            return self::SUCCESS;
        }

        $dryRun = (bool) $input->getOption('dry-run');
        $stats = ['updated' => 0, 'unchanged' => 0, 'kept' => 0, 'missing_in_db' => 0, 'edition_mismatch' => 0];
        $pdo = $this->db->pdo();
        $pdo->beginTransaction();
        foreach ($manifests as $code => $info) {
            $row = $this->db->one('SELECT * FROM qr WHERE code = ?', [$code]);
            if (!$row) {
                $stats['missing_in_db']++;
                $output->writeln("not in db: $code ({$info['file']})");
                continue;
            }
            // the batch it was made in (qr.batch, migration 2026-10-14-segments-batches) – kept when set
            if (!$dryRun && array_key_exists('batch', $row) && $row['batch'] === null) $this->db->run('UPDATE qr SET batch = ? WHERE code = ? AND batch IS NULL', [$info['file'], $code]);
            if ((string) $row['edition_id'] !== $info['edition_id']) {
                $stats['edition_mismatch']++;
                $output->writeln("edition mismatch: $code db={$row['edition_id']} manifest={$info['edition_id']} – skipped");
                continue;
            }
            if ($row['edition_number'] !== null && !$force) {
                $stats[(int) $row['edition_number'] === $info['n'] ? 'unchanged' : 'kept']++;
                continue;
            }
            if (!$dryRun) $this->db->run('UPDATE qr SET edition_number = ? WHERE code = ?', [$info['n'], $code]);
            $stats['updated']++;
        }
        $dryRun ? $pdo->rollBack() : $pdo->commit();
        $output->writeln(($dryRun ? '[dry run] ' : '') . json_encode($stats));
        return self::SUCCESS;
    }
}
