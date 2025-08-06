"""
ERF (Encapsulated Resource File) Parser for NWN2
Handles .mod, .hak, .erf files which are archives containing game resources
Based on the Java NWN2 Editor's ResourceDatabase implementation
"""

import struct
import os
from typing import Dict, List, Optional, BinaryIO, Any, Tuple
from dataclasses import dataclass
from pathlib import Path
import logging

logger = logging.getLogger(__name__)


@dataclass
class ERFHeader:
    """ERF file header information"""
    file_type: str  # "MOD ", "HAK ", "ERF "
    version: str    # "V1.0" or "V1.1"
    localized_string_count: int
    localized_string_size: int
    entry_count: int
    offset_to_localized_string: int
    offset_to_key_list: int
    offset_to_resource_list: int
    build_year: int
    build_day: int
    description_strref: int
    reserved: bytes


@dataclass
class ERFKey:
    """Resource key entry"""
    resref: str  # Resource name (up to 32 chars)
    resource_id: int
    res_type: int  # Resource type (2DA, TLK, etc)
    reserved: int


@dataclass
class ERFResource:
    """Resource data entry"""
    offset_to_resource: int
    resource_size: int


class ERFResourceType:
    """NWN2 Resource types - from Java ResourceDatabase"""
    # File type constants from the Java implementation
    FILE_TYPES = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 2000, 2001, 2002, 2003, 2005, 2007, 2008,
        2009, 2010, 2012, 2013, 2014, 2015, 2016, 2017, 2018, 2022, 2023, 2024, 2025,
        2026, 2027, 2029, 2030, 2031, 2032, 2033, 2034, 2035, 2036, 2037, 2038, 2039,
        2040, 2041, 2042, 2043, 2044, 2045, 2046, 2047, 2048, 2049, 2050, 2051, 2052,
        2053, 2054, 2055, 2056, 2057, 2058, 2059, 2060, 2061, 2062, 2063, 2064, 2065,
        2066, 2067, 3000, 3001, 3002, 3003, 3004, 3005, 3006, 3007, 3008, 3009, 3010,
        3011, 3012, 3013, 3014, 3015, 3016, 3017, 3018, 3019, 3020, 3021, 3022, 3033,
        3034, 3035, 4000, 4001, 4002, 4003, 4004, 4005, 4007, 4008, 9996, 9997, 9998, 9999
    ]
    
    FILE_EXTENSIONS = [
        "res", "bmp", "mve", "tga", "wav", "wfx", "plt", "ini", "mp3", "mpg", "txt",
        "plh", "tex", "mdl", "thg", "fnt", "lua", "slt", "nss", "ncs", "are", "set",
        "ifo", "bic", "wok", "2da", "tlk", "txi", "git", "bti", "uti", "btc", "utc",
        "dlg", "itp", "btt", "utt", "dds", "bts", "uts", "ltr", "gff", "fac", "bte",
        "ute", "btd", "utd", "btp", "utp", "dft", "gic", "gui", "css", "ccs", "btm",
        "utm", "dwk", "pwk", "btg", "utg", "jrl", "sav", "utw", "4pc", "ssf", "hak",
        "nwm", "bik", "ndb", "ptm", "ptt", "bak", "osc", "usc", "trn", "utr", "uen",
        "ult", "sef", "pfx", "cam", "lfx", "bfx", "upe", "ros", "rst", "ifx", "pfb",
        "zip", "wmp", "bbx", "tfx", "wlk", "xml", "scc", "ptx", "ltx", "trx", "mdb",
        "mda", "spt", "gr2", "fxa", "fxe", "jpg", "pwc", "isd", "erf", "bif", "key"
    ]
    
    # Common type constants for convenience
    TDA = 2017  # 2DA files
    IFO = 2014  # Module info  
    UTI = 2025  # Item template
    UTC = 2027  # Creature template
    GFF = 2037  # Generic GFF
    

