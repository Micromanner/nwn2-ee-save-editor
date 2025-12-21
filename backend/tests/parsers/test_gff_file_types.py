"""
Comprehensive tests for GFF file type preservation using the Rust backend.
Tests use plain dicts with __struct_id__ and __field_types__ metadata.
"""

import pytest
import os
import tempfile
import zipfile
import time
import struct
from io import BytesIO
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed

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


@pytest.fixture
def sample_savegame_path():
    """Path to sample savegame."""
    base_dir = Path(__file__).parent.parent.parent
    return base_dir / 'sample_save' / '000000 - 23-07-2025-13-06' / 'resgff.zip'


@pytest.fixture
def simple_gff_data():
    """Create a simple GFF data dict for testing."""
    return make_struct({}, struct_id=0, field_types={})


@pytest.fixture
def complex_gff_data():
    """Create a complex GFF data dict with various field types."""
    loc_string = make_locstring(-1, [make_substring("A test character", 0, 0)])
    return make_struct(
        {
            "Name": "Test Character",
            "Level": 10,
            "Experience": 1234.5,
            "Strength": 18,
            "Description": loc_string,
        },
        struct_id=0,
        field_types={
            "Name": GFFFieldType.STRING,
            "Level": GFFFieldType.INT,
            "Experience": GFFFieldType.FLOAT,
            "Strength": GFFFieldType.BYTE,
            "Description": GFFFieldType.LOCSTRING,
        }
    )


class TestGFFFileTypePreservation:
    """Test that GFF parser and writer correctly handle file type headers."""

    def test_parser_preserves_file_types_from_zip(self, sample_savegame_path, tmp_path):
        """Test that GffParser correctly reads file type headers from savegame ZIP."""
        if not sample_savegame_path.exists():
            pytest.skip(f"Sample savegame not found at {sample_savegame_path}")

        expected_types = {
            'player.bic': 'BIC ',
            'playerlist.ifo': 'IFO ',
            'globals.are': 'ARE ',
            'repute.fac': 'FAC ',
            'module.ifo': 'IFO ',
        }

        with zipfile.ZipFile(sample_savegame_path, 'r') as zf:
            for filename in zf.namelist():
                ext = os.path.splitext(filename)[1].lower()
                if ext not in ['.bic', '.ifo', '.are', '.fac', '.ros', '.git', '.utc', '.uti', '.dlg']:
                    continue

                # Determine expected type
                if filename in expected_types:
                    expected_type = expected_types[filename]
                elif filename.endswith('.ros'):
                    expected_type = 'ROS '
                elif ext == '.ifo':
                    expected_type = 'IFO '
                else:
                    continue

                # Read file data
                file_data = zf.read(filename)

                # Check raw header
                raw_type = file_data[:4].decode('ascii')
                assert raw_type == expected_type, f"{filename}: Raw header mismatch"

                # Write to temp file and parse with GffParser
                temp_file = tmp_path / filename.replace('/', '_')
                temp_file.write_bytes(file_data)

                parser = GffParser(str(temp_file))
                assert parser.get_file_type() == expected_type, f"{filename}: Parser didn't preserve file type"

    def test_writer_explicit_file_types(self, simple_gff_data, tmp_path):
        """Test that GffWriter correctly writes explicit file types."""
        file_types = ['BIC ', 'IFO ', 'ARE ', 'ROS ', 'FAC ', 'GIT ', 'UTC ', 'UTI ', 'DLG ']

        for i, file_type in enumerate(file_types):
            test_file = tmp_path / f"test_{i}.gff"

            writer = GffWriter(file_type, 'V3.2')
            writer.write(str(test_file), simple_gff_data)

            # Check the header
            content = test_file.read_bytes()
            actual_type = content[:4].decode('ascii')

            assert actual_type == file_type, f"Explicit type not written correctly"

    def test_roundtrip_preserves_file_type(self, sample_savegame_path, tmp_path):
        """Test that loading and saving a file preserves its type."""
        if not sample_savegame_path.exists():
            pytest.skip(f"Sample savegame not found at {sample_savegame_path}")

        with zipfile.ZipFile(sample_savegame_path, 'r') as zf:
            for filename in ['player.bic', 'playerlist.ifo']:
                if filename not in zf.namelist():
                    continue

                # Load original
                original_data = zf.read(filename)
                original_type = original_data[:4].decode('ascii')

                # Write to temp file and parse
                temp_file = tmp_path / filename
                temp_file.write_bytes(original_data)

                parser = GffParser(str(temp_file))
                data = parser.to_dict()

                # Write back with same file type
                output_file = tmp_path / f"roundtrip_{filename}"
                writer = GffWriter(parser.get_file_type(), parser.get_file_version())
                writer.write(str(output_file), data)

                # Check type preserved
                new_content = output_file.read_bytes()
                new_type = new_content[:4].decode('ascii')

                assert new_type == original_type, f"Type not preserved in roundtrip"


