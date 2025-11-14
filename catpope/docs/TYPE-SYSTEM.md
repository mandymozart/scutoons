# Type System Documentation

## Overview

The SCUTOONS app uses **JSDoc with TypeScript definitions** for type checking and IntelliSense in VS Code without requiring TypeScript compilation.

## File Structure

```
catpope/
├── jsconfig.json              # VS Code type checking configuration
├── app/
│   ├── types.d.ts            # Main type definitions
│   └── **/*.js               # JavaScript files with JSDoc
```

---

## How It Works

### 1. Type Definitions (`app/types.d.ts`)

All primary types are defined here using TypeScript syntax:

```typescript
interface Chapter {
  title: string;
  author: string;
  number: number;
  language: Language;
  renderer?: Renderer;
  groups: Group[];
}

interface Group {
  label?: string;
  renderer?: Renderer;
  interaction?: Interaction;
  slices?: Slice[];
}
```

### 2. Using Types in JavaScript

Use JSDoc comments to import and apply types:

```javascript
/**
 * @typedef {import('./types.d.ts').Chapter} Chapter
 * @typedef {import('./types.d.ts').Group} Group
 */

/**
 * Load a chapter
 * @param {string} lang - Language code
 * @param {number} chapterNumber - Chapter number
 * @returns {Promise<Chapter>}
 */
async function loadChapter(lang, chapterNumber) {
  // VS Code now knows the return type is Chapter
  const data = await fetch(`/api/chapters/${lang}/${chapterNumber}.json`);
  return data.json();
}
```

### 3. Function Parameters

```javascript
/**
 * @param {Group[]} groups - Array of groups
 * @param {string} lang - Language code
 */
function processGroups(groups, lang) {
  // VS Code provides autocomplete for group properties
  groups.forEach(group => {
    console.log(group.label);     // ✓ autocomplete works
    console.log(group.renderer);  // ✓ autocomplete works
  });
}
```

### 4. Variable Types

```javascript
/**
 * @typedef {import('./types.d.ts').Group} Group
 */

/** @type {Group[]} */
const groups = [];

// Now VS Code knows groups is an array of Group
groups.push({
  label: 'intro',
  slices: []  // ✓ type checking works
});
```

---

## Configuration

### `jsconfig.json`

This file enables type checking app-wide:

```json
{
  "compilerOptions": {
    "checkJs": true,           // Enable type checking in JS files
    "allowJs": true,           // Allow JavaScript files
    "strictNullChecks": true,  // Strict null checking
    "target": "ES2022"         // Modern JavaScript features
  },
  "include": [
    "app/**/*",
    "api/**/*"
  ]
}
```

**Key Settings:**
- `checkJs: true` - Enables type checking for JavaScript
- `strictNullChecks: true` - Catches null/undefined errors
- No TypeScript compilation required!

---

## Type Definitions Reference

### Core Types

**Enums:**
```typescript
enum Renderer {
  VERTICAL = "vertical",
  HORIZONTAL = "horizontal",
}

enum Language {
  DE = "de",
  EN = "en",
  JP = "jp",
  KR = "kr",
  ES = "es",
}

enum Interaction {
  DEFAULT = "webtoon",
  VISUAL_NOVEL = "visual-novel",
  STROLL = "stroll",
}
```

**Interfaces:**
```typescript
interface Chapter {
  title: string;
  author: string;
  number: number;
  language: Language;
  release: string;
  status: Status;
  renderer?: Renderer;
  groups: Group[];
  audio?: AudioConfiguration;
}

interface Group {
  label?: string;
  renderer?: Renderer;
  interaction?: Interaction;
  slices?: Slice[];
}

interface GroupRef {
  src: string;  // Path to external group JSON file
}

interface Slice {
  image: string;
  audio?: AudioEvents;
  dialog?: Dialog[];
  onomatopoeia?: Onomatopoeia[];
}
```

---

## Best Practices

### ✅ DO

