"""
MkDocs hook to copy feature JSON files to the site directory.
"""
import os
import shutil
from pathlib import Path


def on_post_build(config, **kwargs):
    """
    Copy feature JSON files after the build is complete.
    """
    # Get the docs and site directories
    docs_dir = Path(config['docs_dir'])
    site_dir = Path(config['site_dir'])
    
    # Source and destination paths
    source_features = docs_dir / 'features'
    dest_features = site_dir / 'features'
    
    # Only proceed if source exists
    if source_features.exists() and source_features.is_dir():
        # Create destination directory
        dest_features.mkdir(parents=True, exist_ok=True)
        
        # Copy all JSON files
        for json_file in source_features.glob('*.json'):
            dest_file = dest_features / json_file.name
            shutil.copy2(json_file, dest_file)
            print(f"Copied {json_file.name} to site/features/")
