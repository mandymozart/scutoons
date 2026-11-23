/**
 * Entry categories – singular, the category of one entry. Defined once for the content build
 * (`scripts/`, schema + guard) and the app (consultation mode). The values are what authors write
 * in `content/entries/<id>/entry.yaml`.
 * Labels are derived from the values in the app (`categoryLabel`: "Videos"; the glossary is labelled "Marginalia").
 */
export enum EntryCategory {
  Glossary = "glossary",
  Video = "video",
  Text = "text",
  Link = "link",
}

/** All categories in menu order */
export const ENTRY_CATEGORIES: readonly EntryCategory[] = Object.values(EntryCategory);
