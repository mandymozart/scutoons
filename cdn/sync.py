#!/usr/bin/env python3

import os
import sys
import ftplib
import datetime
from pathlib import Path
from dotenv import load_dotenv

# Load environment variables from .env file
load_dotenv()

# Configuration
FTP_SERVER = os.getenv('FTP_SERVER')
FTP_USERNAME = os.getenv('FTP_USERNAME')
FTP_PASSWORD = os.getenv('FTP_PASSWORD')
ROOT_DIR = os.path.join(os.path.dirname(__file__), 'root')
LOGS_DIR = os.path.join(os.path.dirname(__file__), 'logs')

# File extensions to sync (for toon assets)
SYNC_EXTENSIONS = {'.mp3', '.mp4', '.jpg', '.jpeg', '.png', '.gif', '.svg', '.webp', '.zip', '.avif', '.pdf', '.webm', '.mscz'}
# Note: Root .htaccess (cdn/.htaccess) synced to FTP root via sync_root_files()

def format_size(bytes):
    """Format bytes as human readable"""
    for unit in ['B', 'KB', 'MB', 'GB']:
        if bytes < 1024.0:
            return f"{bytes:.2f} {unit}"
        bytes /= 1024.0
    return f"{bytes:.2f} TB"

class FTPSync:
    def __init__(self, toon_id, log_file):
        self.toon_id = toon_id
        self.log_file = log_file
        self.ftp = None
        self.log_entries = []
        self.changes = {
            'uploaded': [],
            'downloaded': [],
            'skipped': [],
            'errors': []
        }
        
    def log(self, message, level='INFO'):
        """Log message to console and log list"""
        timestamp = datetime.datetime.now().strftime('%Y-%m-%d %H:%M:%S')
        log_message = f"[{timestamp}] [{level}] {message}"
        print(log_message)
        self.log_entries.append(log_message)
        
    def connect(self):
        """Connect to FTP server"""
        try:
            self.log(f"Connecting to {FTP_SERVER}...")
            self.ftp = ftplib.FTP(FTP_SERVER)
            self.ftp.login(FTP_USERNAME, FTP_PASSWORD)
            self.log(f"Connected successfully as {FTP_USERNAME}")
            return True
        except Exception as e:
            self.log(f"Failed to connect: {e}", 'ERROR')
            return False
            
    def disconnect(self):
        """Disconnect from FTP server"""
        if self.ftp:
            try:
                self.ftp.quit()
                self.log("Disconnected from FTP server")
            except:
                self.ftp.close()
                
    def get_remote_file_size(self, remote_path):
        """Get size of a specific remote file, returns None if doesn't exist"""
        try:
            # Try to get file size
            self.ftp.voidcmd('TYPE I')  # Binary mode
            size = self.ftp.size(remote_path)
            return size
        except ftplib.error_perm:
            # File doesn't exist
            return None
        except Exception as e:
            # Other error
            self.log(f"Error checking remote file {remote_path}: {e}", 'WARN')
            return None
            
    def ensure_remote_dir(self, remote_path):
        """Ensure remote directory exists"""
        parts = remote_path.strip('/').split('/')
        current = ''
        
        for part in parts:
            current = f"{current}/{part}" if current else f"/{part}"
            try:
                self.ftp.cwd(current)
            except ftplib.error_perm:
                try:
                    self.ftp.mkd(current)
                    self.log(f"Created remote directory: {current}")
                except ftplib.error_perm:
                    pass
                    
    def upload_file(self, local_path, remote_path, file_size):
        """Upload a file to FTP server with progress bar"""
        try:
            # Ensure remote directory exists
            remote_dir = '/'.join(remote_path.split('/')[:-1])
            if remote_dir:
                self.ensure_remote_dir(remote_dir)
            
            # Progress tracking
            uploaded = [0]  # Use list to allow modification in nested function
            
            def progress_callback(block):
                uploaded[0] += len(block)
                percent = (uploaded[0] / file_size) * 100
                bar_length = 40
                filled = int(bar_length * uploaded[0] / file_size)
                bar = '█' * filled + '░' * (bar_length - filled)
                print(f'\r  Progress: [{bar}] {percent:.1f}% ({format_size(uploaded[0])}/{format_size(file_size)})', end='', flush=True)
            
            with open(local_path, 'rb') as f:
                self.ftp.storbinary(f'STOR {remote_path}', f, callback=progress_callback)
            
            print()  # New line after progress bar
            self.log(f"Uploaded: {local_path} -> {remote_path}")
            self.changes['uploaded'].append(remote_path)
            return True
        except Exception as e:
            print()  # New line if error occurs
            self.log(f"Failed to upload {local_path}: {e}", 'ERROR')
            self.changes['errors'].append(f"Upload failed: {local_path}")
            return False
            
    def download_file(self, remote_path, local_path):
        """Download a file from FTP server"""
        try:
            os.makedirs(os.path.dirname(local_path), exist_ok=True)
            with open(local_path, 'wb') as f:
                self.ftp.retrbinary(f'RETR {remote_path}', f.write)
            self.log(f"Downloaded: {remote_path} -> {local_path}")
            self.changes['downloaded'].append(remote_path)
            return True
        except Exception as e:
            self.log(f"Failed to download {remote_path}: {e}", 'ERROR')
            self.changes['errors'].append(f"Download failed: {remote_path}")
            return False
            
    def should_sync_file(self, file_path):
        """Check if file should be synced based on extension"""
        return Path(file_path).suffix.lower() in SYNC_EXTENSIONS
        
    def confirm_action(self, message):
        """Ask user to confirm an action"""
        response = input(f"{message} (y/n/a for all): ").lower()
        if response == 'a':
            return 'all'
        return response == 'y'
        
    def sync_directory(self, local_dir, remote_dir, auto_confirm=False):
        """Sync a directory bidirectionally"""
        self.log(f"Syncing: {local_dir} <-> {remote_dir}")
        
        # Ensure directories exist
        os.makedirs(local_dir, exist_ok=True)
        self.ensure_remote_dir(remote_dir)
        
        # Get local files
        local_files = {}
        for root, dirs, files in os.walk(local_dir):
            for file in files:
                if self.should_sync_file(file):
                    full_path = os.path.join(root, file)
                    rel_path = os.path.relpath(full_path, local_dir)
                    stat = os.stat(full_path)
                    local_files[rel_path] = {
                        'size': stat.st_size,
                        'mod_time': datetime.datetime.fromtimestamp(stat.st_mtime),
                        'full_path': full_path
                    }
        
        # Process each local file
        total_files = len(local_files)
        current_file = 0
        
        print(f"\nFound {total_files} local files to process")
        print(f"Checking remote status...\n")
        
        for file in sorted(local_files.keys()):
            current_file += 1
            local_path = os.path.join(local_dir, file)
            remote_path = f"{remote_dir}/{file}".replace('\\', '/')
            
            # Check if this exact file exists on remote
            print(f'\rChecking [{current_file}/{total_files}] {file[:60]}{"..." if len(file) > 60 else ""}', end='', flush=True)
            remote_size = self.get_remote_file_size(remote_path)
            has_remote = remote_size is not None
            local_info = local_files[file]
            
            # Clear the checking line
            print('\r' + ' ' * 100 + '\r', end='', flush=True)
            
            if not has_remote:
                # File missing on remote - upload
                local_size = local_info['size']
                if auto_confirm:
                    print(f"[{current_file}/{total_files}] Uploading {file} ({format_size(local_size)})...")
                    self.upload_file(local_info['full_path'], remote_path, local_size)
                else:
                    action = self.confirm_action(f"[{current_file}/{total_files}] Upload {file} ({format_size(local_size)}) [MISSING on remote]?")
                    if action == 'all':
                        auto_confirm = True
                        print(f"[{current_file}/{total_files}] Uploading {file} ({format_size(local_size)})...")
                        self.upload_file(local_info['full_path'], remote_path, local_size)
                    elif action:
                        print(f"[{current_file}/{total_files}] Uploading {file} ({format_size(local_size)})...")
                        self.upload_file(local_info['full_path'], remote_path, local_size)
                    else:
                        self.changes['skipped'].append(f"Upload skipped: {file}")
            
            elif local_info['size'] != remote_size:
                # File exists but sizes differ - ask what to do
                action = self.confirm_action(
                    f"[{current_file}/{total_files}] File {file} DIFFERS: "
                    f"local={format_size(local_info['size'])}, remote={format_size(remote_size)}. "
                    f"Upload local version?"
                )
                if action == 'all':
                    auto_confirm = True
                    action = True
                    
                if action:
                    print(f"[{current_file}/{total_files}] Uploading {file} ({format_size(local_info['size'])})...")
                    self.upload_file(local_info['full_path'], remote_path, local_info['size'])
                else:
                    print(f"[{current_file}/{total_files}] Downloading {file} ({format_size(remote_size)})...")
                    self.download_file(remote_path, local_path)
            
            else:
                # Files are the same size - skip
                self.changes['skipped'].append(f"Already synced: {file}")
                    
    def save_log(self):
        """Save log to file"""
        os.makedirs(os.path.dirname(self.log_file), exist_ok=True)
        with open(self.log_file, 'w', encoding='utf-8') as f:
            f.write('\n'.join(self.log_entries))
        self.log(f"Log saved to: {self.log_file}")
        
    def print_summary(self):
        """Print sync summary"""
        print("\n" + "="*60)
        print("SYNC SUMMARY")
        print("="*60)
        print(f"Uploaded: {len(self.changes['uploaded'])} files")
        print(f"Downloaded: {len(self.changes['downloaded'])} files")
        print(f"Skipped: {len(self.changes['skipped'])} files")
        print(f"Errors: {len(self.changes['errors'])} files")
        print("="*60)
        
        if self.changes['errors']:
            print("\nErrors:")
            for error in self.changes['errors']:
                print(f"  - {error}")
                
