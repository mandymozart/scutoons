<?php
/**
 * A claimable code: preview of what it contains + Continue + Add to my account. The viewer claims it, asks for the
 * item choice when needed and decides what to do when the code already belongs to someone.
 * @var callable $e  @var \Scutoons\Infrastructure\Templates\Templates $view  @var array<string, string> $t
 * @var string $heading  @var ?string $hint  @var bool $hintCaption  @var list<array<string, mixed>> $rows (item partial vars)
 * @var ?string $description  @var string $continueUrl  @var string $accountUrl
 */
?>
<div class="panel">
<h1><?= $e($heading) ?></h1>
<?php if ($hint !== null): ?>
<p<?= $hintCaption ? ' class="caption"' : '' ?>><?= $e($hint) ?></p>
<?php endif ?>
<?php if ($rows): ?>
<div class="items">
<?php foreach ($rows as $row): ?>
<?= $view->render('landing/item', $row) ?>
<?php endforeach ?>
</div>
<?php endif ?>
<?php if ($description !== null): ?>
<p class="item-desc"><?= $e($description) ?></p>
<?php endif ?>
<div class="actions">
<a class="btn" id="continue" href="<?= $e($continueUrl) ?>"><?= $e($t['continue']) ?></a><p class="caption"><?= $e($t['continue_hint']) ?></p>
<a class="btn btn-secondary" id="add-account" href="<?= $e($accountUrl) ?>"><?= $e($t['add_account']) ?></a><p class="caption"><?= $e($t['add_account_hint']) ?></p>
</div></div>
