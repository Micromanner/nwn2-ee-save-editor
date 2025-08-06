import struct
from typing import Any, Dict, List, BinaryIO, Union, Optional, Tuple, Set
from enum import IntEnum
from dataclasses import dataclass
import io
import os
import tempfile
from concurrent.futures import ProcessPoolExecutor, as_completed


class GFFError(Exception):
    """Base exception for GFF parsing errors"""
    pass


class GFFVersionError(GFFError):
    """Raised when GFF version is not supported"""
    pass


class GFFCorruptedError(GFFError):
    """Raised when GFF file is corrupted or malformed"""
    pass


class GFFFieldType(IntEnum):
    BYTE = 0
    CHAR = 1
    WORD = 2
    SHORT = 3
    DWORD = 4
    INT = 5
    DWORD64 = 6
    INT64 = 7
    FLOAT = 8
    DOUBLE = 9
    STRING = 10
    RESREF = 11
    LOCSTRING = 12
    VOID = 13
    STRUCT = 14
    LIST = 15


@dataclass
class LocalizedSubstring:
    string: str
    language: int
    gender: int


@dataclass
class LocalizedString:
    string_ref: int
    substrings: List[LocalizedSubstring]


class GFFElement:
    """Represents a single element in the GFF structure"""
    def __init__(self, field_type: int, struct_id: int, label: str, value: Any):
        self.type = field_type
        self.id = struct_id
        self.label = label
        self.value = value
    
    def __repr__(self):
        return f"GFFElement(type={self.type}, label='{self.label}', value={self.value})"
    
    def get_field(self, label: str) -> Optional['GFFElement']:
        """Get a field by label (for structs)"""
        if self.type != GFFFieldType.STRUCT:
            return None
        for field in self.value:
            if field.label == label:
                return field
        return None
    
    def get_value(self, label: str, default=None) -> Any:
        """Get the value of a field by label"""
        field = self.get_field(label)
        return field.value if field else default

    def set_field(self, label: str, new_value: Any):
        """
        Finds a child field by its label and updates its value.
        This method can recursively update nested STRUCTs and LISTs of STRUCTs.
        Note: This method only *updates* existing fields. It does not create new ones.
        """
        if self.type != GFFFieldType.STRUCT:
            raise TypeError("Can only set fields on a STRUCT element.")

        field_to_update = self.get_field(label)
        if not field_to_update:
            # Silently ignore if the field does not exist in the original GFF structure.
            # This can happen if the manager adds a key that wasn't present before.
            return

        # Case 1: The field is a LIST, and the new value is a Python list (of dicts).
        if field_to_update.type == GFFFieldType.LIST and isinstance(new_value, list):
            # If the original list was empty, we can't infer the structure of its items.
            # This is a known limitation. We also can't do anything if the new list is empty.
            if not field_to_update.value or not new_value:
                field_to_update.value = []
                return

            template_item = field_to_update.value[0]
            rebuilt_list = []
            for item_dict in new_value:
                if not isinstance(item_dict, dict) or template_item.type != GFFFieldType.STRUCT:
                    continue  # Skip malformed data

                # "Clone" the structure of the template item (id, type, fields) without its values.
                cloned_item = GFFElement(template_item.type, template_item.id, template_item.label, [])
                for template_field in template_item.value:
                    # Both STRUCT and LIST values must be initialized as empty lists.
                    placeholder_value = [] if template_field.type in [GFFFieldType.STRUCT, GFFFieldType.LIST] else None
                    cloned_item.value.append(
                        GFFElement(template_field.type, 0, template_field.label, placeholder_value)
                    )

                # Populate the cloned structure with data from the dictionary.
                cloned_item.update_from_dict(item_dict)
                rebuilt_list.append(cloned_item)
            
            field_to_update.value = rebuilt_list

        # Case 2: The field is a STRUCT, and the new value is a Python dict.
        elif field_to_update.type == GFFFieldType.STRUCT and isinstance(new_value, dict):
            field_to_update.update_from_dict(new_value)

        # **THE FIX**: Add a case to reconstruct LocalizedString objects from dicts.
        elif field_to_update.type == GFFFieldType.LOCSTRING and isinstance(new_value, dict):
            substrings = [
                LocalizedSubstring(s['string'], s['language'], s['gender'])
                for s in new_value.get('substrings', [])
            ]
            field_to_update.value = LocalizedString(
                string_ref=new_value.get('string_ref', -1),
                substrings=substrings
            )

        # Case 3: Simple data type (int, str, etc.).
        else:
            field_to_update.value = new_value

    def update_from_dict(self, data: Dict[str, Any]):
        """Recursively updates the fields of a STRUCT element from a dictionary."""
        if self.type != GFFFieldType.STRUCT:
            return

        for key, value in data.items():
            self.set_field(key, value)
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert element to dictionary"""
        if self.type == GFFFieldType.STRUCT:
            result = {}
            if isinstance(self.value, list):
                for field in self.value:
                    result[field.label] = field.to_dict()
            else:
                # Single struct
                return self.value.to_dict()
            return result
        elif self.type == GFFFieldType.LIST:
            if isinstance(self.value, list):
                return [item.to_dict() for item in self.value]
            else:
                # Single item list
                return [self.value.to_dict()]
        elif self.type == GFFFieldType.LOCSTRING:
            return {
                'string_ref': self.value.string_ref,
                'substrings': [
                    {'string': s.string, 'language': s.language, 'gender': s.gender}
                    for s in self.value.substrings
                ]
            }
        else:
            # Handle bytes for VOID type
            if isinstance(self.value, bytes):
                return self.value.hex()
            return self.value


class GFFParser:
    """Parser for NWN2 GFF V3.2 binary format"""
    
    def __init__(self, error_recovery: bool = False, streaming_mode: bool = False):
        self.file_type = ""
        self.file_version = ""
        self.top_level_struct = None
        self.error_recovery = error_recovery
        self.recovery_errors = []
        self.streaming_mode = streaming_mode
        
        # Buffer storage (not used in streaming mode)
        self.struct_buffer = b''
        self.field_buffer = b''
        self.label_buffer = b''
        self.field_data_buffer = b''
        self.field_indices_buffer = b''
        self.list_indices_buffer = b''
        
        # Counts
        self.struct_count = 0
        self.field_count = 0
        self.label_count = 0
        self.field_data_length = 0
        self.field_indices_length = 0
        self.list_indices_length = 0
        
    def read(self, file_path: str) -> GFFElement:
        """Read and parse a GFF file"""
        with open(file_path, 'rb') as f:
            return self.load(f)
            
    def load(self, stream: BinaryIO) -> GFFElement:
        """Load GFF from a stream"""
        # Read header (56 bytes for GFF V3.2)
        header = stream.read(56)
        if len(header) != 56:
            raise GFFCorruptedError("GFF header is too short")
            
        # Parse header - preserve original file type (BIC, IFO, etc)
        self.file_type = header[0:4].decode('ascii', errors='ignore')
        self.file_version = header[4:8].decode('ascii', errors='ignore')
        
        if self.file_version not in ["V3.2", "V3.28"]:
            if self.error_recovery:
                self.recovery_errors.append(f"GFF version {self.file_version} is not standard V3.2/V3.28, attempting to parse anyway")
            else:
                raise GFFVersionError(f"GFF version {self.file_version} is not supported (expected V3.2 or V3.28)")
            
        # Read offsets and counts (starting at byte 8)
        struct_offset = self._get_int32(header, 8)
        self.struct_count = self._get_int32(header, 12)
        field_offset = self._get_int32(header, 16)
        self.field_count = self._get_int32(header, 20)
        label_offset = self._get_int32(header, 24)
        self.label_count = self._get_int32(header, 28)
        field_data_offset = self._get_int32(header, 32)
        self.field_data_length = self._get_int32(header, 36)
        field_indices_offset = self._get_int32(header, 40)
        self.field_indices_length = self._get_int32(header, 44)
        list_indices_offset = self._get_int32(header, 48)
        self.list_indices_length = self._get_int32(header, 52)
        
        if self.struct_count < 1:
            if self.error_recovery:
                self.recovery_errors.append("GFF file contains no structures")
                # Create a minimal empty structure to return
                self.top_level_struct = GFFElement(GFFFieldType.STRUCT, 0, "", [])
                return self.top_level_struct
            else:
                raise GFFCorruptedError("GFF file contains no structures")
            
        # Read all arrays using header offsets
        if self.struct_count > 0:
            stream.seek(struct_offset)
            self.struct_buffer = stream.read(12 * self.struct_count)
            
        if self.field_count > 0:
            stream.seek(field_offset)
            self.field_buffer = stream.read(12 * self.field_count)
            
        if self.label_count > 0:
            stream.seek(label_offset)
            self.label_buffer = stream.read(16 * self.label_count)
            
        if self.field_data_length > 0:
            stream.seek(field_data_offset)
            self.field_data_buffer = stream.read(self.field_data_length)
            
        if self.field_indices_length > 0:
            stream.seek(field_indices_offset)
            self.field_indices_buffer = stream.read(self.field_indices_length)
            
        if self.list_indices_length > 0:
            stream.seek(list_indices_offset)
            self.list_indices_buffer = stream.read(self.list_indices_length)
            
        # Decode top-level struct (always at index 0)
        self.top_level_struct = self._decode_struct("", 0)
        
        # Clear buffers to save memory
        self.struct_buffer = b''
        self.field_buffer = b''
        self.label_buffer = b''
        self.field_data_buffer = b''
        self.field_indices_buffer = b''
        self.list_indices_buffer = b''
        
        return self.top_level_struct
    
    def get_file_type(self) -> str:
        """Get the original file type (BIC, IFO, etc)"""
        return self.file_type
        
    def _get_int32(self, buffer: bytes, offset: int) -> int:
        """Read a little-endian 32-bit integer"""
        return struct.unpack_from('<I', buffer, offset)[0]
               
    def _get_int64(self, buffer: bytes, offset: int) -> int:
        """Read a little-endian 64-bit integer"""
        return struct.unpack_from('<Q', buffer, offset)[0]
        
    def _get_float(self, buffer: bytes, offset: int) -> float:
        """Read a 32-bit float"""
        return struct.unpack_from('<f', buffer, offset)[0]
        
    def _get_double(self, buffer: bytes, offset: int) -> float:
        """Read a 64-bit double"""
        return struct.unpack_from('<d', buffer, offset)[0]
        
    def _get_label(self, index: int) -> str:
        """Get a label from the label array"""
        if index >= self.label_count:
            if self.error_recovery:
                self.recovery_errors.append(f"Label index {index} exceeds array size {self.label_count}")
                return f"INVALID_LABEL_{index}"
            raise GFFCorruptedError(f"Label index {index} exceeds array size")
        offset = index * 16
        try:
            label_bytes = self.label_buffer[offset:offset+16]
            return label_bytes.decode('ascii', errors='ignore').strip('\x00')
        except Exception as e:
            if self.error_recovery:
                self.recovery_errors.append(f"Failed to decode label at index {index}: {str(e)}")
                return f"CORRUPT_LABEL_{index}"
            raise
    
    def _decode_field(self, index: int) -> Optional[GFFElement]:
        """Decode a field from the field array"""
        if index >= self.field_count:
            if self.error_recovery:
                self.recovery_errors.append(f"Field index {index} exceeds array size {self.field_count}")
                return None
            raise GFFCorruptedError(f"Field index {index} exceeds array size")
            
        try:
            offset = index * 12
            field_type = self._get_int32(self.field_buffer, offset)
            label_index = self._get_int32(self.field_buffer, offset + 4)
            data_offset = self._get_int32(self.field_buffer, offset + 8)
            
            label = self._get_label(label_index)
        except Exception as e:
            if self.error_recovery:
                self.recovery_errors.append(f"Failed to read field header at index {index}: {str(e)}")
                return None
            raise
        
        # For simple types, decode the value and then create the GFFElement.
        value: Any = None
        try:
            if field_type == GFFFieldType.STRUCT:
                return self._decode_struct(label, data_offset)
            elif field_type == GFFFieldType.LIST:
                return self._decode_list(label, data_offset)
            elif field_type == GFFFieldType.BYTE:
                value = data_offset & 0xFF
            elif field_type == GFFFieldType.CHAR:
                value = chr(data_offset & 0xFF)
            elif field_type == GFFFieldType.WORD:
                value = data_offset & 0xFFFF
            elif field_type == GFFFieldType.SHORT:
                value = (data_offset & 0xFFFF) - (0x10000 if data_offset & 0x8000 else 0)
            elif field_type == GFFFieldType.DWORD:
                value = data_offset
            elif field_type == GFFFieldType.INT:
                value = data_offset if data_offset < 0x80000000 else data_offset - 0x100000000
            elif field_type == GFFFieldType.DWORD64:
                if data_offset + 8 > self.field_data_length:
                    if self.error_recovery:
                        self.recovery_errors.append(f"DWORD64 field at offset {data_offset} exceeds field data buffer")
                        value = 0
                    else:
                        raise GFFCorruptedError(f"DWORD64 field at offset {data_offset} exceeds field data buffer")
                else:
                    value = self._get_int64(self.field_data_buffer, data_offset)
            elif field_type == GFFFieldType.INT64:
                if data_offset + 8 > self.field_data_length:
                    if self.error_recovery:
                        self.recovery_errors.append(f"INT64 field at offset {data_offset} exceeds field data buffer")
                        value = 0
                    else:
                        raise GFFCorruptedError(f"INT64 field at offset {data_offset} exceeds field data buffer")
                else:
                    value = self._get_int64(self.field_data_buffer, data_offset)
                    if value >= 0x8000000000000000:
                        value -= 0x10000000000000000
            elif field_type == GFFFieldType.FLOAT:
                value = self._get_float(self.field_buffer, offset + 8)
            elif field_type == GFFFieldType.DOUBLE:
                if data_offset + 8 > self.field_data_length:
                    if self.error_recovery:
                        self.recovery_errors.append(f"DOUBLE field at offset {data_offset} exceeds field data buffer")
                        value = 0.0
                    else:
                        raise GFFCorruptedError(f"DOUBLE field at offset {data_offset} exceeds field data buffer")
                else:
                    value = self._get_double(self.field_data_buffer, data_offset)
            elif field_type == GFFFieldType.STRING:
                if data_offset + 4 > self.field_data_length:
                    if self.error_recovery:
                        self.recovery_errors.append(f"STRING length at offset {data_offset} exceeds field data buffer")
                        value = ""
                    else:
                        raise GFFCorruptedError(f"STRING length at offset {data_offset} exceeds field data buffer")
                else:
                    string_length = self._get_int32(self.field_data_buffer, data_offset)
                    data_offset += 4
                    if data_offset + string_length > self.field_data_length:
                        if self.error_recovery:
                            self.recovery_errors.append(f"STRING data at offset {data_offset} exceeds field data buffer")
                            value = self.field_data_buffer[data_offset:self.field_data_length].decode('utf-8', errors='ignore')
                        else:
                            raise GFFCorruptedError(f"STRING data at offset {data_offset} exceeds field data buffer")
                    else:
                        value = self.field_data_buffer[data_offset:data_offset+string_length].decode('utf-8', errors='ignore')
            elif field_type == GFFFieldType.RESREF:
                if data_offset + 1 > self.field_data_length:
                    if self.error_recovery:
                        self.recovery_errors.append(f"RESREF length at offset {data_offset} exceeds field data buffer")
                        value = ""
                    else:
                        raise GFFCorruptedError(f"RESREF length at offset {data_offset} exceeds field data buffer")
                else:
                    res_length = struct.unpack_from('<B', self.field_data_buffer, data_offset)[0]
                    data_offset += 1
                    if data_offset + res_length > self.field_data_length:
                        if self.error_recovery:
                            self.recovery_errors.append(f"RESREF data at offset {data_offset} exceeds field data buffer")
                            value = self.field_data_buffer[data_offset:self.field_data_length].decode('utf-8', errors='ignore')
                        else:
                            raise GFFCorruptedError(f"RESREF data at offset {data_offset} exceeds field data buffer")
                    else:
                        value = self.field_data_buffer[data_offset:data_offset+res_length].decode('utf-8', errors='ignore')
            elif field_type == GFFFieldType.LOCSTRING:
                value = self._decode_localized_string(data_offset)
            elif field_type == GFFFieldType.VOID:
                if data_offset + 4 > self.field_data_length:
                    if self.error_recovery:
                        self.recovery_errors.append(f"VOID length at offset {data_offset} exceeds field data buffer")
                        value = b''
                    else:
                        raise GFFCorruptedError(f"VOID length at offset {data_offset} exceeds field data buffer")
                else:
                    void_length = self._get_int32(self.field_data_buffer, data_offset)
                    data_offset += 4
                    if data_offset + void_length > self.field_data_length:
                        if self.error_recovery:
                            self.recovery_errors.append(f"VOID data at offset {data_offset} exceeds field data buffer")
                            value = self.field_data_buffer[data_offset:self.field_data_length]
                        else:
                            raise GFFCorruptedError(f"VOID data at offset {data_offset} exceeds field data buffer")
                    else:
                        value = self.field_data_buffer[data_offset:data_offset+void_length]
            else:
                if self.error_recovery:
                    self.recovery_errors.append(f"Unrecognized field type {field_type}")
                    value = None
                else:
                    raise GFFError(f"Unrecognized field type {field_type}")
            
            return GFFElement(field_type, 0, label, value)
        except Exception as e:
            if self.error_recovery:
                self.recovery_errors.append(f"Failed to decode field '{label}' of type {field_type}: {str(e)}")
                return GFFElement(field_type, 0, label, None)
            raise
        
    def _decode_struct(self, label: str, index: int) -> Optional[GFFElement]:
        """Decode a struct from the struct array"""
        if index >= self.struct_count:
            if self.error_recovery:
                self.recovery_errors.append(f"Structure index {index} exceeds array size {self.struct_count}")
                return GFFElement(GFFFieldType.STRUCT, 0, label, [])
            raise GFFCorruptedError(f"Structure index {index} exceeds array size")
            
        try:
            offset = index * 12
            struct_id = self._get_int32(self.struct_buffer, offset)
            field_index = self._get_int32(self.struct_buffer, offset + 4)
            field_count = self._get_int32(self.struct_buffer, offset + 8)
            
            fields = []
            
            if field_count == 1:
                field = self._decode_field(field_index)
                if field is not None:
                    fields.append(field)
            elif field_count > 1:
                # If high bit is set, it's an offset into field indices array
                # Field index is already a byte offset, no conversion needed
                offset = field_index
                for i in range(field_count):
                    if offset + 4 > self.field_indices_length:
                        if self.error_recovery:
                            self.recovery_errors.append(f"Field indices at offset {offset} exceeds buffer size")
                            break
                        else:
                            raise GFFCorruptedError(f"Field indices at offset {offset} exceeds buffer size")
                    field_idx = self._get_int32(self.field_indices_buffer, offset)
                    offset += 4
                    field = self._decode_field(field_idx)
                    if field is not None:
                        fields.append(field)
                    
            return GFFElement(GFFFieldType.STRUCT, struct_id, label, fields)
        except Exception as e:
            if self.error_recovery:
                self.recovery_errors.append(f"Failed to decode struct at index {index}: {str(e)}")
                return GFFElement(GFFFieldType.STRUCT, 0, label, [])
            raise
        
    def _decode_list(self, label: str, offset: int) -> Optional[GFFElement]:
        """Decode a list from the list indices"""
        try:
            if offset + 4 > self.list_indices_length:
                if self.error_recovery:
                    self.recovery_errors.append(f"List count at offset {offset} exceeds buffer size")
                    return GFFElement(GFFFieldType.LIST, 0, label, [])
                else:
                    raise GFFCorruptedError(f"List count at offset {offset} exceeds buffer size")
                    
            struct_count = self._get_int32(self.list_indices_buffer, offset)
            structs = []
            
            list_offset = offset + 4
            for i in range(struct_count):
                if list_offset + 4 > self.list_indices_length:
                    if self.error_recovery:
                        self.recovery_errors.append(f"List index at offset {list_offset} exceeds buffer size")
                        break
                    else:
                        raise GFFCorruptedError(f"List index at offset {list_offset} exceeds buffer size")
                struct_index = self._get_int32(self.list_indices_buffer, list_offset)
                list_offset += 4
                struct = self._decode_struct("", struct_index)
                if struct is not None:
                    structs.append(struct)
                
            # ALWAYS return a list, just like the Java version.
            # This removes the inconsistent data structure.
            return GFFElement(GFFFieldType.LIST, 0, label, structs)
        except Exception as e:
            if self.error_recovery:
                self.recovery_errors.append(f"Failed to decode list at offset {offset}: {str(e)}")
                return GFFElement(GFFFieldType.LIST, 0, label, [])
            raise
        
    def _decode_localized_string(self, offset: int) -> Optional[LocalizedString]:
        """Decode a localized string"""
        try:
            if offset + 12 > self.field_data_length:
                if self.error_recovery:
                    self.recovery_errors.append(f"LocalizedString header at offset {offset} exceeds field data buffer")
                    return LocalizedString(-1, [])
                else:
                    raise GFFCorruptedError(f"LocalizedString header at offset {offset} exceeds field data buffer")
                    
            total_size = self._get_int32(self.field_data_buffer, offset)
            string_ref = self._get_int32(self.field_data_buffer, offset + 4)
            substring_count = self._get_int32(self.field_data_buffer, offset + 8)
            
            offset += 12
            substrings = []
            
            for i in range(substring_count):
                if offset + 8 > self.field_data_length:
                    if self.error_recovery:
                        self.recovery_errors.append(f"LocalizedString substring header at offset {offset} exceeds buffer")
                        break
                    else:
                        raise GFFCorruptedError(f"LocalizedString substring header at offset {offset} exceeds buffer")
                        
                string_id = self._get_int32(self.field_data_buffer, offset)
                substring_length = self._get_int32(self.field_data_buffer, offset + 4)
                offset += 8
                
                if offset + substring_length > self.field_data_length:
                    if self.error_recovery:
                        self.recovery_errors.append(f"LocalizedString substring data at offset {offset} exceeds buffer")
                        substring = self.field_data_buffer[offset:self.field_data_length].decode('utf-8', errors='ignore')
                        substring_length = self.field_data_length - offset
                    else:
                        raise GFFCorruptedError(f"LocalizedString substring data at offset {offset} exceeds buffer")
                else:
                    if substring_length > 0:
                        substring = self.field_data_buffer[offset:offset+substring_length].decode('utf-8', errors='ignore')
                    else:
                        substring = ""
                    
                language = string_id // 2
                gender = string_id & 1
                substrings.append(LocalizedSubstring(substring, language, gender))
                offset += substring_length
                
            return LocalizedString(string_ref, substrings)
        except Exception as e:
            if self.error_recovery:
                self.recovery_errors.append(f"Failed to decode localized string at offset {offset}: {str(e)}")
                return LocalizedString(-1, [])
            raise
    
    def get_recovery_errors(self) -> List[str]:
        """Get list of recovery errors encountered during parsing"""
        return self.recovery_errors.copy()
    
    @classmethod
    def create_streaming_parser(cls, **kwargs):
        """Create a streaming parser instance
        
        Args:
            **kwargs: Arguments to pass to StreamingGFFParser
            
        Returns:
            StreamingGFFParser instance
        """
        from .gff_streaming import StreamingGFFParser, StreamingOptions
        
        options = kwargs.pop('options', None)
        if options is None and kwargs:
            options = StreamingOptions(**kwargs)
            
        return StreamingGFFParser(options)
    
    @staticmethod
    def parse_file_streaming(file_path: str, field_names: Optional[Set[str]] = None, **kwargs):
        """Parse a GFF file in streaming mode
        
        Args:
            file_path: Path to GFF file
            field_names: Optional set of field names to extract (filters all others)
            **kwargs: Additional options for StreamingOptions
            
        Returns:
            If field_names provided: Dict of field values
            Otherwise: Iterator of GFFElement objects
        """
        from .gff_streaming import StreamingGFFParser, StreamingOptions
        
        options = StreamingOptions(**kwargs) if kwargs else None
        parser = StreamingGFFParser(options)
        
        if field_names:
            return parser.parse_file_filtered(file_path, field_names)
        else:
            return parser.parse_file(file_path)

    @staticmethod
    def _parse_gff_data_worker(filename: str, data_bytes: bytes) -> Dict[str, Any]:
        """
        Worker function for parallel GFF parsing
        Runs in separate process to parse GFF data from bytes
        """
        try:
            # Create temporary file from data
            with tempfile.NamedTemporaryFile(suffix='.gff', delete=False) as tmp:
                tmp.write(data_bytes)
                temp_path = tmp.name
            
            try:
                # Parse the GFF file
                parser = GFFParser()
                result = parser.read(temp_path)
                
                return {
                    'filename': filename,
                    'success': True,
                    'data': parser.top_level_struct.to_dict(),
                    'file_type': parser.file_type,
                    'parser': None,  # Can't serialize parser across processes
                    'error': None
                }
            finally:
                os.unlink(temp_path)
                
        except Exception as e:
            return {
                'filename': filename,
                'success': False,
                'data': None,
                'file_type': 'unknown',
                'parser': None,
                'error': str(e)
            }

    @classmethod
    def parse_multiple_parallel(cls, gff_files: List[Tuple[str, bytes]], max_workers: int = 4) -> Dict[str, Dict[str, Any]]:
        """
        Parse multiple GFF files in parallel using multiprocessing
        
        Args:
            gff_files: List of (filename, data_bytes) tuples
            max_workers: Number of processes to use (default: 4)
            
        Returns:
            Dict mapping filename to parsed data or error info
            
        Example:
            gff_files = [
                ('player.bic', bic_data),
                ('playerlist.ifo', ifo_data),
                ('companion.ros', ros_data)
            ]
            results = GFFParser.parse_multiple_parallel(gff_files)
            player_data = results['player.bic']['data']
        """
        results = {}
        
        if not gff_files:
            return results
        
        # For single files, don't use multiprocessing (overhead not worth it)
        if len(gff_files) == 1:
            filename, data_bytes = gff_files[0]
            result = cls._parse_gff_data_worker(filename, data_bytes)
            results[filename] = result
            return results
        
        # Use multiprocessing for multiple files
        with ProcessPoolExecutor(max_workers=max_workers) as executor:
            # Submit all parsing tasks
            future_to_filename = {
                executor.submit(cls._parse_gff_data_worker, filename, data): filename
                for filename, data in gff_files
            }
            
            # Collect results as they complete
            for future in as_completed(future_to_filename):
                filename = future_to_filename[future]
                try:
                    result = future.result()
                    results[filename] = result
                except Exception as e:
                    results[filename] = {
                        'filename': filename,
                        'success': False,
                        'data': None,
                        'file_type': 'unknown',
                        'parser': None,
                        'error': str(e)
                    }
        
        return results

    @classmethod  
    def parse_files_parallel(cls, file_paths: List[str], max_workers: int = 4) -> Dict[str, Dict[str, Any]]:
        """
        Parse multiple GFF files from disk in parallel
        
        Args:
            file_paths: List of file paths to parse
            max_workers: Number of processes to use
            
        Returns:
            Dict mapping filename to parsed data or error info
            
        Example:
            files = ['player.bic', 'companion1.ros', 'companion2.ros']
            results = GFFParser.parse_files_parallel(files)
        """
        gff_files = []
        
        for file_path in file_paths:
            try:
                with open(file_path, 'rb') as f:
                    data = f.read()
                filename = os.path.basename(file_path)
                gff_files.append((filename, data))
            except Exception as e:
                # Add failed file to results
                filename = os.path.basename(file_path)
                gff_files.append((filename, b''))  # Empty data will cause parse error
        
        return cls.parse_multiple_parallel(gff_files, max_workers)

# ----------------------- Writer
class GFFWriter:
    """Writer for NWN2 GFF V3.2 binary format - COMPLETE FIX"""
    
    def __init__(self, file_type: str = "GFF ", file_version: str = "V3.2"):
        # Ensure file type is exactly 4 characters
        if len(file_type) < 4:
            self.file_type = file_type.ljust(4)
        elif len(file_type) > 4:
            self.file_type = file_type[:4]
        else:
            self.file_type = file_type
        self.file_version = file_version
        
        # Arrays to build
        self.structs = []  # List of (id, field_index, field_count)
        self.fields = []   # List of (type, label_index, data_or_offset)
        self.labels = {}   # Dict of label -> index
        self.field_data = io.BytesIO()
        self.field_indices = []  # List of field indices for structs with > 1 field
        self.list_indices = []   # List of struct indices for lists
        
        # Track byte lengths separately
        self.field_indices_length = 0  # Byte length of field indices
        self.list_indices_length = 0   # Byte length of list indices
        
        self.label_list = []  # List of labels in order
        
        # For deferred struct encoding
        self.struct_queue = []  # Queue of structs to encode
        self.struct_index_map = {}  # Map element to its assigned index
    
    @classmethod
    def from_parser(cls, parser: GFFParser) -> 'GFFWriter':
        """Create a writer that preserves the original file type from parser"""
        return cls(file_type=parser.file_type, file_version=parser.file_version)
        
    def write(self, file_path: str, element: GFFElement):
        """Write a GFF element to file"""
        # Auto-detect file type from extension if not set
        if self.file_type == "GFF ":
            ext = os.path.splitext(file_path)[1].lower()
            if ext == '.bic':
                self.file_type = "BIC "
            elif ext == '.ifo':
                self.file_type = "IFO "
            elif ext == '.are':
                self.file_type = "ARE "
            elif ext == '.git':
                self.file_type = "GIT "
            elif ext == '.uti':
                self.file_type = "UTI "
            elif ext == '.utc':
                self.file_type = "UTC "
            elif ext == '.dlg':
                self.file_type = "DLG "
            elif ext == '.ros':
                self.file_type = "ROS "
            elif ext == '.fac':
                self.file_type = "FAC "
            # Add more as needed
        
        with open(file_path, 'wb') as f:
            self.save(f, element)
            
    def save(self, stream: BinaryIO, element: GFFElement):
        """Save GFF element to a stream"""
        # Reset state
        self.structs = []
        self.fields = []
        self.labels = {}
        self.label_list = []
        self.field_data = io.BytesIO()
        self.field_indices = []
        self.list_indices = []
        self.field_indices_length = 0
        self.list_indices_length = 0
        self.struct_queue = []
        self.struct_index_map = {}
        
        # Build the structure starting from top-level
        if element.type != GFFFieldType.STRUCT:
            raise GFFError("Top-level element must be a STRUCT")
        
        # COMPLETE FIX: Use two-pass encoding
        # Pass 1: Assign struct indices and queue them
        self._assign_struct_index(element)
        
        # Pass 2: Encode all structs in order
        self._encode_all_structs()
        
        # Write header
        header = bytearray(56)
        header[0:4] = self.file_type.encode('ascii')[:4]
        header[4:8] = self.file_version.encode('ascii')[:4]
        
        # Calculate offsets
        offset = 56
        struct_offset = offset
        struct_size = len(self.structs) * 12
        
        field_offset = struct_offset + struct_size
        field_size = len(self.fields) * 12
        
        label_offset = field_offset + field_size
        label_size = len(self.label_list) * 16
        
        field_data_offset = label_offset + label_size
        field_data_size = self.field_data.tell()
        
        field_indices_offset = field_data_offset + field_data_size
        field_indices_size = self.field_indices_length
        
        list_indices_offset = field_indices_offset + field_indices_size
        list_indices_size = self.list_indices_length
        
        # Write header values
        self._put_int32(header, 8, struct_offset)
        self._put_int32(header, 12, len(self.structs))
        self._put_int32(header, 16, field_offset)
        self._put_int32(header, 20, len(self.fields))
        self._put_int32(header, 24, label_offset)
        self._put_int32(header, 28, len(self.label_list))
        self._put_int32(header, 32, field_data_offset)
        self._put_int32(header, 36, field_data_size)
        self._put_int32(header, 40, field_indices_offset)
        self._put_int32(header, 44, field_indices_size)
        self._put_int32(header, 48, list_indices_offset)
        self._put_int32(header, 52, list_indices_size)
        
        # Write everything
        stream.write(header)
        
        # Write structs
        for struct_id, field_index, field_count in self.structs:
            stream.write(struct.pack('<III', struct_id, field_index, field_count))
            
        # Write fields
        for field_type, label_index, data_or_offset in self.fields:
            stream.write(struct.pack('<III', field_type, label_index, data_or_offset))
            
        # Write labels
        for label in self.label_list:
            label_bytes = label.encode('ascii')[:16]
            label_bytes = label_bytes.ljust(16, b'\x00')
            stream.write(label_bytes)
            
        # Write field data
        stream.write(self.field_data.getvalue())
        
        # Write field indices
        for idx in self.field_indices:
            stream.write(struct.pack('<I', idx))
            
        # Write list indices
        for idx in self.list_indices:
            stream.write(struct.pack('<I', idx))
            
    def _put_int32(self, buffer: bytearray, offset: int, value: int):
        """Write a little-endian 32-bit integer"""
        struct.pack_into('<I', buffer, offset, value)
        
    def _get_label_index(self, label: str) -> int:
        """Get or create label index"""
        if label in self.labels:
            return self.labels[label]
        idx = len(self.label_list)
        self.labels[label] = idx
        self.label_list.append(label)
        return idx
        
    def _assign_struct_index(self, element: GFFElement) -> int:
        """Assign an index to a struct and all its nested structs"""
        if id(element) in self.struct_index_map:
            return self.struct_index_map[id(element)]
            
        # Assign index
        struct_idx = len(self.struct_queue)
        self.struct_index_map[id(element)] = struct_idx
        self.struct_queue.append(element)
        
        # Process all fields to find nested structs
        for field in element.value:
            if field.type == GFFFieldType.STRUCT:
                self._assign_struct_index(field)
            elif field.type == GFFFieldType.LIST:
                for item in field.value:
                    if item.type == GFFFieldType.STRUCT:
                        self._assign_struct_index(item)
                        
        return struct_idx
        
    def _encode_all_structs(self):
        """Encode all structs in the correct order"""
        # Pre-allocate space for all structs
        self.structs = [(0, 0, 0)] * len(self.struct_queue)
        
        # Encode each struct
        for idx, element in enumerate(self.struct_queue):
            self._encode_struct_at_index(element, idx)
            
    def _encode_struct_at_index(self, element: GFFElement, struct_idx: int):
        """Encode a struct at a specific index"""
        struct_id = element.id
        fields = element.value if isinstance(element.value, list) else []
        
        # Encode fields
        field_indices = []
        for field in fields:
            field_idx = self._encode_field(field)
            field_indices.append(field_idx)
            
        # Update struct entry at the assigned index
        if len(field_indices) == 0:
            self.structs[struct_idx] = (struct_id, 0, 0)
        elif len(field_indices) == 1:
            self.structs[struct_idx] = (struct_id, field_indices[0], 1)
        else:
            # Multiple fields - store indices
            field_index_offset = self.field_indices_length
            self.field_indices.extend(field_indices)
            self.field_indices_length += len(field_indices) * 4
            self.structs[struct_idx] = (struct_id, field_index_offset, len(field_indices))
            
    def _encode_field(self, element: GFFElement) -> int:
        """Encode a field and return its index"""
        field_idx = len(self.fields)
        label_idx = self._get_label_index(element.label)

        # Encode based on type
        if element.type in [GFFFieldType.BYTE, GFFFieldType.CHAR, GFFFieldType.WORD,
                            GFFFieldType.SHORT, GFFFieldType.DWORD, GFFFieldType.INT]:
            # **THE FIX**: Default None value for simple numeric types to 0.
            value = element.value if element.value is not None else 0
            
            if element.type == GFFFieldType.CHAR and isinstance(value, str):
                value = ord(value[0]) if value else 0
            elif element.type == GFFFieldType.BYTE and isinstance(value, int) and value < 0:
                value = value + 0x100
            elif element.type == GFFFieldType.SHORT and isinstance(value, int) and value < 0:
                value = value + 0x10000
            elif element.type == GFFFieldType.INT and isinstance(value, int) and value < 0:
                value = value + 0x100000000
            
            self.fields.append((element.type, label_idx, int(value)))

        elif element.type == GFFFieldType.FLOAT:
            # Float stored as bytes in data_or_offset
            value = element.value if element.value is not None else 0.0
            float_bytes = struct.pack('<f', value)
            float_int = struct.unpack('<I', float_bytes)[0]
            self.fields.append((element.type, label_idx, float_int))

        elif element.type in [GFFFieldType.DWORD64, GFFFieldType.INT64, GFFFieldType.DOUBLE]:
            # 64-bit types stored in field data
            offset = self.field_data.tell()
            if element.type == GFFFieldType.DOUBLE:
                value = element.value if element.value is not None else 0.0
                self.field_data.write(struct.pack('<d', value))
            else:
                value = element.value if element.value is not None else 0
                if element.type == GFFFieldType.INT64 and value < 0:
                    value = value + 0x10000000000000000
                self.field_data.write(struct.pack('<Q', value))
            self.fields.append((element.type, label_idx, offset))

        elif element.type == GFFFieldType.STRING:
            # String with length prefix
            offset = self.field_data.tell()
            value = element.value if element.value is not None else ""
            string_bytes = value.encode('utf-8')
            self.field_data.write(struct.pack('<I', len(string_bytes)))
            self.field_data.write(string_bytes)
            self.fields.append((element.type, label_idx, offset))

        elif element.type == GFFFieldType.RESREF:
            # ResRef with length byte
            offset = self.field_data.tell()
            value = element.value if element.value is not None else ""
            resref_bytes = value.encode('utf-8')[:32]
            self.field_data.write(struct.pack('<B', len(resref_bytes)))
            self.field_data.write(resref_bytes)
            self.fields.append((element.type, label_idx, offset))

        elif element.type == GFFFieldType.LOCSTRING:
            # Localized string
            offset = self.field_data.tell()
            if element.value is None:
                # Create a default empty LocalizedString if none exists
                self._encode_localized_string(LocalizedString(-1, []))
            else:
                self._encode_localized_string(element.value)
            self.fields.append((element.type, label_idx, offset))

        elif element.type == GFFFieldType.VOID:
            # Binary data with length prefix
            offset = self.field_data.tell()
            data = b''
            if isinstance(element.value, str):
                data = bytes.fromhex(element.value)
            elif isinstance(element.value, bytes):
                data = element.value
            
            self.field_data.write(struct.pack('<I', len(data)))
            self.field_data.write(data)
            self.fields.append((element.type, label_idx, offset))

        elif element.type == GFFFieldType.STRUCT:
            # Get the pre-assigned struct index
            struct_idx = self.struct_index_map.get(id(element), 0) # Use .get for safety
            self.fields.append((element.type, label_idx, struct_idx))

        elif element.type == GFFFieldType.LIST:
            # List of structs
            offset = self._encode_list(element)
            self.fields.append((element.type, label_idx, offset))

        return field_idx
        
    def _encode_list(self, element: GFFElement) -> int:
        """Encode a list and return its offset in list indices"""
        offset = self.list_indices_length
        
        items = element.value
        
        # Write count
        self.list_indices.append(len(items))
        self.list_indices_length += 4
        
        # Get struct indices for each item
        for item in items:
            if item.type != GFFFieldType.STRUCT:
                # This shouldn't happen with a properly formed GFF
                raise GFFError("List contains non-struct element")
            else:
                struct_idx = self.struct_index_map[id(item)]
            self.list_indices.append(struct_idx)
            self.list_indices_length += 4
            
        return offset
        
    def _encode_localized_string(self, loc_string: LocalizedString):
        """Encode a localized string to field data"""
        start_offset = self.field_data.tell()
        
        # Write placeholder for total size
        self.field_data.write(struct.pack('<I', 0))
        
        # Write string ref and count
        # Handle negative string refs (convert to unsigned)
        string_ref = loc_string.string_ref if loc_string.string_ref >= 0 else 0xFFFFFFFF + loc_string.string_ref + 1
        self.field_data.write(struct.pack('<II', string_ref, len(loc_string.substrings)))
        
        # Write substrings
        for substring in loc_string.substrings:
            string_id = substring.language * 2 + substring.gender
            string_bytes = substring.string.encode('utf-8')
            self.field_data.write(struct.pack('<II', string_id, len(string_bytes)))
            self.field_data.write(string_bytes)
            
        # Update total size
        end_offset = self.field_data.tell()
        total_size = end_offset - start_offset - 4
        self.field_data.seek(start_offset)
        self.field_data.write(struct.pack('<I', total_size))
        self.field_data.seek(end_offset)