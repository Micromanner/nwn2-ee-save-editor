#!/usr/bin/env python3
"""
Build FastAPI server as a standalone executable using Nuitka.
Fixed for proper Tauri sidecar integration.
"""
import os
import sys
import shutil
import subprocess
import platform
import time
from pathlib import Path

# Get project directories
BACKEND_DIR = Path(__file__).parent
PROJECT_ROOT = BACKEND_DIR.parent
DIST_DIR = PROJECT_ROOT / "frontend" / "src-tauri" / "binaries"

def build_with_nuitka():
    """Build the FastAPI server with Nuitka."""
    
    # Ensure the output directory exists
    DIST_DIR.mkdir(parents=True, exist_ok=True)
    
    # Clean up previous build artifacts to avoid Windows Defender conflicts
    print("Cleaning previous build artifacts...")
    for pattern in ["*.build", "*.dist", "*.onefile-build", ".nuitka-cache"]:
        for path in DIST_DIR.glob(pattern):
            if path.is_dir():
                shutil.rmtree(path, ignore_errors=True)
                print(f"Removed directory: {path.name}")
            else:
                path.unlink(missing_ok=True)
                print(f"Removed file: {path.name}")
    
    # Generic naming - Tauri handles platform-specific suffixes automatically
    if sys.platform == "win32":
        output_name = "fastapi-server.exe"
    else:
        output_name = "fastapi-server"

    # Skip the large icon cache directory since icons are not used anymore
    include_cache = False
    
    # Set up caching directory via environment variable
    cache_dir = DIST_DIR / ".nuitka-cache"
    cache_dir.mkdir(exist_ok=True)
    os.environ["NUITKA_CACHE_DIR"] = str(cache_dir)
    
    # Define the Nuitka command
    cmd = [
        sys.executable,  # Use the current python interpreter
        "-m", "nuitka",
        f"--output-filename={output_name}",
        f"--output-dir={DIST_DIR}",
        "--onefile",              # Create a single executable
        "--standalone",           # Bundle all dependencies
        "--assume-yes-for-downloads",  # Auto-download C compiler if needed
        "--lto=yes",              # Link-Time Optimization for smaller binary
        "--follow-imports",       # Find all modules used by the script
        
        # Include necessary packages explicitly (fixed package names)
        "--include-package=fastapi",
        "--include-package=uvicorn",
        "--include-package=pydantic",
        "--include-package=starlette",
        "--include-package=fastapi_routers",
        "--include-package=fastapi_core", 
        "--include-package=fastapi_models",
        "--include-package=config",
        "--include-package=character",
        "--include-package=gamedata",
        "--include-package=parsers",
        "--include-package=utils",  # Added missing utils package
        
        # More specific exclusions to reduce binary size and avoid warnings
        "--nofollow-import-to=django",
        "--nofollow-import-to=pytest",
        "--nofollow-import-to=pip",
        "--nofollow-import-to=setuptools",
        "--nofollow-import-to=wheel",
        "--nofollow-import-to=distutils", 
        "--nofollow-import-to=tkinter",
        "--nofollow-import-to=matplotlib",
        "--nofollow-import-to=numpy",
        "--nofollow-import-to=pandas",
        
        # Show progress
        "--show-progress",
        "--show-memory",
        
        "--remove-output",        # Clean up build artifacts
        str(BACKEND_DIR / "fastapi_server.py")
    ]
    

    print(f"Building FastAPI server for {sys.platform} with Nuitka...")
    print(f"Output file: {output_name}")
    print(f"Command: {' '.join(cmd)}")
    
    # Run the Nuitka build
    result = subprocess.run(cmd, cwd=BACKEND_DIR)
    
    if result.returncode != 0:
        print("\nNuitka build failed!")
        return False
        
    # Check if the output file was created
    output_file = DIST_DIR / output_name
    
    if output_file.exists():
        print(f"Successfully created: {output_name}")
        return True
    else:
        print(f"Expected output file not found: {output_file}")
        return False

def main():
    print("\nFastAPI Sidecar Build - Nuitka Edition")
    print("=========================================")
    
    start_time = time.time()
    
    success = build_with_nuitka()
    
    end_time = time.time()
    build_time = end_time - start_time
    
    # Get final binary name (what Tauri expects)
    if sys.platform == "win32":
        output_name = "fastapi-server.exe"
    else:
        output_name = "fastapi-server"
    
    final_path = DIST_DIR / output_name
    
    if success and final_path.exists():
        file_size_mb = final_path.stat().st_size / (1024 * 1024)
        print("\nBuild completed successfully!")
        print("=========================================")
        print(f"Binary: {final_path}")
        print(f"Size: {file_size_mb:.1f} MB")
        print(f"Build time: {build_time:.1f} seconds")
        print(f"Ready for Tauri sidecar usage")
        return 0
    else:
        print(f"\nBuild failed or binary not found at {final_path}")
        return 1

if __name__ == "__main__":
    sys.exit(main())