def discover_toons():
    """Discover toon directories in cdn/root/"""
    if not os.path.exists(ROOT_DIR):
        return []
    
    toons = []
    for item in os.listdir(ROOT_DIR):
        item_path = os.path.join(ROOT_DIR, item)
        if os.path.isdir(item_path):
            toons.append(item)
    
    return sorted(toons)

def select_toons(toons):
    """Interactive toon selection with checkboxes"""
    if not toons:
        print("ERROR: No toon directories found in cdn/root/")
        print("Create a directory like cdn/root/catpope/ and add assets to sync")
        return []
    
    print("\nAvailable toons to sync:")
    print("-" * 40)
    for i, toon in enumerate(toons, 1):
        print(f"  [{i}] {toon}")
    print("-" * 40)
    print("\nOptions:")
    print("  - Enter 'all' to sync all toons")
    print("  - Enter numbers separated by spaces (e.g., '1 3' to sync toons 1 and 3)")
    print("  - Press Enter to sync all toons")
    
    selection = input("\nYour selection: ").strip().lower()
    
    # Default to all if empty
    if not selection:
        return toons
    
    # Sync all option
    if selection == 'all':
        return toons
    
    # Parse individual selections
    try:
        selected_indices = [int(num.strip()) for num in selection.split()]
        selected_toons = []
        for idx in selected_indices:
            if 1 <= idx <= len(toons):
                selected_toons.append(toons[idx - 1])
            else:
                print(f"Warning: Ignoring invalid selection {idx}")
        
        if not selected_toons:
            print("No valid toons selected. Exiting.")
            return []
        
        return selected_toons
    except ValueError:
        print("ERROR: Invalid input. Please enter numbers or 'all'")
        return []

