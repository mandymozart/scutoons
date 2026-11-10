# The address book in the deck editor (#477): opens the editor on catpope's rig with one far world node selected
# (demo/address-book/far.rig.toml), so the inspector shows its address row. Click the address, type (for example "cat"):
# the typeahead lists what the toon's address book offers a deck (characters and items), a pick becomes the address's holder.
#   .\run-address-book-demo.ps1                 the book from the demo file (demo/address-book/book.json), no server needed
#   .\run-address-book-demo.ps1 -Toon catpope   the book from the API: GET <api>/addresses/catpope with $env:SCUTOONS_TOKEN
#                                               (-Api http://localhost:9000/api for the local API; default: the editor's)
#   -Frames N -Query cat                        a check run: types the query, writes target\address-book-demo.png and quits
# The editor writes its draft back, so the demo rig is copied to target\address-book-demo.rig.toml first.
# Needs: target\debug\deck-editor.exe (cargo build -p deck-editor).
# (ASCII only: Windows PowerShell 5.1 reads a script without a BOM as ANSI)
param([string]$Toon = '', [string]$Api = '', [int]$Frames = 0, [string]$Query = 'cat')
$editor = $PSScriptRoot
$deck = (Resolve-Path (Join-Path $editor '..')).Path
$exe = Join-Path $deck 'target\debug\deck-editor.exe'
if (-not (Test-Path $exe)) { Write-Error "missing $exe - run: cargo build -p deck-editor (in $deck)"; exit 1 }
$draft = Join-Path $deck 'target\address-book-demo.rig.toml'
Copy-Item (Join-Path $editor 'demo\address-book\far.rig.toml') $draft -Force
$a = @('--draft', $draft, '--select', 'drone_far', '--volume', '0')
if ($Toon -ne '') { $a += @('--toon', $Toon) } else { $a += @('--book', (Join-Path $editor 'demo\address-book\book.json')) }
if ($Api -ne '') { $a += @('--api', $Api) }
if ($Frames -gt 0) { $a += @('--far', $Query, '--frames', "$Frames", '--size', '1600,900', '--screenshot', (Join-Path $deck 'target\address-book-demo.png')) }
Push-Location $deck
try { & $exe @a } finally { Pop-Location }
