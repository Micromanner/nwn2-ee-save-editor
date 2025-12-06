"""
Comprehensive tests for ERF writer functionality
Tests creating, modifying, and writing ERF/MOD/HAK files
"""

import pytest
import struct
import os
import tempfile
from pathlib import Path

from nwn2_rust import ErfParser


class TestErfWriterCreate:
    """Tests for creating new ERF archives"""

    def test_create_new_mod_v10(self):
        """Test creating a new MOD file with V1.0 format"""
        parser = ErfParser.new_archive("MOD", "V1.0")

        assert parser.get_erf_type() == "MOD"
        assert parser.get_version() == "V1.0"
        assert parser.get_resource_count() == 0

    def test_create_new_mod_v11(self):
        """Test creating a new MOD file with V1.1 format"""
        parser = ErfParser.new_archive("MOD", "V1.1")

        assert parser.get_erf_type() == "MOD"
        assert parser.get_version() == "V1.1"
        assert parser.get_resource_count() == 0

    def test_create_new_hak(self):
        """Test creating a new HAK file"""
        parser = ErfParser.new_archive("HAK")

        assert parser.get_erf_type() == "HAK"
        assert parser.get_version() == "V1.1"
        assert parser.get_resource_count() == 0

    def test_create_new_erf(self):
        """Test creating a new ERF file"""
        parser = ErfParser.new_archive("ERF")

        assert parser.get_erf_type() == "ERF"
        assert parser.get_version() == "V1.1"
        assert parser.get_resource_count() == 0

    def test_create_invalid_type(self):
        """Test error handling for invalid ERF type"""
        with pytest.raises(ValueError, match="Invalid ERF type"):
            ErfParser.new_archive("INVALID")

    def test_create_invalid_version(self):
        """Test error handling for invalid version"""
        with pytest.raises(ValueError, match="Invalid version"):
            ErfParser.new_archive("MOD", "V2.0")


class TestErfWriterAddResource:
    """Tests for adding resources to ERF archives"""

    def test_add_single_resource(self):
        """Test adding a single resource"""
        parser = ErfParser.new_archive("HAK")
        test_data = b"2DA V2.0\n\nColumn1 Column2\n0 Value1 Value2\n"

        parser.add_resource("test", 2017, test_data)

        assert parser.get_resource_count() == 1
        assert parser.has_resource("test.2da")
        assert parser.get_resource_size("test.2da") == len(test_data)
        assert parser.get_resource_type("test.2da") == 2017

    def test_add_multiple_resources(self):
        """Test adding multiple resources"""
        parser = ErfParser.new_archive("MOD")

        resources = [
            ("test1", 2017, b"2DA data 1"),
            ("test2", 2017, b"2DA data 2"),
            ("module", 2014, b"IFO data"),
            ("item", 2025, b"UTI data"),
        ]

        for name, res_type, data in resources:
            parser.add_resource(name, res_type, data)

        assert parser.get_resource_count() == 4
        assert parser.has_resource("test1.2da")
        assert parser.has_resource("test2.2da")
        assert parser.has_resource("module.ifo")
        assert parser.has_resource("item.uti")

    def test_add_resource_case_insensitive(self):
        """Test that resource lookup is case insensitive"""
        parser = ErfParser.new_archive("HAK")
        parser.add_resource("TestFile", 2017, b"test data")

        assert parser.has_resource("testfile.2da")
        assert parser.has_resource("TESTFILE.2DA")
        assert parser.has_resource("TestFile.2da")

    def test_add_resource_with_extension(self):
        """Test adding resource with extension in name"""
        parser = ErfParser.new_archive("HAK")
        parser.add_resource("test.2da", 2017, b"2DA data")

        assert parser.has_resource("test.2da")


class TestErfWriterRemoveResource:
    """Tests for removing resources from ERF archives"""

    def test_remove_existing_resource(self):
        """Test removing an existing resource"""
        parser = ErfParser.new_archive("HAK")
        parser.add_resource("test", 2017, b"test data")
        parser.add_resource("test2", 2017, b"test data 2")

        assert parser.get_resource_count() == 2

        result = parser.remove_resource("test.2da")
        assert result is True
        assert parser.get_resource_count() == 1
        assert not parser.has_resource("test.2da")
        assert parser.has_resource("test2.2da")

    def test_remove_nonexistent_resource(self):
        """Test removing a non-existent resource returns False"""
        parser = ErfParser.new_archive("HAK")
        parser.add_resource("test", 2017, b"test data")

        result = parser.remove_resource("nonexistent.2da")
        assert result is False
        assert parser.get_resource_count() == 1


class TestErfWriterUpdateResource:
    """Tests for updating resources in ERF archives"""

    def test_update_existing_resource(self):
        """Test updating an existing resource"""
        parser = ErfParser.new_archive("HAK")
        original_data = b"original data"
        new_data = b"updated data with more content"

        parser.add_resource("test", 2017, original_data)
        assert parser.get_resource_size("test.2da") == len(original_data)

        parser.update_resource("test.2da", new_data)
        assert parser.get_resource_size("test.2da") == len(new_data)

    def test_update_nonexistent_resource(self):
        """Test error when updating non-existent resource"""
        parser = ErfParser.new_archive("HAK")

        with pytest.raises(ValueError, match="Resource not found"):
            parser.update_resource("nonexistent.2da", b"data")


