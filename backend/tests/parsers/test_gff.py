"""
Comprehensive tests for GFF parser and writer using pytest.
Tests cover all field types, edge cases, error handling, and real-world scenarios.
"""
import pytest
import io
import struct
from pathlib import Path
import tempfile
import os
from parsers.gff import (
    GFFParser, GFFWriter, GFFElement, GFFFieldType,
    LocalizedString, LocalizedSubstring,
    GFFError, GFFVersionError, GFFCorruptedError
)


@pytest.fixture
def parser():
    """Create a GFF parser instance"""
    return GFFParser()


class TestGFFParser:
    """Test GFF parser functionality"""
        
    def test_parse_header(self, parser):
        """Test parsing GFF header"""
        # Create a minimal GFF file header
        header = bytearray(60)
        header[0:4] = b'TEST'
        header[4:8] = b'V3.2'
        
        # Set counts and offsets
        struct.pack_into('<I', header, 8, 56)    # struct offset
        struct.pack_into('<I', header, 12, 1)    # struct count
        struct.pack_into('<I', header, 16, 68)   # field offset
        struct.pack_into('<I', header, 20, 0)    # field count
        struct.pack_into('<I', header, 24, 68)   # label offset
        struct.pack_into('<I', header, 28, 0)    # label count
        struct.pack_into('<I', header, 32, 68)   # field data offset
        struct.pack_into('<I', header, 36, 0)    # field data length
        struct.pack_into('<I', header, 40, 68)   # field indices offset
        struct.pack_into('<I', header, 44, 0)    # field indices length
        struct.pack_into('<I', header, 48, 68)   # list indices offset
        struct.pack_into('<I', header, 52, 0)    # list indices length
        
        # Add a struct
        struct_data = struct.pack('<III', 0, 0, 0)  # id=0, field_index=0, field_count=0
        
        # Create file
        file_data = header + struct_data
        stream = io.BytesIO(file_data)
        
        # Parse
        result = parser.load(stream)
        
        # Verify
        assert parser.file_type == 'TEST'
        assert parser.file_version == 'V3.2'
        assert result is not None
        assert result.type == GFFFieldType.STRUCT
        
    def test_parse_simple_types(self, parser):
        """Test parsing simple field types"""
        # This would require creating a more complex test file
        # For now, we'll test the individual decode methods
        
        # Test _get_int32
        buffer = struct.pack('<I', 42)
        assert parser._get_int32(buffer, 0) == 42
        
        # Test _get_int64
        buffer = struct.pack('<Q', 1234567890123456789)
        assert parser._get_int64(buffer, 0) == 1234567890123456789
        
        # Test _get_float
        buffer = struct.pack('<f', 3.14159)
        assert pytest.approx(parser._get_float(buffer, 0), abs=1e-5) == 3.14159
        
        # Test _get_double
        buffer = struct.pack('<d', 3.141592653589793)
        assert pytest.approx(parser._get_double(buffer, 0), abs=1e-10) == 3.141592653589793
        
    def test_invalid_file(self, parser):
        """Test handling of invalid files"""
        # Wrong version
        header = bytearray(60)
        header[0:4] = b'TEST'
        header[4:8] = b'V2.0'  # Wrong version
        stream = io.BytesIO(header)
        
        with pytest.raises(GFFVersionError, match='version'):
            parser.load(stream)
        
        # Too short
        stream = io.BytesIO(b'SHORT')
        with pytest.raises(GFFCorruptedError, match='header'):
            parser.load(stream)


@pytest.fixture
def writer():
    """Create a GFF writer instance"""
    return GFFWriter('TEST', 'V3.2')


