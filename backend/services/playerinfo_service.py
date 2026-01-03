
import struct
import os
from dataclasses import dataclass, field
from typing import List, Optional

@dataclass
class PlayerClassEntry:
    name: str = ""
    level: int = 0

@dataclass
class PlayerInfoData:
    first_name: str = ""
    last_name: str = "" # Note: playerinfo.bin usually stores full name as one string
    name: str = ""      # The actual string stored in file
    
    unknown1: int = 0
    subrace: str = ""
    alignment: str = ""
    unknown2: int = 1
    unknown3: int = 3
    unknown4: int = 3
    
    classes: List[PlayerClassEntry] = field(default_factory=list)
    
    deity: str = ""
    
    str: int = 10
    dex: int = 10
    con: int = 10
    int: int = 10
    wis: int = 10
    cha: int = 10
    
    unknown5: int = -1
    unknown6: int = 2
    unknown7: int = 2
    unknown8: int = 0
    unknown9: int = 6
    unknown10: int = 0

class PlayerInfo:
    @staticmethod
    def is_valid_file(file_path: str) -> bool:
        """
        Validates if the file appears to be a valid playerinfo.bin.
        Since there is no magic header, we check if the first field (Name) 
        has a reasonable length and if the file structure can be parsed.
        """
        if not os.path.exists(file_path) or os.path.getsize(file_path) < 4:
            return False

        try:
            with open(file_path, 'rb') as f:
                # Check name length (arbitrary sanity check: < 256 chars)
                name_len = struct.unpack('<I', f.read(4))[0]
                if name_len > 256:
                    return False
                
                # Try to parse the minimal structure to confirm it matches expected format
                f.seek(0)
                dummy = PlayerInfo()
                dummy._read_string(f) # Name
                struct.unpack('<I', f.read(4)) # Unknown1
                dummy._read_string(f) # Subrace
                dummy._read_string(f) # Alignment
                # If we got here without EOF error, it's likely valid
                return True
        except (struct.error, OSError, UnicodeDecodeError):
            return False

    @staticmethod
    def get_player_name(file_path: str, encoding: str = 'cp1252') -> str:
        """
        Fast extraction of just the player name (first field).
        Highly optimized for scanning many files.
        """
        if not os.path.exists(file_path):
            return ""
        try:
            with open(file_path, 'rb') as f:
                len_bytes = f.read(4)
                if not len_bytes: return ""
                length = struct.unpack('<I', len_bytes)[0]
                
                # Sanity checks
                if length == 0: return ""
                if length > 256: return "" # Name shouldn't be huge
                
                name_bytes = f.read(length)
                try:
                    return name_bytes.decode(encoding)
                except UnicodeDecodeError:
                    return name_bytes.decode('latin-1', errors='replace')
        except Exception:
            return ""

    def __init__(self, file_path: str = None, encoding: str = 'cp1252'):
        self.file_path = file_path
        self.encoding = encoding
        self.data = PlayerInfoData()
        
        if file_path and os.path.exists(file_path):
            self.load(file_path)

    def _safe_read_uint(self, f, size=4, signed=False) -> int:
        data = f.read(size)
        if len(data) < size:
            return 0
        fmt = '<i' if signed else '<I'
        if size == 1: fmt = 'b' if signed else 'B'
        return struct.unpack(fmt, data)[0]

    def _read_string(self, f) -> str:
        len_bytes = f.read(4)
        if len(len_bytes) < 4:
            return ""
        length = struct.unpack('<I', len_bytes)[0]
        if length == 0:
            return ""
        if length > 65536: return ""
        string_bytes = f.read(length)
        if len(string_bytes) < length: return ""
        try:
            return string_bytes.decode(self.encoding)
        except UnicodeDecodeError:
            return string_bytes.decode('latin-1', errors='replace')

    def _write_string(self, f, s: str):
        try:
            encoded = s.encode(self.encoding)
        except UnicodeEncodeError:
            # Fallback if character cannot be encoded in target codepage
            encoded = s.encode(self.encoding, errors='replace')
            
        f.write(struct.pack('<I', len(encoded)))
        f.write(encoded)

    def load(self, file_path: str):
        with open(file_path, 'rb') as f:
            # Read first name
            self.data.first_name = self._read_string(f)
            
            # Peek ahead to determine format variant
            # Format A: FirstName → LastName(string) → Subrace → Alignment
            # Format B: Name → Unknown1(int32) → Subrace → Alignment
            peek_pos = f.tell()
            first_4_bytes = f.read(4)
            
            if len(first_4_bytes) == 4:
                peek_value = struct.unpack('<I', first_4_bytes)[0]
                
                # Try to read as string to determine format
                # If we can read peek_value bytes and they're mostly printable, it's Format A
                if 2 <= peek_value <= 50:  # Reasonable name length
                    peek_string_bytes = f.read(peek_value)
                    # Check if it's mostly printable ASCII (typical for names)
                    printable_count = sum(1 for b in peek_string_bytes if 32 <= b < 127)
                    is_likely_string = len(peek_string_bytes) > 0 and printable_count >= len(peek_string_bytes) * 0.8
                    
                    # Reset to peek position
                    f.seek(peek_pos)
                    
                    if is_likely_string:
                        # Format A: FirstName → LastName → Subrace → Alignment
                        self.data.last_name = self._read_string(f)
                        self.data.name = f"{self.data.first_name} {self.data.last_name}".strip()
                        self.data.unknown1 = 0
                    else:
                        # Format B: Name → Unknown1 → Subrace → Alignment
                        self.data.name = self.data.first_name
                        self.data.last_name = ""
                        self.data.unknown1 = self._safe_read_uint(f)
                else:
                    # Large value or 0-1, likely Format B with Unknown1
                    f.seek(peek_pos)
                    self.data.name = self.data.first_name
                    self.data.last_name = ""
                    self.data.unknown1 = self._safe_read_uint(f)
            else:
                 # EOF reached or truncated
                 self.data.name = self.data.first_name
            
            self.data.subrace = self._read_string(f)
            self.data.alignment = self._read_string(f)
            
            self.data.unknown2 = self._safe_read_uint(f)
            self.data.unknown3 = self._safe_read_uint(f)
            self.data.unknown4 = self._safe_read_uint(f)
            
            class_count = self._safe_read_uint(f)
            self.data.classes = []
            for _ in range(class_count):
                cls_name = self._read_string(f)
                cls_level = self._safe_read_uint(f, 1)
                self.data.classes.append(PlayerClassEntry(cls_name, cls_level))
            
            self.data.deity = self._read_string(f)
            
            self.data.str = self._safe_read_uint(f)
            self.data.dex = self._safe_read_uint(f)
            self.data.con = self._safe_read_uint(f)
            self.data.int = self._safe_read_uint(f)
            self.data.wis = self._safe_read_uint(f)
            self.data.cha = self._safe_read_uint(f)
            
            # Read remainder of file to preserve unknown footer
            self.data.unknown5 = self._safe_read_uint(f, 4, signed=True) # -1 is signed int usually
            self.data.unknown6 = self._safe_read_uint(f)
            self.data.unknown7 = self._safe_read_uint(f)
            self.data.unknown8 = self._safe_read_uint(f)
            self.data.unknown9 = self._safe_read_uint(f)
            self.data.unknown10 = self._safe_read_uint(f)

    def save(self, file_path: str = None):
        target_path = file_path or self.file_path
        if not target_path:
            raise ValueError("No file path specified")

        with open(target_path, 'wb') as f:
            # Write first name
            self._write_string(f, self.data.first_name or self.data.name.split()[0] if self.data.name else "")
            
            # Determine format: if last_name exists, use Format A, else Format B
            if self.data.last_name:
                # Format A: FirstName → LastName → Subrace → Alignment
                self._write_string(f, self.data.last_name)
            else:
                # Format B: Name → Unknown1 → Subrace → Alignment
                f.write(struct.pack('<I', self.data.unknown1))
            
            self._write_string(f, self.data.subrace)
            self._write_string(f, self.data.alignment)
            
            f.write(struct.pack('<I', self.data.unknown2))
            f.write(struct.pack('<I', self.data.unknown3))
            f.write(struct.pack('<I', self.data.unknown4))
            
            f.write(struct.pack('<I', len(self.data.classes)))
            for cls in self.data.classes:
                self._write_string(f, cls.name)
                f.write(struct.pack('B', cls.level))
                
            self._write_string(f, self.data.deity)
            
            f.write(struct.pack('<I', self.data.str))
            f.write(struct.pack('<I', self.data.dex))
            f.write(struct.pack('<I', self.data.con))
            f.write(struct.pack('<I', self.data.int))
            f.write(struct.pack('<I', self.data.wis))
            f.write(struct.pack('<I', self.data.cha))
            
            f.write(struct.pack('<i', self.data.unknown5)) # -1 is signed int usually
            f.write(struct.pack('<I', self.data.unknown6))
            f.write(struct.pack('<I', self.data.unknown7))
            f.write(struct.pack('<I', self.data.unknown8))
            f.write(struct.pack('<I', self.data.unknown9))
            f.write(struct.pack('<I', self.data.unknown10))

    def update_from_gff_data(self, gff_data: dict, subrace_name: str, alignment_name: str, classes: List[tuple]):
        """
        Update fields from GFF data.
        gff_data: Dict containing the character Struct (e.g. from Mod_PlayerList element)
        subrace_name: Resolved string for subrace (from ID in GFF + 2DA lookup)
        alignment_name: Resolved string for alignment
        classes: List of (ClassName, Level) strings/ints
        """
        
        # 1. Update Name
        first = ""
        last = ""
        
        if 'FirstName' in gff_data:
            val = gff_data['FirstName']
            if isinstance(val, dict) and 'substrings' in val: # LocString
                # Simple fallback: first English substring or first available
                found = False
                for sub in val['substrings']:
                    if sub.get('language') == 0: # English
                        first = sub.get('string', "")
                        found = True
                        break
                if not found and val['substrings']:
                     first = val['substrings'][0].get('string', "")
            elif isinstance(val, str):
                first = val

        if 'LastName' in gff_data:
             val = gff_data['LastName']
             if isinstance(val, dict) and 'substrings' in val:
                found = False
                for sub in val['substrings']:
                    if sub.get('language') == 0:
                        last = sub.get('string', "")
                        found = True
                        break
                if not found and val['substrings']:
                     last = val['substrings'][0].get('string', "")
             elif isinstance(val, str):
                last = val

        self.data.first_name = first
        self.data.last_name = last
        self.data.name = f"{first} {last}".strip()

        # 2. Update Strings passed in
        self.data.subrace = subrace_name
        self.data.alignment = alignment_name
        
        # 3. Update Classes
        self.data.classes = []
        for cls_name, cls_lvl in classes:
            self.data.classes.append(PlayerClassEntry(cls_name, int(cls_lvl)))
            
        # 4. Update Deity
        if 'Deity' in gff_data:
            self.data.deity = str(gff_data['Deity'])
            
        # 5. Update Stats
        # GFF stores them as base stats usually. playerinfo.bin likely wants base stats too.
        if 'Str' in gff_data: self.data.str = int(gff_data['Str'])
        if 'Dex' in gff_data: self.data.dex = int(gff_data['Dex'])
        if 'Con' in gff_data: self.data.con = int(gff_data['Con'])
        if 'Int' in gff_data: self.data.int = int(gff_data['Int'])
        if 'Wis' in gff_data: self.data.wis = int(gff_data['Wis'])
        if 'Cha' in gff_data: self.data.cha = int(gff_data['Cha'])