def sync_toon(toon_id, ftp_connection=None):
    """Sync a single toon"""
    local_root = os.path.join(ROOT_DIR, toon_id)
    remote_root = f'/{toon_id}'
    log_file = os.path.join(LOGS_DIR, f'sync_{toon_id}_{datetime.datetime.now().strftime("%Y%m%d_%H%M%S")}.log')
    
    print("\n" + "="*60)
    print(f"SYNCING: {toon_id}")
    print("="*60)
    print(f"Local:  {local_root}")
    print(f"Remote: {remote_root}")
    print("="*60 + "\n")
    
    syncer = FTPSync(toon_id, log_file)
    
    # Reuse FTP connection if provided
    if ftp_connection:
        syncer.ftp = ftp_connection
    else:
        if not syncer.connect():
            return False
    
    try:
        syncer.sync_directory(local_root, remote_root)
        syncer.print_summary()
        syncer.save_log()
        return True
    except Exception as e:
        syncer.log(f"Unexpected error: {e}", 'ERROR')
        return False

def sync_root_config(ftp_connection):
    """Sync root-level config files like .htaccess"""
    cdn_dir = os.path.dirname(__file__)
    config_files = ['.htaccess']
    
    print("="*60)
    print("SYNCING ROOT CONFIG")
    print("="*60)
    
    for filename in config_files:
        local_path = os.path.join(cdn_dir, filename)
        if not os.path.exists(local_path):
            print(f"⊘ {filename} not found locally, skipping")
            continue
        
        try:
            # Check if file exists on remote
            remote_size = None
            try:
                ftp_connection.voidcmd('TYPE I')
                remote_size = ftp_connection.size(f'/{filename}')
            except:
                pass  # File doesn't exist remotely
            
            local_size = os.path.getsize(local_path)
            
            if remote_size is None:
                print(f"↑ Uploading {filename} ({format_size(local_size)})...")
                with open(local_path, 'rb') as f:
                    ftp_connection.storbinary(f'STOR /{filename}', f)
                print(f"✓ Uploaded {filename}")
            elif local_size != remote_size:
                print(f"↻ Updating {filename} (local: {format_size(local_size)}, remote: {format_size(remote_size)})...")
                with open(local_path, 'rb') as f:
                    ftp_connection.storbinary(f'STOR /{filename}', f)
                print(f"✓ Updated {filename}")
            else:
                print(f"✓ {filename} already up to date")
        except Exception as e:
            print(f"✗ Failed to sync {filename}: {e}")
    
    print()

