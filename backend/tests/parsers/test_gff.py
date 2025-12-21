"""
Comprehensive tests for GFF parser and writer using the Rust backend.
Tests use plain dicts with __struct_id__ and __field_types__ metadata.
"""
import pytest
import struct
from pathlib import Path
import tempfile
import os
import nwn2_rust
from nwn2_rust import GffParser, GffWriter


# GFF Field Type constants
class GFFFieldType:
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


def make_struct(fields: dict, struct_id: int = 0, field_types: dict = None) -> dict:
    """Helper to create a struct dict with proper metadata"""
    result = {"__struct_id__": struct_id}
    if field_types:
        result["__field_types__"] = field_types
    result.update(fields)
    return result


def make_locstring(string_ref: int = -1, substrings: list = None) -> dict:
    """Helper to create a localized string dict"""
    return {
        "string_ref": string_ref,
        "substrings": substrings or []
    }


def make_substring(text: str, language: int = 0, gender: int = 0) -> dict:
    """Helper to create a localized substring dict"""
    return {
        "string": text,
        "language": language,
        "gender": gender
    }


class TestGFFParser:
    """Test GFF parser functionality"""

    def test_parse_header(self, tmp_path):
        """Test parsing GFF header"""
        # Create a minimal valid GFF file
        test_file = tmp_path / "test.gff"

        # Create minimal GFF with empty root struct
        header = bytearray(56)
        header[0:4] = b'TEST'
        header[4:8] = b'V3.2'

        # Set counts and offsets
        struct_offset = 56
        struct.pack_into('<I', header, 8, struct_offset)   # struct offset
        struct.pack_into('<I', header, 12, 1)              # struct count
        struct.pack_into('<I', header, 16, struct_offset + 12)  # field offset
        struct.pack_into('<I', header, 20, 0)              # field count
        struct.pack_into('<I', header, 24, struct_offset + 12)  # label offset
        struct.pack_into('<I', header, 28, 0)              # label count
        struct.pack_into('<I', header, 32, struct_offset + 12)  # field data offset
        struct.pack_into('<I', header, 36, 0)              # field data length
        struct.pack_into('<I', header, 40, struct_offset + 12)  # field indices offset
        struct.pack_into('<I', header, 44, 0)              # field indices length
        struct.pack_into('<I', header, 48, struct_offset + 12)  # list indices offset
        struct.pack_into('<I', header, 52, 0)              # list indices length

        # Add a struct: id=0, field_index=0, field_count=0
        struct_data = struct.pack('<III', 0, 0, 0)

        # Write file
        test_file.write_bytes(header + struct_data)

        # Parse
        parser = GffParser(str(test_file))
        assert parser.get_file_type() == 'TEST'
        assert parser.get_file_version() == 'V3.2'

        data = parser.to_dict()
        assert isinstance(data, dict)
        assert "__struct_id__" in data

    def test_nonstandard_version_read(self, tmp_path):
        """Test that parser can read files with non-standard versions (lenient parsing)"""
        test_file = tmp_path / "test.gff"

        # Create a complete header with non-standard version but valid structure
        header = bytearray(56)
        header[0:4] = b'TEST'
        header[4:8] = b'V2.0'  # Non-standard version

        # Set offsets and counts for a minimal valid structure
        struct_offset = 56
        struct.pack_into('<I', header, 8, struct_offset)   # struct offset
        struct.pack_into('<I', header, 12, 1)              # struct count
        struct.pack_into('<I', header, 16, struct_offset + 12)  # field offset
        struct.pack_into('<I', header, 20, 0)              # field count
        struct.pack_into('<I', header, 24, struct_offset + 12)  # label offset
        struct.pack_into('<I', header, 28, 0)              # label count
        struct.pack_into('<I', header, 32, struct_offset + 12)  # field data offset
        struct.pack_into('<I', header, 36, 0)              # field data length
        struct.pack_into('<I', header, 40, struct_offset + 12)  # field indices offset
        struct.pack_into('<I', header, 44, 0)              # field indices length
        struct.pack_into('<I', header, 48, struct_offset + 12)  # list indices offset
        struct.pack_into('<I', header, 52, 0)              # list indices length

        # Add struct data
        struct_data = struct.pack('<III', 0, 0, 0)
        test_file.write_bytes(header + struct_data)

        # Rust parser is lenient and accepts non-standard versions
        parser = GffParser(str(test_file))
        assert parser.get_file_type() == 'TEST'
        assert parser.get_file_version() == 'V2.0'

    def test_invalid_file_too_short(self, tmp_path):
        """Test handling of file that is too short"""
        test_file = tmp_path / "test.gff"
        test_file.write_bytes(b'SHORT')

        with pytest.raises(Exception):
            GffParser(str(test_file))


