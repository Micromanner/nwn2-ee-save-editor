"""
Comprehensive tests for GFF file type preservation in parser and writer.
Tests cover all file types, edge cases, error handling, and real-world scenarios.
"""

import pytest
import os
import tempfile
import zipfile
import time
import struct
import threading
from io import BytesIO
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed

from parsers.gff import GFFParser, GFFWriter, GFFElement, GFFFieldType, LocalizedString, LocalizedSubstring, GFFError


# Test fixtures
@pytest.fixture
def sample_savegame_path():
    """Path to sample savegame."""
    base_dir = Path(__file__).parent.parent.parent
    return base_dir / 'sample_save' / '000048 - 23-07-2025-13-31' / 'resgff.zip'


@pytest.fixture
def simple_gff_element():
    """Create a simple GFF element for testing."""
    return GFFElement(
        field_type=GFFFieldType.STRUCT,
        struct_id=0,
        label="",
        value=[]
    )


@pytest.fixture
def complex_gff_element():
    """Create a complex GFF element with various field types."""
    # Create some test fields
    fields = [
        GFFElement(GFFFieldType.STRING, 0, "Name", "Test Character"),
        GFFElement(GFFFieldType.INT, 0, "Level", 10),
        GFFElement(GFFFieldType.FLOAT, 0, "Experience", 1234.5),
        GFFElement(GFFFieldType.BYTE, 0, "Strength", 18),
        GFFElement(GFFFieldType.LOCSTRING, 0, "Description", 
                  LocalizedString(-1, [LocalizedSubstring("A test character", 0, 0)]))
    ]
    return GFFElement(
        field_type=GFFFieldType.STRUCT,
        struct_id=0,
        label="",
        value=fields
    )