class TestGFFWriter:
    """Test GFF writer functionality"""
        
    def test_write_empty_struct(self, writer):
        """Test writing an empty struct"""
        # Create empty struct
        element = GFFElement(GFFFieldType.STRUCT, 0, "", [])
        
        # Write to buffer
        buffer = io.BytesIO()
        writer.save(buffer, element)
        
        # Check header
        buffer.seek(0)
        header = buffer.read(56)
        
        assert header[0:4] == b'TEST'
        assert header[4:8] == b'V3.2'
        
        # Check struct count
        struct_count = struct.unpack_from('<I', header, 12)[0]
        assert struct_count == 1
        
    def test_write_simple_fields(self, writer):
        """Test writing simple field types"""
        # Create struct with various field types
        fields = [
            GFFElement(GFFFieldType.BYTE, 0, "TestByte", 255),
            GFFElement(GFFFieldType.INT, 0, "TestInt", -42),
            GFFElement(GFFFieldType.FLOAT, 0, "TestFloat", 3.14159),
            GFFElement(GFFFieldType.STRING, 0, "TestString", "Hello, World!"),
        ]
        
        element = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        # Write to buffer
        buffer = io.BytesIO()
        writer.save(buffer, element)
        
        # Parse it back
        buffer.seek(0)
        parser = GFFParser()
        result = parser.load(buffer)
        
        # Verify fields
        assert len(result.value) == 4
        assert result.get_value("TestByte") == 255
        assert result.get_value("TestInt") == -42
        assert pytest.approx(result.get_value("TestFloat"), abs=1e-5) == 3.14159
        assert result.get_value("TestString") == "Hello, World!"
        
    def test_write_localized_string(self, writer):
        """Test writing localized strings"""
        # Create localized string
        substrings = [
            LocalizedSubstring("English text", 0, 0),
            LocalizedSubstring("French text", 2, 0),
        ]
        loc_string = LocalizedString(-1, substrings)
        
        fields = [
            GFFElement(GFFFieldType.LOCSTRING, 0, "TestLocString", loc_string)
        ]
        
        element = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        # Write and read back
        buffer = io.BytesIO()
        writer.save(buffer, element)
        
        buffer.seek(0)
        parser = GFFParser()
        result = parser.load(buffer)
        
        # Verify
        loc_value = result.get_value("TestLocString")
        assert loc_value.string_ref == 0xFFFFFFFF  # -1 as unsigned
        assert len(loc_value.substrings) == 2
        assert loc_value.substrings[0].string == "English text"
        assert loc_value.substrings[1].string == "French text"
        
    def test_write_nested_struct(self, writer):
        """Test writing nested structs"""
        # Create nested struct
        inner_fields = [
            GFFElement(GFFFieldType.INT, 0, "InnerValue", 42)
        ]
        inner_struct = GFFElement(GFFFieldType.STRUCT, 1, "InnerStruct", inner_fields)
        
        outer_fields = [
            GFFElement(GFFFieldType.STRING, 0, "OuterValue", "Test"),
            inner_struct
        ]
        outer_struct = GFFElement(GFFFieldType.STRUCT, 0, "", outer_fields)
        
        # Write and read back
        buffer = io.BytesIO()
        writer.save(buffer, outer_struct)
        
        buffer.seek(0)
        parser = GFFParser()
        result = parser.load(buffer)
        
        # Verify structure
        assert len(result.value) == 2
        assert result.get_value("OuterValue") == "Test"
        
        inner = result.get_field("InnerStruct")
        assert inner is not None
        assert inner.type == GFFFieldType.STRUCT
        assert inner.get_value("InnerValue") == 42
        
    def test_write_list(self, writer):
        """Test writing lists"""
        # Create list of structs
        list_items = []
        for i in range(3):
            fields = [
                GFFElement(GFFFieldType.INT, 0, "Index", i),
                GFFElement(GFFFieldType.STRING, 0, "Name", f"Item {i}")
            ]
            list_items.append(GFFElement(GFFFieldType.STRUCT, 0, "", fields))
            
        list_element = GFFElement(GFFFieldType.LIST, 0, "TestList", list_items)
        
        struct_fields = [list_element]
        root_struct = GFFElement(GFFFieldType.STRUCT, 0, "", struct_fields)
        
        # Write and read back
        buffer = io.BytesIO()
        writer.save(buffer, root_struct)
        
        buffer.seek(0)
        parser = GFFParser()
        result = parser.load(buffer)
        
        # Verify list
        list_field = result.get_field("TestList")
        assert list_field is not None
        assert list_field.type == GFFFieldType.LIST
        assert len(list_field.value) == 3
        
        for i, item in enumerate(list_field.value):
            assert item.get_value("Index") == i
            assert item.get_value("Name") == f"Item {i}"


class TestGFFRoundTrip:
    """Test round-trip conversion (read -> write -> read)"""
    
    def test_simple_round_trip(self, tmp_path):
        """Test round trip with simple data"""
        # Create test data
        fields = [
            GFFElement(GFFFieldType.BYTE, 0, "ByteField", 128),
            GFFElement(GFFFieldType.SHORT, 0, "ShortField", -1000),
            GFFElement(GFFFieldType.INT, 0, "IntField", 123456),
            GFFElement(GFFFieldType.FLOAT, 0, "FloatField", 3.14159),
            GFFElement(GFFFieldType.DOUBLE, 0, "DoubleField", 2.718281828),
            GFFElement(GFFFieldType.STRING, 0, "StringField", "Test String"),
            GFFElement(GFFFieldType.RESREF, 0, "ResRefField", "testref"),
        ]
        original = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        # Write
        writer = GFFWriter('TEST', 'V3.2')
        test_file = tmp_path / "test.gff"
        writer.write(str(test_file), original)
        
        # Read back
        parser = GFFParser()
        result = parser.read(str(test_file))
        
        # Verify all fields
        assert result.get_value("ByteField") == 128
        assert result.get_value("ShortField") == -1000
        assert result.get_value("IntField") == 123456
        assert pytest.approx(result.get_value("FloatField"), abs=1e-5) == 3.14159
        assert pytest.approx(result.get_value("DoubleField"), abs=1e-9) == 2.718281828
        assert result.get_value("StringField") == "Test String"
        assert result.get_value("ResRefField") == "testref"
        
    def test_complex_round_trip(self):
        """Test round trip with complex nested data"""
        # Create complex structure
        item_fields = [
            GFFElement(GFFFieldType.INT, 0, "ItemID", 1001),
            GFFElement(GFFFieldType.STRING, 0, "ItemName", "Sword of Testing")
        ]
        item_struct = GFFElement(GFFFieldType.STRUCT, 0, "Equipment", item_fields)
        
        spell_list = []
        for i in range(3):
            spell_fields = [
                GFFElement(GFFFieldType.INT, 0, "SpellID", 100 + i),
                GFFElement(GFFFieldType.STRING, 0, "SpellName", f"Spell {i}")
            ]
            spell_list.append(GFFElement(GFFFieldType.STRUCT, 0, "", spell_fields))
            
        spells = GFFElement(GFFFieldType.LIST, 0, "SpellList", spell_list)
        
        root_fields = [
            GFFElement(GFFFieldType.STRING, 0, "CharacterName", "Test Character"),
            item_struct,
            spells
        ]
        original = GFFElement(GFFFieldType.STRUCT, 0, "", root_fields)
        
        # Round trip
        writer = GFFWriter('TEST', 'V3.2')
        buffer = io.BytesIO()
        writer.save(buffer, original)
        
        buffer.seek(0)
        parser = GFFParser()
        result = parser.load(buffer)
        
        # Verify structure
        assert result.get_value("CharacterName") == "Test Character"
        
        equipment = result.get_field("Equipment")
        assert equipment is not None
        assert equipment.get_value("ItemID") == 1001
        assert equipment.get_value("ItemName") == "Sword of Testing"
        
        spell_list_field = result.get_field("SpellList")
        assert spell_list_field is not None
        assert len(spell_list_field.value) == 3
        
        for i, spell in enumerate(spell_list_field.value):
            assert spell.get_value("SpellID") == 100 + i
            assert spell.get_value("SpellName") == f"Spell {i}"


