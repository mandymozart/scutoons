# Sequence Navigation Component

## Overview
A reusable navigation component that appears at the end of chapters and songs, allowing users to navigate through the season sequence.

## Features
- **Previous/Next Navigation**: Navigate to adjacent **published** items in the season sequence
- **Status-Aware**: Automatically skips items with `status: "coming-soon"` or other unpublished statuses
- **Overview Button**: Always centered, returns to the toon overview
- **Edge Handling**: Displays disabled buttons at sequence boundaries
- **Mixed Content**: Supports navigation between chapters and songs
- **Responsive Design**: Adapts to mobile and desktop layouts

## Implementation

### Component Location
`catpope/app/components/SequenceNavigation.js`

### Used In
- `catpope/app/views/ChapterView.js` - At the end of each chapter
- `catpope/app/views/SongView.js` - At the end of each song

### Styling
`catpope/assets/styles.css` - Added `.sequence-navigation` styles with responsive breakpoints

## How It Works

1. **Data Loading**: 
   - Loads `season1.json` to get the sequence structure
   - Loads all items (chapters/songs) in the sequence to check their metadata, including `status` field

2. **Status Filtering**: 
   - Filters out items with `status: "coming-soon"` or other unpublished statuses
   - Only items with `status: "published"` or no status field are included in navigation

3. **Position Detection**: 
   - Finds the current item's position within the **filtered** (published-only) sequence

4. **Navigation Generation**:
   - Previous button: Links to the previous **published** sequence item
   - Overview button: Always links back to toon overview
   - Next button: Links to the next **published** sequence item

5. **Edge Cases**:
   - At sequence start: Previous button is disabled
   - At sequence end: Next button is disabled
   - Coming-soon items: Completely skipped in navigation
   - Can be extended to support cross-season navigation

## API

```javascript
renderSequenceNavigation({
  sequenceItems: [...],    // Array of loaded sequence items with full metadata
  currentType: 'chapter',  // 'chapter' or 'song'
  currentNumber: 1         // Current item number
})
```

**sequenceItems** structure:
```javascript
[
  {
    itemType: 'song',
    itemNumber: 1,
    status: 'published',  // or 'coming-soon', etc.
    title: '...',
    // ... other item metadata
  },
  {
    itemType: 'chapter',
    itemNumber: 1,
    status: 'published',
    title: '...',
    // ... other item metadata
  }
]
```

## Future Enhancements
- Cross-season navigation (when season 2+ exists)
- Keyboard shortcuts (arrow keys)
- Swipe gestures on mobile
- Progress indicator showing position in sequence
- Can be made sticky/floating for persistent navigation across scroll
