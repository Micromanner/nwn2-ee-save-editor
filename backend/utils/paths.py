import os
import sys
from pathlib import Path

def get_writable_dir(sub_dir: str = "logs") -> Path:
    """Get a writable directory path, standardizing on AppData or local files."""
    # Robust check for Nuitka/Frozen environment
    is_frozen = (
        getattr(sys, "frozen", False) 
        or "__compiled__" in globals() 
        or os.path.basename(sys.executable).lower() in ["fastapi-server.exe", "fastapi-server"]
    )

    if is_frozen:
        app_data = os.getenv("LOCALAPPDATA") or os.getenv("APPDATA") or os.path.expanduser("~")
        base_dir = Path(app_data) / "NWN2EE Save Editor"
    else:
        # Resolve relative to the backend root (parent of 'utils')
        base_dir = Path(__file__).parent.parent
        
    target_dir = base_dir / sub_dir
    
    try:
        target_dir.mkdir(parents=True, exist_ok=True)
        # Test write access
        test_file = target_dir / ".write_test"
        test_file.touch()
        test_file.unlink()
        return target_dir
    except (PermissionError, OSError, IOError):
        # Fallback to AppData if local write fails
        if not is_frozen:
            app_data = os.getenv("LOCALAPPDATA") or os.getenv("APPDATA") or os.path.expanduser("~")
            target_dir = Path(app_data) / "NWN2EE Save Editor" / sub_dir
            target_dir.mkdir(parents=True, exist_ok=True)
            return target_dir
        raise