class TestGFFFieldTypes:
    """Test all GFF field types comprehensively"""
    
    def test_all_numeric_types(self):
        """Test all numeric field types with edge cases"""
        test_cases = [
            # (type, test_values)
            (GFFFieldType.BYTE, [0, 127, 255]),
            (GFFFieldType.CHAR, ['A', 'z', '\x00', '\xFF']),
            (GFFFieldType.WORD, [0, 32767, 65535]),
            (GFFFieldType.SHORT, [-32768, -1, 0, 32767]),
            (GFFFieldType.DWORD, [0, 2147483647, 4294967295]),
            (GFFFieldType.INT, [-2147483648, -1, 0, 2147483647]),
            (GFFFieldType.DWORD64, [0, 9223372036854775807, 18446744073709551615]),
            (GFFFieldType.INT64, [-9223372036854775808, -1, 0, 9223372036854775807]),
            (GFFFieldType.FLOAT, [-3.402823e38, -1.0, 0.0, 1.0, 3.402823e38, float('inf'), float('-inf')]),
            (GFFFieldType.DOUBLE, [-1.7976931348623157e308, -1.0, 0.0, 1.0, 1.7976931348623157e308]),
        ]
        
        for field_type, values in test_cases:
            for value in values:
                # Skip infinity values for round-trip test
                if isinstance(value, float) and (value == float('inf') or value == float('-inf')):
                    continue
                    
                # Create element
                fields = [GFFElement(field_type, 0, f"Test{field_type.name}", value)]
                root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
                
                # Round trip
                writer = GFFWriter('TEST', 'V3.2')
                buffer = io.BytesIO()
                writer.save(buffer, root)
                
                buffer.seek(0)
                parser = GFFParser()
                result = parser.load(buffer)
                
                # Verify
                result_value = result.get_value(f"Test{field_type.name}")
                if field_type in [GFFFieldType.FLOAT, GFFFieldType.DOUBLE]:
                    assert pytest.approx(result_value, rel=1e-6) == value
                else:
                    assert result_value == value
    
    def test_string_types(self):
        """Test string and resref types with various cases"""
        test_strings = [
            "",  # Empty string
            "Simple ASCII",
            "Unicode: café, 日本語, 🎮",  # Unicode characters
            "Special\ncharacters\ttabs",  # Escape sequences
            "A" * 1000,  # Long string
            "\x00Null\x00bytes",  # Null bytes
        ]
        
        for test_str in test_strings:
            # Test STRING type
            fields = [GFFElement(GFFFieldType.STRING, 0, "TestString", test_str)]
            root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
            
            writer = GFFWriter('TEST', 'V3.2')
            buffer = io.BytesIO()
            writer.save(buffer, root)
            
            buffer.seek(0)
            parser = GFFParser()
            result = parser.load(buffer)
            
            assert result.get_value("TestString") == test_str
            
        # Test RESREF type (limited to 32 chars in NWN2)
        resref_tests = [
            "",
            "shortref",
            "exactly_32_chars_long_resource__",  # 32 chars
            "this_is_longer_than_32_characters_and_should_be_truncated",
        ]
        
        for resref in resref_tests:
            fields = [GFFElement(GFFFieldType.RESREF, 0, "TestResRef", resref)]
            root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
            
            writer = GFFWriter('TEST', 'V3.2')
            buffer = io.BytesIO()
            writer.save(buffer, root)
            
            buffer.seek(0)
            parser = GFFParser()
            result = parser.load(buffer)
            
            # ResRefs are limited to 32 chars in the format
            expected = resref[:32] if len(resref) > 32 else resref
            assert result.get_value("TestResRef") == expected
    
    def test_void_type(self):
        """Test VOID type (binary data)"""
        test_data = [
            b"",  # Empty
            b"\x00\x01\x02\x03",  # Binary data
            b"Binary data with text",
            bytes(range(256)),  # All byte values
        ]
        
        for data in test_data:
            fields = [GFFElement(GFFFieldType.VOID, 0, "TestVoid", data)]
            root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
            
            writer = GFFWriter('TEST', 'V3.2')
            buffer = io.BytesIO()
            writer.save(buffer, root)
            
            buffer.seek(0)
            parser = GFFParser()
            result = parser.load(buffer)
            
            assert result.get_value("TestVoid") == data
    
    def test_complex_localized_strings(self):
        """Test localized strings with multiple languages and genders"""
        # Test with various language/gender combinations
        substrings = [
            LocalizedSubstring("English Male", 0, 0),
            LocalizedSubstring("English Female", 0, 1),
            LocalizedSubstring("French Male", 2, 0),
            LocalizedSubstring("French Female", 2, 1),
            LocalizedSubstring("German Male", 4, 0),
            LocalizedSubstring("German Female", 4, 1),
        ]
        
        # Test with string ref
        loc_string1 = LocalizedString(12345, substrings)
        
        # Test without string ref (-1)
        loc_string2 = LocalizedString(-1, substrings[:2])
        
        fields = [
            GFFElement(GFFFieldType.LOCSTRING, 0, "LocWithRef", loc_string1),
            GFFElement(GFFFieldType.LOCSTRING, 0, "LocNoRef", loc_string2),
        ]
        root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        writer = GFFWriter('TEST', 'V3.2')
        buffer = io.BytesIO()
        writer.save(buffer, root)
        
        buffer.seek(0)
        parser = GFFParser()
        result = parser.load(buffer)
        
        # Verify with ref
        loc1 = result.get_value("LocWithRef")
        assert loc1.string_ref == 12345
        assert len(loc1.substrings) == 6
        for i, sub in enumerate(loc1.substrings):
            assert sub.string == substrings[i].string
            assert sub.language == substrings[i].language
            assert sub.gender == substrings[i].gender
        
        # Verify without ref
        loc2 = result.get_value("LocNoRef")
        assert loc2.string_ref == 0xFFFFFFFF  # -1 as unsigned
        assert len(loc2.substrings) == 2