class TestGFFWriter:
    """Test GFF writer functionality"""

    def test_write_empty_struct(self, tmp_path):
        """Test writing an empty struct"""
        test_file = tmp_path / "test.gff"

        # Create empty struct
        data = make_struct({}, struct_id=0, field_types={})

        # Write to file
        writer = GffWriter('TEST', 'V3.2')
        writer.write(str(test_file), data)

        # Check header
        content = test_file.read_bytes()
        assert content[0:4] == b'TEST'
        assert content[4:8] == b'V3.2'

        # Check struct count
        struct_count = struct.unpack_from('<I', content, 12)[0]
        assert struct_count == 1

    def test_write_simple_fields(self, tmp_path):
        """Test writing simple field types"""
        test_file = tmp_path / "test.gff"

        # Create struct with various field types
        data = make_struct(
            {
                "TestByte": 255,
                "TestInt": -42,
                "TestFloat": 3.14159,
                "TestString": "Hello, World!",
            },
            struct_id=0,
            field_types={
                "TestByte": GFFFieldType.BYTE,
                "TestInt": GFFFieldType.INT,
                "TestFloat": GFFFieldType.FLOAT,
                "TestString": GFFFieldType.STRING,
            }
        )

        # Write
        writer = GffWriter('TEST', 'V3.2')
        writer.write(str(test_file), data)

        # Read back
        parser = GffParser(str(test_file))
        result = parser.to_dict()

        # Verify fields
        assert result.get("TestByte") == 255
        assert result.get("TestInt") == -42
        assert pytest.approx(result.get("TestFloat"), abs=1e-5) == 3.14159
        assert result.get("TestString") == "Hello, World!"

    def test_write_localized_string(self, tmp_path):
        """Test writing localized strings"""
        test_file = tmp_path / "test.gff"

        # Create localized string
        loc_string = make_locstring(
            string_ref=-1,
            substrings=[
                make_substring("English text", language=0, gender=0),
                make_substring("French text", language=2, gender=0),
            ]
        )

        data = make_struct(
            {"TestLocString": loc_string},
            struct_id=0,
            field_types={"TestLocString": GFFFieldType.LOCSTRING}
        )

        # Write and read back
        writer = GffWriter('TEST', 'V3.2')
        writer.write(str(test_file), data)

        parser = GffParser(str(test_file))
        result = parser.to_dict()

        # Verify
        loc_value = result.get("TestLocString")
        assert isinstance(loc_value, dict)
        # -1 stored as unsigned 0xFFFFFFFF
        assert loc_value["string_ref"] == -1 or loc_value["string_ref"] == 0xFFFFFFFF
        assert len(loc_value["substrings"]) == 2
        assert loc_value["substrings"][0]["string"] == "English text"
        assert loc_value["substrings"][1]["string"] == "French text"

    def test_write_nested_struct(self, tmp_path):
        """Test writing nested structs"""
        test_file = tmp_path / "test.gff"

        # Create nested struct
        inner_struct = make_struct(
            {"InnerValue": 42},
            struct_id=1,
            field_types={"InnerValue": GFFFieldType.INT}
        )

        data = make_struct(
            {
                "OuterValue": "Test",
                "InnerStruct": inner_struct
            },
            struct_id=0,
            field_types={
                "OuterValue": GFFFieldType.STRING,
                "InnerStruct": GFFFieldType.STRUCT
            }
        )

        # Write and read back
        writer = GffWriter('TEST', 'V3.2')
        writer.write(str(test_file), data)

        parser = GffParser(str(test_file))
        result = parser.to_dict()

        # Verify structure
        assert result.get("OuterValue") == "Test"
        inner = result.get("InnerStruct")
        assert inner is not None
        assert isinstance(inner, dict)
        assert inner.get("InnerValue") == 42

    def test_write_list(self, tmp_path):
        """Test writing lists"""
        test_file = tmp_path / "test.gff"

        # Create list of structs
        list_items = []
        for i in range(3):
            item = make_struct(
                {"Index": i, "Name": f"Item {i}"},
                struct_id=0,
                field_types={
                    "Index": GFFFieldType.INT,
                    "Name": GFFFieldType.STRING
                }
            )
            list_items.append(item)

        data = make_struct(
            {"TestList": list_items},
            struct_id=0,
            field_types={"TestList": GFFFieldType.LIST}
        )

        # Write and read back
        writer = GffWriter('TEST', 'V3.2')
        writer.write(str(test_file), data)

        parser = GffParser(str(test_file))
        result = parser.to_dict()

        # Verify list
        test_list = result.get("TestList")
        assert test_list is not None
        assert isinstance(test_list, list)
        assert len(test_list) == 3

        for i, item in enumerate(test_list):
            assert item.get("Index") == i
            assert item.get("Name") == f"Item {i}"