def sync_root_files(ftp_connection):
    """Sync root-level files (not in toon folders, not config)"""
    log_file = os.path.join(LOGS_DIR, f'sync_root_{datetime.datetime.now().strftime("%Y%m%d_%H%M%S")}.log')
    syncer = FTPSync('root', log_file)
    syncer.ftp = ftp_connection
    
    print("="*60)
    print("SYNCING ROOT FILES")
    print("="*60)
    print(f"Local:  {ROOT_DIR}")
    print(f"Remote: /")
    print("="*60 + "\n")
    
    try:
        # Get only files directly in ROOT_DIR, not subdirectories
        local_files = {}
        for item in os.listdir(ROOT_DIR):
            item_path = os.path.join(ROOT_DIR, item)
            # Skip directories (toon folders)
            if os.path.isdir(item_path):
                continue
            # Skip non-matching extensions
            if not any(item.endswith(ext) for ext in SYNC_EXTENSIONS):
                continue
            
            stat = os.stat(item_path)
            local_files[item] = {
                'size': stat.st_size,
                'mod_time': datetime.datetime.fromtimestamp(stat.st_mtime),
                'full_path': item_path
            }
        
        if not local_files:
            print("No root files found to sync\n")
            return True
        
        print(f"Found {len(local_files)} root file(s) to process\n")
        
        # Process each file
        auto_confirm = False
        for file, info in local_files.items():
            remote_path = f'/{file}'
            
            # Check remote
            remote_size = syncer.get_remote_file_size(remote_path)
            has_remote = remote_size is not None
            
            if not has_remote:
                if auto_confirm:
                    print(f"Uploading {file} ({format_size(info['size'])})...")
                    syncer.upload_file(info['full_path'], remote_path, info['size'])
                else:
                    action = syncer.confirm_action(f"Upload {file} ({format_size(info['size'])}) [MISSING on remote]?")
                    if action == 'all':
                        auto_confirm = True
                    if action in ['yes', 'all']:
                        syncer.upload_file(info['full_path'], remote_path, info['size'])
            elif info['size'] != remote_size:
                if auto_confirm:
                    print(f"Uploading {file} ({format_size(info['size'])})...")
                    syncer.upload_file(info['full_path'], remote_path, info['size'])
                else:
                    action = syncer.confirm_action(f"Upload {file} (local: {format_size(info['size'])}, remote: {format_size(remote_size)}) [SIZE MISMATCH]?")
                    if action == 'all':
                        auto_confirm = True
                    if action in ['yes', 'all']:
                        syncer.upload_file(info['full_path'], remote_path, info['size'])
        
        syncer.print_summary()
        syncer.save_log()
        return True
    except Exception as e:
        syncer.log(f"Unexpected error: {e}", 'ERROR')
        return False