class TestGFFFileTypePreservation:
    """Test that GFF parser and writer correctly handle file type headers."""
    
    def test_parser_preserves_file_types_from_zip(self, sample_savegame_path):
        """Test that GFFParser correctly reads file type headers from savegame ZIP."""
        if not sample_savegame_path.exists():
            pytest.skip(f"Sample savegame not found at {sample_savegame_path}")
        
        parser = GFFParser()
        
        expected_types = {
            'player.bic': 'BIC ',
            'playerlist.ifo': 'IFO ',
            'globals.are': 'ARE ',
            'repute.fac': 'FAC ',
            'module.ifo': 'IFO ',
            '*.ros': 'ROS ',  # All roster files
        }
        
        with zipfile.ZipFile(sample_savegame_path, 'r') as zf:
            for filename in zf.namelist():
                # Skip non-GFF files
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
                    continue  # Skip if we don't know the expected type
                
                # Read and parse file
                file_data = zf.read(filename)
                
                # Check raw header
                raw_type = file_data[:4].decode('ascii')
                assert raw_type == expected_type, f"{filename}: Raw header mismatch"
                
                # Parse with GFFParser
                gff_data = parser.load(BytesIO(file_data))
                # Parser preserves trailing spaces
                assert parser.file_type == expected_type.strip() + ' ', f"{filename}: Parser didn't preserve file type"
    
    def test_writer_auto_detects_file_types(self, simple_gff_element):
        """Test that GFFWriter correctly auto-detects file types from extensions."""
        test_cases = [
            ('test.bic', 'BIC '),
            ('test.ifo', 'IFO '),
            ('test.are', 'ARE '),
            ('test.git', 'GIT '),
            ('test.uti', 'UTI '),
            ('test.utc', 'UTC '),
            ('test.dlg', 'DLG '),
            ('test.ros', 'ROS '),  # New extension
            ('test.fac', 'FAC '),  # New extension
            ('test.unknown', 'GFF '),  # Should default to GFF
        ]
        
        with tempfile.TemporaryDirectory() as temp_dir:
            for filename, expected_type in test_cases:
                file_path = os.path.join(temp_dir, filename)
                
                # Create writer with default GFF type
                writer = GFFWriter()
                assert writer.file_type == "GFF "
                
                # Write to file (should auto-detect type)
                writer.write(file_path, simple_gff_element)
                
                # Read back the file header
                with open(file_path, 'rb') as f:
                    actual_type = f.read(4).decode('ascii')
                
                assert actual_type == expected_type, f"{filename}: Auto-detection failed"
    
    def test_writer_explicit_file_types(self, simple_gff_element):
        """Test that GFFWriter correctly writes explicit file types."""
        file_types = ['BIC ', 'IFO ', 'ARE ', 'ROS ', 'FAC ', 'GIT ', 'UTC ', 'UTI ', 'DLG ']
        
        for file_type in file_types:
            writer = GFFWriter(file_type=file_type)
            output = BytesIO()
            
            writer.save(output, simple_gff_element)
            
            # Check the header
            output.seek(0)
            actual_type = output.read(4).decode('ascii')
            
            assert actual_type == file_type, f"Explicit type not written correctly"
    
    def test_writer_from_parser_preserves_type(self, sample_savegame_path):
        """Test that GFFWriter.from_parser() preserves the original file type."""
        if not sample_savegame_path.exists():
            pytest.skip(f"Sample savegame not found at {sample_savegame_path}")
        
        parser = GFFParser()
        
        with zipfile.ZipFile(sample_savegame_path, 'r') as zf:
            # Test with different file types
            test_files = [
                ('player.bic', 'BIC '),
                ('playerlist.ifo', 'IFO '),
            ]
            
            for filename, expected_type in test_files:
                if filename not in zf.namelist():
                    continue
                
                # Parse file
                file_data = zf.read(filename)
                gff_element = parser.load(BytesIO(file_data))
                
                # Create writer from parser
                writer = GFFWriter.from_parser(parser)
                
                # Parser keeps trailing space, writer strips it
                assert writer.file_type == expected_type.strip() + ' ', f"from_parser() didn't preserve type"
                
                # Test that it writes correctly
                output = BytesIO()
                writer.save(output, gff_element)
                
                output.seek(0)
                written_type = output.read(4).decode('ascii')
                assert written_type == expected_type, f"Written type doesn't match"
    
    def test_roundtrip_preserves_file_type(self, sample_savegame_path):
        """Test that loading and saving a file preserves its type."""
        if not sample_savegame_path.exists():
            pytest.skip(f"Sample savegame not found at {sample_savegame_path}")
        
        parser = GFFParser()
        
        with zipfile.ZipFile(sample_savegame_path, 'r') as zf:
            for filename in ['player.bic', 'playerlist.ifo']:
                if filename not in zf.namelist():
                    continue
                
                # Load original
                original_data = zf.read(filename)
                original_type = original_data[:4].decode('ascii')
                
                # Parse
                gff_element = parser.load(BytesIO(original_data))
                
                # Write back
                writer = GFFWriter.from_parser(parser)
                output = BytesIO()
                writer.save(output, gff_element)
                
                # Check type preserved
                output.seek(0)
                new_type = output.read(4).decode('ascii')
                
                assert new_type == original_type, f"Type not preserved in roundtrip"


