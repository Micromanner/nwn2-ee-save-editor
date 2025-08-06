#!/usr/bin/env python3
"""
Enhanced GFF (Generic File Format) creator for generating valid test data,
including module.ifo and campaign.cam files for NWN2.

This version corrects critical bugs related to offset calculations found in the
original test implementation, ensuring the generated files are parsable.
"""
import struct
from typing import List, Dict, Any

# GFF Field Types, as defined by the BioWare GFF specification.
class GFFFieldType:
    """GFF field type constants"""
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
    CExoString = 10
    ResRef = 11
    CExoLocString = 12
    VOID = 13
    Struct = 14
    List = 15


class GFFBuilder:
    """A builder for creating GFF v3.2 files with a proper binary structure."""

    def __init__(self, file_type: str):
        self.file_type = file_type
        self.structs = []
        self.fields = []
        self.labels = {}
        self.field_data = bytearray()
        self.field_indices = bytearray()
        self.list_indices = bytearray()

    def add_struct(self, struct_type: int = 0xFFFFFFFF) -> int:
        """Adds a new struct definition and returns its index."""
        struct_idx = len(self.structs)
        self.structs.append({'type': struct_type, 'fields': []})
        return struct_idx

    def add_field(self, struct_idx: int, label: str, field_type: int, value: Any):
        """Adds a field to a specific struct."""
        if label not in self.labels:
            self.labels[label] = len(self.labels)

        field = {'type': field_type, 'label_idx': self.labels[label], 'value': value}
        self.structs[struct_idx]['fields'].append(field)

    def add_cexostring(self, struct_idx: int, label: str, value: str):
        self.add_field(struct_idx, label, GFFFieldType.CExoString, value)

    def add_dword(self, struct_idx: int, label: str, value: int):
        self.add_field(struct_idx, label, GFFFieldType.DWORD, value)

    def add_byte(self, struct_idx: int, label: str, value: int):
        self.add_field(struct_idx, label, GFFFieldType.BYTE, value & 0xFF)

    def add_resref(self, struct_idx: int, label: str, value: str):
        self.add_field(struct_idx, label, GFFFieldType.ResRef, value[:16])

    def add_list(self, struct_idx: int, label: str, struct_indices: List[int]):
        self.add_field(struct_idx, label, GFFFieldType.List, struct_indices)

    def add_float(self, struct_idx: int, label: str, value: float):
        self.add_field(struct_idx, label, GFFFieldType.FLOAT, value)

    def add_void(self, struct_idx: int, label: str, value: bytes):
        self.add_field(struct_idx, label, GFFFieldType.VOID, value)

    def add_cexolocstring(self, struct_idx: int, label: str, value: str, string_ref: int = -1):
        # -1 (0xFFFFFFFF) is the standard for no TLK reference.
        locstring = {
            'string_ref': string_ref,
            'substrings': [{'language': 0, 'gender': 0, 'string': value}]
        }
        self.add_field(struct_idx, label, GFFFieldType.CExoLocString, locstring)

    def build(self) -> bytes:
        """Builds all data sections and assembles the final GFF file bytes."""
        # This must be the first step, as it populates all the data arrays.
        self._build_field_data_and_indices()

        # --- Calculate Offsets ---
        # All offsets are from the beginning of the file.
        header_size = 60  # GFF V3.2 header is 60 bytes.
        struct_array_offset = header_size
        struct_array_size = len(self.structs) * 12

        field_array_offset = struct_array_offset + struct_array_size
        field_array_size = len(self.fields) * 12

        label_array_offset = field_array_offset + field_array_size
        label_array_size = len(self.labels) * 16

        field_data_offset = label_array_offset + label_array_size
        field_data_size = len(self.field_data)

        field_indices_offset = field_data_offset + field_data_size
        field_indices_size = len(self.field_indices)

        list_indices_offset = field_indices_offset + field_indices_size

        # --- Assemble the File ---
        output = bytearray()

        # Header
        output.extend(b'GFF ')
        output.extend(b'V3.2')
        output.extend(self.file_type.encode('ascii').ljust(4))
        output.extend(struct.pack('<IIIIIIIIIIII',
            struct_array_offset, len(self.structs),
            field_array_offset, len(self.fields),
            label_array_offset, len(self.labels),
            field_data_offset, field_data_size,
            field_indices_offset, field_indices_size,
            list_indices_offset, len(self.list_indices)
        ))

        # Struct Array
        for s in self.structs:
            output.extend(struct.pack('<I', s['type']))
            output.extend(struct.pack('<I', s['data_or_offset']))
            output.extend(struct.pack('<I', s['field_count']))

        # Field Array
        for f in self.fields:
            output.extend(struct.pack('<I', f['type']))
            output.extend(struct.pack('<I', f['label_idx']))
            output.extend(f['data_or_offset_bytes'])

        # Label Array (16 bytes per label)
        sorted_labels = sorted(self.labels.items(), key=lambda item: item[1])
        for label, _ in sorted_labels:
            output.extend(label.encode('ascii').ljust(16, b'\x00'))

        # Data Blocks (appended in order)
        output.extend(self.field_data)
        output.extend(self.field_indices)
        output.extend(self.list_indices)

        return bytes(output)

    def _build_field_data_and_indices(self):
        """Populates the field_data, field_indices, and list_indices arrays."""
        for s in self.structs:
            struct_field_indices = []
            for field in s['fields']:
                field_idx = len(self.fields)
                struct_field_indices.append(field_idx)
                processed_field = self._process_field(field)
                self.fields.append(processed_field)

            s['field_count'] = len(struct_field_indices)
            if s['field_count'] == 0:
                s['data_or_offset'] = 0xFFFFFFFF
            elif s['field_count'] == 1:
                # If there's only one field, the offset is just the field's index.
                s['data_or_offset'] = struct_field_indices[0]
            else:
                # If multiple fields, store their indices in the FieldIndices array
                # and point to the start of that list.
                # FIX: This must be a BYTE offset, not an element index.
                s['data_or_offset'] = len(self.field_indices)
                for idx in struct_field_indices:
                    self.field_indices.extend(struct.pack('<I', idx))

    def _process_field(self, field: Dict) -> Dict:
        """Converts a high-level field definition into its low-level binary representation."""
        field_dict = {'type': field['type'], 'label_idx': field['label_idx']}
        value = field['value']

        # Simple types are stored directly in the 4-byte data/offset part of the field.
        if field['type'] == GFFFieldType.BYTE:
            field_dict['data_or_offset_bytes'] = struct.pack('<I', value & 0xFF)
        elif field['type'] == GFFFieldType.DWORD or field['type'] == GFFFieldType.INT:
            field_dict['data_or_offset_bytes'] = struct.pack('<I', value)
        elif field['type'] == GFFFieldType.FLOAT:
            field_dict['data_or_offset_bytes'] = struct.pack('<f', value)

        # Complex types are stored in the FieldData block, and we store the offset.
        elif field['type'] == GFFFieldType.CExoString:
            offset = len(self.field_data)
            string_bytes = value.encode('utf-8')
            self.field_data.extend(struct.pack('<I', len(string_bytes)))
            self.field_data.extend(string_bytes)
            field_dict['data_or_offset_bytes'] = struct.pack('<I', offset)
        elif field['type'] == GFFFieldType.ResRef:
            offset = len(self.field_data)
            resref_bytes = value.encode('ascii')
            self.field_data.extend(struct.pack('<B', len(resref_bytes)))
            self.field_data.extend(resref_bytes)
            field_dict['data_or_offset_bytes'] = struct.pack('<I', offset)
        elif field['type'] == GFFFieldType.VOID:
            offset = len(self.field_data)
            self.field_data.extend(struct.pack('<I', len(value)))
            self.field_data.extend(value)
            field_dict['data_or_offset_bytes'] = struct.pack('<I', offset)
        elif field['type'] == GFFFieldType.CExoLocString:
            offset = len(self.field_data)
            # Reserve 4 bytes for total size, to be filled in later.
            size_placeholder_pos = len(self.field_data)
            self.field_data.extend(b'\x00\x00\x00\x00')
            self.field_data.extend(struct.pack('<i', value['string_ref']))
            self.field_data.extend(struct.pack('<I', len(value['substrings'])))
            for sub in value['substrings']:
                lang_id = (sub['language'] << 1) | sub['gender']
                self.field_data.extend(struct.pack('<I', lang_id))
                string_bytes = sub['string'].encode('utf-8')
                self.field_data.extend(struct.pack('<I', len(string_bytes)))
                self.field_data.extend(string_bytes)
            # Go back and write the correct total size.
            total_size = len(self.field_data) - size_placeholder_pos - 4
            self.field_data[size_placeholder_pos:size_placeholder_pos+4] = struct.pack('<I', total_size)
            field_dict['data_or_offset_bytes'] = struct.pack('<I', offset)
        elif field['type'] == GFFFieldType.List:
            # Lists of structs are stored in the ListIndices block.
            # FIX: This must be a BYTE offset, not an element index.
            offset = len(self.list_indices)
            self.list_indices.extend(struct.pack('<I', len(value)))
            for struct_idx in value:
                self.list_indices.extend(struct.pack('<I', struct_idx))
            field_dict['data_or_offset_bytes'] = struct.pack('<I', offset)
        else:
            # Default for unhandled types.
            field_dict['data_or_offset_bytes'] = struct.pack('<I', 0)

        return field_dict

