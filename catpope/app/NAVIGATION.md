# Navigation System

## Architecture

The navigation system has been modularized into three clean layers:

### 1. **NavigationStore** (`stores/NavigationStore.js`)
Simple state storage only. No routing logic.

```javascript
// Stores current navigation state
currentView = 'toon'
currentChapter = 1
currentSong = 1

// Methods
setView(view, params)  // Update state
getState()            // Get current state
```

### 2. **Navigation Utils** (`utils/navigation.js`)
Core routing logic with validation.

```javascript
// Parse and validate URL parameters
parseURL() → NavigationParams

// Update URL with validation
updateURL(params) → NavigationParams

// Navigate and render
navigateToView(scutoon, view, params)

// Render based on navigation params
renderView(scutoon, navParams)
```

**Features:**
- ✅ Validates view names against `VALID_VIEWS`
- ✅ Sanitizes chapter/song numbers (1-999 range)
- ✅ Strips default values from URL (cleaner URLs)
- ✅ Centralized routing logic

### 3. **Main Entry Point** (`main.js`)
Handles browser navigation events.

```javascript
handleNavigation()
- Parse URL
- Init app (QR code, membership)
- Render view
- Scroll to top
```

## Usage

### From Components/Views
Use the `scutoon.navigateTo()` wrapper:

```javascript
// Navigate to view
scutoon.navigateTo('chapter', { chapter: 2 })
scutoon.navigateTo('song', { song: 5 })
scutoon.navigateTo('account')
```

### Programmatic Navigation
```javascript
import { navigateToView, parseURL } from './utils/navigation.js';

// Navigate
await navigateToView(scutoon, 'chapter', { chapter: 3 });

// Get current params
const navParams = parseURL();
console.log(navParams.view, navParams.chapter);
```

## URL Format

Clean URLs with defaults stripped:

```
?view=chapter&chapter=2
?view=song&song=5  
?view=account
/                    (defaults to toon view)
```

## Validation

- **Views**: Must be in `VALID_VIEWS` array
- **Chapters**: 1-999, defaults to 1
- **Songs**: 1-999, defaults to 1

## Benefits

1. **Single Source of Truth** - All routing logic in one place
2. **Type Safety** - NavigationParams class ensures consistency
3. **Validation** - Range checks prevent invalid navigation
4. **Cleaner URLs** - Strips default values
5. **No Duplication** - Removed duplicate switch statements
6. **Testable** - Pure functions easy to test