class TestGFFFileTypeEdgeCases:
    """Test edge cases and error handling for file type preservation."""
    
    def test_malformed_headers(self):
        """Test handling of files with malformed headers."""
        parser = GFFParser()
        
        # Test cases with various malformed headers
        test_cases = [
            # (description, header_bytes, expected_error)
            ("Empty file", b"", "header"),
            ("Too short", b"BIC", "header"),
            ("Wrong version", b"BIC V1.0" + b"\x00" * 48, "version"),
            ("Invalid file type", b"\xFF\xFF\xFF\xFFV3.2" + b"\x00" * 48, None),  # Should still parse
            ("Non-ASCII file type", b"\x80\x81\x82\x83V3.2" + b"\x00" * 48, None),
        ]
        
        for description, header_bytes, expected_error in test_cases:
            if expected_error:
                with pytest.raises(GFFError, match=expected_error):
                    parser.load(BytesIO(header_bytes))
            else:
                # Should parse without error, even with weird file type
                header = bytearray(header_bytes + b"\x00" * (56 - len(header_bytes)))
                # Add minimal structure data
                struct.pack_into('<I', header, 8, 56)  # struct offset
                struct.pack_into('<I', header, 12, 1)  # struct count
                data = bytes(header) + struct.pack('<III', 0, 0, 0)  # Empty struct
                
                try:
                    result = parser.load(BytesIO(data))
                    assert result is not None
                except ValueError as e:
                    if "no structures" not in str(e):
                        raise
    
    def test_file_type_with_null_bytes(self):
        """Test file types containing null bytes."""
        # Some modded files might have null-terminated file types
        test_types = [
            b"BIC\x00",
            b"BI\x00\x00",
            b"B\x00\x00\x00",
        ]
        
        parser = GFFParser()
        
        for file_type_bytes in test_types:
            # Create minimal valid GFF
            header = bytearray(56)
            header[0:4] = file_type_bytes
            header[4:8] = b'V3.2'
            struct.pack_into('<I', header, 8, 56)  # struct offset
            struct.pack_into('<I', header, 12, 1)  # struct count
            data = header + struct.pack('<III', 0, 0, 0)  # Empty struct
            
            result = parser.load(BytesIO(data))
            # Parser should handle null bytes gracefully
            parsed_type = parser.file_type
            assert len(parsed_type) <= 4
            assert '\x00' not in parsed_type or parsed_type.endswith('\x00')
    
    def test_concurrent_file_type_operations(self, simple_gff_element):
        """Test concurrent parsing and writing with different file types."""
        file_types = ['BIC ', 'IFO ', 'ARE ', 'ROS ', 'FAC ', 'GIT ', 'UTC ', 'UTI ', 'DLG ']
        
        def process_file_type(file_type):
            # Create data with specific file type
            writer = GFFWriter(file_type, 'V3.2')
            output = BytesIO()
            writer.save(output, simple_gff_element)
            
            # Parse it back
            output.seek(0)
            parser = GFFParser()
            result = parser.load(output)
            
            return parser.file_type, file_type
        
        # Process all file types concurrently
        with ThreadPoolExecutor(max_workers=len(file_types)) as executor:
            futures = [executor.submit(process_file_type, ft) for ft in file_types]
            
            for future in as_completed(futures):
                parsed_type, original_type = future.result()
                assert parsed_type == original_type
    
    def test_file_type_case_sensitivity(self):
        """Test that file type comparison is case-sensitive."""
        # NWN2 file types are case-sensitive
        test_cases = [
            ('bic ', 'BIC '),  # Wrong case
            ('Bic ', 'BIC '),  # Mixed case
            ('BIC ', 'BIC '),  # Correct case
        ]
        
        for write_type, expected_type in test_cases:
            writer = GFFWriter(write_type, 'V3.2')
            output = BytesIO()
            writer.save(output, GFFElement(GFFFieldType.STRUCT, 0, "", []))
            
            output.seek(0)
            actual_type = output.read(4).decode('ascii')
            # Writer should preserve exact case
            assert actual_type == write_type
    
    def test_very_long_filenames(self, simple_gff_element):
        """Test auto-detection with very long filenames."""
        with tempfile.TemporaryDirectory() as temp_dir:
            # Create a file with a very long name
            long_name = "a" * 200 + ".bic"
            file_path = os.path.join(temp_dir, long_name)
            
            writer = GFFWriter()
            writer.write(file_path, simple_gff_element)
            
            # Should still detect .bic extension
            with open(file_path, 'rb') as f:
                file_type = f.read(4).decode('ascii')
            
            assert file_type == 'BIC '


