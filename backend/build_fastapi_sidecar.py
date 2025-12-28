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
    # But PRESERVE the cache to speed up subsequent builds!
    print("Cleaning previous build artifacts...")
    for pattern in ["*.build", "*.dist", "*.onefile-build"]: # Removed .nuitka-cache from cleanup
        for path in DIST_DIR.glob(pattern):
            if path.is_dir():
                shutil.rmtree(path, ignore_errors=True)
                print(f"Removed directory: {path.name}")
            else:
                path.unlink(missing_ok=True)
                print(f"Removed file: {path.name}")
    
    if sys.platform == "win32":
        exe_name = "fastapi-server.exe"
        generic_dir = "fastapi_server.dist"
        target_specific_dir = "fastapi-server-x86_64-pc-windows-msvc"
    elif sys.platform == "darwin":
        raise RuntimeError("macOS not supported - NWN2:EE is not available for macOS")
    else:  # Linux
        exe_name = "fastapi-server"
        generic_dir = "fastapi_server.dist"
        target_specific_dir = "fastapi-server-x86_64-unknown-linux-gnu"
    
    # Build with generic directory name first
    output_name = exe_name

    # Skip the large icon cache directory since icons are not used anymore
    include_cache = False
    
    # Set up caching directory via environment variable
    cache_dir = DIST_DIR / ".nuitka-cache"
    cache_dir.mkdir(exist_ok=True)
    os.environ["NUITKA_CACHE_DIR"] = str(cache_dir)
    
    # Define the Nuitka command
    # Determine which Python to use (prefer venv)
    venv_python = BACKEND_DIR / "venv" / "Scripts" / "python.exe"
    if sys.platform == "win32" and venv_python.exists():
        python_exe = str(venv_python)
        print(f"Using venv Python: {python_exe}")
    elif (BACKEND_DIR / "venv" / "bin" / "python").exists():
        python_exe = str(BACKEND_DIR / "venv" / "bin" / "python")
        print(f"Using venv Python: {python_exe}")
    else:
        python_exe = sys.executable
        print(f"Using system Python: {python_exe}")
        
    cmd = [
        python_exe,
        "-m", "nuitka",
        f"--output-filename={output_name}",
        f"--output-dir={DIST_DIR}",
        "--standalone",
        "--assume-yes-for-downloads",
        "--lto=yes",
        "--follow-imports",

        "--python-flag=-OO",

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
        "--include-package=nwn2_rust",
        "--include-package=services",
        "--include-package=utils",

        "--nofollow-import-to=django",
        "--nofollow-import-to=pytest",
        "--nofollow-import-to=test",
        "--nofollow-import-to=tests",
        "--nofollow-import-to=pip",
        "--nofollow-import-to=setuptools",
        "--nofollow-import-to=wheel",
        "--nofollow-import-to=distutils",
        "--nofollow-import-to=tkinter",
        "--nofollow-import-to=matplotlib",
        "--nofollow-import-to=numpy",
        "--nofollow-import-to=pandas",
        "--nofollow-import-to=PIL",
        "--nofollow-import-to=Pillow",
        "--nofollow-import-to=maturin",
        "--nofollow-import-to=watchdog",
        "--nofollow-import-to=flake8",
        "--nofollow-import-to=pycodestyle",
        "--nofollow-import-to=pyflakes",
        "--nofollow-import-to=mccabe",
        "--nofollow-import-to=email",
        "--nofollow-import-to=xml",

        "--show-progress",
        "--show-memory",

        "--windows-console-mode=disable",

        "--remove-output",
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
        
    # Check if the output directory and executable were created
    output_dir = DIST_DIR / generic_dir
    output_file = output_dir / output_name
    
    if output_dir.exists() and output_file.exists():
        print(f"Successfully created: {generic_dir}/{output_name}")
        
        # Create target-specific copy for production builds
        target_dir = DIST_DIR / target_specific_dir
        if not target_dir.exists() or output_dir.stat().st_mtime > target_dir.stat().st_mtime:
            if target_dir.exists():
                shutil.rmtree(target_dir)
            shutil.copytree(output_dir, target_dir)
            print(f"Created target-specific binary: {target_specific_dir}")
        
        return True
    else:
        print(f"Expected output directory/file not found: {output_dir}/{output_name}")
        return False

def main():
    print("\nFastAPI Sidecar Build - Nuitka Edition")
    print("=========================================")
    
    start_time = time.time()
    
    success = build_with_nuitka()
    
    end_time = time.time()
    build_time = end_time - start_time
    
    if sys.platform == "win32":
        exe_name = "fastapi-server.exe"
        generic_dir = "fastapi_server.dist"
        target_specific_dir = "fastapi-server-x86_64-pc-windows-msvc"
    elif sys.platform == "darwin":
        raise RuntimeError("macOS not supported - NWN2:EE is not available for macOS")
    else:  # Linux
        exe_name = "fastapi-server"
        generic_dir = "fastapi_server.dist"
        target_specific_dir = "fastapi-server-x86_64-unknown-linux-gnu"
    
    final_dir = DIST_DIR / generic_dir
    final_path = final_dir / exe_name
    target_dir = DIST_DIR / target_specific_dir
    
    if success and final_path.exists():
        file_size_mb = final_path.stat().st_size / (1024 * 1024)
        print("\nBuild completed successfully!")
        print("=========================================")
        print(f"Generic binary: {final_path}")
        if target_dir.exists():
            print(f"Target-specific directory: {target_dir}")
        print(f"Executable size: {file_size_mb:.1f} MB")
        print(f"Build time: {build_time:.1f} seconds")
        print(f"Ready for Tauri sidecar usage")
        return 0
    else:
        print(f"\nBuild failed or binary not found at {final_path}")
        return 1

if __name__ == "__main__":
    sys.exit(main())