class TestGFFRoundTrip:
    """Test round-trip conversion (write -> read)"""

    def test_simple_round_trip(self, tmp_path):
        """Test round trip with simple data"""
        test_file = tmp_path / "test.gff"

        # Create test data
        data = make_struct(
            {
                "ByteField": 128,
                "ShortField": -1000,
                "IntField": 123456,
                "FloatField": 3.14159,
                "DoubleField": 2.718281828,
                "StringField": "Test String",
                "ResRefField": "testref",
            },
            struct_id=0,
            field_types={
                "ByteField": GFFFieldType.BYTE,
                "ShortField": GFFFieldType.SHORT,
                "IntField": GFFFieldType.INT,
                "FloatField": GFFFieldType.FLOAT,
                "DoubleField": GFFFieldType.DOUBLE,
                "StringField": GFFFieldType.STRING,
                "ResRefField": GFFFieldType.RESREF,
            }
        )

        # Write
        writer = GffWriter('TEST', 'V3.2')
        writer.write(str(test_file), data)

        # Read back
        parser = GffParser(str(test_file))
        result = parser.to_dict()

        # Verify all fields
        assert result.get("ByteField") == 128
        assert result.get("ShortField") == -1000
        assert result.get("IntField") == 123456
        assert pytest.approx(result.get("FloatField"), abs=1e-5) == 3.14159
        assert pytest.approx(result.get("DoubleField"), abs=1e-9) == 2.718281828
        assert result.get("StringField") == "Test String"
        assert result.get("ResRefField") == "testref"

    def test_complex_round_trip(self, tmp_path):
        """Test round trip with complex nested data"""
        test_file = tmp_path / "test.gff"

        # Create complex structure
        equipment = make_struct(
            {"ItemID": 1001, "ItemName": "Sword of Testing"},
            struct_id=0,
            field_types={
                "ItemID": GFFFieldType.INT,
                "ItemName": GFFFieldType.STRING
            }
        )

        spell_list = []
        for i in range(3):
            spell = make_struct(
                {"SpellID": 100 + i, "SpellName": f"Spell {i}"},
                struct_id=0,
                field_types={
                    "SpellID": GFFFieldType.INT,
                    "SpellName": GFFFieldType.STRING
                }
            )
            spell_list.append(spell)

        data = make_struct(
            {
                "CharacterName": "Test Character",
                "Equipment": equipment,
                "SpellList": spell_list
            },
            struct_id=0,
            field_types={
                "CharacterName": GFFFieldType.STRING,
                "Equipment": GFFFieldType.STRUCT,
                "SpellList": GFFFieldType.LIST
            }
        )

        # Write
        writer = GffWriter('TEST', 'V3.2')
        writer.write(str(test_file), data)

        # Read back
        parser = GffParser(str(test_file))
        result = parser.to_dict()

        # Verify structure
        assert result.get("CharacterName") == "Test Character"

        eq = result.get("Equipment")
        assert eq is not None
        assert eq.get("ItemID") == 1001
        assert eq.get("ItemName") == "Sword of Testing"

        spells = result.get("SpellList")
        assert spells is not None
        assert len(spells) == 3

        for i, spell in enumerate(spells):
            assert spell.get("SpellID") == 100 + i
            assert spell.get("SpellName") == f"Spell {i}"