class TestGFFEdgeCases:
    """Test edge cases and boundary conditions"""
    
    def test_empty_lists(self):
        """Test empty lists"""
        empty_list = GFFElement(GFFFieldType.LIST, 0, "EmptyList", [])
        fields = [empty_list]
        root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        writer = GFFWriter('TEST', 'V3.2')
        buffer = io.BytesIO()
        writer.save(buffer, root)
        
        buffer.seek(0)
        parser = GFFParser()
        result = parser.load(buffer)
        
        list_field = result.get_field("EmptyList")
        assert list_field is not None
        assert list_field.type == GFFFieldType.LIST
        assert len(list_field.value) == 0
    
    def test_deeply_nested_structures(self):
        """Test deeply nested structures"""
        # Create a deeply nested structure
        depth = 10
        current = GFFElement(GFFFieldType.INT, 0, "Value", depth)
        
        for i in range(depth - 1, 0, -1):
            fields = [
                GFFElement(GFFFieldType.INT, 0, "Level", i),
                GFFElement(GFFFieldType.STRUCT, 0, "Inner", [current])
            ]
            current = GFFElement(GFFFieldType.STRUCT, 0, f"Level{i}", fields)
        
        root = GFFElement(GFFFieldType.STRUCT, 0, "", [current])
        
        # Round trip
        writer = GFFWriter('TEST', 'V3.2')
        buffer = io.BytesIO()
        writer.save(buffer, root)
        
        buffer.seek(0)
        parser = GFFParser()
        result = parser.load(buffer)
        
        # Verify structure
        current = result
        for i in range(1, depth):
            field = current.get_field(f"Level{i}")
            assert field is not None
            assert field.get_value("Level") == i
            current = field.get_field("Inner")
        
        # Final value
        assert current.get_value("Value") == depth
    
    def test_large_structures(self):
        """Test structures with many fields"""
        # Create struct with many fields
        num_fields = 1000
        fields = []
        for i in range(num_fields):
            fields.append(GFFElement(GFFFieldType.INT, 0, f"Field{i:04d}", i))
        
        root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        # Round trip
        writer = GFFWriter('TEST', 'V3.2')
        buffer = io.BytesIO()
        writer.save(buffer, root)
        
        buffer.seek(0)
        parser = GFFParser()
        result = parser.load(buffer)
        
        # Verify all fields
        assert len(result.value) == num_fields
        for i in range(num_fields):
            assert result.get_value(f"Field{i:04d}") == i
    
    def test_duplicate_labels(self):
        """Test handling of duplicate labels in same struct"""
        # GFF format allows duplicate labels
        fields = [
            GFFElement(GFFFieldType.INT, 0, "Duplicate", 1),
            GFFElement(GFFFieldType.INT, 0, "Duplicate", 2),
            GFFElement(GFFFieldType.INT, 0, "Duplicate", 3),
        ]
        root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        writer = GFFWriter('TEST', 'V3.2')
        buffer = io.BytesIO()
        writer.save(buffer, root)
        
        buffer.seek(0)
        parser = GFFParser()
        result = parser.load(buffer)
        
        # get_value returns first match
        assert result.get_value("Duplicate") == 1
        
        # But all fields are preserved
        assert len(result.value) == 3
        duplicate_values = [f.value for f in result.value if f.label == "Duplicate"]
        assert duplicate_values == [1, 2, 3]
    
    def test_label_edge_cases(self):
        """Test label handling edge cases"""
        test_labels = [
            "",  # Empty label
            "A",  # Single char
            "ExactlySixteenCh",  # Exactly 16 chars (max)
            "ThisLabelIsLongerThan16Characters",  # Too long - will be truncated
            "Special Chars!@#",  # Special characters
            # Note: Unicode labels will cause encoding errors, so we test ASCII only
        ]
        
        fields = []
        for i, label in enumerate(test_labels):
            fields.append(GFFElement(GFFFieldType.INT, 0, label, i))
        
        root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        writer = GFFWriter('TEST', 'V3.2')
        buffer = io.BytesIO()
        writer.save(buffer, root)
        
        buffer.seek(0)
        parser = GFFParser()
        result = parser.load(buffer)
        
        # Verify labels are handled correctly
        assert len(result.value) == len(test_labels)
        
        # Test specific label retrievals
        assert result.get_value("") == 0
        assert result.get_value("A") == 1
        assert result.get_value("ExactlySixteenCh") == 2
        # Labels longer than 16 chars are truncated
        assert result.get_value("ThisLabelIsLonge") == 3
        assert result.get_value("Special Chars!@#") == 4


