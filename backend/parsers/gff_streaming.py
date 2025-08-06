"""
Streaming GFF parser for memory-efficient processing of large files

This module provides a streaming parser that can process GFF files without loading
the entire file into memory. Useful for large module files or when processing
many files in parallel.
"""

import struct
from typing import Iterator, Optional, BinaryIO, Dict, Set, Callable, Any
from dataclasses import dataclass
from pathlib import Path
import io

from .gff import GFFFieldType, GFFElement, LocalizedString, LocalizedSubstring


@dataclass
class StreamingOptions:
    """Options for streaming GFF parser"""
    # Fields to load immediately vs stream
    eager_fields: Optional[Set[str]] = None  # Fields to load immediately
    lazy_fields: Optional[Set[str]] = None   # Fields to stream on demand
    
    # Memory limits
    max_field_data_memory: int = 10 * 1024 * 1024  # 10MB default
    max_string_length: int = 1024 * 1024  # 1MB max string
    
    # Callbacks
    field_filter: Optional[Callable[[str, int], bool]] = None  # Return True to include field
    struct_processor: Optional[Callable[[GFFElement], None]] = None  # Process structs as they're parsed


class LazyGFFElement(GFFElement):
    """GFF Element that loads data on demand"""
    
    def __init__(self, field_type: int, struct_id: int, label: str, 
                 file_path: str, offset: int, size: int, parser: 'StreamingGFFParser'):
        super().__init__(field_type, struct_id, label, None)
        self._file_path = file_path
        self._offset = offset
        self._size = size
        self._parser = parser
        self._loaded = False
        
    @property
    def value(self):
        """Load value on first access"""
        if not self._loaded:
            self._load_value()
        return self._value
        
    @value.setter
    def value(self, val):
        self._value = val
        self._loaded = True
        
    def _load_value(self):
        """Load the actual value from file"""
        with open(self._file_path, 'rb') as stream:
            stream.seek(self._offset)
            
            if self.type == GFFFieldType.STRING:
                string_length = struct.unpack('<I', stream.read(4))[0]
                self._value = stream.read(string_length).decode('utf-8', errors='ignore')
            elif self.type == GFFFieldType.VOID:
                void_length = struct.unpack('<I', stream.read(4))[0]
                self._value = stream.read(void_length)
            elif self.type == GFFFieldType.LOCSTRING:
                # Need to temporarily set stream in parser
                old_stream = self._parser._stream
                self._parser._stream = stream
                try:
                    self._value = self._parser._decode_localized_string_at(self._offset - self._parser.field_data_offset)
                finally:
                    self._parser._stream = old_stream
            else:
                # For other complex types, delegate to parser
                old_stream = self._parser._stream
                self._parser._stream = stream
                try:
                    self._value = self._parser._decode_field_data_at(self.type, self._offset - self._parser.field_data_offset)
                finally:
                    self._parser._stream = old_stream
                
            self._loaded = True


