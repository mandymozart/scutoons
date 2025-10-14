# SCUTOONS App 

## Directory Structure

```
app/
├── main.js                 # Entry point - initializes app and handles navigation
├── Scutoon.js             # Main application class - orchestrates views and stores
│
├── stores/                # State management
│   ├── index.js          # Barrel export for all stores
│   ├── MembershipStore.js # Handles membership validation and QR codes
│   └── NavigationStore.js # Manages routing and navigation state
│
├── views/                 # Page-level views
│   ├── index.js          # Barrel export for all views
│   ├── ToonView.js       # Main overview with chapters/songs list
│   ├── ChapterView.js    # Webtoon chapter reader
│   ├── SongView.js       # Song player and details
│   ├── SeasonView.js     # Season details page
│   ├── AccountView.js    # User account and membership details
│   ├── CreditsView.js    # Full credits page
│   ├── PlayerView.js     # Music player
│   └── MembershipRequiredView.js # Locked content message
│
├── components/            # Reusable UI components
│   ├── index.js          # Barrel export for all components
│   ├── ChapterItem.js    # Single chapter card
│   ├── SongItem.js       # Single song card
│   ├── ChapterSlice.js   # Webtoon image slice
│   ├── SequenceItem.js   # Mixed chapter/song items
│   └── Navigation.js     # Navigation menu components
│
└── utils/                 # Utility functions
    ├── index.js          # Barrel export for all utils
    ├── scroll.js         # Scroll handling utilities
    ├── template.js       # Template rendering helpers
    ├── api.js            # API calls for loading content
    ├── imageLoader.js    # Sequential image loading for webtoon
    └── player.js         # Music player functionality
```

## Usage

### Development Server
ES6 modules require HTTP(S) protocol (not `file://`). Use:

```bash
# Python
python -m http.server 8000

# Node.js
npx serve

# VS Code
# Install "Live Server" extension
```

### Adding New Views
1. Create view file in `app/views/YourView.js`
2. Export the view class
3. Add to `app/views/index.js`
4. Add route handling in `Scutoon.js`

### Adding New Components
1. Create component file in `app/components/YourComponent.js`
2. Export render function(s)
3. Add to `app/components/index.js`
4. Import and use in views

### Adding New Utilities
1. Create utility file in `app/utils/yourUtil.js`
2. Export functions
3. Add to `app/utils/index.js`
4. Import where needed