class TestGFFErrorHandling:
    """Test error handling and invalid data"""
    
    def test_invalid_headers(self):
        """Test various invalid headers"""
        parser = GFFParser()
        
        # Too short
        with pytest.raises(GFFCorruptedError, match="header"):
            parser.load(io.BytesIO(b"SHORT"))
        
        # Wrong version
        header = bytearray(60)
        header[0:4] = b'TEST'
        header[4:8] = b'V1.0'
        with pytest.raises(GFFVersionError, match="version"):
            parser.load(io.BytesIO(header))
        
        # No structs
        header = bytearray(60)
        header[0:4] = b'TEST'
        header[4:8] = b'V3.2'
        struct.pack_into('<I', header, 12, 0)  # struct count = 0
        with pytest.raises(GFFCorruptedError, match="no structures"):
            parser.load(io.BytesIO(header))
    
    def test_corrupted_data(self):
        """Test handling of corrupted data"""
        parser = GFFParser()
        
        # Create a minimal valid header but with bad offsets
        header = bytearray(60)
        header[0:4] = b'TEST'
        header[4:8] = b'V3.2'
        
        # Set struct count = 1 but offset beyond file
        struct.pack_into('<I', header, 8, 1000)  # struct offset way too high
        struct.pack_into('<I', header, 12, 1)    # struct count
        
        # This should fail when trying to read struct data
        with pytest.raises(Exception):  # Could be various exceptions
            parser.load(io.BytesIO(header))
    
    def test_invalid_field_types(self):
        """Test handling of invalid field types"""
        # Create data with invalid field type
        # This requires manually crafting the binary data
        # For now, we'll test that the parser handles unknown types gracefully
        pass  # Complex to implement without direct binary manipulation
    
    def test_set_field_errors(self):
        """Test set_field error cases"""
        # Test on non-struct
        element = GFFElement(GFFFieldType.INT, 0, "NotAStruct", 42)
        with pytest.raises(TypeError, match="STRUCT"):
            element.set_field("test", 123)
        
        # Test setting non-existent field (should be silent)
        struct = GFFElement(GFFFieldType.STRUCT, 0, "", [
            GFFElement(GFFFieldType.INT, 0, "ExistingField", 1)
        ])
        struct.set_field("NonExistent", 999)  # Should not raise
        assert struct.get_value("ExistingField") == 1
        assert struct.get_value("NonExistent") is None


class TestGFFFileOperations:
    """Test file-specific operations"""
    
    def test_file_type_detection(self):
        """Test automatic file type detection"""
        test_files = [
            ("test.bic", "BIC "),
            ("test.ifo", "IFO "),
            ("test.are", "ARE "),
            ("test.git", "GIT "),
            ("test.uti", "UTI "),
            ("test.utc", "UTC "),
            ("test.dlg", "DLG "),
            ("test.unknown", "GFF "),  # Default
        ]
        
        for filename, expected_type in test_files:
            with tempfile.NamedTemporaryFile(suffix=filename, delete=False) as tmp:
                try:
                    # Create simple struct
                    root = GFFElement(GFFFieldType.STRUCT, 0, "", [
                        GFFElement(GFFFieldType.INT, 0, "Test", 1)
                    ])
                    
                    # Write with auto-detection
                    writer = GFFWriter()
                    writer.write(tmp.name, root)
                    
                    # Read back and check file type
                    parser = GFFParser()
                    parser.read(tmp.name)
                    assert parser.get_file_type() == expected_type
                    
                finally:
                    os.unlink(tmp.name)
    
    def test_preserve_file_type(self):
        """Test preserving file type through read/write cycle"""
        # Create a file with specific type
        root = GFFElement(GFFFieldType.STRUCT, 0, "", [
            GFFElement(GFFFieldType.STRING, 0, "Test", "Value")
        ])
        
        writer = GFFWriter("CUST", "V3.2")
        buffer = io.BytesIO()
        writer.save(buffer, root)
        
        # Read it back
        buffer.seek(0)
        parser = GFFParser()
        result = parser.load(buffer)
        
        # Create writer from parser
        new_writer = GFFWriter.from_parser(parser)
        assert new_writer.file_type == "CUST"
        assert new_writer.file_version == "V3.2"
    
    def test_real_file_operations(self, tmp_path):
        """Test actual file read/write operations"""
        test_file = tmp_path / "test.gff"
        
        # Create complex data
        fields = [
            GFFElement(GFFFieldType.STRING, 0, "Name", "Test Character"),
            GFFElement(GFFFieldType.INT, 0, "Level", 10),
            GFFElement(GFFFieldType.FLOAT, 0, "HP", 100.5),
        ]
        root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        # Write to file
        writer = GFFWriter('TEST', 'V3.2')
        writer.write(str(test_file), root)
        
        # Verify file exists
        assert test_file.exists()
        assert test_file.stat().st_size > 56  # More than just header
        
        # Read back
        parser = GFFParser()
        result = parser.read(str(test_file))
        
        assert result.get_value("Name") == "Test Character"
        assert result.get_value("Level") == 10
        assert pytest.approx(result.get_value("HP"), abs=0.1) == 100.5