class StreamingGFFParser:
    """Memory-efficient streaming GFF parser"""
    
    def __init__(self, options: Optional[StreamingOptions] = None):
        self.options = options or StreamingOptions()
        self.file_type = ""
        self.file_version = ""
        
        # Stream reference and file path
        self._stream: Optional[BinaryIO] = None
        self._file_path: Optional[str] = None
        
        # Header information
        self.struct_offset = 0
        self.struct_count = 0
        self.field_offset = 0
        self.field_count = 0
        self.label_offset = 0
        self.label_count = 0
        self.field_data_offset = 0
        self.field_data_length = 0
        self.field_indices_offset = 0
        self.field_indices_length = 0
        self.list_indices_offset = 0
        self.list_indices_length = 0
        
        # Cached data (small arrays that we need frequently)
        self._label_cache: Optional[bytes] = None
        
    def parse_file(self, file_path: str) -> Iterator[GFFElement]:
        """Parse a GFF file in streaming mode, yielding top-level elements"""
        self._file_path = file_path
        with open(file_path, 'rb') as f:
            yield from self.parse_stream(f)
            
    def parse_stream(self, stream: BinaryIO) -> Iterator[GFFElement]:
        """Parse a GFF stream, yielding top-level elements"""
        self._stream = stream
        
        # Read and validate header
        self._read_header()
        
        # Cache labels if small enough
        if self.label_count * 16 < 1024 * 1024:  # Cache if < 1MB
            self._cache_labels()
            
        # Stream the top-level struct
        yield self._stream_struct("", 0)
        
    def parse_file_filtered(self, file_path: str, field_names: Set[str]) -> Dict[str, Any]:
        """Parse only specific fields from a GFF file"""
        result = {}
        self._file_path = file_path
        
        # Set up field filter
        original_filter = self.options.field_filter
        self.options.field_filter = lambda label, _: label in field_names
        
        try:
            with open(file_path, 'rb') as f:
                for element in self.parse_stream(f):
                    if element.type == GFFFieldType.STRUCT:
                        for field in element.value:
                            if field.label in field_names:
                                result[field.label] = field.value
        finally:
            self.options.field_filter = original_filter
            
        return result
        
    def _read_header(self):
        """Read and validate GFF header"""
        header = self._stream.read(56)
        if len(header) != 56:
            raise ValueError("GFF header is too short")
            
        # Parse header
        self.file_type = header[0:4].decode('ascii', errors='ignore').strip('\x00')
        self.file_version = header[4:8].decode('ascii', errors='ignore').strip('\x00')
        
        if self.file_version != "V3.2":
            raise ValueError(f"GFF version {self.file_version} is not supported")
            
        # Read offsets and counts
        self.struct_offset = struct.unpack('<I', header[8:12])[0]
        self.struct_count = struct.unpack('<I', header[12:16])[0]
        self.field_offset = struct.unpack('<I', header[16:20])[0]
        self.field_count = struct.unpack('<I', header[20:24])[0]
        self.label_offset = struct.unpack('<I', header[24:28])[0]
        self.label_count = struct.unpack('<I', header[28:32])[0]
        self.field_data_offset = struct.unpack('<I', header[32:36])[0]
        self.field_data_length = struct.unpack('<I', header[36:40])[0]
        self.field_indices_offset = struct.unpack('<I', header[40:44])[0]
        self.field_indices_length = struct.unpack('<I', header[44:48])[0]
        self.list_indices_offset = struct.unpack('<I', header[48:52])[0]
        self.list_indices_length = struct.unpack('<I', header[52:56])[0]
        
    def _cache_labels(self):
        """Cache the label array for faster access"""
        self._stream.seek(self.label_offset)
        self._label_cache = self._stream.read(self.label_count * 16)
        
    def _get_label(self, index: int) -> str:
        """Get a label by index"""
        if index >= self.label_count:
            raise ValueError(f"Label index {index} exceeds array size")
            
        if self._label_cache:
            # Use cached labels
            offset = index * 16
            label_bytes = self._label_cache[offset:offset+16]
        else:
            # Read from stream
            self._stream.seek(self.label_offset + index * 16)
            label_bytes = self._stream.read(16)
            
        return label_bytes.decode('ascii', errors='ignore').strip('\x00')
        
    def _stream_struct(self, label: str, struct_index: int) -> GFFElement:
        """Stream a struct from the file"""
        # Check bounds
        if struct_index >= self.struct_count:
            raise ValueError(f"Struct index {struct_index} exceeds struct count {self.struct_count}")
            
        # Read struct header
        self._stream.seek(self.struct_offset + struct_index * 12)
        struct_data = self._stream.read(12)
        
        if len(struct_data) < 12:
            raise ValueError(f"Incomplete struct data at index {struct_index}")
        
        struct_id = struct.unpack('<I', struct_data[0:4])[0]
        field_index = struct.unpack('<I', struct_data[4:8])[0]
        field_count = struct.unpack('<I', struct_data[8:12])[0]
        
        # Stream fields
        fields = []
        
        if field_count == 1:
            # field_index is the actual field index when count == 1
            field = self._stream_field(field_index)
            if field is not None:
                fields.append(field)
        elif field_count > 1:
            # field_index is byte offset into field indices array when count > 1
            offset = field_index
            for i in range(field_count):
                self._stream.seek(self.field_indices_offset + offset)
                field_idx = struct.unpack('<I', self._stream.read(4))[0]
                offset += 4
                field = self._stream_field(field_idx)
                if field is not None:
                    fields.append(field)
                    
        element = GFFElement(GFFFieldType.STRUCT, struct_id, label, fields)
        
        # Process struct if callback provided
        if self.options.struct_processor:
            self.options.struct_processor(element)
            
        return element
        
    def _stream_field(self, field_index: int) -> Optional[GFFElement]:
        """Stream a field from the file"""
        # Check bounds
        if field_index >= self.field_count:
            raise ValueError(f"Field index {field_index} exceeds field count {self.field_count}")
            
        # Read field header
        self._stream.seek(self.field_offset + field_index * 12)
        field_data = self._stream.read(12)
        
        if len(field_data) < 12:
            raise ValueError(f"Incomplete field data at index {field_index}")
        
        field_type = struct.unpack('<I', field_data[0:4])[0]
        label_index = struct.unpack('<I', field_data[4:8])[0]
        data_or_offset = struct.unpack('<I', field_data[8:12])[0]
        
        label = self._get_label(label_index)
        
        # Apply field filter if set
        if self.options.field_filter and not self.options.field_filter(label, field_type):
            return None
            
        # Check if this is a lazy field
        if self.options.lazy_fields and label in self.options.lazy_fields:
            # Return lazy element for large data types
            if field_type in [GFFFieldType.STRING, GFFFieldType.VOID, GFFFieldType.LOCSTRING]:
                if not self._file_path:
                    raise ValueError("Cannot create lazy element without file path")
                return LazyGFFElement(field_type, 0, label, self._file_path, 
                                    self.field_data_offset + data_or_offset, 0, self)
                                    
        # Handle different field types
        if field_type == GFFFieldType.STRUCT:
            return self._stream_struct(label, data_or_offset)
        elif field_type == GFFFieldType.LIST:
            return self._stream_list(label, data_or_offset)
        else:
            # For simple types, decode immediately
            value = self._decode_simple_field(field_type, data_or_offset)
            return GFFElement(field_type, 0, label, value)
            
    def _stream_list(self, label: str, offset: int) -> GFFElement:
        """Stream a list from the file"""
        # Read list count. The `offset` is the byte offset into the list indices data block.
        self._stream.seek(self.list_indices_offset + offset)
        struct_count = struct.unpack('<I', self._stream.read(4))[0]
        
        structs = []
        
        # The list of struct indices starts 4 bytes after the count.
        # We must manage our position in the list_indices array manually because the 
        # _stream_struct call will move the file pointer, invalidating sequential reads.
        list_index_offset = offset + 4
        for i in range(struct_count):
            self._stream.seek(self.list_indices_offset + list_index_offset)
            struct_index = struct.unpack('<I', self._stream.read(4))[0]
            
            struct_element = self._stream_struct("", struct_index)
            structs.append(struct_element)
            
            # Move to the next index in the list for the next iteration
            list_index_offset += 4
            
        return GFFElement(GFFFieldType.LIST, 0, label, structs)
        
    def _decode_simple_field(self, field_type: int, data_or_offset: int) -> Any:
        """Decode simple field types"""
        if field_type == GFFFieldType.BYTE:
            return data_or_offset & 0xFF
        elif field_type == GFFFieldType.CHAR:
            return chr(data_or_offset & 0xFF)
        elif field_type == GFFFieldType.WORD:
            return data_or_offset & 0xFFFF
        elif field_type == GFFFieldType.SHORT:
            return (data_or_offset & 0xFFFF) - (0x10000 if data_or_offset & 0x8000 else 0)
        elif field_type == GFFFieldType.DWORD:
            return data_or_offset
        elif field_type == GFFFieldType.INT:
            return data_or_offset if data_or_offset < 0x80000000 else data_or_offset - 0x100000000
        elif field_type == GFFFieldType.FLOAT:
            # Float is stored directly in the field
            return struct.unpack('<f', struct.pack('<I', data_or_offset))[0]
        else:
            # Complex types need field data access
            return self._decode_field_data_at(field_type, data_or_offset)
            
    def _decode_field_data_at(self, field_type: int, offset: int) -> Any:
        """Decode field data at specific offset"""
        self._stream.seek(self.field_data_offset + offset)
        
        if field_type == GFFFieldType.DWORD64:
            return struct.unpack('<Q', self._stream.read(8))[0]
        elif field_type == GFFFieldType.INT64:
            value = struct.unpack('<Q', self._stream.read(8))[0]
            return value if value < 0x8000000000000000 else value - 0x10000000000000000
        elif field_type == GFFFieldType.DOUBLE:
            return struct.unpack('<d', self._stream.read(8))[0]
        elif field_type == GFFFieldType.STRING:
            length = struct.unpack('<I', self._stream.read(4))[0]
            if length > self.options.max_string_length:
                raise ValueError(f"String too long: {length} bytes")
            return self._stream.read(length).decode('utf-8', errors='ignore')
        elif field_type == GFFFieldType.RESREF:
            length = struct.unpack('<B', self._stream.read(1))[0]
            return self._stream.read(length).decode('utf-8', errors='ignore')
        elif field_type == GFFFieldType.VOID:
            length = struct.unpack('<I', self._stream.read(4))[0]
            return self._stream.read(length)
        elif field_type == GFFFieldType.LOCSTRING:
            return self._decode_localized_string_at(offset)
        else:
            raise ValueError(f"Unknown field type: {field_type}")
            
    def _decode_localized_string_at(self, offset: int) -> LocalizedString:
        """Decode localized string at specific offset"""
        self._stream.seek(self.field_data_offset + offset)
        
        total_size = struct.unpack('<I', self._stream.read(4))[0]
        string_ref = struct.unpack('<I', self._stream.read(4))[0]
        substring_count = struct.unpack('<I', self._stream.read(4))[0]
        
        substrings = []
        for i in range(substring_count):
            string_id = struct.unpack('<I', self._stream.read(4))[0]
            substring_length = struct.unpack('<I', self._stream.read(4))[0]
            
            if substring_length > 0:
                substring = self._stream.read(substring_length).decode('utf-8', errors='ignore')
            else:
                substring = ""
                
            language = string_id // 2
            gender = string_id & 1
            substrings.append(LocalizedSubstring(substring, language, gender))
            
        return LocalizedString(string_ref, substrings)


def extract_character_name(bic_file: str) -> Optional[str]:
    """Example: Extract just the character name from a BIC file efficiently"""
    parser = StreamingGFFParser()
    result = parser.parse_file_filtered(bic_file, {"FirstName", "LastName"})
    
    first_name = ""
    last_name = ""
    
    if "FirstName" in result and isinstance(result["FirstName"], LocalizedString):
        if result["FirstName"].substrings:
            first_name = result["FirstName"].substrings[0].string
            
    if "LastName" in result and isinstance(result["LastName"], LocalizedString):
        if result["LastName"].substrings:
            last_name = result["LastName"].substrings[0].string
            
    if first_name or last_name:
        return f"{first_name} {last_name}".strip()
    return None


def count_module_areas(module_file: str) -> int:
    """Example: Count areas in a module without loading all area data"""
    parser = StreamingGFFParser()
    
    for element in parser.parse_file(module_file):
        # The top-level element is a struct containing module fields
        if element.type == GFFFieldType.STRUCT:
            for field in element.value:
                if field.label == "Mod_Area_list" and field.type == GFFFieldType.LIST:
                    return len(field.value)
    
    return 0