def main():
    """Main sync function"""
    print("="*60)
    print("SCUTOONS CDN SYNC TOOL")
    print("="*60)
    print(f"Root directory: {ROOT_DIR}")
    print(f"Syncing extensions: {', '.join(SYNC_EXTENSIONS)}")
    print("="*60)
    
    # Validate environment
    if not all([FTP_SERVER, FTP_USERNAME, FTP_PASSWORD]):
        print("\nERROR: Missing FTP credentials in .env file")
        print("Required: FTP_SERVER, FTP_USERNAME, FTP_PASSWORD")
        return 1
    
    # Select what to sync
    print("\nWhat do you want to sync?")
    print("-" * 40)
    print("  [1] Root config (.htaccess)")
    print("  [2] Root files (main app assets)")
    print("  [3] Toon assets (catpope, etc.)")
    print("  [all] All of the above")
    print("-" * 40)
    
    selection = input("\nYour selection (1 2 3 or 'all'): ").strip().lower()
    
    # Parse selection
    sync_config = False
    sync_root = False
    sync_toons = False
    
    if selection == 'all' or not selection:
        sync_config = sync_root = sync_toons = True
    else:
        parts = selection.split()
        sync_config = '1' in parts
        sync_root = '2' in parts
        sync_toons = '3' in parts
    
    if not any([sync_config, sync_root, sync_toons]):
        print("No valid selection. Exiting.")
        return 1
    
    # Connect to FTP
    print(f"\nConnecting to {FTP_SERVER}...")
    try:
        ftp = ftplib.FTP(FTP_SERVER)
        ftp.login(FTP_USERNAME, FTP_PASSWORD)
        print(f"✓ Connected as {FTP_USERNAME}\n")
    except Exception as e:
        print(f"ERROR: Failed to connect: {e}")
        return 1
    
    try:
        # Sync root config if selected
        if sync_config:
            sync_root_config(ftp)
        
        # Sync root files if selected
        if sync_root:
            sync_root_files(ftp)
        
        # Sync toons if selected
        selected_toons = []
        if sync_toons:
            toons = discover_toons()
            if toons:
                selected_toons = select_toons(toons)
                if selected_toons:
                    print(f"\n✓ Will sync {len(selected_toons)} toon(s): {', '.join(selected_toons)}\n")
        
        # Sync each selected toon
        success_count = 0
        for toon_id in selected_toons:
            if sync_toon(toon_id, ftp):
                success_count += 1
        
        print("\n" + "="*60)
        print("OVERALL SUMMARY")
        print("="*60)
        if sync_config:
            print("✓ Root config synced")
        if sync_root:
            print("✓ Root files synced")
        if selected_toons:
            print(f"✓ Toon assets: {success_count}/{len(selected_toons)} synced successfully")
        print("="*60 + "\n")
        
        # Success if no toons were selected, or all selected toons synced successfully
        if not selected_toons:
            return 0
        return 0 if success_count == len(selected_toons) else 1
        
    except KeyboardInterrupt:
        print("\n\nSync interrupted by user")
        return 1
    finally:
        try:
            ftp.quit()
            print("Disconnected from FTP server")
        except:
            ftp.close()

if __name__ == '__main__':
    sys.exit(main())