**Import types at the top:**
```javascript
/**
 * @typedef {import('../types.d.ts').Chapter} Chapter
 * @typedef {import('../types.d.ts').Group} Group
 */
```

**Document function signatures:**
```javascript
/**
 * Process groups
 * @param {Group[]} groups - Array of groups
 * @param {string} lang - Language code
 * @returns {Promise<Group[]>}
 */
async function processGroups(groups, lang) {
  // ...
}
```

**Use type assertions when needed:**
```javascript
/** @type {Chapter} */
const chapter = await loadChapter('de', 1);
```

### ❌ DON'T

**Don't skip type imports:**
```javascript
// ❌ BAD - no type checking
function processGroups(groups, lang) {
  // ...
}

// ✓ GOOD - with types
/**
 * @param {Group[]} groups
 */
function processGroups(groups, lang) {
  // ...
}
```

**Don't use `any` unnecessarily:**
```javascript
// ❌ BAD
/** @type {any} */
const data = loadData();

// ✓ GOOD
/** @type {Chapter} */
const data = loadData();
```

---

## Common Patterns

### Loading External Groups

```javascript
/**
 * @typedef {import('../types.d.ts').Group} Group
 * @typedef {import('../types.d.ts').GroupRef} GroupRef
 */

/**
 * @param {(Group|GroupRef)[]} groups
 * @returns {Promise<Group[]>}
 */
async function populateGroups(groups) {
  const result = [];
  for (const item of groups) {
    if (item.src) {
      // GroupRef - load from file
      const group = await loadJSON(item.src);
      result.push(group);
    } else {
      // Inline Group
      result.push(item);
    }
  }
  return result;
}
```

### Type Guards

```javascript
/**
 * Check if group has interaction
 * @param {Group} group
 * @returns {boolean}
 */
function isInteractiveGroup(group) {
  return Boolean(group.interaction);
}
```

### Generic Functions

```javascript
/**
 * @template T
 * @param {T[]} items
 * @param {(item: T) => boolean} predicate
 * @returns {T|undefined}
 */
function findItem(items, predicate) {
  return items.find(predicate);
}
```

---

## IDE Benefits

With this setup, VS Code provides:

✅ **Autocomplete** - Property suggestions  
✅ **Type checking** - Catch errors before runtime  
✅ **Inline documentation** - JSDoc comments on hover  
✅ **Go to definition** - Jump to type definitions  
✅ **Refactoring** - Rename symbols safely  
✅ **Error highlighting** - See type errors inline  

---

## Updating Types

When adding new types, update `app/types.d.ts`:

```typescript
// Add new enum
enum NewEnum {
  VALUE1 = "value1",
  VALUE2 = "value2",
}

// Add new interface
interface NewInterface {
  id: string;
  name: string;
}

// Extend existing interface (if needed, use intersection)
type ExtendedChapter = Chapter & {
  customField: string;
};
```

Then use it in your JavaScript:

```javascript
/**
 * @typedef {import('./types.d.ts').NewInterface} NewInterface
 */

/** @type {NewInterface} */
const item = {
  id: '123',
  name: 'Test'
};
```

---

## Troubleshooting

### Types not recognized?

1. Restart VS Code
2. Check `jsconfig.json` includes your files
3. Verify type import path is correct

### Autocomplete not working?

1. Ensure `checkJs: true` in `jsconfig.json`
2. Verify JSDoc syntax is correct
3. Check that types are exported from `.d.ts`

### False positives?

Use type assertions or `@ts-ignore`:

```javascript
// Specific ignore
// @ts-ignore - legacy code, will fix later
const value = legacyFunction();

// Or use type assertion
/** @type {Chapter} */
const chapter = (unsafeData);
```

---

## Summary

- **No TypeScript compilation** - Pure JavaScript with type hints
- **Full IDE support** - IntelliSense, autocomplete, type checking
- **Single source of truth** - All types in `app/types.d.ts`
- **Gradual adoption** - Add types where needed
- **Standard JSDoc** - Compatible with all editors

This gives you TypeScript-level type safety without the build complexity!
