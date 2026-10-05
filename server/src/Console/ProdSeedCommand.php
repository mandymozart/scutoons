<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Infrastructure\Database\ProdSeed;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;

/**
 * Production's content + catalog into the local database (ProdSeed, docs/DATABASE.md "Production seed"):
 *   db:prod-seed                         load server/db/prod-seed.sql (no-op when that file is already loaded)
 *   db:prod-seed --from=<phpMyAdmin dump>  first write server/db/prod-seed.sql from a dump (only the allowed tables)
 *   --force                              load again even when unchanged (local edits of those tables are lost)
 * Never on production: it empties the content and catalog tables. `npm run dev:db` runs it after the migrations.
 */
#[AsCommand(name: 'db:prod-seed', description: 'Load production\'s content + catalog (server/db/prod-seed.sql) into the local database')]
final class ProdSeedCommand extends Command
{
    public const FILE = 'db/prod-seed.sql';

    public function __construct(private readonly Database $db, private readonly Settings $settings)
    {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addOption('from', null, InputOption::VALUE_REQUIRED, 'a phpMyAdmin SQL dump: extract the allowed tables into server/db/prod-seed.sql first')
            ->addOption('file', null, InputOption::VALUE_REQUIRED, 'the seed file', self::FILE)
            ->addOption('force', null, InputOption::VALUE_NONE, 'load even when this file is already loaded');
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $file = (string) $input->getOption('file');
        if (!preg_match('#^([A-Za-z]:)?[/\\\\]#', $file)) $file = $this->settings->root() . '/' . $file;
        $from = $input->getOption('from');
        if (is_string($from) && $from !== '') {
            $dump = @file_get_contents($from);
            if ($dump === false) {
                $output->writeln("<error>Cannot read $from</error>");
                return self::FAILURE;
            }
            ['statements' => $statements, 'skipped' => $skipped] = ProdSeed::extract($dump);
            @mkdir(dirname($file), 0777, true);
            file_put_contents($file, ProdSeed::render($statements, basename($from)));
            $output->writeln("Wrote $file (" . array_sum(array_map('count', $statements)) . ' INSERTs of ' . count(array_filter($statements)) . ' tables'
                . ($skipped !== [] ? '; left out: ' . implode(', ', array_keys($skipped)) : '') . ')');
        }
        $sql = is_file($file) ? (string) file_get_contents($file) : null;
        if ($sql === null) {
            $output->writeln(self::missing($file));
            return 2;
        }
        $result = ProdSeed::load($this->db, $sql, (bool) $input->getOption('force'));
        $output->writeln($result['loaded'] ? 'Loaded ' . basename($file) . ':' : basename($file) . ' is already loaded (unchanged; --force loads it again):');
        foreach ($result['rows'] as $table => $count) $output->writeln(sprintf('  %-20s %d', $table, $count));
        if ($result['skipped'] !== []) $output->writeln('  left out (not seed tables): ' . implode(', ', array_keys($result['skipped'])));
        return self::SUCCESS;
    }

    /** How to get the file (exit code 2 = missing) */
    public static function missing(string $file): string
    {
        return "No production seed at $file – the local database has only the migrations' starter rows.\n"
            . "Get it (docs/DATABASE.md \"Production seed\"): phpMyAdmin → database d0487143 → Export → Custom → SQL, data only,\n"
            . "tables " . implode(', ', ProdSeed::TABLES) . " → save as server/db/prod-seed.sql\n"
            . "(or any full export: npm run dev:db -- seed --from=<dump.sql> keeps only those tables), then npm run dev:db again.";
    }
}