class TestGFFFieldTypes:
    """Test all GFF field types comprehensively"""

    def test_all_numeric_types(self, tmp_path):
        """Test all numeric field types with various values"""
        test_cases = [
            # (type_id, field_name, test_values)
            (GFFFieldType.BYTE, "TestByte", [0, 127, 255]),
            (GFFFieldType.WORD, "TestWord", [0, 32767, 65535]),
            (GFFFieldType.SHORT, "TestShort", [-32768, -1, 0, 32767]),
            (GFFFieldType.DWORD, "TestDword", [0, 2147483647, 4294967295]),
            (GFFFieldType.INT, "TestInt", [-2147483648, -1, 0, 2147483647]),
        ]

        test_idx = 0
        for field_type, field_name, values in test_cases:
            for value in values:
                # Use unique file for each test to avoid Windows file locking
                test_file = tmp_path / f"test_{test_idx}.gff"
                test_idx += 1

                # Create data
                data = make_struct(
                    {field_name: value},
                    struct_id=0,
                    field_types={field_name: field_type}
                )

                # Round trip
                writer = GffWriter('TEST', 'V3.2')
                writer.write(str(test_file), data)

                parser = GffParser(str(test_file))
                result = parser.to_dict()

                # Verify
                assert result.get(field_name) == value, f"Failed for {field_name}={value}"

    def test_64bit_numeric_types(self, tmp_path):
        """Test 64-bit numeric types"""
        test_cases = [
            (GFFFieldType.DWORD64, "TestDword64", [0, 9223372036854775807]),
            (GFFFieldType.INT64, "TestInt64", [-9223372036854775808, -1, 0, 9223372036854775807]),
        ]

        test_idx = 0
        for field_type, field_name, values in test_cases:
            for value in values:
                # Use unique file for each test to avoid Windows file locking
                test_file = tmp_path / f"test64_{test_idx}.gff"
                test_idx += 1

                data = make_struct(
                    {field_name: value},
                    struct_id=0,
                    field_types={field_name: field_type}
                )

                writer = GffWriter('TEST', 'V3.2')
                writer.write(str(test_file), data)

                parser = GffParser(str(test_file))
                result = parser.to_dict()

                assert result.get(field_name) == value, f"Failed for {field_name}={value}"

    def test_float_types(self, tmp_path):
        """Test float and double types"""
        float_values = [-1.0, 0.0, 1.0, 3.14159]
        double_values = [-1.0, 0.0, 1.0, 2.718281828459045]

        test_idx = 0
        for value in float_values:
            test_file = tmp_path / f"test_float_{test_idx}.gff"
            test_idx += 1

            data = make_struct(
                {"TestFloat": value},
                struct_id=0,
                field_types={"TestFloat": GFFFieldType.FLOAT}
            )

            writer = GffWriter('TEST', 'V3.2')
            writer.write(str(test_file), data)

            parser = GffParser(str(test_file))
            result = parser.to_dict()

            assert pytest.approx(result.get("TestFloat"), rel=1e-5) == value

        for value in double_values:
            test_file = tmp_path / f"test_double_{test_idx}.gff"
            test_idx += 1

            data = make_struct(
                {"TestDouble": value},
                struct_id=0,
                field_types={"TestDouble": GFFFieldType.DOUBLE}
            )

            writer = GffWriter('TEST', 'V3.2')
            writer.write(str(test_file), data)

            parser = GffParser(str(test_file))
            result = parser.to_dict()

            assert pytest.approx(result.get("TestDouble"), rel=1e-10) == value

    def test_string_types(self, tmp_path):
        """Test string and resref types with various cases"""
        test_strings = [
            "",  # Empty string
            "Simple ASCII",
            "A" * 1000,  # Long string
        ]

        test_idx = 0
        for test_str in test_strings:
            test_file = tmp_path / f"test_str_{test_idx}.gff"
            test_idx += 1

            # Test STRING type
            data = make_struct(
                {"TestString": test_str},
                struct_id=0,
                field_types={"TestString": GFFFieldType.STRING}
            )

            writer = GffWriter('TEST', 'V3.2')
            writer.write(str(test_file), data)

            parser = GffParser(str(test_file))
            result = parser.to_dict()

            assert result.get("TestString") == test_str

        # Test RESREF type (limited to 32 chars in NWN2)
        resref_tests = [
            "",
            "shortref",
            "exactly_32_chars_long_resource__",  # 32 chars
        ]

        for resref in resref_tests:
            test_file = tmp_path / f"test_resref_{test_idx}.gff"
            test_idx += 1

            data = make_struct(
                {"TestResRef": resref},
                struct_id=0,
                field_types={"TestResRef": GFFFieldType.RESREF}
            )

            writer = GffWriter('TEST', 'V3.2')
            writer.write(str(test_file), data)

            parser = GffParser(str(test_file))
            result = parser.to_dict()

            # ResRefs are limited to 32 chars in the format
            expected = resref[:32] if len(resref) > 32 else resref
            assert result.get("TestResRef") == expected

    def test_void_type(self, tmp_path):
        """Test VOID type (binary data)"""
        test_data = [
            b"",  # Empty
            b"\x00\x01\x02\x03",  # Binary data
            b"Binary data with text",
            bytes(range(256)),  # All byte values
        ]

        test_idx = 0
        for binary_data in test_data:
            test_file = tmp_path / f"test_void_{test_idx}.gff"
            test_idx += 1

            data = make_struct(
                {"TestVoid": binary_data},
                struct_id=0,
                field_types={"TestVoid": GFFFieldType.VOID}
            )

            writer = GffWriter('TEST', 'V3.2')
            writer.write(str(test_file), data)

            parser = GffParser(str(test_file))
            result = parser.to_dict()

            assert result.get("TestVoid") == binary_data

    def test_complex_localized_strings(self, tmp_path):
        """Test localized strings with multiple languages and genders"""
        test_file = tmp_path / "test.gff"

        # Test with various language/gender combinations
        substrings = [
            make_substring("English Male", language=0, gender=0),
            make_substring("English Female", language=0, gender=1),
            make_substring("French Male", language=2, gender=0),
            make_substring("French Female", language=2, gender=1),
            make_substring("German Male", language=4, gender=0),
            make_substring("German Female", language=4, gender=1),
        ]

        # Test with string ref
        loc_string1 = make_locstring(string_ref=12345, substrings=substrings)

        # Test without string ref (-1)
        loc_string2 = make_locstring(string_ref=-1, substrings=substrings[:2])

        data = make_struct(
            {
                "LocWithRef": loc_string1,
                "LocNoRef": loc_string2,
            },
            struct_id=0,
            field_types={
                "LocWithRef": GFFFieldType.LOCSTRING,
                "LocNoRef": GFFFieldType.LOCSTRING,
            }
        )

        writer = GffWriter('TEST', 'V3.2')
        writer.write(str(test_file), data)

        parser = GffParser(str(test_file))
        result = parser.to_dict()

        # Verify with ref
        loc1 = result.get("LocWithRef")
        assert loc1["string_ref"] == 12345
        assert len(loc1["substrings"]) == 6
        for i, sub in enumerate(loc1["substrings"]):
            assert sub["string"] == substrings[i]["string"]
            assert sub["language"] == substrings[i]["language"]
            assert sub["gender"] == substrings[i]["gender"]

        # Verify without ref
        loc2 = result.get("LocNoRef")
        # -1 may be stored as unsigned
        assert loc2["string_ref"] == -1 or loc2["string_ref"] == 0xFFFFFFFF
        assert len(loc2["substrings"]) == 2


