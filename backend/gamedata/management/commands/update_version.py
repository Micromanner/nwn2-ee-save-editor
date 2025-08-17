#!/usr/bin/env python3
"""
Version management script for NWN2 Save Editor
Updates version across all project files in sync
"""

import json
import re
import argparse
from pathlib import Path

def update_version(new_version: str, root_dir: Path = None):
    """Update version in all project files"""
    if root_dir is None:
        root_dir = Path(__file__).parent.parent.parent.parent.parent
    
    # Validate semantic version format
    if not re.match(r'^\d+\.\d+\.\d+$', new_version):
        raise ValueError(f"Invalid version format: {new_version}. Use semantic versioning (e.g., 1.0.0)")
    
    files_updated = []
    
    # Update frontend package.json
    package_json_path = root_dir / "frontend" / "package.json"
    if package_json_path.exists():
        with open(package_json_path, 'r') as f:
            data = json.load(f)
        data['version'] = new_version
        with open(package_json_path, 'w') as f:
            json.dump(data, f, indent=2)
        files_updated.append(str(package_json_path))
    
    # Update Tauri config
    tauri_config_path = root_dir / "frontend" / "src-tauri" / "tauri.conf.json"
    if tauri_config_path.exists():
        with open(tauri_config_path, 'r') as f:
            data = json.load(f)
        data['version'] = new_version
        with open(tauri_config_path, 'w') as f:
            json.dump(data, f, indent=2)
        files_updated.append(str(tauri_config_path))
    
    # Update Rust extensions pyproject.toml
    pyproject_path = root_dir / "backend" / "rust_extensions" / "pyproject.toml"
    if pyproject_path.exists():
        content = pyproject_path.read_text()
        content = re.sub(r'version = "[^"]*"', f'version = "{new_version}"', content)
        pyproject_path.write_text(content)
        files_updated.append(str(pyproject_path))
    
    # Update icon cache pyproject.toml
    icon_cache_path = root_dir / "backend" / "parsers" / "rust_icon_cache" / "pyproject.toml"
    if icon_cache_path.exists():
        content = icon_cache_path.read_text()
        content = re.sub(r'version = "[^"]*"', f'version = "{new_version}"', content)
        icon_cache_path.write_text(content)
        files_updated.append(str(icon_cache_path))
    
    return files_updated

def main():
    parser = argparse.ArgumentParser(description='Update version across all project files')
    parser.add_argument('version', help='New version (e.g., 1.0.0)')
    parser.add_argument('--dry-run', action='store_true', help='Show what would be updated without making changes')
    
    args = parser.parse_args()
    
    try:
        if args.dry_run:
            print(f"Would update version to {args.version} in:")
            root_dir = Path(__file__).parent.parent.parent.parent.parent
            files = [
                root_dir / "frontend" / "package.json",
                root_dir / "frontend" / "src-tauri" / "tauri.conf.json", 
                root_dir / "backend" / "rust_extensions" / "pyproject.toml",
                root_dir / "backend" / "parsers" / "rust_icon_cache" / "pyproject.toml"
            ]
            for file in files:
                if file.exists():
                    print(f"  - {file}")
        else:
            files_updated = update_version(args.version)
            print(f"Updated version to {args.version} in {len(files_updated)} files:")
            for file in files_updated:
                print(f"  - {file}")
            print("\nNext steps:")
            print(f"1. git add .")
            print(f"2. git commit -m 'Bump version to {args.version}'")
            print(f"3. git tag -a v{args.version} -m 'Release v{args.version}'")
            print(f"4. git push origin main --tags")
            
    except Exception as e:
        print(f"Error: {e}")
        return 1
    
    return 0

if __name__ == "__main__":
    exit(main())