class TestErfWriterToBytes:
    """Tests for serializing ERF archives to bytes"""

    def test_to_bytes_empty_archive(self):
        """Test serializing an empty archive"""
        parser = ErfParser.new_archive("HAK", "V1.1")
        data = parser.to_bytes()

        assert data[:4] == b"HAK "
        assert data[4:8] == b"V1.1"
        assert len(data) == 160

    def test_to_bytes_with_resources(self):
        """Test serializing an archive with resources"""
        parser = ErfParser.new_archive("MOD", "V1.0")
        test_data = b"2DA V2.0\ntest content"

        parser.add_resource("test", 2017, test_data)
        data = parser.to_bytes()

        assert data[:4] == b"MOD "
        assert data[4:8] == b"V1.0"

        entry_count = struct.unpack_from("<I", data, 16)[0]
        assert entry_count == 1

        assert test_data in data

    def test_to_bytes_v10_key_size(self):
        """Test V1.0 format uses 16-char names (24 byte key entries)"""
        parser = ErfParser.new_archive("MOD", "V1.0")
        parser.add_resource("test", 2017, b"test data")
        parser.add_resource("test2", 2017, b"test data 2")
        data = parser.to_bytes()

        offset_to_keys = struct.unpack_from("<I", data, 24)[0]
        offset_to_resources = struct.unpack_from("<I", data, 28)[0]
        key_section_size = offset_to_resources - offset_to_keys
        assert key_section_size == 2 * 24

    def test_to_bytes_v11_key_size(self):
        """Test V1.1 format uses 32-char names (40 byte key entries)"""
        parser = ErfParser.new_archive("MOD", "V1.1")
        parser.add_resource("test", 2017, b"test data")
        parser.add_resource("test2", 2017, b"test data 2")
        data = parser.to_bytes()

        offset_to_keys = struct.unpack_from("<I", data, 24)[0]
        offset_to_resources = struct.unpack_from("<I", data, 28)[0]
        key_section_size = offset_to_resources - offset_to_keys
        assert key_section_size == 2 * 40


class TestErfWriterWrite:
    """Tests for writing ERF archives to files"""

    def test_write_mod_file(self):
        """Test writing a MOD file"""
        parser = ErfParser.new_archive("MOD", "V1.0")
        test_data = b"GFF module data content"
        parser.add_resource("module", 2014, test_data)

        with tempfile.NamedTemporaryFile(suffix=".mod", delete=False) as f:
            output_path = f.name

        try:
            parser.write(output_path)
            assert os.path.exists(output_path)

            with open(output_path, "rb") as f:
                written_data = f.read()

            assert written_data[:4] == b"MOD "
            assert test_data in written_data
        finally:
            os.unlink(output_path)

    def test_write_hak_file(self):
        """Test writing a HAK file"""
        parser = ErfParser.new_archive("HAK", "V1.1")
        parser.add_resource("appearances", 2017, b"2DA appearance data")
        parser.add_resource("baseitems", 2017, b"2DA baseitems data")

        with tempfile.NamedTemporaryFile(suffix=".hak", delete=False) as f:
            output_path = f.name

        try:
            parser.write(output_path)
            assert os.path.exists(output_path)

            with open(output_path, "rb") as f:
                written_data = f.read()

            assert written_data[:4] == b"HAK "
            assert written_data[4:8] == b"V1.1"
        finally:
            os.unlink(output_path)