class TestGFFEdgeCases:
    """Test edge cases and boundary conditions"""

    def test_empty_lists(self, tmp_path):
        """Test empty lists"""
        test_file = tmp_path / "test.gff"

        data = make_struct(
            {"EmptyList": []},
            struct_id=0,
            field_types={"EmptyList": GFFFieldType.LIST}
        )

        writer = GffWriter('TEST', 'V3.2')
        writer.write(str(test_file), data)

        parser = GffParser(str(test_file))
        result = parser.to_dict()

        assert result.get("EmptyList") == []

    def test_deeply_nested_structures(self, tmp_path):
        """Test deeply nested structures"""
        test_file = tmp_path / "test.gff"

        # Create a deeply nested structure
        depth = 10

        # Start with innermost
        current = make_struct(
            {"Value": depth},
            struct_id=0,
            field_types={"Value": GFFFieldType.INT}
        )

        for i in range(depth - 1, 0, -1):
            current = make_struct(
                {
                    "Level": i,
                    "Inner": current
                },
                struct_id=0,
                field_types={
                    "Level": GFFFieldType.INT,
                    "Inner": GFFFieldType.STRUCT
                }
            )

        data = make_struct(
            {"Level1": current},
            struct_id=0,
            field_types={"Level1": GFFFieldType.STRUCT}
        )

        # Round trip
        writer = GffWriter('TEST', 'V3.2')
        writer.write(str(test_file), data)

        parser = GffParser(str(test_file))
        result = parser.to_dict()

        # Navigate to the deepest level
        current = result.get("Level1")
        for i in range(1, depth):
            assert current.get("Level") == i
            current = current.get("Inner")

        assert current.get("Value") == depth

    def test_large_structures(self, tmp_path):
        """Test structures with many fields"""
        test_file = tmp_path / "test.gff"

        # Create struct with many fields
        num_fields = 1000
        fields = {}
        field_types = {}
        for i in range(num_fields):
            fields[f"Field{i:04d}"] = i
            field_types[f"Field{i:04d}"] = GFFFieldType.INT

        data = make_struct(fields, struct_id=0, field_types=field_types)

        # Round trip
        writer = GffWriter('TEST', 'V3.2')
        writer.write(str(test_file), data)

        parser = GffParser(str(test_file))
        result = parser.to_dict()

        # Verify all fields (excluding metadata)
        result_fields = {k: v for k, v in result.items()
                        if not k.startswith("__")}
        assert len(result_fields) == num_fields
        for i in range(num_fields):
            assert result.get(f"Field{i:04d}") == i

    def test_label_edge_cases(self, tmp_path):
        """Test label handling edge cases"""
        test_file = tmp_path / "test.gff"

        test_labels = [
            "",  # Empty label
            "A",  # Single char
            "ExactlySixteenCh",  # Exactly 16 chars (max)
        ]

        fields = {}
        field_types = {}
        for i, label in enumerate(test_labels):
            fields[label] = i
            field_types[label] = GFFFieldType.INT

        data = make_struct(fields, struct_id=0, field_types=field_types)

        writer = GffWriter('TEST', 'V3.2')
        writer.write(str(test_file), data)

        parser = GffParser(str(test_file))
        result = parser.to_dict()

        # Verify labels are handled correctly
        assert result.get("") == 0
        assert result.get("A") == 1
        assert result.get("ExactlySixteenCh") == 2


