Status: live 2026-10-15 – the grants desk now lives in Proton Drive: `Projects/Scutoons/Grants/` (CLAUDE.md, agents/ RULES, REGISTER, FUNDERS, DECISIONS, PLAN, Kit/APPLICANT.md). That desk is the truth; this file is only the pointer and the original concept

# Grants – funding applications for the toons

Owner 2026-10-15: "we will need to apply for our toons to get funding for individual albums or performances or
chapters … MA7, BMWKMS, film funding and others perhaps EU". The documents live in Proton Drive, not in this repo:
`C:\Users\Nutzer\Proton Drive\porschuetz\My files\Projects\` (contains personal data, passports and budgets).

## 1. Folders (Proton Drive)

One rule: **`<Project>/<Funder>/<Programme> <YYYY-N>/`**. "Cat Pope Yellow Mouse" already follows it. N = the
funder's round within the year (BMWKMS: 1 = 15 March, 2 = 15 October).

```
Projects/
  Scutoons/Grants/                 ← the cross-toon desk
    REGISTER.md                    one line per application, all toons (status, amount, GZ, deadlines)
    CALENDAR.md                    funders and their deadlines, checked once a year
    Kit/                           reusable parts: bios (DE/EN), CVs, portfolio, press, work-sample links, ID scans
  Cat Pope Yellow Mouse/
    BMWKMS/Projektförderung Musik 2026-2/
      STATUS.md                    card: funder, deadline, amount, period, status, GZ, contact, attachment checklist, log
      Beilage-*.md                 texts as source (character counts) → exported as PDF for the upload
      submitted/                   exactly what went out + receipt (Empfangsbestätigung)
      decision/                    Bescheid, correspondence
      report/                      proof of use (Abrechnung / Verwendungsnachweis) – own deadline!
    MA7/Darstellende Künste 2026-2/ …
```

To align later (move only with the owner's OK): `Cat Pope/Proposals/*` and `Scutoons/Proposals/*` →
`<Project>/<Funder>/<Programme> <YYYY-N>/`; `Mandy Mozart/MA7/Komposition` → add the year.

## 2. Workflow per application

1. **Scout**: a call fits a toon's work (album, performance, chapter, film). Enter it in CALENDAR / REGISTER.
2. **Fit check**: eligibility (person or legal entity, residence), the project hasn't started yet, the same project
   hasn't already been rejected by this funder, overlaps with other funders.
3. **Open**: create the folder + STATUS.md; save the call's guidelines and form template in it (they change).
4. **Draft**: texts as .md with character limits; reuse Kit/ and earlier applications; Claude drafts them.
5. **Money check**: the financing plan must be the same in every linked application (MA 7 names BMWKMS and the
   other way round); the "funding in the last 3 years" fields come from REGISTER.
6. **Owner review → owner submits**: Claude never submits (government login, signature, sending on the owner's behalf).
7. **Archive**: what was submitted + receipt → `submitted/`; GZ into STATUS + REGISTER.
8. **Follow up**: queries (Rückfragen), corrections, decision → `decision/`; on approval, enter the report deadline.
9. **Report**: proof of use with the same budget form (MA 7: plan/actual); missing reports block new grants.

## 3. Funders (deadlines are checked in CALENDAR, not trusted from here)

- **Stadt Wien MA 7**: Darstellende Kunst (free groups), Musik individual grants, composition.
- **BMWKMS (Bund)**: Projektförderung Musik/Musiktheater (15 Oct for projects from January, 15 Mar from July),
  composition grants (15 Sep / 15 Apr), start-up and project scholarships.
- **Film / animation** (webtoon → film): Filmfonds Wien, ÖFI, FISA+, ORF/Fernsehfonds – to research.
- **Music**: Österreichischer Musikfonds (album production), SKE Fonds (AKM), Austrian Music Export – to research.
- **EU**: Creative Europe (Culture – cooperation projects with partners in other countries; MEDIA – animation,
  games) – needs partners, longer lead time.
- Residencies / international: e.g. Providenza (Corsica), MMCA (Seoul) – already used.

## 4. In SCUTOONS later (only after the owner's OK)

- A Claude skill `grant-application` (workflow steps 3–7 as a checklist) that works on the Drive folders.
- Perhaps later: an application as an entity in SCUTOONS linked to toon / season / chapter / album, so that the
  REGISTER is generated and the reports can use reading/event stats (`reading_stats`, `list_events`).