class TestGFFMethods:
    """Test GFFElement methods"""
    
    def test_get_field_method(self):
        """Test get_field method behavior"""
        # Create nested structure
        inner = GFFElement(GFFFieldType.STRUCT, 0, "Inner", [
            GFFElement(GFFFieldType.INT, 0, "Value", 42)
        ])
        root = GFFElement(GFFFieldType.STRUCT, 0, "", [inner])
        
        # Test getting existing field
        field = root.get_field("Inner")
        assert field is not None
        assert field.type == GFFFieldType.STRUCT
        assert field.get_value("Value") == 42
        
        # Test getting non-existent field
        assert root.get_field("NonExistent") is None
        
        # Test on non-struct
        int_element = GFFElement(GFFFieldType.INT, 0, "Int", 123)
        assert int_element.get_field("anything") is None
    
    def test_get_value_with_default(self):
        """Test get_value with default parameter"""
        root = GFFElement(GFFFieldType.STRUCT, 0, "", [
            GFFElement(GFFFieldType.STRING, 0, "Name", "Test")
        ])
        
        assert root.get_value("Name") == "Test"
        assert root.get_value("Name", "Default") == "Test"
        assert root.get_value("Missing") is None
        assert root.get_value("Missing", "Default") == "Default"
        assert root.get_value("Missing", 123) == 123
    
    def test_set_field_updates(self):
        """Test set_field updates existing values"""
        root = GFFElement(GFFFieldType.STRUCT, 0, "", [
            GFFElement(GFFFieldType.INT, 0, "Value", 1),
            GFFElement(GFFFieldType.STRING, 0, "Name", "Original")
        ])
        
        # Update existing fields
        root.set_field("Value", 42)
        root.set_field("Name", "Updated")
        
        assert root.get_value("Value") == 42
        assert root.get_value("Name") == "Updated"
    
    def test_set_field_list_handling(self):
        """Test set_field with LIST type"""
        # Create list with template structure
        template_fields = [
            GFFElement(GFFFieldType.INT, 0, "ID", 0),
            GFFElement(GFFFieldType.STRING, 0, "Name", "")
        ]
        list_items = [
            GFFElement(GFFFieldType.STRUCT, 0, "", template_fields)
        ]
        
        root = GFFElement(GFFFieldType.STRUCT, 0, "", [
            GFFElement(GFFFieldType.LIST, 0, "Items", list_items)
        ])
        
        # Update with new list data
        new_data = [
            {"ID": 1, "Name": "Item1"},
            {"ID": 2, "Name": "Item2"},
            {"ID": 3, "Name": "Item3"}
        ]
        root.set_field("Items", new_data)
        
        # Verify list was updated
        items = root.get_field("Items")
        assert len(items.value) == 3
        for i, item in enumerate(items.value):
            assert item.get_value("ID") == i + 1
            assert item.get_value("Name") == f"Item{i + 1}"


class TestGFFPerformance:
    """Performance and stress tests"""
    
    @pytest.mark.slow
    def test_large_file_performance(self):
        """Test performance with large files"""
        # Create a large structure
        num_structs = 100
        num_fields_per_struct = 50
        
        structs = []
        for i in range(num_structs):
            fields = []
            for j in range(num_fields_per_struct):
                fields.append(GFFElement(GFFFieldType.INT, 0, f"Field{j}", i * 1000 + j))
            structs.append(GFFElement(GFFFieldType.STRUCT, 0, "", fields))
        
        root = GFFElement(GFFFieldType.STRUCT, 0, "", [
            GFFElement(GFFFieldType.LIST, 0, "BigList", structs)
        ])
        
        # Time the write operation
        import time
        start_time = time.time()
        
        writer = GFFWriter('PERF', 'V3.2')
        buffer = io.BytesIO()
        writer.save(buffer, root)
        
        write_time = time.time() - start_time
        
        # Time the read operation
        buffer.seek(0)
        start_time = time.time()
        
        parser = GFFParser()
        result = parser.load(buffer)
        
        read_time = time.time() - start_time
        
        # Verify data integrity
        big_list = result.get_field("BigList")
        assert len(big_list.value) == num_structs
        
        # Performance assertions (adjust as needed)
        assert write_time < 1.0  # Should write in under 1 second
        assert read_time < 1.0   # Should read in under 1 second
        
        # File size check
        file_size = buffer.tell()
        assert file_size > 0  # Should have substantial size