def create_module_ifo(name: str, hak_list: List[str] = None, custom_tlk: str = "") -> bytes:
    """Creates the binary data for a valid module.ifo file."""
    builder = GFFBuilder("IFO ")
    top = builder.add_struct() # Top-level struct has type 0xFFFFFFFF

    # Add mandatory and common fields for a module.
    builder.add_cexolocstring(top, "Mod_Name", name)
    builder.add_void(top, "Mod_ID", b'\x00' * 16) # Needs a unique ID, but zeros are fine for tests.
    builder.add_resref(top, "Mod_Entry_Area", "startarea")
    builder.add_float(top, "Mod_Entry_X", 0.0)
    builder.add_float(top, "Mod_Entry_Y", 0.0)
    builder.add_float(top, "Mod_Entry_Z", 0.0)

    if hak_list:
        hak_structs = []
        for hak in hak_list:
            hak_struct = builder.add_struct(0) # Structs in a list have a type ID.
            builder.add_cexostring(hak_struct, "Mod_Hak", hak)
            hak_structs.append(hak_struct)
        builder.add_list(top, "Mod_HakList", hak_structs)

    if custom_tlk:
        builder.add_cexostring(top, "Mod_CustomTlk", custom_tlk)

    return builder.build()


def create_campaign_cam(name: str, modules: List[Dict[str, str]]) -> bytes:
    """Creates the binary data for a valid campaign.cam file."""
    builder = GFFBuilder("CAM ")
    top = builder.add_struct()

    builder.add_cexolocstring(top, "DisplayName", name)
    builder.add_resref(top, "StartModule", modules[0]['ModuleName'] if modules else "")

    if modules:
        mod_structs = []
        for mod_info in modules:
            mod_struct = builder.add_struct(0)
            # NWN2 campaign files use 'ModNames' for the list and 'ModuleName' for the field.
            builder.add_resref(mod_struct, "ModuleName", mod_info.get("ModuleName", ""))
            mod_structs.append(mod_struct)
        builder.add_list(top, "ModNames", mod_structs)

    return builder.build()


if __name__ == '__main__':
    # This block allows for standalone testing of the GFF builder.
    # It will create two example files in the current directory.
    print("Creating test 'module.ifo'...")
    module_data = create_module_ifo(
        name="My Test Module",
        hak_list=["hak1", "hak2_override"],
        custom_tlk="dialog_custom.tlk"
    )
    with open("test_module.ifo", "wb") as f:
        f.write(module_data)
    print(" -> 'test_module.ifo' created successfully.")

    print("\nCreating test 'campaign.cam'...")
    campaign_data = create_campaign_cam(
        name="My Test Campaign",
        modules=[
            {'ModuleName': '01_intro_module'},
            {'ModuleName': '02_main_chapter'},
        ]
    )
    with open("test_campaign.cam", "wb") as f:
        f.write(campaign_data)
    print(" -> 'test_campaign.cam' created successfully.")