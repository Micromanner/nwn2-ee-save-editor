#!/usr/bin/env python3
"""
Build FastAPI server as a standalone executable using Nuitka.
Fixed for proper Tauri sidecar integration.
"""
import os
import sys
import shutil
import subprocess
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
        "--python-flag=no_docstrings",

        "--include-package=fastapi",
        "--include-package=uvicorn",
        "--include-package=pydantic",
        "--include-package=starlette",
        "--include-package=fastapi_routers",
        "--include-package=fastapi_models",
        "--include-package=config",
        "--include-package=character",
        "--include-package=gamedata",
        "--include-package=nwn2_rust",
        "--include-package=services",
        "--include-package=utils",

        # Anti-bloat plugin options
        "--noinclude-setuptools-mode=nofollow",
        "--noinclude-pytest-mode=nofollow",
        "--noinclude-unittest-mode=nofollow",
        "--noinclude-pydoc-mode=nofollow",
        "--noinclude-IPython-mode=nofollow",

        # Custom exclusions for unused stdlib/packages
        "--noinclude-custom-mode=xml:nofollow",
        "--noinclude-custom-mode=xmlrpc:nofollow",
        "--noinclude-custom-mode=ftplib:nofollow",
        "--noinclude-custom-mode=imaplib:nofollow",
        "--noinclude-custom-mode=poplib:nofollow",
        "--noinclude-custom-mode=smtplib:nofollow",
        "--noinclude-custom-mode=cgi:nofollow",
        "--noinclude-custom-mode=cgitb:nofollow",
        "--noinclude-custom-mode=tarfile:nofollow",
        "--noinclude-custom-mode=webbrowser:nofollow",
        "--noinclude-custom-mode=pydoc:nofollow",
        "--noinclude-custom-mode=doctest:nofollow",
        "--noinclude-custom-mode=argparse:nofollow",
        "--noinclude-custom-mode=difflib:nofollow",
        "--noinclude-custom-mode=filecmp:nofollow",
        "--noinclude-custom-mode=fileinput:nofollow",
        "--noinclude-custom-mode=netrc:nofollow",
        "--noinclude-custom-mode=pipes:nofollow",
        "--noinclude-custom-mode=telnetlib:nofollow",
        "--noinclude-custom-mode=uu:nofollow",
        "--noinclude-custom-mode=xdrlib:nofollow",
        "--noinclude-custom-mode=chunk:nofollow",
        "--noinclude-custom-mode=colorsys:nofollow",
        "--noinclude-custom-mode=imghdr:nofollow",
        "--noinclude-custom-mode=sndhdr:nofollow",
        "--noinclude-custom-mode=sunau:nofollow",
        "--noinclude-custom-mode=wave:nofollow",
        "--noinclude-custom-mode=aifc:nofollow",
        "--noinclude-custom-mode=cmd:nofollow",
        "--noinclude-custom-mode=code:nofollow",
        "--noinclude-custom-mode=codeop:nofollow",
        "--noinclude-custom-mode=pdb:nofollow",
        "--noinclude-custom-mode=profile:nofollow",
        "--noinclude-custom-mode=pstats:nofollow",
        "--noinclude-custom-mode=timeit:nofollow",
        "--noinclude-custom-mode=trace:nofollow",
        "--noinclude-custom-mode=tracemalloc:nofollow",
        "--noinclude-custom-mode=symtable:nofollow",
        "--noinclude-custom-mode=tabnanny:nofollow",
        "--noinclude-custom-mode=py_compile:nofollow",
        "--noinclude-custom-mode=compileall:nofollow",
        "--noinclude-custom-mode=dis:nofollow",
        "--noinclude-custom-mode=pickletools:nofollow",
        "--noinclude-custom-mode=mailbox:nofollow",
        "--noinclude-custom-mode=mailcap:nofollow",
        "--noinclude-custom-mode=mimetypes:nofollow",
        "--noinclude-custom-mode=nntplib:nofollow",
        "--noinclude-custom-mode=optparse:nofollow",
        "--noinclude-custom-mode=getpass:nofollow",
        "--noinclude-custom-mode=tty:nofollow",
        "--noinclude-custom-mode=pty:nofollow",
        "--noinclude-custom-mode=termios:nofollow",
        "--noinclude-custom-mode=curses:nofollow",
        "--noinclude-custom-mode=readline:nofollow",
        "--noinclude-custom-mode=rlcompleter:nofollow",
        "--noinclude-custom-mode=fractions:nofollow",
        "--noinclude-custom-mode=getopt:nofollow",
        "--noinclude-custom-mode=graphlib:nofollow",
        "--noinclude-custom-mode=keyword:nofollow",
        "--noinclude-custom-mode=modulefinder:nofollow",
        "--noinclude-custom-mode=numbers:nofollow",
        "--noinclude-custom-mode=opcode:nofollow",
        "--noinclude-custom-mode=pyclbr:nofollow",
        "--noinclude-custom-mode=quopri:nofollow",
        "--noinclude-custom-mode=reprlib:nofollow",
        "--noinclude-custom-mode=runpy:nofollow",
        "--noinclude-custom-mode=sched:nofollow",
        "--noinclude-custom-mode=shlex:nofollow",
        "--noinclude-custom-mode=statistics:nofollow",
        "--noinclude-custom-mode=stringprep:nofollow",
        "--noinclude-custom-mode=token:nofollow",
        "--noinclude-custom-mode=tokenize:nofollow",

        # Exclude packages not used
        "--nofollow-import-to=test",
        "--nofollow-import-to=tests",
        "--nofollow-import-to=pip",
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

        "--show-progress",
        "--show-memory",

        "--remove-output",
        str(BACKEND_DIR / "fastapi_server.py")
    ]

    if sys.platform == "win32":
        cmd.insert(-2, "--windows-console-mode=disable")
    

    print(f"Building FastAPI server for {sys.platform} with Nuitka...")
    print(f"Output file: {output_name}")
    print(f"Command: {' '.join(cmd)}")
    sys.stdout.flush()

    # Run the Nuitka build - stream all output live
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