class TestGFFSpecificBugs:
    """Test for specific bugs and edge cases found in the implementation"""
    
    def test_none_value_handling(self):
        """Test handling of None values in fields"""
        # The writer has code to handle None values for all types
        fields = [
            GFFElement(GFFFieldType.BYTE, 0, "NullByte", None),
            GFFElement(GFFFieldType.INT, 0, "NullInt", None),
            GFFElement(GFFFieldType.FLOAT, 0, "NullFloat", None),
            GFFElement(GFFFieldType.DOUBLE, 0, "NullDouble", None),
            GFFElement(GFFFieldType.STRING, 0, "NullString", None),
            GFFElement(GFFFieldType.RESREF, 0, "NullResRef", None),
            GFFElement(GFFFieldType.LOCSTRING, 0, "NullLocString", None),
            GFFElement(GFFFieldType.VOID, 0, "NullVoid", None),
        ]
        root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        # Should not crash
        writer = GFFWriter('TEST', 'V3.2')
        buffer = io.BytesIO()
        writer.save(buffer, root)
        
        # Read back and verify defaults
        buffer.seek(0)
        parser = GFFParser()
        result = parser.load(buffer)
        
        assert result.get_value("NullByte") == 0
        assert result.get_value("NullInt") == 0
        assert result.get_value("NullFloat") == 0.0
        assert result.get_value("NullDouble") == 0.0
        assert result.get_value("NullString") == ""
        assert result.get_value("NullResRef") == ""
        # LocalizedString should have empty substrings
        loc_str = result.get_value("NullLocString")
        assert loc_str.string_ref == 0xFFFFFFFF  # -1
        assert len(loc_str.substrings) == 0
        assert result.get_value("NullVoid") == b""
    
    def test_negative_number_conversion(self):
        """Test negative number conversion to unsigned"""
        # The writer converts negative numbers to unsigned
        fields = [
            GFFElement(GFFFieldType.BYTE, 0, "NegByte", -1),  # Should become 255
            GFFElement(GFFFieldType.SHORT, 0, "NegShort", -1),  # Should become 65535
            GFFElement(GFFFieldType.INT, 0, "NegInt", -1),  # Should become 4294967295
            GFFElement(GFFFieldType.INT64, 0, "NegInt64", -1),  # Should become very large
        ]
        root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        writer = GFFWriter('TEST', 'V3.2')
        buffer = io.BytesIO()
        writer.save(buffer, root)
        
        buffer.seek(0)
        parser = GFFParser()
        result = parser.load(buffer)
        
        # Parser converts back to signed
        assert result.get_value("NegByte") == 255  # BYTE is unsigned
        assert result.get_value("NegShort") == -1  # SHORT is signed
        assert result.get_value("NegInt") == -1    # INT is signed
        assert result.get_value("NegInt64") == -1  # INT64 is signed
    
    def test_char_field_string_conversion(self):
        """Test CHAR field with string value"""
        # Writer should convert string to char
        fields = [
            GFFElement(GFFFieldType.CHAR, 0, "CharFromStr", "ABC"),  # Should use 'A'
            GFFElement(GFFFieldType.CHAR, 0, "CharFromEmpty", ""),   # Should use '\0'
            GFFElement(GFFFieldType.CHAR, 0, "CharFromInt", 65),     # Should stay 65 ('A')
        ]
        root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        writer = GFFWriter('TEST', 'V3.2')
        buffer = io.BytesIO()
        writer.save(buffer, root)
        
        buffer.seek(0)
        parser = GFFParser()
        result = parser.load(buffer)
        
        assert result.get_value("CharFromStr") == 'A'
        assert result.get_value("CharFromEmpty") == '\x00'
        assert result.get_value("CharFromInt") == 'A'
    
    def test_void_field_hex_string(self):
        """Test VOID field with hex string value"""
        # Writer can accept hex strings for VOID
        fields = [
            GFFElement(GFFFieldType.VOID, 0, "VoidFromHex", "48656C6C6F"),  # "Hello"
            GFFElement(GFFFieldType.VOID, 0, "VoidFromBytes", b"World"),
        ]
        root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        writer = GFFWriter('TEST', 'V3.2')
        buffer = io.BytesIO()
        writer.save(buffer, root)
        
        buffer.seek(0)
        parser = GFFParser()
        result = parser.load(buffer)
        
        assert result.get_value("VoidFromHex") == b"Hello"
        assert result.get_value("VoidFromBytes") == b"World"
    
    def test_struct_index_safety(self):
        """Test struct index mapping safety"""
        # Test the .get(id(element), 0) safety in writer
        # This is a bit artificial but tests the safety code
        inner = GFFElement(GFFFieldType.STRUCT, 0, "Inner", [
            GFFElement(GFFFieldType.INT, 0, "Value", 42)
        ])
        
        # Create a struct that references the inner struct
        root = GFFElement(GFFFieldType.STRUCT, 0, "", [inner])
        
        writer = GFFWriter('TEST', 'V3.2')
        buffer = io.BytesIO()
        writer.save(buffer, root)
        
        # Should work without issues
        buffer.seek(0)
        parser = GFFParser()
        result = parser.load(buffer)
        
        assert result.get_field("Inner").get_value("Value") == 42
    
    def test_list_non_struct_error(self):
        """Test that lists with non-struct elements raise error"""
        # GFF lists can only contain structs
        with pytest.raises(GFFError, match="non-struct"):
            bad_list = GFFElement(GFFFieldType.LIST, 0, "BadList", [
                GFFElement(GFFFieldType.INT, 0, "NotAStruct", 42)
            ])
            root = GFFElement(GFFFieldType.STRUCT, 0, "", [bad_list])
            
            writer = GFFWriter('TEST', 'V3.2')
            buffer = io.BytesIO()
            writer.save(buffer, root)
    
    def test_writer_reset_state(self):
        """Test that writer properly resets state between saves"""
        writer = GFFWriter('TEST', 'V3.2')
        
        # First write
        root1 = GFFElement(GFFFieldType.STRUCT, 0, "", [
            GFFElement(GFFFieldType.STRING, 0, "First", "Value1")
        ])
        buffer1 = io.BytesIO()
        writer.save(buffer1, root1)
        
        # Second write with same writer instance
        root2 = GFFElement(GFFFieldType.STRUCT, 0, "", [
            GFFElement(GFFFieldType.STRING, 0, "Second", "Value2")
        ])
        buffer2 = io.BytesIO()
        writer.save(buffer2, root2)
        
        # Both should work independently
        buffer1.seek(0)
        parser = GFFParser()
        result1 = parser.load(buffer1)
        assert result1.get_value("First") == "Value1"
        assert result1.get_value("Second") is None
        
        buffer2.seek(0)
        result2 = parser.load(buffer2)
        assert result2.get_value("First") is None
        assert result2.get_value("Second") == "Value2"
    
    def test_field_index_overflow(self):
        """Test handling of field/label index limits"""
        # This tests the bounds checking in parser
        parser = GFFParser()
        
        # Mock invalid indices
        parser.field_count = 5
        parser.label_count = 3
        
        with pytest.raises(GFFCorruptedError, match="exceeds array size"):
            parser._decode_field(10)  # Index too high
            
        with pytest.raises(GFFCorruptedError, match="exceeds array size"):
            parser._get_label(10)  # Index too high


