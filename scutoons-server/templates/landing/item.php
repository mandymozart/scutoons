<?php
/**
 * One item row: icon (cut-out PNG from the CDN, first letter as fallback) + name + variant / edition lines.
 * @var callable $e  @var string $name  @var ?string $icon  @var list<string> $lines  @var bool $highlight
 */
$fallback = (string) json_encode(mb_substr($name, 0, 1));
?>
<div class="item<?= $highlight ? ' single' : '' ?>"><span class="icon"><?php if ($icon !== null): ?><img src="<?= $e($icon) ?>" alt="" loading="lazy" onerror="this.replaceWith(Object.assign(document.createElement('span'),{className:'fallback',textContent:<?= $e($fallback) ?>}))"><?php endif ?></span><span><span class="item-name"><?= $e($name) ?></span><?php foreach ($lines as $line): ?><span class="item-meta caption"><?= $e($line) ?></span><?php endforeach ?></span></div>