class ERFParser:
    """Parser for ERF format files (MOD, HAK, ERF)"""
    
    def __init__(self):
        self.header: Optional[ERFHeader] = None
        self.keys: List[ERFKey] = []
        self.resources: List[ERFResource] = []
        self._file_handle: Optional[BinaryIO] = None
        self._file_path: Optional[str] = None
    
    def __enter__(self):
        """Context manager support"""
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager cleanup"""
        if self._file_handle:
            self._file_handle.close()
            self._file_handle = None
    
    def read(self, file_path: str) -> 'ERFParser':
        """Read and parse an ERF file"""
        self._file_path = file_path
        
        # Validate file exists and is readable
        if not os.path.exists(file_path):
            raise FileNotFoundError(f"ERF file not found: {file_path}")
        
        if not os.access(file_path, os.R_OK):
            raise PermissionError(f"Cannot read ERF file: {file_path}")
        
        # Check file size
        file_size = os.path.getsize(file_path)
        if file_size < 160:  # Minimum header size
            raise ValueError(f"File too small to be valid ERF: {file_path}")
        
        with open(file_path, 'rb') as f:
            self._parse_header(f)
            self._parse_keys(f)
            self._parse_resources(f)
        
        logger.info(f"Parsed ERF file: {file_path}")
        logger.info(f"Type: {self.header.file_type}, Resources: {self.header.entry_count}")
        
        return self
    
    def _parse_header(self, f: BinaryIO):
        """Parse ERF header - matches Java implementation"""
        # ERF header is 160 bytes
        header_data = f.read(160)
        if len(header_data) != 160:
            raise ValueError(f"{self._file_path}: Header is too short")
        
        # Validate file type signature
        try:
            file_type = header_data[0:4].decode('ascii')
            version = header_data[4:8].decode('ascii')
        except UnicodeDecodeError:
            raise ValueError(f"{self._file_path}: Invalid file signature")
            
        if file_type not in ['MOD ', 'HAK ', 'ERF ']:
            raise ValueError(f"{self._file_path}: Unknown file type '{file_type}'")
        
        # Read header fields in the same order as Java
        self.header = ERFHeader(
            file_type=file_type,
            version=version,
            localized_string_count=self._get_integer(header_data, 8),
            localized_string_size=self._get_integer(header_data, 12),
            entry_count=self._get_integer(header_data, 16),
            offset_to_localized_string=self._get_integer(header_data, 20),
            offset_to_key_list=self._get_integer(header_data, 24),
            offset_to_resource_list=self._get_integer(header_data, 28),
            build_year=self._get_short(header_data, 32),
            build_day=self._get_short(header_data, 34),
            description_strref=self._get_integer(header_data, 36),
            reserved=header_data[40:160]
        )
        
        # Validate version like Java does
        if self.header.version not in ["V1.0", "V1.1"]:
            raise ValueError(f"{self._file_path}: File version is not 1.0 or 1.1")
        
        if self.header.entry_count == 0:
            raise ValueError(f"{self._file_path}: No resource entries")
    
    def _parse_keys(self, f: BinaryIO):
        """Parse resource keys - matches Java implementation"""
        # Determine key length based on version
        if self.header.version == "V1.0":
            key_length = 24
            name_length = 16
        else:  # V1.1
            key_length = 40
            name_length = 32
        
        # Only seek if we're reading from a real file
        if hasattr(f, 'name') or self.header.offset_to_key_list > 0:
            f.seek(self.header.offset_to_key_list)
        
        for i in range(self.header.entry_count):
            key_data = f.read(key_length)
            if len(key_data) != key_length:
                raise ValueError(f"{self._file_path}: Premature end-of-data while reading entry keys")
            
            # Extract null-terminated string like Java does
            name_end = 0
            for j in range(name_length):
                if key_data[j] == 0:
                    name_end = j
                    break
            else:
                name_end = name_length
            
            try:
                resref = key_data[:name_end].decode('ascii')
            except UnicodeDecodeError:
                raise ValueError(f"{self._file_path}: Invalid ASCII in resource name at index {i}")
            
            # Parse remaining fields
            res_type = self._get_short(key_data, name_length + 4)
            
            # Handle extension based on file type
            if res_type == 0xFFFF:  # Special case
                entry_name = resref
            else:
                # Find extension from type
                ext = None
                try:
                    type_index = ERFResourceType.FILE_TYPES.index(res_type)
                    ext = ERFResourceType.FILE_EXTENSIONS[type_index]
                except ValueError:
                    raise ValueError(f"{self._file_path}: File type {res_type} is not recognized")
                entry_name = f"{resref}.{ext}"
            
            self.keys.append(ERFKey(
                resref=entry_name,
                resource_id=self._get_integer(key_data, name_length) if self.header.version == "V1.0" else i,
                res_type=res_type,
                reserved=0
            ))
    
    def _parse_resources(self, f: BinaryIO):
        """Parse resource entries"""
        # Only seek if we're reading from a real file
        if hasattr(f, 'name') or self.header.offset_to_resource_list > 0:
            f.seek(self.header.offset_to_resource_list)
        
        for i in range(self.header.entry_count):
            # Each resource entry is 8 bytes
            offset, size = struct.unpack('<II', f.read(8))
            
            self.resources.append(ERFResource(
                offset_to_resource=offset,
                resource_size=size
            ))
    
    def list_resources(self, resource_type: Optional[int] = None) -> List[Dict[str, Any]]:
        """List all resources, optionally filtered by type"""
        resources = []
        
        for i, key in enumerate(self.keys):
            if resource_type is None or key.res_type == resource_type:
                resources.append({
                    'name': key.resref,
                    'type': key.res_type,
                    'type_name': self._get_type_name(key.res_type),
                    'size': self.resources[i].resource_size,
                    'index': i
                })
        
        return resources
    
    def extract_resource(self, resource_name: str, output_path: Optional[str] = None) -> bytes:
        """Extract a specific resource by name"""
        if not self._file_path:
            raise RuntimeError("No ERF file loaded")
            
        # Find the resource
        resource_index = None
        for i, key in enumerate(self.keys):
            if key.resref.lower() == resource_name.lower():
                resource_index = i
                break
        
        if resource_index is None:
            raise ValueError(f"Resource '{resource_name}' not found")
        
        # Validate resource data
        resource = self.resources[resource_index]
        file_size = os.path.getsize(self._file_path)
        
        if resource.offset_to_resource + resource.resource_size > file_size:
            raise ValueError(f"Resource data extends beyond file boundary for '{resource_name}'")
        
        # Read the resource data
        with open(self._file_path, 'rb') as f:
            f.seek(resource.offset_to_resource)
            data = f.read(resource.resource_size)
            
            if len(data) != resource.resource_size:
                raise IOError(f"Failed to read complete resource data for '{resource_name}'")
        
        # Save to file if requested
        if output_path:
            # Create output directory if needed
            output_dir = os.path.dirname(output_path)
            if output_dir:
                os.makedirs(output_dir, exist_ok=True)
                
            with open(output_path, 'wb') as out:
                out.write(data)
            logger.info(f"Extracted {resource_name} to {output_path}")
        
        return data
    
    def extract_all_2da(self, output_dir: str) -> List[str]:
        """Extract all 2DA files from the ERF"""
        extracted = []
        os.makedirs(output_dir, exist_ok=True)
        
        for resource in self.list_resources(resource_type=ERFResourceType.TDA):
            name = resource['name']
            output_path = os.path.join(output_dir, f"{name}.2da")
            self.extract_resource(name, output_path)
            extracted.append(output_path)
        
        return extracted
    
    def get_module_info(self) -> Optional[Dict[str, Any]]:
        """Get module.ifo data if this is a module file"""
        if self.header.file_type != "MOD ":
            return None
        
        # Look for module.ifo
        for resource in self.list_resources(resource_type=ERFResourceType.IFO):
            if resource['name'].lower() == 'module.ifo':
                # Extract and parse the IFO (it's a GFF file)
                data = self.extract_resource(resource['name'])
                # Would parse with GFFParser here
                return {'has_module_ifo': True, 'size': len(data)}
        
        return None
    
    def _get_type_name(self, res_type: int) -> str:
        """Get human-readable resource type name"""
        try:
            type_index = ERFResourceType.FILE_TYPES.index(res_type)
            return ERFResourceType.FILE_EXTENSIONS[type_index].upper()
        except ValueError:
            return f"Type_{res_type}"
    
    def _get_short(self, buffer: bytes, offset: int) -> int:
        """Read little-endian short from buffer - matches Java"""
        return (buffer[offset] & 0xFF) | ((buffer[offset + 1] & 0xFF) << 8)
    
    def _get_integer(self, buffer: bytes, offset: int) -> int:
        """Read little-endian integer from buffer - matches Java"""
        return ((buffer[offset] & 0xFF) | 
                ((buffer[offset + 1] & 0xFF) << 8) |
                ((buffer[offset + 2] & 0xFF) << 16) |
                ((buffer[offset + 3] & 0xFF) << 24))


class HakpakReader:
    """Convenience class for reading hakpak files"""
    
    def __init__(self, nwn2_path: Optional[Path] = None):
        self.nwn2_path = nwn2_path
        self.parser = ERFParser()
    
    def read_hakpak(self, hakpak_name: str) -> ERFParser:
        """Read a hakpak file by name"""
        # Try to find the hakpak
        hakpak_path = self._find_hakpak(hakpak_name)
        if not hakpak_path:
            raise FileNotFoundError(f"Hakpak '{hakpak_name}' not found")
        
        return self.parser.read(str(hakpak_path))
    
    def extract_2da_files(self, hakpak_name: str, output_dir: str) -> List[str]:
        """Extract all 2DA files from a hakpak"""
        self.read_hakpak(hakpak_name)
        return self.parser.extract_all_2da(output_dir)
    
    def _find_hakpak(self, hakpak_name: str) -> Optional[Path]:
        """Find hakpak file in standard locations"""
        if not hakpak_name.endswith('.hak'):
            hakpak_name += '.hak'
        
        # Check user documents first
        user_hak = Path.home() / 'Documents' / 'Neverwinter Nights 2' / 'hak' / hakpak_name
        if user_hak.exists():
            return user_hak
        
        # Check NWN2 installation if provided
        if self.nwn2_path:
            install_hak = self.nwn2_path / 'hak' / hakpak_name
            if install_hak.exists():
                return install_hak
        
        # Check current directory as fallback
        local_hak = Path(hakpak_name)
        if local_hak.exists():
            return local_hak
        
        return None