class TestErfWriterRoundTrip:
    """Tests for read-modify-write cycles"""

    def test_roundtrip_v10(self):
        """Test read -> write -> read cycle for V1.0"""
        original = ErfParser.new_archive("MOD", "V1.0")
        resources = [
            ("test1", 2017, b"2DA data content 1"),
            ("module", 2014, b"IFO module info"),
            ("item", 2025, b"UTI item template"),
        ]

        for name, res_type, data in resources:
            original.add_resource(name, res_type, data)

        with tempfile.TemporaryDirectory() as tmpdir:
            output_path = os.path.join(tmpdir, "test.mod")
            original.write(output_path)

            data = original.to_bytes()
            loaded = ErfParser()
            loaded.parse_from_bytes(data)

            assert loaded.get_erf_type() == "MOD"
            assert loaded.get_version() == "V1.0"
            assert loaded.get_resource_count() == 3

            for name, res_type, expected_data in resources:
                full_name = f"{name}.{self._type_to_ext(res_type)}"
                assert loaded.has_resource(full_name)
                actual_data = loaded.extract_resource(full_name)
                assert actual_data == expected_data

    def test_roundtrip_v11(self):
        """Test read -> write -> read cycle for V1.1"""
        original = ErfParser.new_archive("HAK", "V1.1")
        long_name = "verylongresourcename"
        resources = [
            (long_name, 2017, b"2DA data with long name"),
            ("short", 2025, b"UTI data"),
        ]

        for name, res_type, data in resources:
            original.add_resource(name, res_type, data)

        data = original.to_bytes()
        loaded = ErfParser()
        loaded.parse_from_bytes(data)

        assert loaded.get_erf_type() == "HAK"
        assert loaded.get_version() == "V1.1"
        assert loaded.get_resource_count() == 2

        for name, res_type, expected_data in resources:
            full_name = f"{name}.{self._type_to_ext(res_type)}"
            assert loaded.has_resource(full_name)
            actual_data = loaded.extract_resource(full_name)
            assert actual_data == expected_data

    def test_roundtrip_modify_and_save(self):
        """Test read -> modify -> write -> read cycle using bytes"""
        original = ErfParser.new_archive("MOD", "V1.1")
        original.add_resource("original", 2017, b"original data")

        data1 = original.to_bytes()

        modified = ErfParser()
        modified.parse_from_bytes(data1)
        modified.load_all_resources()

        modified.add_resource("newfile", 2025, b"new item data")
        modified.update_resource("original.2da", b"modified data")

        data2 = modified.to_bytes()

        final = ErfParser()
        final.parse_from_bytes(data2)

        assert final.get_resource_count() == 2
        assert final.has_resource("original.2da")
        assert final.has_resource("newfile.uti")
        assert final.extract_resource("original.2da") == b"modified data"
        assert final.extract_resource("newfile.uti") == b"new item data"

    def _type_to_ext(self, res_type: int) -> str:
        """Helper to map resource type to extension"""
        type_map = {
            2017: "2da",
            2014: "ifo",
            2025: "uti",
            2027: "utc",
        }
        return type_map.get(res_type, "unk")


class TestErfWriterLargeFiles:
    """Tests for handling large ERF files"""

    def test_many_resources(self):
        """Test creating archive with many resources"""
        parser = ErfParser.new_archive("HAK", "V1.1")

        for i in range(100):
            data = f"Resource {i} content data".encode()
            parser.add_resource(f"resource{i}", 2017, data)

        assert parser.get_resource_count() == 100

        data = parser.to_bytes()
        entry_count = struct.unpack_from("<I", data, 16)[0]
        assert entry_count == 100

    def test_large_resource(self):
        """Test creating archive with large resource"""
        parser = ErfParser.new_archive("MOD", "V1.1")
        large_data = b"X" * (1024 * 1024)

        parser.add_resource("large", 2017, large_data)

        data = parser.to_bytes()

        loaded = ErfParser()
        loaded.parse_from_bytes(data)

        extracted = loaded.extract_resource("large.2da")
        assert extracted == large_data


class TestErfWriterEdgeCases:
    """Tests for edge cases and error handling"""

    def test_empty_resource_data(self):
        """Test adding resource with empty data"""
        parser = ErfParser.new_archive("HAK")
        parser.add_resource("empty", 2017, b"")

        assert parser.has_resource("empty.2da")
        assert parser.get_resource_size("empty.2da") == 0

        data = parser.to_bytes()
        loaded = ErfParser()
        loaded.parse_from_bytes(data)

        extracted = loaded.extract_resource("empty.2da")
        assert extracted == b""

    def test_binary_resource_data(self):
        """Test adding resource with binary data"""
        parser = ErfParser.new_archive("MOD")
        binary_data = bytes(range(256))

        parser.add_resource("binary", 2037, binary_data)

        data = parser.to_bytes()
        loaded = ErfParser()
        loaded.parse_from_bytes(data)

        extracted = loaded.extract_resource("binary.gff")
        assert extracted == binary_data

    def test_special_characters_in_data(self):
        """Test resource data with null bytes and special characters"""
        parser = ErfParser.new_archive("HAK")
        special_data = b"\x00\x01\x02\xff\xfe\xfd" + b"normal text" + b"\x00" * 10

        parser.add_resource("special", 2037, special_data)

        data = parser.to_bytes()
        loaded = ErfParser()
        loaded.parse_from_bytes(data)

        extracted = loaded.extract_resource("special.gff")
        assert extracted == special_data


class TestErfWriterContains:
    """Tests for __contains__ protocol"""

    def test_contains_existing(self):
        """Test 'in' operator for existing resource"""
        parser = ErfParser.new_archive("HAK")
        parser.add_resource("test", 2017, b"data")

        assert "test.2da" in parser

    def test_contains_nonexistent(self):
        """Test 'in' operator for non-existent resource"""
        parser = ErfParser.new_archive("HAK")

        assert "nonexistent.2da" not in parser


class TestErfWriterLen:
    """Tests for __len__ protocol"""

    def test_len_empty(self):
        """Test len() on empty archive"""
        parser = ErfParser.new_archive("HAK")
        assert len(parser) == 0

    def test_len_with_resources(self):
        """Test len() with resources"""
        parser = ErfParser.new_archive("HAK")
        parser.add_resource("test1", 2017, b"data1")
        parser.add_resource("test2", 2017, b"data2")

        assert len(parser) == 2