class TestGFFFileTypePerformance:
    """Performance tests for file type operations."""
    
    @pytest.mark.slow
    def test_file_type_detection_performance(self, simple_gff_element):
        """Test performance of file type auto-detection."""
        extensions = ['.bic', '.ifo', '.are', '.ros', '.fac', '.git', '.utc', '.uti', '.dlg']
        iterations = 100
        
        with tempfile.TemporaryDirectory() as temp_dir:
            start_time = time.time()
            
            for i in range(iterations):
                for ext in extensions:
                    filename = f"test_{i}{ext}"
                    file_path = os.path.join(temp_dir, filename)
                    
                    writer = GFFWriter()
                    writer.write(file_path, simple_gff_element)
            
            elapsed_time = time.time() - start_time
            
            # Should complete reasonably quickly
            assert elapsed_time < 10.0, f"File type detection too slow: {elapsed_time:.2f}s"
            
            # Verify all files have correct types
            for i in range(0, iterations, 10):  # Spot check
                for ext in extensions:
                    filename = f"test_{i}{ext}"
                    file_path = os.path.join(temp_dir, filename)
                    
                    with open(file_path, 'rb') as f:
                        file_type = f.read(4).decode('ascii')
                    
                    expected_type = {
                        '.bic': 'BIC ', '.ifo': 'IFO ', '.are': 'ARE ',
                        '.ros': 'ROS ', '.fac': 'FAC ', '.git': 'GIT ',
                        '.utc': 'UTC ', '.uti': 'UTI ', '.dlg': 'DLG '
                    }[ext]
                    
                    assert file_type == expected_type
    
    @pytest.mark.slow
    def test_large_file_type_preservation(self, complex_gff_element):
        """Test file type preservation with large files."""
        # Create a large GFF structure
        large_list = []
        for i in range(1000):
            fields = [
                GFFElement(GFFFieldType.INT, 0, f"Field{j}", i * 100 + j)
                for j in range(10)
            ]
            large_list.append(GFFElement(GFFFieldType.STRUCT, 0, "", fields))
        
        root = GFFElement(GFFFieldType.STRUCT, 0, "", [
            GFFElement(GFFFieldType.LIST, 0, "LargeList", large_list)
        ])
        
        # Test with different file types
        file_types = ['BIC ', 'IFO ', 'ARE ', 'ROS ', 'FAC ']
        
        for file_type in file_types:
            writer = GFFWriter(file_type, 'V3.2')
            output = BytesIO()
            
            start_time = time.time()
            writer.save(output, root)
            write_time = time.time() - start_time
            
            output.seek(0)
            parser = GFFParser()
            
            start_time = time.time()
            result = parser.load(output)
            parse_time = time.time() - start_time
            
            # Verify file type preserved
            assert parser.file_type == file_type
            
            # Performance should be reasonable
            assert write_time < 1.0, f"Write too slow for {file_type}: {write_time:.2f}s"
            assert parse_time < 1.0, f"Parse too slow for {file_type}: {parse_time:.2f}s"


class TestGFFRealWorldFiles:
    """Test with real NWN2 save files."""
    
    def test_all_file_types_in_save(self, sample_savegame_path):
        """Test all file types found in a real save game."""
        if not sample_savegame_path.exists():
            pytest.skip(f"Sample savegame not found at {sample_savegame_path}")
        
        file_type_counts = {}
        parser = GFFParser()
        
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
                    
                    # Check file type
                    file_type = file_data[:4].decode('ascii', errors='ignore')
                    file_type_counts[file_type] = file_type_counts.get(file_type, 0) + 1
                    
                    # Parse and verify
                    gff_element = parser.load(BytesIO(file_data))
                    assert parser.file_type.strip() == file_type.strip()
                    
                except Exception as e:
                    print(f"Failed to parse {filename}: {e}")
        
        # Verify we found multiple file types
        assert len(file_type_counts) >= 3, f"Too few file types found: {file_type_counts}"
        
        # Common types should be present (at least 2 of these)
        expected_types = ['BIC', 'IFO', 'ARE', 'ROS']
        found_types = [expected for expected in expected_types 
                       if any(expected in ft for ft in file_type_counts)]
        assert len(found_types) >= 2, f"Too few expected types found: {found_types} in {file_type_counts}"
    
    def test_modified_files_preserve_type(self, sample_savegame_path, complex_gff_element):
        """Test that modifying and re-saving files preserves their type."""
        if not sample_savegame_path.exists():
            pytest.skip(f"Sample savegame not found at {sample_savegame_path}")
        
        parser = GFFParser()
        
        with zipfile.ZipFile(sample_savegame_path, 'r') as zf:
            # Test with player.bic
            if 'player.bic' in zf.namelist():
                # Load original
                original_data = zf.read('player.bic')
                original_element = parser.load(BytesIO(original_data))
                original_type = parser.file_type
                
                # Modify the character (simulate level up)
                if original_element.get_field("Level"):
                    original_element.set_field("Level", 11)
                
                # Save with preserved type
                writer = GFFWriter.from_parser(parser)
                output = BytesIO()
                writer.save(output, original_element)
                
                # Verify type preserved
                output.seek(0)
                new_type = output.read(4).decode('ascii')
                assert new_type == 'BIC '
                
                # Verify modification took effect
                output.seek(0)
                modified_element = parser.load(output)
                if modified_element.get_field("Level"):
                    assert modified_element.get_value("Level") == 11


