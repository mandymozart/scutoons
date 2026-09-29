<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Infrastructure\Database\MigrationRunner;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;

#[AsCommand(name: 'migrate', description: 'Apply pending migrations (migrations/<driver>/) to the configured database')]
final class MigrateCommand extends Command
{
    public function __construct(private readonly Database $db, private readonly Settings $settings)
    {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addOption('status', null, InputOption::VALUE_NONE, 'Only list the migrations and whether they are applied')
            ->addOption('db', null, InputOption::VALUE_REQUIRED, 'Another SQLite file instead of the configured database')
            ->setHelp(<<<'HELP'
                Applies the dated migrations that match the database driver (*.sqlite.sql for SQLite, *.mysql.sql for
                MySQL/MariaDB) and records them in schema_migrations (same names as before, nothing re-runs). A fresh
                database first gets migrations/schema/qr.<driver>.sql.

                Production (shared host, no shell): run the pending *.mysql.sql files in phpMyAdmin – they are idempotent.
                HELP);
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $file = $input->getOption('db');
        $db = is_string($file) ? Database::sqlite($file) : $this->db;
        $runner = new MigrationRunner($db, $this->settings->root() . '/migrations');
        $output->writeln('Database driver: ' . $db->driver());
        if ($input->getOption('status')) {
            if ($runner->needsBaseSchema()) $output->writeln('  pending  base schema (no qr table)');
            foreach ($runner->status() as $name => $appliedAt) {
                $output->writeln(($appliedAt !== null ? '  applied  ' : '  pending  ') . $name);
            }
            return self::SUCCESS;
        }
        $applied = $runner->migrate(fn(string $line) => $output->writeln($line));
        $output->writeln($applied ? count($applied) . ' migration(s) applied.' : 'Nothing to migrate.');
        return self::SUCCESS;
    }
}
