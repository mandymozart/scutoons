<?php
/**
 * A claimable code: preview of what it contains + the actions (agents/ITEMS.md "Ownership, claims, gifting"):
 * - not registered yet → "Add to my account" first, with "Register it now – otherwise someone else can" (the
 *   registration strategy: the moment of the first scan), then Continue (read on this device);
 * - registered to an account → Read (Continue) + "This is mine? File a claim" (the viewer's claim form). Reading with
 *   someone else's code is fine – the page never says whose it is.
 * The viewer claims it, asks for the item choice when needed and files ownership claims.
 * @var callable $e  @var \Qr\View $view  @var array<string, string> $t
 * @var string $heading  @var ?string $hint  @var bool $hintCaption  @var list<array<string, mixed>> $rows (item partial vars)
 * @var ?string $description  @var string $continueUrl  @var string $accountUrl  @var bool $claimed  @var string $claimUrl
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
<?php if ($claimed): ?>
<p class="caption" id="registered"><?= $e($t['registered']) ?></p>
<a class="btn" id="continue" href="<?= $e($continueUrl) ?>"><?= $e($t['read']) ?></a><p class="caption"><?= $e($t['continue_hint']) ?></p>
<a class="btn btn-secondary" id="file-claim" href="<?= $e($claimUrl) ?>"><?= $e($t['file_claim']) ?></a><p class="caption"><?= $e($t['file_claim_hint']) ?></p>
<?php else: ?>
<p class="warn" id="register-now"><?= $e($t['register_now']) ?></p>
<a class="btn" id="add-account" href="<?= $e($accountUrl) ?>"><?= $e($t['add_account']) ?></a><p class="caption"><?= $e($t['add_account_hint']) ?></p>
<a class="btn btn-secondary" id="continue" href="<?= $e($continueUrl) ?>"><?= $e($t['continue']) ?></a><p class="caption"><?= $e($t['continue_hint']) ?></p>
<?php endif ?>
</div></div>