class TestGFFFileOperations:
    """Test file-specific operations"""

    def test_file_type_detection(self, tmp_path):
        """Test file type is preserved through write/read"""
        test_types = [
            ("test.bic", "BIC "),
            ("test.uti", "UTI "),
            ("test.utc", "UTC "),
            ("test.dlg", "DLG "),
            ("test.ifo", "IFO "),
        ]

        for filename, expected_type in test_types:
            test_file = tmp_path / filename

            # Create simple struct
            data = make_struct(
                {"Test": 1},
                struct_id=0,
                field_types={"Test": GFFFieldType.INT}
            )

            # Write with explicit file type
            writer = GffWriter(expected_type, 'V3.2')
            writer.write(str(test_file), data)

            # Read back and check file type
            parser = GffParser(str(test_file))
            assert parser.get_file_type() == expected_type

    def test_preserve_file_type(self, tmp_path):
        """Test preserving file type through read/write cycle"""
        test_file = tmp_path / "test.gff"

        # Create a file with specific type
        data = make_struct(
            {"Test": "Value"},
            struct_id=0,
            field_types={"Test": GFFFieldType.STRING}
        )

        writer = GffWriter("CUST", "V3.2")
        writer.write(str(test_file), data)

        # Read it back
        parser = GffParser(str(test_file))
        assert parser.get_file_type() == "CUST"
        assert parser.get_file_version() == "V3.2"

    def test_real_file_operations(self, tmp_path):
        """Test actual file read/write operations"""
        test_file = tmp_path / "test.gff"

        # Create complex data
        data = make_struct(
            {
                "Name": "Test Character",
                "Level": 10,
                "HP": 100.5,
            },
            struct_id=0,
            field_types={
                "Name": GFFFieldType.STRING,
                "Level": GFFFieldType.INT,
                "HP": GFFFieldType.FLOAT,
            }
        )

        # Write to file
        writer = GffWriter('TEST', 'V3.2')
        writer.write(str(test_file), data)

        # Verify file exists
        assert test_file.exists()
        assert test_file.stat().st_size > 56  # More than just header

        # Read back
        parser = GffParser(str(test_file))
        result = parser.to_dict()

        assert result.get("Name") == "Test Character"
        assert result.get("Level") == 10
        assert pytest.approx(result.get("HP"), abs=0.1) == 100.5


