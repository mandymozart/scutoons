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

# File extensions to download (same as sync tool)
DOWNLOAD_EXTENSIONS = {'.mp3', '.mp4', '.jpg', '.jpeg', '.png', '.gif', '.svg', '.webp', '.zip', '.avif', '.pdf', '.webm', '.mscz'}

def format_size(bytes):
    """Format bytes as human readable"""
    for unit in ['B', 'KB', 'MB', 'GB']:
        if bytes < 1024.0:
            return f"{bytes:.2f} {unit}"
        bytes /= 1024.0
    return f"{bytes:.2f} TB"

class FTPDownloader:
    def __init__(self, log_file):
        self.log_file = log_file
        self.ftp = None
        self.log_entries = []
        self.stats = {
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
                
    def should_download_file(self, filename):
        """Check if file should be downloaded based on extension"""
        return Path(filename).suffix.lower() in DOWNLOAD_EXTENSIONS
        
    def get_remote_file_list(self, remote_path='/'):
        """Recursively get all files from remote server"""
        files = []
        
        try:
            self.ftp.cwd(remote_path)
            items = []
            self.ftp.retrlines('LIST', items.append)
            
            for item in items:
                parts = item.split(None, 8)
                if len(parts) < 9:
                    continue
                    
                permissions = parts[0]
                name = parts[8]
                
                # Skip . and ..
                if name in ['.', '..']:
                    continue
                
                item_path = f"{remote_path}/{name}".replace('//', '/')
                
                # Check if directory (starts with 'd')
                if permissions.startswith('d'):
                    # Recursively get files from subdirectory
                    files.extend(self.get_remote_file_list(item_path))
                else:
                    # It's a file
                    if self.should_download_file(name):
                        try:
                            # Get file size
                            size = self.ftp.size(item_path)
                            files.append({
                                'path': item_path,
                                'name': name,
                                'size': size if size else 0
                            })
                        except:
                            # If size fails, still add the file with unknown size
                            files.append({
                                'path': item_path,
                                'name': name,
                                'size': 0
                            })
                            
        except Exception as e:
            self.log(f"Error listing {remote_path}: {e}", 'ERROR')
            
        return files
        
    def download_file(self, remote_path, local_path, file_size):
        """Download a file from FTP server with progress bar"""
        try:
            # Ensure local directory exists
            os.makedirs(os.path.dirname(local_path), exist_ok=True)
            
            # Progress tracking
            downloaded = [0]
            
            def progress_callback(block):
                downloaded[0] += len(block)
                if file_size > 0:
                    percent = (downloaded[0] / file_size) * 100
                    bar_length = 40
                    filled = int(bar_length * downloaded[0] / file_size)
                    bar = '█' * filled + '░' * (bar_length - filled)
                    print(f'\r  Progress: [{bar}] {percent:.1f}% ({format_size(downloaded[0])}/{format_size(file_size)})', end='', flush=True)
                else:
                    print(f'\r  Downloaded: {format_size(downloaded[0])}', end='', flush=True)
            
            with open(local_path, 'wb') as f:
                self.ftp.retrbinary(f'RETR {remote_path}', lambda data: (f.write(data), progress_callback(data)))
            
            print()  # New line after progress bar
            self.log(f"Downloaded: {remote_path} -> {local_path}")
            self.stats['downloaded'].append(remote_path)
            return True
        except Exception as e:
            print()  # New line if error occurs
            self.log(f"Failed to download {remote_path}: {e}", 'ERROR')
            self.stats['errors'].append(f"Download failed: {remote_path}")
            return False
            
    def download_all(self, skip_existing=True, auto_confirm=False):
        """Download all files from remote server"""
        self.log("Scanning remote server for files...")
        remote_files = self.get_remote_file_list('/')
        
        if not remote_files:
            self.log("No files found on remote server")
            return True
            
        total_size = sum(f['size'] for f in remote_files)
        self.log(f"Found {len(remote_files)} files ({format_size(total_size)} total)")
        
        # Process each file
        total_files = len(remote_files)
        current_file = 0
        
        for file_info in remote_files:
            current_file += 1
            remote_path = file_info['path']
            file_size = file_info['size']
            
            # Determine local path
            # Remove leading slash and map to ROOT_DIR
            relative_path = remote_path.lstrip('/')
            local_path = os.path.join(ROOT_DIR, relative_path)
            
            # Check if file already exists locally
            if os.path.exists(local_path):
                local_size = os.path.getsize(local_path)
                
                if skip_existing and local_size == file_size:
                    print(f"[{current_file}/{total_files}] ✓ Skipping {relative_path} (already exists, same size)")
                    self.stats['skipped'].append(remote_path)
                    continue
                elif local_size != file_size:
                    if not auto_confirm:
                        print(f"\n[{current_file}/{total_files}] File exists with different size:")
                        print(f"  Local:  {format_size(local_size)}")
                        print(f"  Remote: {format_size(file_size)}")
                        response = input("  Overwrite? (y/n/a for all): ").lower()
                        if response == 'a':
                            auto_confirm = True
                        elif response != 'y':
                            self.stats['skipped'].append(remote_path)
                            continue
                    print(f"[{current_file}/{total_files}] ↻ Updating {relative_path} ({format_size(file_size)})...")
                else:
                    if not auto_confirm:
                        response = input(f"[{current_file}/{total_files}] Overwrite existing {relative_path}? (y/n/a for all): ").lower()
                        if response == 'a':
                            auto_confirm = True
                        elif response != 'y':
                            self.stats['skipped'].append(remote_path)
                            continue
                    print(f"[{current_file}/{total_files}] ↻ Downloading {relative_path} ({format_size(file_size)})...")
            else:
                print(f"[{current_file}/{total_files}] ↓ Downloading {relative_path} ({format_size(file_size)})...")
            
            self.download_file(remote_path, local_path, file_size)
            
        return True
        
    def save_log(self):
        """Save log to file"""
        os.makedirs(os.path.dirname(self.log_file), exist_ok=True)
        with open(self.log_file, 'w', encoding='utf-8') as f:
            f.write('\n'.join(self.log_entries))
        self.log(f"Log saved to: {self.log_file}")
        
    def print_summary(self):
        """Print download summary"""
        print("\n" + "="*60)
        print("DOWNLOAD SUMMARY")
        print("="*60)
        print(f"Downloaded: {len(self.stats['downloaded'])} files")
        print(f"Skipped: {len(self.stats['skipped'])} files")
        print(f"Errors: {len(self.stats['errors'])} files")
        print("="*60)
        
        if self.stats['errors']:
            print("\nErrors:")
            for error in self.stats['errors']:
                print(f"  - {error}")

def main():
    """Main download function"""
    print("="*60)
    print("SCUTOONS CDN DOWNLOAD TOOL")
    print("="*60)
    print(f"Download to: {ROOT_DIR}")
    print(f"File types: {', '.join(DOWNLOAD_EXTENSIONS)}")
    print("="*60)
    
    # Validate environment
    if not all([FTP_SERVER, FTP_USERNAME, FTP_PASSWORD]):
        print("\nERROR: Missing FTP credentials in .env file")
        print("Required: FTP_SERVER, FTP_USERNAME, FTP_PASSWORD")
        return 1
    
    # Ask user preferences
    print("\nDownload options:")
    skip_existing = input("Skip files that already exist with same size? (Y/n): ").strip().lower() != 'n'
    auto_confirm = input("Auto-overwrite files with different sizes? (y/N): ").strip().lower() == 'y'
    
    print()
    
    # Create log file
    log_file = os.path.join(LOGS_DIR, f'download_{datetime.datetime.now().strftime("%Y%m%d_%H%M%S")}.log')
    
    # Create downloader
    downloader = FTPDownloader(log_file)
    
    try:
        if not downloader.connect():
            return 1
            
        downloader.download_all(skip_existing=skip_existing, auto_confirm=auto_confirm)
        downloader.print_summary()
        downloader.save_log()
        
        return 0 if not downloader.stats['errors'] else 1
        
    except KeyboardInterrupt:
        print("\n\nDownload interrupted by user")
        return 1
    finally:
        downloader.disconnect()

if __name__ == '__main__':
    sys.exit(main())