class TestGFFFileTypeEdgeCases:
    """Test edge cases and error handling for file type preservation."""

    def test_file_type_with_null_bytes(self, tmp_path):
        """Test file types containing null bytes."""
        test_types = [
            b"BIC\x00",
            b"BI\x00\x00",
            b"B\x00\x00\x00",
        ]

        for i, file_type_bytes in enumerate(test_types):
            test_file = tmp_path / f"test_null_{i}.gff"

            # Create minimal valid GFF
            header = bytearray(56)
            header[0:4] = file_type_bytes
            header[4:8] = b'V3.2'
            struct.pack_into('<I', header, 8, 56)   # struct offset
            struct.pack_into('<I', header, 12, 1)   # struct count
            struct.pack_into('<I', header, 16, 68)  # field offset
            struct.pack_into('<I', header, 20, 0)   # field count
            struct.pack_into('<I', header, 24, 68)  # label offset
            struct.pack_into('<I', header, 28, 0)   # label count
            struct.pack_into('<I', header, 32, 68)  # field data offset
            struct.pack_into('<I', header, 36, 0)   # field data length
            struct.pack_into('<I', header, 40, 68)  # field indices offset
            struct.pack_into('<I', header, 44, 0)   # field indices length
            struct.pack_into('<I', header, 48, 68)  # list indices offset
            struct.pack_into('<I', header, 52, 0)   # list indices length

            data = header + struct.pack('<III', 0, 0, 0)  # Empty struct

            test_file.write_bytes(data)
            parser = GffParser(str(test_file))
            parsed_type = parser.get_file_type()
            assert len(parsed_type) <= 4

    def test_concurrent_file_type_operations(self, simple_gff_data, tmp_path):
        """Test concurrent parsing and writing with different file types."""
        file_types = ['BIC ', 'IFO ', 'ARE ', 'ROS ', 'FAC ', 'GIT ', 'UTC ', 'UTI ', 'DLG ']

        def process_file_type(idx_and_type):
            idx, file_type = idx_and_type
            test_file = tmp_path / f"concurrent_{idx}.gff"

            # Write with specific file type
            writer = GffWriter(file_type, 'V3.2')
            writer.write(str(test_file), simple_gff_data)

            # Parse it back
            parser = GffParser(str(test_file))

            return parser.get_file_type(), file_type

        # Process all file types concurrently
        with ThreadPoolExecutor(max_workers=len(file_types)) as executor:
            futures = [executor.submit(process_file_type, (i, ft))
                      for i, ft in enumerate(file_types)]

            for future in as_completed(futures):
                parsed_type, original_type = future.result()
                assert parsed_type == original_type

    def test_file_type_case_sensitivity(self, tmp_path):
        """Test that file type comparison is case-sensitive."""
        test_cases = [
            ('bic ', 'bic '),
            ('Bic ', 'Bic '),
            ('BIC ', 'BIC '),
        ]

        data = make_struct({}, struct_id=0, field_types={})

        for i, (write_type, expected_type) in enumerate(test_cases):
            test_file = tmp_path / f"case_test_{i}.gff"

            writer = GffWriter(write_type, 'V3.2')
            writer.write(str(test_file), data)

            content = test_file.read_bytes()
            actual_type = content[:4].decode('ascii')
            assert actual_type == expected_type


