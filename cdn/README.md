# CDN Sync Tool

This tool syncs assets between your local development environment and the CDN server (cdn.scutoons.com) via FTP.

## Setup
0. Create a virtual environment and activate it:
   ```bash
   python -m venv .venv
   .venv\Scripts\activate
   ```

1. **Install Python dependencies:**
   ```bash
   cd cdn
   pip install -r requirements.txt
   ```

2. **Configure FTP credentials:**
   - Copy `.env.example` to `.env`
   - Fill in your FTP credentials

3. **Prepare assets for sync:**
   - Place assets in the `cdn/root/{TOON_ID}/` directory
   - The directory structure in `root/{TOON_ID}/` will be mirrored on the CDN
   - For catpope: `cdn/root/catpope/audio/`, `cdn/root/catpope/images/`, `cdn/root/catpope/videos/`, `cdn/root/catpope/merch/`
   - For other toons: `cdn/root/othertoon/audio/`, etc.

## Usage

Run the sync script:
```bash
python sync.py
```

You'll be prompted to select what to sync:
```
What do you want to sync?
----------------------------------------
  [1] Root config (.htaccess)
  [2] Root files (main app assets)
  [3] Toon assets (catpope, etc.)
  [all] All of the above
----------------------------------------
Your selection (1 2 3 or 'all'):
```

**Examples:**
- `all` or press Enter → Syncs everything
- `1` → Only root config (.htaccess)
- `1 3` → Root config + toon assets
- `2 3` → Root files + toon assets

The script will then:
1. **Compare files**: Compare local and remote files by size
2. **Confirm operations**: Prompt you to confirm uploads/downloads
   - Type `a` for "all" to auto-confirm remaining operations
3. **Log everything**: Save detailed logs to `logs/sync_{type}_{timestamp}.log`

## File Syncing

The script syncs these file types:
- Audio: `.mp3`
- Video: `.mp4`, `.webm`, `.avif`
- Images: `.jpg`, `.jpeg`, `.png`, `.gif`, `.svg`, `.webp`
- Archives: `.zip`
- Scores: `.mscz`
- Config: `.htaccess` (for CORS)
- Other: `.pdf`

## CORS Configuration

A `.htaccess` file is included in the `cdn/` directory with CORS headers configured to:
- ✅ Allow requests from `scutoons.com` and all subdomains
- ✅ Allow requests from `localhost:8000` (for local development)
- ✅ Cache static assets for optimal performance
- ✅ Enable compression for faster loading

**To deploy CORS configuration:**

Run `python sync.py` and select option `[1]` for root config only:
```bash
python sync.py
# Enter: 1
```

Or sync everything with `all`.

**Requirements:**
- Ensure your hosting provider has `mod_headers` and `mod_expires` enabled

## Bidirectional Sync

- **Upload**: Files that exist locally but not on CDN
- **Download**: Files that exist on CDN but not locally
- **Conflict**: If file sizes differ, you'll be prompted to choose upload or download

## Directory Structure

```
cdn/
├── .htaccess (root config → syncs to /)
├── .env (gitignored, your credentials)
├── .env.example (template)
├── sync.py (sync script)
├── requirements.txt (Python dependencies)
├── root/ (local mirror of CDN content)
│   ├── image123.jpg (root file → syncs to /)
│   ├── video.mp4 (root file → syncs to /)
│   ├── catpope/ (toon assets → syncs to /catpope/)
│   │   ├── audio/
│   │   ├── images/
│   │   ├── videos/
│   │   └── merch/
│   └── othertoon/ (future releases)
│       ├── audio/
│       └── ...
└── logs/ (sync logs)
```
