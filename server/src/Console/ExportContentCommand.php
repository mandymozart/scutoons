<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\Content\ContentService;
use Scutoons\Domain\Library\Library;
use Scutoons\Infrastructure\Database\Database;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputArgument;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;

#[AsCommand(name: 'export-content', description: 'Write the content store back into the file layout (the import/export format)')]
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
            ->addOption('toon', null, InputOption::VALUE_REQUIRED, 'Only this toon (or scutoons)')
            ->addOption('draft', null, InputOption::VALUE_NONE, 'The newest version of every file instead of the published one')
            ->addOption('db', null, InputOption::VALUE_REQUIRED, 'Read this SQLite file instead of the configured database')
            ->setHelp(<<<'HELP'
                The reverse of import-content (agents/DATA.md §4): every published document (or, with --draft, the newest
                version) as <dir>/<toon>/<locale>/<path>; the toon index entries <toon>/_/index.json together as <dir>/toons.json
                (creation order) and scutoons/_/events.json as <dir>/events.json. `import-content <dir>` reads the result
                back. Use it as a backup, to diff the store against files, or to seed another environment.
                HELP);
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $dir = rtrim(str_replace('\\', '/', (string) $input->getArgument('dir')), '/');
        $draft = (bool) $input->getOption('draft');
        $onlyToon = $input->getOption('toon');
        $db = is_string($input->getOption('db')) ? Database::sqlite((string) $input->getOption('db')) : $this->db;

        $rows = $draft
            ? $db->all('SELECT v.toon, v.locale, v.path, v.json FROM content_versions v JOIN (SELECT MAX(id) AS id, MIN(id) AS first FROM content_versions GROUP BY toon, locale, path) m ON m.id = v.id ORDER BY m.first')
            : $db->all("SELECT v.toon, v.locale, v.path, v.json FROM content_versions v JOIN (SELECT toon, locale, path, MIN(id) AS first FROM content_versions GROUP BY toon, locale, path) m ON m.toon = v.toon AND m.locale = v.locale AND m.path = v.path WHERE v.status = 'published' ORDER BY m.first, v.id");
        $files = [];
        $index = [];
        foreach ($rows as $row) {
            [$toon, $locale, $path] = [(string) $row['toon'], (string) $row['locale'], (string) $row['path']];
            if (is_string($onlyToon) && $toon !== $onlyToon) continue;
            $doc = json_decode((string) $row['json']);
            if ($locale === ContentService::NEUTRAL_LOCALE && $path === 'index.json') $index[$toon] = $doc;
            elseif ($toon === Library::PLATFORM && $locale === ContentService::NEUTRAL_LOCALE && $path === 'events.json') $files['events.json'] = $doc;
            else $files["$toon/$locale/$path"] = $doc;
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