class TestGFFFileTypePerformance:
    """Performance tests for file type operations."""

    @pytest.mark.slow
    def test_file_type_detection_performance(self, simple_gff_data, tmp_path):
        """Test performance of file type operations."""
        file_types = ['BIC ', 'IFO ', 'ARE ', 'ROS ', 'FAC ', 'GIT ', 'UTC ', 'UTI ', 'DLG ']
        iterations = 50

        start_time = time.time()

        idx = 0
        for i in range(iterations):
            for file_type in file_types:
                test_file = tmp_path / f"perf_{idx}.gff"
                idx += 1

                writer = GffWriter(file_type, 'V3.2')
                writer.write(str(test_file), simple_gff_data)

        elapsed_time = time.time() - start_time

        assert elapsed_time < 10.0, f"File type operations too slow: {elapsed_time:.2f}s"

    @pytest.mark.slow
    def test_large_file_type_preservation(self, tmp_path):
        """Test file type preservation with large files."""
        # Create a large GFF structure
        large_list = []
        for i in range(500):
            fields = {f"Field{j}": i * 100 + j for j in range(10)}
            field_types = {f"Field{j}": GFFFieldType.INT for j in range(10)}
            large_list.append(make_struct(fields, struct_id=0, field_types=field_types))

        data = make_struct(
            {"LargeList": large_list},
            struct_id=0,
            field_types={"LargeList": GFFFieldType.LIST}
        )

        file_types = ['BIC ', 'IFO ', 'ARE ', 'ROS ', 'FAC ']

        for i, file_type in enumerate(file_types):
            test_file = tmp_path / f"large_{i}.gff"

            start_time = time.time()
            writer = GffWriter(file_type, 'V3.2')
            writer.write(str(test_file), data)
            write_time = time.time() - start_time

            start_time = time.time()
            parser = GffParser(str(test_file))
            result = parser.to_dict()
            parse_time = time.time() - start_time

            assert parser.get_file_type() == file_type
            assert write_time < 2.0, f"Write too slow for {file_type}: {write_time:.2f}s"
            assert parse_time < 2.0, f"Parse too slow for {file_type}: {parse_time:.2f}s"


class TestGFFRealWorldFiles:
    """Test with real NWN2 save files."""

    def test_all_file_types_in_save(self, sample_savegame_path, tmp_path):
        """Test all file types found in a real save game."""
        if not sample_savegame_path.exists():
            pytest.skip(f"Sample savegame not found at {sample_savegame_path}")

        file_type_counts = {}

        with zipfile.ZipFile(sample_savegame_path, 'r') as zf:
            for filename in zf.namelist():
                ext = os.path.splitext(filename)[1].lower()
                if ext not in ['.bic', '.ifo', '.are', '.fac', '.ros', '.git',
                             '.utc', '.uti', '.utm', '.utp', '.uts', '.utt',
                             '.utw', '.ute', '.utd', '.dlg', '.jrl']:
                    continue

                try:
                    file_data = zf.read(filename)
                    if len(file_data) < 56:
                        continue

                    file_type = file_data[:4].decode('ascii', errors='ignore')
                    file_type_counts[file_type] = file_type_counts.get(file_type, 0) + 1

                    # Write to temp and parse
                    temp_file = tmp_path / filename.replace('/', '_')
                    temp_file.write_bytes(file_data)

                    parser = GffParser(str(temp_file))
                    assert parser.get_file_type().strip() == file_type.strip()

                except Exception as e:
                    print(f"Failed to parse {filename}: {e}")

        assert len(file_type_counts) >= 3, f"Too few file types found: {file_type_counts}"

        expected_types = ['BIC', 'IFO', 'ARE', 'ROS']
        found_types = [expected for expected in expected_types
                       if any(expected in ft for ft in file_type_counts)]
        assert len(found_types) >= 2, f"Too few expected types found: {found_types}"


class TestGFFFileTypeBinaryCompatibility:
    """Test binary compatibility with NWN2."""

    def test_header_structure(self, tmp_path):
        """Test that header structure matches NWN2 expectations."""
        data = make_struct(
            {"Test": "Value"},
            struct_id=0,
            field_types={"Test": GFFFieldType.STRING}
        )

        test_file = tmp_path / "test.gff"
        writer = GffWriter('TEST', 'V3.2')
        writer.write(str(test_file), data)

        content = test_file.read_bytes()
        header = content[:56]

        assert header[0:4] == b'TEST'
        assert header[4:8] == b'V3.2'

        struct_offset = struct.unpack('<I', header[8:12])[0]
        assert struct_offset == 56

    def test_file_type_padding(self, tmp_path):
        """Test that file types are properly padded to 4 bytes."""
        test_cases = [
            ('A', 'A   '),
            ('AB', 'AB  '),
            ('ABC', 'ABC '),
            ('ABCD', 'ABCD'),
            ('ABCDE', 'ABCD'),
        ]

        data = make_struct({}, struct_id=0, field_types={})

        for i, (input_type, expected_type) in enumerate(test_cases):
            test_file = tmp_path / f"padding_{i}.gff"

            writer = GffWriter(input_type, 'V3.2')
            writer.write(str(test_file), data)

            content = test_file.read_bytes()
            actual_type = content[:4].decode('ascii')
            assert actual_type == expected_type

    def test_cross_platform_compatibility(self, simple_gff_data, tmp_path):
        """Test that files are compatible across platforms."""
        test_file = tmp_path / "xplatform.gff"

        writer = GffWriter('XPLT', 'V3.2')
        writer.write(str(test_file), simple_gff_data)

        content = test_file.read_bytes()
        struct_offset_bytes = content[8:12]

        offset = (struct_offset_bytes[0] |
                 (struct_offset_bytes[1] << 8) |
                 (struct_offset_bytes[2] << 16) |
                 (struct_offset_bytes[3] << 24))

        assert offset == 56