class TestGFFWriterDump:
    """Test GFF writer dump (bytes) functionality"""

    def test_dump_returns_bytes(self, tmp_path):
        """Test that dump returns valid bytes"""
        data = make_struct(
            {"Value": 42},
            struct_id=0,
            field_types={"Value": GFFFieldType.INT}
        )

        writer = GffWriter('TEST', 'V3.2')
        result = writer.dump(data)

        assert isinstance(result, bytes)
        assert len(result) > 56  # Header size
        assert result[0:4] == b'TEST'
        assert result[4:8] == b'V3.2'

    def test_dump_and_parse(self, tmp_path):
        """Test that dumped bytes can be parsed"""
        test_file = tmp_path / "test.gff"

        data = make_struct(
            {"TestField": "Test Value"},
            struct_id=0,
            field_types={"TestField": GFFFieldType.STRING}
        )

        writer = GffWriter('TEST', 'V3.2')
        bytes_data = writer.dump(data)

        # Write bytes to file and parse
        test_file.write_bytes(bytes_data)

        parser = GffParser(str(test_file))
        result = parser.to_dict()

        assert result.get("TestField") == "Test Value"


class TestGFFPerformance:
    """Performance tests"""

    @pytest.mark.slow
    def test_large_file_performance(self, tmp_path):
        """Test performance with large files"""
        import time
        test_file = tmp_path / "perf_test.gff"

        # Create a large structure
        num_structs = 100
        num_fields_per_struct = 50

        structs = []
        for i in range(num_structs):
            fields = {}
            field_types = {}
            for j in range(num_fields_per_struct):
                fields[f"Field{j}"] = i * 1000 + j
                field_types[f"Field{j}"] = GFFFieldType.INT
            structs.append(make_struct(fields, struct_id=0, field_types=field_types))

        data = make_struct(
            {"BigList": structs},
            struct_id=0,
            field_types={"BigList": GFFFieldType.LIST}
        )

        # Time the write operation
        start_time = time.time()
        writer = GffWriter('PERF', 'V3.2')
        writer.write(str(test_file), data)
        write_time = time.time() - start_time

        # Time the read operation
        start_time = time.time()
        parser = GffParser(str(test_file))
        result = parser.to_dict()
        read_time = time.time() - start_time

        # Verify data integrity
        big_list = result.get("BigList")
        assert len(big_list) == num_structs

        # Performance assertions
        assert write_time < 2.0  # Should write in under 2 seconds
        assert read_time < 2.0   # Should read in under 2 seconds

        # File size check
        file_size = test_file.stat().st_size
        assert file_size > 0


