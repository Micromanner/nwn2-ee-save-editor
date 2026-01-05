"""Parser for playerinfo.bin binary format."""
import struct
import os
from dataclasses import dataclass, field
from typing import List
from loguru import logger


class PlayerInfoParseError(Exception):
    """Raised when playerinfo.bin parsing fails."""
    pass


@dataclass
class PlayerClassEntry:
    name: str = ""
    level: int = 0


@dataclass
class PlayerInfoData:
    first_name: str = ""
    last_name: str = ""
    name: str = ""

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
    """Parser for NWN2 playerinfo.bin files."""

    @staticmethod
    def is_valid_file(file_path: str) -> bool:
        """Check if file appears to be a valid playerinfo.bin."""
        if not os.path.exists(file_path) or os.path.getsize(file_path) < 4:
            return False

        try:
            with open(file_path, 'rb') as f:
                name_len = struct.unpack('<I', f.read(4))[0]
                if name_len > 256:
                    return False
                f.seek(0)
                dummy = PlayerInfo()
                dummy._read_string(f)
                struct.unpack('<I', f.read(4))
                dummy._read_string(f)
                dummy._read_string(f)
                return True
        except (struct.error, OSError, UnicodeDecodeError, PlayerInfoParseError):
            return False

    @staticmethod
    def get_player_name(file_path: str, encoding: str = 'cp1252') -> str:
        """Extract player name from playerinfo.bin."""
        if not os.path.exists(file_path):
            raise FileNotFoundError(f"File not found: {file_path}")
        with open(file_path, 'rb') as f:
            len_bytes = f.read(4)
            if len(len_bytes) < 4:
                raise PlayerInfoParseError(f"Truncated file: {file_path}")
            length = struct.unpack('<I', len_bytes)[0]
            if length == 0:
                raise PlayerInfoParseError(f"Empty name in: {file_path}")
            if length > 256:
                raise PlayerInfoParseError(f"Name length {length} exceeds maximum in: {file_path}")
            name_bytes = f.read(length)
            if len(name_bytes) < length:
                raise PlayerInfoParseError(f"Truncated name data in: {file_path}")
            try:
                return name_bytes.decode(encoding)
            except UnicodeDecodeError:
                return name_bytes.decode('latin-1', errors='replace')

    def __init__(self, file_path: str = None, encoding: str = 'cp1252'):
        self.file_path = file_path
        self.encoding = encoding
        self.data = PlayerInfoData()

        if file_path and os.path.exists(file_path):
            self.load(file_path)

    def _read_uint(self, f, size=4, signed=False) -> int:
        """Read unsigned/signed integer from file."""
        data = f.read(size)
        if len(data) < size:
            raise PlayerInfoParseError(f"Unexpected EOF reading {size}-byte integer")
        fmt = '<i' if signed else '<I'
        if size == 1:
            fmt = 'b' if signed else 'B'
        return struct.unpack(fmt, data)[0]

    def _read_string(self, f) -> str:
        """Read length-prefixed string from file."""
        len_bytes = f.read(4)
        if len(len_bytes) < 4:
            raise PlayerInfoParseError("Unexpected EOF reading string length")
        length = struct.unpack('<I', len_bytes)[0]
        if length == 0:
            return ""
        if length > 65536:
            raise PlayerInfoParseError(f"String length {length} exceeds maximum")
        string_bytes = f.read(length)
        if len(string_bytes) < length:
            raise PlayerInfoParseError(f"Unexpected EOF reading string data (expected {length} bytes)")
        try:
            return string_bytes.decode(self.encoding)
        except UnicodeDecodeError:
            return string_bytes.decode('latin-1', errors='replace')

    def _write_string(self, f, s: str):
        """Write length-prefixed string to file."""
        try:
            encoded = s.encode(self.encoding)
        except UnicodeEncodeError:
            encoded = s.encode(self.encoding, errors='replace')

        f.write(struct.pack('<I', len(encoded)))
        f.write(encoded)

    def load(self, file_path: str):
        """Load playerinfo.bin from file."""
        logger.debug(f"Loading playerinfo.bin: {file_path}")
        with open(file_path, 'rb') as f:
            self.data.first_name = self._read_string(f)

            peek_pos = f.tell()
            first_4_bytes = f.read(4)

            if len(first_4_bytes) == 4:
                peek_value = struct.unpack('<I', first_4_bytes)[0]

                if 2 <= peek_value <= 50:
                    peek_string_bytes = f.read(peek_value)
                    printable_count = sum(1 for b in peek_string_bytes if 32 <= b < 127)
                    is_likely_string = len(peek_string_bytes) > 0 and printable_count >= len(peek_string_bytes) * 0.8

                    f.seek(peek_pos)

                    if is_likely_string:
                        self.data.last_name = self._read_string(f)
                        self.data.name = f"{self.data.first_name} {self.data.last_name}".strip()
                        self.data.unknown1 = 0
                    else:
                        self.data.name = self.data.first_name
                        self.data.last_name = ""
                        self.data.unknown1 = self._read_uint(f)
                else:
                    f.seek(peek_pos)
                    self.data.name = self.data.first_name
                    self.data.last_name = ""
                    self.data.unknown1 = self._read_uint(f)
            else:
                raise PlayerInfoParseError("Truncated file after first name")

            self.data.subrace = self._read_string(f)
            self.data.alignment = self._read_string(f)

            self.data.unknown2 = self._read_uint(f)
            self.data.unknown3 = self._read_uint(f)
            self.data.unknown4 = self._read_uint(f)

            class_count = self._read_uint(f)
            self.data.classes = []
            for _ in range(class_count):
                cls_name = self._read_string(f)
                cls_level = self._read_uint(f, 1)
                self.data.classes.append(PlayerClassEntry(cls_name, cls_level))

            self.data.deity = self._read_string(f)

            self.data.str = self._read_uint(f)
            self.data.dex = self._read_uint(f)
            self.data.con = self._read_uint(f)
            self.data.int = self._read_uint(f)
            self.data.wis = self._read_uint(f)
            self.data.cha = self._read_uint(f)

            self.data.unknown5 = self._read_uint(f, 4, signed=True)
            self.data.unknown6 = self._read_uint(f)
            self.data.unknown7 = self._read_uint(f)
            self.data.unknown8 = self._read_uint(f)
            self.data.unknown9 = self._read_uint(f)
            self.data.unknown10 = self._read_uint(f)

    def save(self, file_path: str = None):
        """Save playerinfo.bin to file."""
        target_path = file_path or self.file_path
        if not target_path:
            raise ValueError("No file path specified")

        logger.debug(f"Saving playerinfo.bin: {target_path}")
        with open(target_path, 'wb') as f:
            self._write_string(f, self.data.first_name or self.data.name.split()[0] if self.data.name else "")

            if self.data.last_name:
                self._write_string(f, self.data.last_name)
            else:
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

            f.write(struct.pack('<i', self.data.unknown5))
            f.write(struct.pack('<I', self.data.unknown6))
            f.write(struct.pack('<I', self.data.unknown7))
            f.write(struct.pack('<I', self.data.unknown8))
            f.write(struct.pack('<I', self.data.unknown9))
            f.write(struct.pack('<I', self.data.unknown10))

    def update_from_gff_data(self, gff_data: dict, subrace_name: str, alignment_name: str, classes: List[tuple]):
        """Update fields from GFF character data."""
        first = ""
        last = ""

        if 'FirstName' in gff_data:
            val = gff_data['FirstName']
            if isinstance(val, dict) and 'substrings' in val:
                found = False
                for sub in val['substrings']:
                    if sub.get('language') == 0:
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

        self.data.subrace = subrace_name
        self.data.alignment = alignment_name

        self.data.classes = []
        for cls_name, cls_lvl in classes:
            self.data.classes.append(PlayerClassEntry(cls_name, int(cls_lvl)))

        if 'Deity' in gff_data:
            self.data.deity = str(gff_data['Deity'])

        if 'Str' in gff_data:
            self.data.str = int(gff_data['Str'])
        if 'Dex' in gff_data:
            self.data.dex = int(gff_data['Dex'])
        if 'Con' in gff_data:
            self.data.con = int(gff_data['Con'])
        if 'Int' in gff_data:
            self.data.int = int(gff_data['Int'])
        if 'Wis' in gff_data:
            self.data.wis = int(gff_data['Wis'])
        if 'Cha' in gff_data:
            self.data.cha = int(gff_data['Cha'])