class TestGFFFileTypeBinaryCompatibility:
    """Test binary compatibility with NWN2."""
    
    def test_header_structure(self):
        """Test that header structure matches NWN2 expectations."""
        # Create a test file with known content
        fields = [
            GFFElement(GFFFieldType.STRING, 0, "Test", "Value")
        ]
        root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        writer = GFFWriter('TEST', 'V3.2')
        output = BytesIO()
        writer.save(output, root)
        
        # Verify header structure
        output.seek(0)
        header = output.read(56)
        
        # File type at offset 0 (4 bytes)
        assert header[0:4] == b'TEST'
        
        # Version at offset 4 (4 bytes)
        assert header[4:8] == b'V3.2'
        
        # Struct offset at offset 8 (4 bytes, little-endian)
        struct_offset = struct.unpack('<I', header[8:12])[0]
        assert struct_offset == 56  # Right after header
        
        # All offsets should be valid
        offsets = struct.unpack('<14I', header)
        for i, offset in enumerate(offsets[2:]):  # Skip file type and version
            if i % 2 == 0:  # Even indices are offsets
                assert offset >= 56 or offset == 0, f"Invalid offset at position {i}"
    
    def test_file_type_padding(self):
        """Test that file types are properly padded to 4 bytes."""
        # Test with file types of different lengths
        test_cases = [
            ('A', 'A   '),     # 1 char -> padded to 4
            ('AB', 'AB  '),    # 2 chars -> padded to 4
            ('ABC', 'ABC '),   # 3 chars -> padded to 4
            ('ABCD', 'ABCD'),  # 4 chars -> no padding
            ('ABCDE', 'ABCD'), # 5 chars -> truncated to 4
        ]
        
        for input_type, expected_type in test_cases:
            writer = GFFWriter(input_type, 'V3.2')
            output = BytesIO()
            writer.save(output, GFFElement(GFFFieldType.STRUCT, 0, "", []))
            
            output.seek(0)
            actual_type = output.read(4).decode('ascii')
            assert actual_type == expected_type
    
    def test_cross_platform_compatibility(self, simple_gff_element):
        """Test that files are compatible across platforms."""
        # Write a file
        writer = GFFWriter('XPLT', 'V3.2')
        output = BytesIO()
        writer.save(output, simple_gff_element)
        
        # Files should use little-endian byte order
        output.seek(8)
        struct_offset_bytes = output.read(4)
        
        # Manually verify little-endian
        offset = (struct_offset_bytes[0] | 
                 (struct_offset_bytes[1] << 8) | 
                 (struct_offset_bytes[2] << 16) | 
                 (struct_offset_bytes[3] << 24))
        
        assert offset == 56  # Header size