class TestGFFBinaryCompatibility:
    """Test GFF binary format compatibility"""

    def test_binary_compatibility(self, tmp_path):
        """Test that written files maintain NWN2 binary compatibility"""
        test_file = tmp_path / "test.bic"

        # Create a typical character-like structure
        data = make_struct(
            {
                "FirstName": "Test",
                "LastName": "Character",
                "Level": 10,
                "HP": 100.0,
                "Strength": 18,
                "Dexterity": 14,
                "Constitution": 16,
                "Intelligence": 12,
                "Wisdom": 10,
                "Charisma": 8,
            },
            struct_id=0,
            field_types={
                "FirstName": GFFFieldType.STRING,
                "LastName": GFFFieldType.STRING,
                "Level": GFFFieldType.INT,
                "HP": GFFFieldType.FLOAT,
                "Strength": GFFFieldType.BYTE,
                "Dexterity": GFFFieldType.BYTE,
                "Constitution": GFFFieldType.BYTE,
                "Intelligence": GFFFieldType.BYTE,
                "Wisdom": GFFFieldType.BYTE,
                "Charisma": GFFFieldType.BYTE,
            }
        )

        # Write as BIC file
        writer = GffWriter('BIC ', 'V3.2')
        writer.write(str(test_file), data)

        # Verify header is correct for NWN2
        content = test_file.read_bytes()
        assert content[0:8] == b'BIC V3.2'

        # Verify it can be read back
        parser = GffParser(str(test_file))
        result = parser.to_dict()

        assert parser.get_file_type() == 'BIC '
        assert result.get("FirstName") == "Test"
        assert result.get("Level") == 10


class TestGFFStructId:
    """Test struct_id preservation"""

    def test_struct_id_preservation(self, tmp_path):
        """Test that struct IDs are preserved through round trip"""
        test_file = tmp_path / "test.gff"

        # Create struct with specific struct_id
        inner = make_struct(
            {"Value": 42},
            struct_id=12345,
            field_types={"Value": GFFFieldType.INT}
        )

        data = make_struct(
            {"Inner": inner},
            struct_id=0xFFFFFFFF,
            field_types={"Inner": GFFFieldType.STRUCT}
        )

        writer = GffWriter('TEST', 'V3.2')
        writer.write(str(test_file), data)

        parser = GffParser(str(test_file))
        result = parser.to_dict()

        # Verify struct IDs
        assert result.get("__struct_id__") == 0xFFFFFFFF
        assert result.get("Inner", {}).get("__struct_id__") == 12345

    def test_list_item_struct_ids(self, tmp_path):
        """Test that list item struct IDs are preserved"""
        test_file = tmp_path / "test.gff"

        list_items = [
            make_struct({"Index": i}, struct_id=i * 10,
                       field_types={"Index": GFFFieldType.INT})
            for i in range(5)
        ]

        data = make_struct(
            {"TestList": list_items},
            struct_id=0,
            field_types={"TestList": GFFFieldType.LIST}
        )

        writer = GffWriter('TEST', 'V3.2')
        writer.write(str(test_file), data)

        parser = GffParser(str(test_file))
        result = parser.to_dict()

        items = result.get("TestList")
        for i, item in enumerate(items):
            assert item.get("Index") == i
            assert item.get("__struct_id__") == i * 10
