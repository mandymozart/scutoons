# JSDoc + TypeScript Exports Explained

## Why `export` is Required

When using JSDoc with `@typedef {import('./types.d.ts').TypeName}`, TypeScript treats the `.d.ts` file as a **module**, not ambient declarations.

### Without `export` (Ambient)
```typescript
// types.d.ts
interface Chapter { }  // ❌ Cannot import
```

This creates **global/ambient** types. They're available everywhere without imports, but:
- ❌ Cannot use `import()` syntax in JSDoc
- ❌ No module isolation
- ❌ Pollutes global namespace

### With `export` (Module)
```typescript
// types.d.ts
export interface Chapter { }  // ✅ Can import
```

This creates **module exports**:
- ✅ Can use `import()` in JSDoc
- ✅ Module-scoped (clean)
- ✅ Works with IntelliSense

---

## How to Use in JavaScript

### 1. Import Types with JSDoc
```javascript
/**
 * @typedef {import('./types.d.ts').Chapter} Chapter
 * @typedef {import('./types.d.ts').Group} Group
 */
```

**Result:** VS Code now knows what `Chapter` and `Group` are!

### 2. Use Types in JSDoc Comments
```javascript
/**
 * Load a chapter
 * @param {string} lang - Language code
 * @param {number} chapterNumber - Chapter number
 * @returns {Promise<Chapter>}
 */
async function loadChapter(lang, chapterNumber) {
  // Full autocomplete for Chapter!
  const data = await fetch(`/api/${lang}/chapter${chapterNumber}.json`);
  return data.json();
}
```

### 3. Type Function Parameters
```javascript
/**
 * Process groups
 * @param {Group[]} groups - Array of groups
 * @param {string} lang - Language code
 * @returns {Promise<void>}
 */
async function processGroups(groups, lang) {
  groups.forEach(group => {
    console.log(group.label);     // ✅ Autocomplete!
    console.log(group.renderer);  // ✅ Type checking!
  });
}
```

### 4. Type Variables
```javascript
/** @type {Group[]} */
const groups = [];

groups.push({
  label: 'intro',
  renderer: 'vertical',  // ✅ Validated!
  slices: []
});
```

---

## TypeScript Checker Warnings

You may see this warning:
```
A declaration file cannot be imported without 'import type'.
Did you mean to import an implementation file '../types.ts' instead?
```

**This is EXPECTED and can be ignored.** 

### Why?
- TypeScript's strict checker prefers `import type` syntax
- But JSDoc uses `import()` in comments, which works differently
- The warning is pedantic - IntelliSense still works perfectly

### What Works
✅ IntelliSense/autocomplete  
✅ Type checking in IDE  
✅ Go to definition  
✅ Hover documentation  
✅ Refactoring support  

---

## Export Everything

In `types.d.ts`, **all** types/interfaces/enums must be exported:

```typescript
// ✅ CORRECT
export enum Renderer {
  VERTICAL = "vertical",
  HORIZONTAL = "horizontal",
}

export interface Chapter {
  title: string;
  groups: Group[];
}

export interface Group {
  label: string;
  renderer: Renderer;
  slices: Slice[];
}

// ❌ WRONG - cannot import
interface Slice {
  image: string;
}
```

---

## Benefits

### Pure JavaScript
- No TypeScript compilation
- No build step changes
- Works in any JS file

### Full Type Safety
- Catch errors before runtime
- IntelliSense everywhere
- Self-documenting code

### IDE Integration
- Works in VS Code
- Works in WebStorm
- Works in any LSP-compatible editor

---

## Summary

**Q: Do I need `export` for JSDoc?**  
**A: YES.** Without `export`, you cannot use `import()` syntax in JSDoc.

**Q: What about the TypeScript warnings?**  
**A: Ignore them.** They're expected when using JSDoc with `.d.ts` files. IntelliSense works perfectly.

**Q: Do I need to compile TypeScript?**  
**A: NO.** This is pure JavaScript with type hints via JSDoc.

---

## Real Example from SCUTOONS

```javascript
// app/utils/GroupUtils.js
/**
 * @typedef {import('../types.d.ts').Group} Group
 * @typedef {import('../types.d.ts').GroupRef} GroupRef
 */

/**
 * Populate groups by loading external group files
 * @param {(Group|GroupRef)[]} groups - Array of groups or refs
 * @param {string} lang - Language code
 * @param {number} chapterNumber - Chapter number
 * @returns {Promise<Group[]>} Array of populated groups
 */
export async function populateGroups(groups, lang, chapterNumber) {
  const populatedGroups = [];
  
  for (const item of groups) {
    if (item.src) {  // ✅ TypeScript knows this is GroupRef
      const groupData = await loadJSON(`api/static/catpope/${lang}/${item.src}`);
      populatedGroups.push(groupData);
    } else {  // ✅ TypeScript knows this is Group
      populatedGroups.push(item);
    }
  }
  
  return populatedGroups;
}
```

**Result:**
- ✅ `item.src` autocomplete when inside `if`
- ✅ `item.label`, `item.renderer` autocomplete in `else`
- ✅ Return type validated as `Group[]`
- ✅ No TypeScript compilation needed!

---

This is how modern JavaScript projects get TypeScript-level safety without the build complexity!
