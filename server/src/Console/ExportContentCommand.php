<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\Content\EntityKey;
use Scutoons\Infrastructure\Database\Database;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputArgument;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;

#[AsCommand(name: 'export-content', description: 'Write the content store into the interchange format (JSON folder layout)')]
final class ExportContentCommand extends Command
{
    private const FLAGS = JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION;

    public function __construct(private readonly Database $db)
    {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addArgument('dir', InputArgument::REQUIRED, 'Target folder (created; existing files are overwritten)')
            ->addOption('toon', null, InputOption::VALUE_REQUIRED, 'Only this toon (its entities, without the platform-wide ones)')
            ->addOption('draft', null, InputOption::VALUE_NONE, 'The newest version of every entity instead of the published one')
            ->addOption('db', null, InputOption::VALUE_REQUIRED, 'Read this SQLite file instead of the configured database')
            ->setHelp(<<<'HELP'
                The reverse of import-content (agents/DATA.md §4, Console\ContentFiles): every published entity (or, with
                --draft, the newest version) as a file of the interchange layout; the toon index entries together as
                toons.json (creation order). `import-content <dir>` reads the result back. Use it as a backup, to diff, or to
                refresh the seed (shared/content/seed) for new environments and the production SQL.
                HELP);
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $dir = rtrim(str_replace('\\', '/', (string) $input->getArgument('dir')), '/');
        $draft = (bool) $input->getOption('draft');
        $onlyToon = $input->getOption('toon');
        $db = is_string($input->getOption('db')) ? Database::sqlite((string) $input->getOption('db')) : $this->db;

        $rows = $draft
            ? $db->all('SELECT v.entity_key, v.json FROM entity_versions v JOIN (SELECT MAX(id) AS id, MIN(id) AS first FROM entity_versions GROUP BY entity_key) m ON m.id = v.id ORDER BY m.first')
            : $db->all("SELECT v.entity_key, v.json FROM entity_versions v JOIN (SELECT entity_key, MIN(id) AS first FROM entity_versions GROUP BY entity_key) m ON m.entity_key = v.entity_key WHERE v.status = 'published' ORDER BY m.first, v.id");
        $files = [];
        $index = [];
        foreach ($rows as $row) {
            if (!EntityKey::isValid($row['entity_key'])) continue; // rows of kinds that no longer exist
            $key = EntityKey::parse($row['entity_key']);
            if (is_string($onlyToon) && $key->toon !== $onlyToon) continue;
            $doc = json_decode((string) $row['json']);
            if ($key->kind === 'toon') $index[(string) $key->toon] = $doc;
            else $files[ContentFiles::pathOf($key)] = $doc;
        }
        if ($index !== []) $files['toons.json'] = array_values($index);

        foreach ($files as $relative => $doc) {
            $target = "$dir/$relative";
            if (!is_dir(dirname($target))) mkdir(dirname($target), 0775, true);
            file_put_contents($target, json_encode($doc, self::FLAGS) . "\n");
            $output->writeln("  wrote $relative");
        }
        $output->writeln(sprintf("\n%d files (%s) → %s", count($files), $draft ? 'newest versions' : 'published', $dir));
        return self::SUCCESS;
    }
}
