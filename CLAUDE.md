# SCUTOONS – project entry point

**Mandatory for every session except hotfixes: work on a GitHub issue** (mandymozart/scutoons, the board "SCUTOONS").
Start from the issue the owner names or create it before the first edit, name it in the commits (`Refs #N` /
`Fixes #N`), comment the result on it at the end. Open work never lives in docs or memory. How: RULES.md section C.

**Read at start, in this order, nothing else (≈ 250 lines):** the memory index → `agents/RULES.md` (the law book, whole) →
`agents/HANDOVER.md` (the current state). The map of everything else, with sizes: `agents/README.md`.

**Then only what you do:**
- Orchestrating, night shift, writing a brief: `agents/ORCHESTRATION.md` ("Brief template") + `C:\Development\agents\ORCHESTRATION.md`.
- Merging or touching deck / shared / viewer code, or auditing: `agents/AUDIT.md`.
- A decision's evidence: `grep -n '\[key\]' agents/DECISIONS.md` (the key is in the rule). Tool trap: `agents/GOTCHAS.md`.
- How a thing is: the topic doc of the area, only the part you touch (list in `agents/README.md` §2).
- `agents/research/` (line 1 = status) · `agents/archive/` (frozen, grep only).

Keep context small: grep and targeted reads, never whole files.