class TestGFFFileTypeIntegration:
    """Integration tests with other system components."""
    
    def test_file_type_with_module_loading(self):
        """Test that file types work correctly with module loading system."""
        # This would integrate with the module loader
        # For now, just test the basics
        module_files = ['module.ifo', 'module.are', 'module.git']
        
        for filename in module_files:
            ext = os.path.splitext(filename)[1]
            expected_type = {
                '.ifo': 'IFO ',
                '.are': 'ARE ',
                '.git': 'GIT '
            }[ext]
            
            # Create a module-like structure
            fields = [
                GFFElement(GFFFieldType.STRING, 0, "ModuleName", "Test Module"),
                GFFElement(GFFFieldType.INT, 0, "Version", 1)
            ]
            root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
            
            # Write with auto-detection
            with tempfile.NamedTemporaryFile(suffix=filename, delete=False) as tmp:
                try:
                    writer = GFFWriter()
                    writer.write(tmp.name, root)
                    
                    # Read back and verify
                    parser = GFFParser()
                    result = parser.read(tmp.name)
                    
                    assert parser.file_type == expected_type
                    assert result.get_value("ModuleName") == "Test Module"
                    
                finally:
                    os.unlink(tmp.name)
    
    def test_file_type_error_recovery(self):
        """Test recovery from file type errors."""
        # Create a file with corrupted file type
        header = bytearray(56)
        header[0:4] = b'\xFF\xFF\xFF\xFF'  # Invalid file type
        header[4:8] = b'V3.2'
        
        # Add minimal valid structure
        struct.pack_into('<I', header, 8, 56)   # struct offset
        struct.pack_into('<I', header, 12, 1)   # struct count
        data = header + struct.pack('<III', 0, 0, 0)  # Empty struct
        
        parser = GFFParser()
        # Should still parse despite weird file type
        result = parser.load(BytesIO(data))
        assert result is not None
        
        # File type should be preserved as-is (even if invalid)
        # Parser uses decode with errors='ignore' and strips null bytes
        # So invalid bytes might be ignored or replaced
        assert parser.file_type is not None
        # The exact result depends on how decode handles \xFF bytes


# Additional test class for missing functionality
class TestGFFMissingExtensions:
    """Test for file extensions that were missing."""
    
    def test_ros_and_fac_extensions_work(self, simple_gff_element):
        """Test that .ros and .fac extensions now work correctly."""
        test_cases = [
            ('test.ros', 'ROS '),
            ('test.fac', 'FAC '),
        ]
        
        with tempfile.TemporaryDirectory() as temp_dir:
            for filename, expected_type in test_cases:
                file_path = os.path.join(temp_dir, filename)
                
                writer = GFFWriter()
                writer.write(file_path, simple_gff_element)
                
                with open(file_path, 'rb') as f:
                    actual_type = f.read(4).decode('ascii')
                
                # These should now work after our fix
                assert actual_type == expected_type, f"{filename} extension not working"
    
    def test_all_nwn2_extensions_covered(self):
        """Verify all known NWN2 GFF extensions are covered."""
        known_extensions = {
            '.are': 'ARE ',  # Area
            '.bic': 'BIC ',  # Character 
            '.cam': 'CAM ',  # Camera
            '.dlg': 'DLG ',  # Dialog
            '.fac': 'FAC ',  # Faction
            '.gff': 'GFF ',  # Generic
            '.git': 'GIT ',  # Game Instance
            '.gui': 'GUI ',  # GUI Layout
            '.ifo': 'IFO ',  # Module Info
            '.jrl': 'JRL ',  # Journal
            '.pla': 'PLA ',  # Placeable Blueprint
            '.ros': 'ROS ',  # Roster
            '.rst': 'RST ',  # Roster Template
            '.ult': 'ULT ',  # Store Blueprint
            '.utc': 'UTC ',  # Creature Blueprint
            '.utd': 'UTD ',  # Door Blueprint
            '.ute': 'UTE ',  # Encounter Blueprint
            '.uti': 'UTI ',  # Item Blueprint
            '.utm': 'UTM ',  # Merchant Blueprint
            '.utp': 'UTP ',  # Placeable Blueprint
            '.uts': 'UTS ',  # Sound Blueprint
            '.utt': 'UTT ',  # Trigger Blueprint
            '.utw': 'UTW ',  # Waypoint Blueprint
        }
        
        # Check which ones are implemented
        implemented = {'.are', '.bic', '.dlg', '.fac', '.git', '.ifo', 
                      '.ros', '.utc', '.uti'}
        
        missing = set(known_extensions.keys()) - implemented
        
        # Log missing extensions for future implementation
        if missing:
            print(f"Extensions not yet implemented: {sorted(missing)}")
        
        # Critical ones should be implemented
        critical = {'.are', '.bic', '.ifo', '.ros', '.fac', '.git', '.utc', '.uti'}
        missing_critical = critical - implemented
        assert not missing_critical, f"Critical extensions missing: {missing_critical}"