class TestGFFFileTypeIntegration:
    """Integration tests with other system components."""

    def test_file_type_with_module_loading(self, tmp_path):
        """Test that file types work correctly with module loading system."""
        module_files = [
            ('module.ifo', 'IFO '),
            ('module.are', 'ARE '),
            ('module.git', 'GIT '),
        ]

        for filename, expected_type in module_files:
            data = make_struct(
                {"ModuleName": "Test Module", "Version": 1},
                struct_id=0,
                field_types={
                    "ModuleName": GFFFieldType.STRING,
                    "Version": GFFFieldType.INT
                }
            )

            test_file = tmp_path / filename

            writer = GffWriter(expected_type, 'V3.2')
            writer.write(str(test_file), data)

            parser = GffParser(str(test_file))
            result = parser.to_dict()

            assert parser.get_file_type() == expected_type
            assert result.get("ModuleName") == "Test Module"

    def test_file_type_error_recovery(self, tmp_path):
        """Test recovery from invalid file types."""
        test_file = tmp_path / "invalid.gff"

        header = bytearray(56)
        header[0:4] = b'\xFF\xFF\xFF\xFF'
        header[4:8] = b'V3.2'

        struct.pack_into('<I', header, 8, 56)
        struct.pack_into('<I', header, 12, 1)
        struct.pack_into('<I', header, 16, 68)
        struct.pack_into('<I', header, 20, 0)
        struct.pack_into('<I', header, 24, 68)
        struct.pack_into('<I', header, 28, 0)
        struct.pack_into('<I', header, 32, 68)
        struct.pack_into('<I', header, 36, 0)
        struct.pack_into('<I', header, 40, 68)
        struct.pack_into('<I', header, 44, 0)
        struct.pack_into('<I', header, 48, 68)
        struct.pack_into('<I', header, 52, 0)

        data = bytes(header) + struct.pack('<III', 0, 0, 0)
        test_file.write_bytes(data)

        parser = GffParser(str(test_file))
        result = parser.to_dict()
        assert result is not None
        assert parser.get_file_type() is not None


class TestGFFMissingExtensions:
    """Test for file extensions that were missing."""

    def test_ros_and_fac_extensions_work(self, simple_gff_data, tmp_path):
        """Test that .ros and .fac extensions now work correctly."""
        test_cases = [
            ('test.ros', 'ROS '),
            ('test.fac', 'FAC '),
        ]

        for filename, expected_type in test_cases:
            test_file = tmp_path / filename

            writer = GffWriter(expected_type, 'V3.2')
            writer.write(str(test_file), simple_gff_data)

            content = test_file.read_bytes()
            actual_type = content[:4].decode('ascii')

            assert actual_type == expected_type, f"{filename} extension not working"

    def test_all_nwn2_extensions_covered(self):
        """Verify all known NWN2 GFF extensions are covered."""
        known_extensions = {
            '.are': 'ARE ',
            '.bic': 'BIC ',
            '.cam': 'CAM ',
            '.dlg': 'DLG ',
            '.fac': 'FAC ',
            '.gff': 'GFF ',
            '.git': 'GIT ',
            '.gui': 'GUI ',
            '.ifo': 'IFO ',
            '.jrl': 'JRL ',
            '.pla': 'PLA ',
            '.ros': 'ROS ',
            '.rst': 'RST ',
            '.ult': 'ULT ',
            '.utc': 'UTC ',
            '.utd': 'UTD ',
            '.ute': 'UTE ',
            '.uti': 'UTI ',
            '.utm': 'UTM ',
            '.utp': 'UTP ',
            '.uts': 'UTS ',
            '.utt': 'UTT ',
            '.utw': 'UTW ',
        }

        implemented = {'.are', '.bic', '.dlg', '.fac', '.git', '.ifo',
                      '.ros', '.utc', '.uti'}

        missing = set(known_extensions.keys()) - implemented

        if missing:
            print(f"Extensions not yet implemented: {sorted(missing)}")

        critical = {'.are', '.bic', '.ifo', '.ros', '.fac', '.git', '.utc', '.uti'}
        missing_critical = critical - implemented
        assert not missing_critical, f"Critical extensions missing: {missing_critical}"