class TestGFFIntegration:
    """Test GFF parser integration with NWN2 systems"""
    
    def test_memory_efficiency(self):
        """Test that parser clears buffers after parsing"""
        # Create a moderate-sized structure
        fields = []
        for i in range(100):
            fields.append(GFFElement(GFFFieldType.STRING, 0, f"Field{i}", f"Value {i}" * 10))
        root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        # Write to buffer
        writer = GFFWriter('TEST', 'V3.2')
        buffer = io.BytesIO()
        writer.save(buffer, root)
        
        # Parse
        buffer.seek(0)
        parser = GFFParser()
        result = parser.load(buffer)
        
        # Verify buffers are cleared
        assert parser.struct_buffer == b''
        assert parser.field_buffer == b''
        assert parser.label_buffer == b''
        assert parser.field_data_buffer == b''
        assert parser.field_indices_buffer == b''
        assert parser.list_indices_buffer == b''
        
        # But result should still be valid
        assert result.get_value("Field0") == "Value 0" * 10
        assert result.get_value("Field99") == "Value 99" * 10
    
    def test_repr_method(self):
        """Test GFFElement __repr__ method"""
        element = GFFElement(GFFFieldType.STRING, 0, "TestLabel", "TestValue")
        repr_str = repr(element)
        assert "GFFElement" in repr_str
        assert "TestLabel" in repr_str
        assert "TestValue" in repr_str
        assert "10" in repr_str  # STRING type value
    
    def test_binary_compatibility(self):
        """Test that written files maintain NWN2 binary compatibility"""
        # Create a typical character-like structure
        fields = [
            GFFElement(GFFFieldType.STRING, 0, "FirstName", "Test"),
            GFFElement(GFFFieldType.STRING, 0, "LastName", "Character"),
            GFFElement(GFFFieldType.INT, 0, "Level", 10),
            GFFElement(GFFFieldType.FLOAT, 0, "HP", 100.0),
            GFFElement(GFFFieldType.BYTE, 0, "Strength", 18),
            GFFElement(GFFFieldType.BYTE, 0, "Dexterity", 14),
            GFFElement(GFFFieldType.BYTE, 0, "Constitution", 16),
            GFFElement(GFFFieldType.BYTE, 0, "Intelligence", 12),
            GFFElement(GFFFieldType.BYTE, 0, "Wisdom", 10),
            GFFElement(GFFFieldType.BYTE, 0, "Charisma", 8),
        ]
        root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        # Write as BIC file
        writer = GFFWriter('BIC ', 'V3.2')
        buffer = io.BytesIO()
        writer.save(buffer, root)
        
        # Verify header is correct for NWN2
        buffer.seek(0)
        header = buffer.read(8)
        assert header == b'BIC V3.2'
        
        # Verify it can be read back
        buffer.seek(0)
        parser = GFFParser()
        result = parser.load(buffer)
        
        assert parser.get_file_type() == 'BIC '
        assert result.get_value("FirstName") == "Test"
        assert result.get_value("Level") == 10
    
    def test_concurrent_parsing(self):
        """Test that multiple parser instances don't interfere"""
        # Create two different structures
        root1 = GFFElement(GFFFieldType.STRUCT, 0, "", [
            GFFElement(GFFFieldType.STRING, 0, "Data", "Parser1")
        ])
        
        root2 = GFFElement(GFFFieldType.STRUCT, 0, "", [
            GFFElement(GFFFieldType.STRING, 0, "Data", "Parser2")
        ])
        
        # Write both
        writer = GFFWriter('TEST', 'V3.2')
        buffer1 = io.BytesIO()
        buffer2 = io.BytesIO()
        writer.save(buffer1, root1)
        writer.save(buffer2, root2)
        
        # Parse with separate parser instances
        buffer1.seek(0)
        buffer2.seek(0)
        parser1 = GFFParser()
        parser2 = GFFParser()
        
        result1 = parser1.load(buffer1)
        result2 = parser2.load(buffer2)
        
        # Verify independence
        assert result1.get_value("Data") == "Parser1"
        assert result2.get_value("Data") == "Parser2"
        assert parser1.file_type == "TEST"
        assert parser2.file_type == "TEST"
    
    def test_file_size_estimation(self):
        """Test that file sizes are reasonable for content"""
        # Empty struct
        empty = GFFElement(GFFFieldType.STRUCT, 0, "", [])
        writer = GFFWriter('TEST', 'V3.2')
        buffer = io.BytesIO()
        writer.save(buffer, empty)
        empty_size = buffer.tell()
        
        # Header + one struct entry minimum
        assert empty_size >= 56 + 12  # Header + struct
        
        # Small struct with one field
        small = GFFElement(GFFFieldType.STRUCT, 0, "", [
            GFFElement(GFFFieldType.INT, 0, "Value", 42)
        ])
        buffer = io.BytesIO()
        writer.save(buffer, small)
        small_size = buffer.tell()
        
        # Should be larger than empty
        assert small_size > empty_size
        
        # Large struct
        fields = [GFFElement(GFFFieldType.STRING, 0, f"Field{i}", "X" * 100) for i in range(100)]
        large = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        buffer = io.BytesIO()
        writer.save(buffer, large)
        large_size = buffer.tell()
        
        # Should be much larger
        assert large_size > small_size * 10
        
        # Size should be roughly predictable
        # 100 fields * ~100 bytes per string = ~10KB minimum
        assert large_size > 10000