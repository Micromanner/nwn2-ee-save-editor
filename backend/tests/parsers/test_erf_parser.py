"""
Comprehensive tests for ERF parser
Tests .mod, .hak, and .erf file parsing
"""

import pytest
import struct
import os
import tempfile
from pathlib import Path
from io import BytesIO
from unittest.mock import patch, MagicMock

from parsers import ERFParser
from nwn2_rust import ErfParser
ERFHeader = ErfParser  # For compatibility - adjust as needed based on actual API
ERFKey = None  # These may need adjustment based on actual nwn2_rust API
ERFResource = None
ERFResourceType = None
HakpakReader = None


class TestERFHeader:
    """Test ERF header parsing"""
    
    def test_valid_v10_header(self):
        """Test parsing a valid V1.0 ERF header"""
        # Create a valid V1.0 header
        header_data = bytearray(160)
        header_data[0:4] = b'MOD '
        header_data[4:8] = b'V1.0'
        # localized_string_count
        struct.pack_into('<I', header_data, 8, 0)
        # localized_string_size
        struct.pack_into('<I', header_data, 12, 0)
        # entry_count
        struct.pack_into('<I', header_data, 16, 10)
        # offset_to_localized_string
        struct.pack_into('<I', header_data, 20, 160)
        # offset_to_key_list
        struct.pack_into('<I', header_data, 24, 160)
        # offset_to_resource_list
        struct.pack_into('<I', header_data, 28, 400)
        # build_year
        struct.pack_into('<H', header_data, 32, 2024)
        # build_day
        struct.pack_into('<H', header_data, 34, 100)
        # description_strref
        struct.pack_into('<I', header_data, 36, 0xFFFFFFFF)
        
        with tempfile.NamedTemporaryFile(delete=False) as f:
            f.write(header_data)
            # Write dummy key and resource data
            f.write(b'\x00' * 1000)
            f.flush()
            
            parser = ERFParser()
            parser._file_path = f.name
            
            with open(f.name, 'rb') as rf:
                parser._parse_header(rf)
            
            assert parser.header.file_type == 'MOD '
            assert parser.header.version == 'V1.0'
            assert parser.header.entry_count == 10
            assert parser.header.build_year == 2024
            assert parser.header.build_day == 100
            
            os.unlink(f.name)
    
    def test_valid_v11_header(self):
        """Test parsing a valid V1.1 ERF header"""
        header_data = bytearray(160)
        header_data[0:4] = b'HAK '
        header_data[4:8] = b'V1.1'
        struct.pack_into('<I', header_data, 16, 5)  # entry_count
        struct.pack_into('<I', header_data, 24, 160)  # offset_to_key_list
        struct.pack_into('<I', header_data, 28, 360)  # offset_to_resource_list
        
        with tempfile.NamedTemporaryFile(delete=False) as f:
            f.write(header_data)
            f.write(b'\x00' * 1000)
            f.flush()
            
            parser = ERFParser()
            parser._file_path = f.name
            
            with open(f.name, 'rb') as rf:
                parser._parse_header(rf)
            
            assert parser.header.file_type == 'HAK '
            assert parser.header.version == 'V1.1'
            assert parser.header.entry_count == 5
            
            os.unlink(f.name)
    
    def test_erf_file_type(self):
        """Test ERF file type header"""
        header_data = bytearray(160)
        header_data[0:4] = b'ERF '
        header_data[4:8] = b'V1.0'
        struct.pack_into('<I', header_data, 16, 1)
        struct.pack_into('<I', header_data, 24, 160)
        struct.pack_into('<I', header_data, 28, 184)
        
        with tempfile.NamedTemporaryFile(delete=False) as f:
            f.write(header_data)
            f.write(b'\x00' * 200)
            f.flush()
            
            parser = ERFParser()
            parser._file_path = f.name
            
            with open(f.name, 'rb') as rf:
                parser._parse_header(rf)
            
            assert parser.header.file_type == 'ERF '
            
            os.unlink(f.name)
    
    def test_invalid_version(self):
        """Test error handling for invalid version"""
        header_data = bytearray(160)
        header_data[0:4] = b'MOD '
        header_data[4:8] = b'V2.0'  # Invalid version
        
        with tempfile.NamedTemporaryFile(delete=False) as f:
            f.write(header_data)
            f.flush()
            
            parser = ERFParser()
            parser._file_path = f.name
            
            with pytest.raises(ValueError, match="File version is not 1.0 or 1.1"):
                with open(f.name, 'rb') as rf:
                    parser._parse_header(rf)
            
            os.unlink(f.name)
    
    def test_empty_resource_entries(self):
        """Test error handling for empty resource entries"""
        header_data = bytearray(160)
        header_data[0:4] = b'MOD '
        header_data[4:8] = b'V1.0'
        struct.pack_into('<I', header_data, 16, 0)  # entry_count = 0
        
        with tempfile.NamedTemporaryFile(delete=False) as f:
            f.write(header_data)
            f.flush()
            
            parser = ERFParser()
            parser._file_path = f.name
            
            with pytest.raises(ValueError, match="No resource entries"):
                with open(f.name, 'rb') as rf:
                    parser._parse_header(rf)
            
            os.unlink(f.name)
    
    def test_short_header(self):
        """Test error handling for truncated header"""
        header_data = b'MOD V1.0' + b'\x00' * 50  # Only 58 bytes
        
        with tempfile.NamedTemporaryFile(delete=False) as f:
            f.write(header_data)
            f.flush()
            
            parser = ERFParser()
            parser._file_path = f.name
            
            with pytest.raises(ValueError, match="Header is too short"):
                with open(f.name, 'rb') as rf:
                    parser._parse_header(rf)
            
            os.unlink(f.name)


class TestERFKeys:
    """Test ERF key parsing"""
    
    def test_v10_key_parsing(self):
        """Test parsing V1.0 format keys (24 bytes each)"""
        parser = ERFParser()
        parser._file_path = "test.mod"  # Set file path to avoid None in error messages
        parser.header = ERFHeader(
            file_type='MOD ',
            version='V1.0',
            localized_string_count=0,
            localized_string_size=0,
            entry_count=2,
            offset_to_localized_string=0,
            offset_to_key_list=0,  # Start at 0 for BytesIO
            offset_to_resource_list=48,  # After 2 keys of 24 bytes each
            build_year=2024,
            build_day=100,
            description_strref=0xFFFFFFFF,
            reserved=b'\x00' * 120
        )
        
        # Create key data for V1.0 (16 byte name + 4 byte id + 2 byte type + 2 byte reserved)
        key_data = BytesIO()
        
        # Key 1: "test.2da"
        key_data.write(b'test\x00' + b'\x00' * 11)  # 16 byte name
        key_data.write(struct.pack('<I', 0))  # resource_id
        key_data.write(struct.pack('<H', 2017))  # res_type (2DA)
        key_data.write(struct.pack('<H', 0))  # reserved
        
        # Key 2: "module.ifo"
        key_data.write(b'module\x00' + b'\x00' * 9)  # 16 byte name
        key_data.write(struct.pack('<I', 1))  # resource_id
        key_data.write(struct.pack('<H', 2014))  # res_type (IFO)
        key_data.write(struct.pack('<H', 0))  # reserved
        
        key_data.seek(0)
        parser._parse_keys(key_data)
        
        assert len(parser.keys) == 2
        assert parser.keys[0].resref == 'test.2da'
        assert parser.keys[0].res_type == 2017
        assert parser.keys[0].resource_id == 0
        assert parser.keys[1].resref == 'module.ifo'
        assert parser.keys[1].res_type == 2014
        assert parser.keys[1].resource_id == 1
    
    def test_v11_key_parsing(self):
        """Test parsing V1.1 format keys (40 bytes each)"""
        parser = ERFParser()
        parser._file_path = "test.hak"
        parser.header = ERFHeader(
            file_type='HAK ',
            version='V1.1',
            localized_string_count=0,
            localized_string_size=0,
            entry_count=2,
            offset_to_localized_string=0,
            offset_to_key_list=0,  # Start at 0 for BytesIO
            offset_to_resource_list=80,  # After 2 keys of 40 bytes each
            build_year=2024,
            build_day=100,
            description_strref=0xFFFFFFFF,
            reserved=b'\x00' * 120
        )
        
        # Create key data for V1.1 (32 byte name + 4 byte id + 2 byte type + 2 byte reserved)
        key_data = BytesIO()
        
        # Key 1: "verylongresourcename.uti"
        key_data.write(b'verylongresourcename\x00' + b'\x00' * 11)  # 32 byte name
        key_data.write(struct.pack('<I', 0))  # resource_id (ignored in V1.1)
        key_data.write(struct.pack('<H', 2025))  # res_type (UTI)
        key_data.write(struct.pack('<H', 0))  # reserved
        
        # Key 2: "creature.utc"
        key_data.write(b'creature\x00' + b'\x00' * 23)  # 32 byte name
        key_data.write(struct.pack('<I', 1))  # resource_id (ignored in V1.1)
        key_data.write(struct.pack('<H', 2027))  # res_type (UTC)
        key_data.write(struct.pack('<H', 0))  # reserved
        
        key_data.seek(0)
        parser._parse_keys(key_data)
        
        assert len(parser.keys) == 2
        assert parser.keys[0].resref == 'verylongresourcename.uti'
        assert parser.keys[0].res_type == 2025
        assert parser.keys[0].resource_id == 0  # V1.1 uses index
        assert parser.keys[1].resref == 'creature.utc'
        assert parser.keys[1].res_type == 2027
        assert parser.keys[1].resource_id == 1  # V1.1 uses index
    
    def test_unknown_resource_type(self):
        """Test error handling for unknown resource type"""
        parser = ERFParser()
        parser._file_path = "test.mod"
        parser.header = ERFHeader(
            file_type='MOD ',
            version='V1.0',
            localized_string_count=0,
            localized_string_size=0,
            entry_count=1,
            offset_to_localized_string=0,
            offset_to_key_list=0,
            offset_to_resource_list=24,
            build_year=2024,
            build_day=100,
            description_strref=0xFFFFFFFF,
            reserved=b'\x00' * 120
        )
        
        key_data = BytesIO()
        key_data.write(b'test\x00' + b'\x00' * 11)  # 16 byte name
        key_data.write(struct.pack('<I', 0))  # resource_id
        key_data.write(struct.pack('<H', 12345))  # Invalid res_type
        key_data.write(struct.pack('<H', 0))  # reserved
        
        key_data.seek(0)
        with pytest.raises(ValueError, match="File type 12345 is not recognized"):
            parser._parse_keys(key_data)
    
    def test_special_ffff_type(self):
        """Test special 0xFFFF resource type handling"""
        parser = ERFParser()
        parser._file_path = "test.mod"
        parser.header = ERFHeader(
            file_type='MOD ',
            version='V1.0',
            localized_string_count=0,
            localized_string_size=0,
            entry_count=1,
            offset_to_localized_string=0,
            offset_to_key_list=0,
            offset_to_resource_list=24,
            build_year=2024,
            build_day=100,
            description_strref=0xFFFFFFFF,
            reserved=b'\x00' * 120
        )
        
        key_data = BytesIO()
        key_data.write(b'special\x00' + b'\x00' * 8)  # 16 byte name
        key_data.write(struct.pack('<I', 0))  # resource_id
        key_data.write(struct.pack('<H', 0xFFFF))  # Special type
        key_data.write(struct.pack('<H', 0))  # reserved
        
        key_data.seek(0)
        parser._parse_keys(key_data)
        
        assert len(parser.keys) == 1
        assert parser.keys[0].resref == 'special'  # No extension added
        assert parser.keys[0].res_type == 0xFFFF
    
    def test_premature_eof_in_keys(self):
        """Test error handling for premature end of file in keys"""
        parser = ERFParser()
        parser._file_path = "test.mod"
        parser.header = ERFHeader(
            file_type='MOD ',
            version='V1.0',
            localized_string_count=0,
            localized_string_size=0,
            entry_count=2,
            offset_to_localized_string=0,
            offset_to_key_list=0,
            offset_to_resource_list=48,
            build_year=2024,
            build_day=100,
            description_strref=0xFFFFFFFF,
            reserved=b'\x00' * 120
        )
        
        # Only provide one key worth of data when two are expected
        key_data = BytesIO()
        key_data.write(b'test\x00' + b'\x00' * 11)  # 16 byte name
        key_data.write(struct.pack('<I', 0))  # resource_id
        key_data.write(struct.pack('<H', 2017))  # res_type
        key_data.write(struct.pack('<H', 0))  # reserved
        # Missing second key
        
        key_data.seek(0)
        with pytest.raises(ValueError, match="Premature end-of-data while reading entry keys"):
            parser._parse_keys(key_data)


class TestERFResources:
    """Test ERF resource parsing"""
    
    def test_resource_parsing(self):
        """Test parsing resource entries"""
        parser = ERFParser()
        parser._file_path = "test.mod"
        parser.header = ERFHeader(
            file_type='MOD ',
            version='V1.0',
            localized_string_count=0,
            localized_string_size=0,
            entry_count=3,
            offset_to_localized_string=0,
            offset_to_key_list=0,
            offset_to_resource_list=0,  # Start at 0 for BytesIO
            build_year=2024,
            build_day=100,
            description_strref=0xFFFFFFFF,
            reserved=b'\x00' * 120
        )
        
        resource_data = BytesIO()
        # Resource 1
        resource_data.write(struct.pack('<II', 256, 1024))  # offset, size
        # Resource 2
        resource_data.write(struct.pack('<II', 1280, 2048))  # offset, size
        # Resource 3
        resource_data.write(struct.pack('<II', 3328, 512))  # offset, size
        
        resource_data.seek(0)
        parser._parse_resources(resource_data)
        
        assert len(parser.resources) == 3
        assert parser.resources[0].offset_to_resource == 256
        assert parser.resources[0].resource_size == 1024
        assert parser.resources[1].offset_to_resource == 1280
        assert parser.resources[1].resource_size == 2048
        assert parser.resources[2].offset_to_resource == 3328
        assert parser.resources[2].resource_size == 512


class TestERFParser:
    """Test full ERF parser functionality"""
    
    def create_test_erf(self, file_type='MOD ', version='V1.0', resources=None):
        """Helper to create a complete test ERF file"""
        if resources is None:
            resources = [
                ('test', 2017, b'2DA V2.0\n\nColumn1 Column2\n0 Value1 Value2\n'),
                ('module', 2014, b'GFF V3.2MOD \x00\x00\x00\x00' + b'\x00' * 100),
            ]
        
        # Calculate offsets
        header_size = 160
        key_size = 24 if version == 'V1.0' else 40
        resource_size = 8
        
        offset_to_keys = header_size
        offset_to_resources = offset_to_keys + (len(resources) * key_size)
        offset_to_data = offset_to_resources + (len(resources) * resource_size)
        
        # Create header
        header_data = bytearray(header_size)
        header_data[0:4] = file_type.encode('ascii')
        header_data[4:8] = version.encode('ascii')
        struct.pack_into('<I', header_data, 8, 0)  # localized_string_count
        struct.pack_into('<I', header_data, 12, 0)  # localized_string_size
        struct.pack_into('<I', header_data, 16, len(resources))  # entry_count
        struct.pack_into('<I', header_data, 20, header_size)  # offset_to_localized_string
        struct.pack_into('<I', header_data, 24, offset_to_keys)  # offset_to_key_list
        struct.pack_into('<I', header_data, 28, offset_to_resources)  # offset_to_resource_list
        struct.pack_into('<H', header_data, 32, 2024)  # build_year
        struct.pack_into('<H', header_data, 34, 100)  # build_day
        struct.pack_into('<I', header_data, 36, 0xFFFFFFFF)  # description_strref
        
        # Create keys
        key_data = bytearray()
        name_size = 16 if version == 'V1.0' else 32
        
        for i, (name, res_type, _) in enumerate(resources):
            # Name
            name_bytes = name.encode('ascii')[:name_size]
            key_entry = bytearray(key_size)
            key_entry[:len(name_bytes)] = name_bytes
            
            # Resource ID and type
            struct.pack_into('<I', key_entry, name_size, i)
            struct.pack_into('<H', key_entry, name_size + 4, res_type)
            struct.pack_into('<H', key_entry, name_size + 6, 0)
            
            key_data.extend(key_entry)
        
        # Create resource entries and data
        resource_entries = bytearray()
        resource_data = bytearray()
        current_offset = offset_to_data
        
        for _, _, data in resources:
            resource_entries.extend(struct.pack('<II', current_offset, len(data)))
            resource_data.extend(data)
            current_offset += len(data)
        
        # Combine all parts
        return bytes(header_data + key_data + resource_entries + resource_data)
    
    def test_read_complete_mod_file(self):
        """Test reading a complete MOD file"""
        erf_data = self.create_test_erf('MOD ', 'V1.0')
        
        with tempfile.NamedTemporaryFile(suffix='.mod', delete=False) as f:
            f.write(erf_data)
            f.flush()
            
            parser = ERFParser()
            result = parser.read(f.name)
            
            assert result is parser
            assert parser.header.file_type == 'MOD '
            assert parser.header.version == 'V1.0'
            assert parser.header.entry_count == 2
            assert len(parser.keys) == 2
            assert len(parser.resources) == 2
            
            os.unlink(f.name)
    
    def test_read_complete_hak_file(self):
        """Test reading a complete HAK file"""
        erf_data = self.create_test_erf('HAK ', 'V1.1')
        
        with tempfile.NamedTemporaryFile(suffix='.hak', delete=False) as f:
            f.write(erf_data)
            f.flush()
            
            parser = ERFParser()
            result = parser.read(f.name)
            
            assert parser.header.file_type == 'HAK '
            assert parser.header.version == 'V1.1'
            
            os.unlink(f.name)
    
    def test_read_complete_erf_file(self):
        """Test reading a complete ERF file"""
        erf_data = self.create_test_erf('ERF ', 'V1.0')
        
        with tempfile.NamedTemporaryFile(suffix='.erf', delete=False) as f:
            f.write(erf_data)
            f.flush()
            
            parser = ERFParser()
            result = parser.read(f.name)
            
            assert parser.header.file_type == 'ERF '
            
            os.unlink(f.name)
    
    def test_list_resources(self):
        """Test listing resources"""
        erf_data = self.create_test_erf('MOD ', 'V1.0', [
            ('test', 2017, b'2DA data'),
            ('module', 2014, b'IFO data'),
            ('item', 2025, b'UTI data'),
        ])
        
        with tempfile.NamedTemporaryFile(suffix='.mod', delete=False) as f:
            f.write(erf_data)
            f.flush()
            
            parser = ERFParser()
            parser.read(f.name)
            
            # List all resources
            all_resources = parser.list_resources()
            assert len(all_resources) == 3
            assert all_resources[0]['name'] == 'test.2da'
            assert all_resources[0]['type'] == 2017
            assert all_resources[0]['type_name'] == '2DA'
            assert all_resources[0]['size'] == 8
            assert all_resources[0]['index'] == 0
            
            # Filter by type
            tda_resources = parser.list_resources(resource_type=2017)
            assert len(tda_resources) == 1
            assert tda_resources[0]['name'] == 'test.2da'
            
            os.unlink(f.name)
    
    def test_extract_resource(self):
        """Test extracting a specific resource"""
        test_data = b'This is test 2DA data'
        erf_data = self.create_test_erf('MOD ', 'V1.0', [
            ('test', 2017, test_data),
            ('module', 2014, b'IFO data'),
        ])
        
        with tempfile.NamedTemporaryFile(suffix='.mod', delete=False) as f:
            f.write(erf_data)
            f.flush()
            
            parser = ERFParser()
            parser.read(f.name)
            
            # Extract to memory
            data = parser.extract_resource('test.2da')
            assert data == test_data
            
            # Extract to file
            with tempfile.NamedTemporaryFile(delete=False) as out:
                out_path = out.name
            
            parser.extract_resource('test.2da', out_path)
            with open(out_path, 'rb') as f2:
                assert f2.read() == test_data
            
            os.unlink(f.name)
            os.unlink(out_path)
    
    def test_extract_resource_not_found(self):
        """Test error when extracting non-existent resource"""
        erf_data = self.create_test_erf('MOD ', 'V1.0')
        
        with tempfile.NamedTemporaryFile(suffix='.mod', delete=False) as f:
            f.write(erf_data)
            f.flush()
            
            parser = ERFParser()
            parser.read(f.name)
            
            with pytest.raises(ValueError, match="Resource 'nonexistent.2da' not found"):
                parser.extract_resource('nonexistent.2da')
            
            os.unlink(f.name)
    
    def test_extract_resource_case_insensitive(self):
        """Test case-insensitive resource extraction"""
        test_data = b'Test data'
        erf_data = self.create_test_erf('MOD ', 'V1.0', [
            ('TestFile', 2017, test_data),
        ])
        
        with tempfile.NamedTemporaryFile(suffix='.mod', delete=False) as f:
            f.write(erf_data)
            f.flush()
            
            parser = ERFParser()
            parser.read(f.name)
            
            # Should find with different case
            data = parser.extract_resource('testfile.2da')
            assert data == test_data
            
            data = parser.extract_resource('TESTFILE.2DA')
            assert data == test_data
            
            os.unlink(f.name)
    
    def test_extract_all_2da(self):
        """Test extracting all 2DA files"""
        erf_data = self.create_test_erf('MOD ', 'V1.0', [
            ('test1', 2017, b'2DA data 1'),
            ('test2', 2017, b'2DA data 2'),
            ('module', 2014, b'IFO data'),
            ('test3', 2017, b'2DA data 3'),
        ])
        
        with tempfile.NamedTemporaryFile(suffix='.mod', delete=False) as f:
            f.write(erf_data)
            f.flush()
            
            parser = ERFParser()
            parser.read(f.name)
            
            with tempfile.TemporaryDirectory() as tmpdir:
                extracted = parser.extract_all_2da(tmpdir)
                
                assert len(extracted) == 3
                assert all(path.endswith('.2da') for path in extracted)
                
                # Check extracted files
                for i, path in enumerate(sorted(extracted)):
                    with open(path, 'rb') as ef:
                        content = ef.read()
                        assert content.startswith(b'2DA data')
            
            os.unlink(f.name)
    
    def test_get_module_info(self):
        """Test getting module info from MOD files"""
        module_data = b'GFF module data here'
        erf_data = self.create_test_erf('MOD ', 'V1.0', [
            ('module', 2014, module_data),
            ('test', 2017, b'2DA data'),
        ])
        
        with tempfile.NamedTemporaryFile(suffix='.mod', delete=False) as f:
            f.write(erf_data)
            f.flush()
            
            parser = ERFParser()
            parser.read(f.name)
            
            info = parser.get_module_info()
            assert info is not None
            assert info['has_module_ifo'] is True
            assert info['size'] == len(module_data)
            
            os.unlink(f.name)
    
    def test_get_module_info_non_module(self):
        """Test get_module_info returns None for non-MOD files"""
        erf_data = self.create_test_erf('HAK ', 'V1.1')
        
        with tempfile.NamedTemporaryFile(suffix='.hak', delete=False) as f:
            f.write(erf_data)
            f.flush()
            
            parser = ERFParser()
            parser.read(f.name)
            
            info = parser.get_module_info()
            assert info is None
            
            os.unlink(f.name)
    
    def test_file_not_found(self):
        """Test error handling for non-existent file"""
        parser = ERFParser()
        with pytest.raises(FileNotFoundError):
            parser.read('/nonexistent/file.mod')
    
    def test_corrupted_file(self):
        """Test handling of corrupted file"""
        with tempfile.NamedTemporaryFile(suffix='.mod', delete=False) as f:
            f.write(b'This is not an ERF file')
            f.flush()
            
            parser = ERFParser()
            with pytest.raises(ValueError):
                parser.read(f.name)
            
            os.unlink(f.name)


class TestHakpakReader:
    """Test HakpakReader convenience class"""
    
    def test_find_hakpak_in_current_dir(self):
        """Test finding hakpak in current directory"""
        reader = HakpakReader()
        
        # Create a test hak in current directory
        test_hak = Path('test.hak')
        test_hak.write_bytes(b'dummy')
        
        try:
            found = reader._find_hakpak('test')
            assert found == test_hak
            
            # Should also work with .hak extension
            found = reader._find_hakpak('test.hak')
            assert found == test_hak
        finally:
            test_hak.unlink()
    
    def test_find_hakpak_not_found(self):
        """Test hakpak not found"""
        reader = HakpakReader()
        found = reader._find_hakpak('nonexistent')
        assert found is None
    
    @patch('pathlib.Path.home')
    def test_find_hakpak_in_user_documents(self, mock_home):
        """Test finding hakpak in user documents"""
        mock_home.return_value = Path('/mock/home')
        reader = HakpakReader()
        
        with patch.object(Path, 'exists') as mock_exists:
            mock_exists.return_value = True
            
            found = reader._find_hakpak('test')
            expected = Path('/mock/home/Documents/Neverwinter Nights 2/hak/test.hak')
            assert found == expected
    
    def test_read_hakpak(self):
        """Test reading a hakpak file"""
        # Create a minimal valid HAK file
        erf_data = self.create_minimal_erf('HAK ', 'V1.1')
        
        with tempfile.NamedTemporaryFile(suffix='.hak', delete=False) as f:
            f.write(erf_data)
            f.flush()
            
            # Make it findable
            test_hak = Path('test.hak')
            test_hak.write_bytes(erf_data)
            
            try:
                reader = HakpakReader()
                parser = reader.read_hakpak('test')
                
                assert parser.header.file_type == 'HAK '
                assert parser.header.version == 'V1.1'
            finally:
                os.unlink(f.name)
                test_hak.unlink()
    
    def test_read_hakpak_not_found(self):
        """Test error when hakpak not found"""
        reader = HakpakReader()
        with pytest.raises(FileNotFoundError, match="Hakpak 'nonexistent' not found"):
            reader.read_hakpak('nonexistent')
    
    def test_extract_2da_files(self):
        """Test extracting 2DA files from hakpak"""
        # Create HAK with 2DA files
        erf_data = self.create_minimal_erf('HAK ', 'V1.1', [
            ('appearances', 2017, b'2DA V2.0\nappearances data'),
            ('baseitems', 2017, b'2DA V2.0\nbaseitems data'),
        ])
        
        test_hak = Path('test.hak')
        test_hak.write_bytes(erf_data)
        
        try:
            reader = HakpakReader()
            with tempfile.TemporaryDirectory() as tmpdir:
                extracted = reader.extract_2da_files('test', tmpdir)
                
                assert len(extracted) == 2
                assert all(path.endswith('.2da') for path in extracted)
                
                # Verify content
                for path in extracted:
                    with open(path, 'rb') as f:
                        content = f.read()
                        assert content.startswith(b'2DA V2.0')
        finally:
            test_hak.unlink()
    
    def create_minimal_erf(self, file_type='HAK ', version='V1.1', resources=None):
        """Helper to create minimal ERF for HakpakReader tests"""
        if resources is None:
            resources = [('test', 2017, b'test data')]
        
        # Same as TestERFParser.create_test_erf but separated for clarity
        header_size = 160
        key_size = 24 if version == 'V1.0' else 40
        resource_size = 8
        
        offset_to_keys = header_size
        offset_to_resources = offset_to_keys + (len(resources) * key_size)
        offset_to_data = offset_to_resources + (len(resources) * resource_size)
        
        header_data = bytearray(header_size)
        header_data[0:4] = file_type.encode('ascii')
        header_data[4:8] = version.encode('ascii')
        struct.pack_into('<I', header_data, 16, len(resources))
        struct.pack_into('<I', header_data, 24, offset_to_keys)
        struct.pack_into('<I', header_data, 28, offset_to_resources)
        struct.pack_into('<H', header_data, 32, 2024)
        struct.pack_into('<H', header_data, 34, 100)
        struct.pack_into('<I', header_data, 36, 0xFFFFFFFF)
        
        key_data = bytearray()
        name_size = 16 if version == 'V1.0' else 32
        
        for i, (name, res_type, _) in enumerate(resources):
            key_entry = bytearray(key_size)
            name_bytes = name.encode('ascii')[:name_size]
            key_entry[:len(name_bytes)] = name_bytes
            struct.pack_into('<I', key_entry, name_size, i)
            struct.pack_into('<H', key_entry, name_size + 4, res_type)
            key_data.extend(key_entry)
        
        resource_entries = bytearray()
        resource_data = bytearray()
        current_offset = offset_to_data
        
        for _, _, data in resources:
            resource_entries.extend(struct.pack('<II', current_offset, len(data)))
            resource_data.extend(data)
            current_offset += len(data)
        
        return bytes(header_data + key_data + resource_entries + resource_data)


class TestERFUtilityMethods:
    """Test utility methods in ERF parser"""
    
    def test_get_short(self):
        """Test little-endian short reading"""
        parser = ERFParser()
        
        # Test basic short
        buffer = b'\x34\x12'  # 0x1234 in little-endian
        assert parser._get_short(buffer, 0) == 0x1234
        
        # Test with offset
        buffer = b'\x00\x00\x34\x12'
        assert parser._get_short(buffer, 2) == 0x1234
        
        # Test edge values
        buffer = b'\xFF\xFF'  # Max unsigned short
        assert parser._get_short(buffer, 0) == 0xFFFF
        
        buffer = b'\x00\x00'  # Zero
        assert parser._get_short(buffer, 0) == 0
    
    def test_get_integer(self):
        """Test little-endian integer reading"""
        parser = ERFParser()
        
        # Test basic integer
        buffer = b'\x78\x56\x34\x12'  # 0x12345678 in little-endian
        assert parser._get_integer(buffer, 0) == 0x12345678
        
        # Test with offset
        buffer = b'\x00\x00\x78\x56\x34\x12'
        assert parser._get_integer(buffer, 2) == 0x12345678
        
        # Test edge values
        buffer = b'\xFF\xFF\xFF\xFF'  # Max unsigned int
        assert parser._get_integer(buffer, 0) == 0xFFFFFFFF
        
        buffer = b'\x00\x00\x00\x00'  # Zero
        assert parser._get_integer(buffer, 0) == 0
    
    def test_get_type_name(self):
        """Test resource type name lookup"""
        parser = ERFParser()
        
        # Test known types
        assert parser._get_type_name(2017) == '2DA'
        assert parser._get_type_name(2014) == 'IFO'
        assert parser._get_type_name(2025) == 'UTI'
        assert parser._get_type_name(2027) == 'UTC'
        assert parser._get_type_name(2037) == 'GFF'
        
        # Test unknown type
        assert parser._get_type_name(99999) == 'Type_99999'


class TestERFEdgeCases:
    """Test edge cases and error conditions"""
    
    def test_empty_file(self):
        """Test handling of empty file"""
        with tempfile.NamedTemporaryFile(suffix='.mod', delete=False) as f:
            f.flush()  # Empty file
            
            parser = ERFParser()
            with pytest.raises(ValueError):
                parser.read(f.name)
            
            os.unlink(f.name)
    
    def test_large_resource_count(self):
        """Test handling large number of resources"""
        # Create ERF with many resources
        resources = [(f'resource{i}', 2017, f'Data {i}'.encode()) for i in range(100)]
        
        header_size = 160
        key_size = 24
        resource_size = 8
        
        offset_to_keys = header_size
        offset_to_resources = offset_to_keys + (len(resources) * key_size)
        offset_to_data = offset_to_resources + (len(resources) * resource_size)
        
        header_data = bytearray(header_size)
        header_data[0:4] = b'MOD '
        header_data[4:8] = b'V1.0'
        struct.pack_into('<I', header_data, 16, len(resources))
        struct.pack_into('<I', header_data, 24, offset_to_keys)
        struct.pack_into('<I', header_data, 28, offset_to_resources)
        struct.pack_into('<H', header_data, 32, 2024)
        struct.pack_into('<H', header_data, 34, 100)
        
        # Create keys and resources
        key_data = bytearray()
        resource_entries = bytearray()
        resource_data = bytearray()
        current_offset = offset_to_data
        
        for i, (name, res_type, data) in enumerate(resources):
            # Key
            key_entry = bytearray(key_size)
            name_bytes = name.encode('ascii')[:16]
            key_entry[:len(name_bytes)] = name_bytes
            struct.pack_into('<I', key_entry, 16, i)
            struct.pack_into('<H', key_entry, 20, res_type)
            key_data.extend(key_entry)
            
            # Resource
            resource_entries.extend(struct.pack('<II', current_offset, len(data)))
            resource_data.extend(data)
            current_offset += len(data)
        
        erf_data = bytes(header_data + key_data + resource_entries + resource_data)
        
        with tempfile.NamedTemporaryFile(suffix='.mod', delete=False) as f:
            f.write(erf_data)
            f.flush()
            
            parser = ERFParser()
            parser.read(f.name)
            
            assert parser.header.entry_count == 100
            assert len(parser.keys) == 100
            assert len(parser.resources) == 100
            
            # Test we can extract a resource
            data = parser.extract_resource('resource50.2da')
            assert data == b'Data 50'
            
            os.unlink(f.name)
    
    def test_zero_size_resource(self):
        """Test handling of zero-size resources"""
        # Create ERF with zero-size resource
        header_size = 160
        key_size = 24
        resource_size = 8
        
        offset_to_keys = header_size
        offset_to_resources = offset_to_keys + key_size
        offset_to_data = offset_to_resources + resource_size
        
        header_data = bytearray(header_size)
        header_data[0:4] = b'MOD '
        header_data[4:8] = b'V1.0'
        struct.pack_into('<I', header_data, 16, 1)  # 1 resource
        struct.pack_into('<I', header_data, 24, offset_to_keys)
        struct.pack_into('<I', header_data, 28, offset_to_resources)
        
        # Key for empty resource
        key_data = bytearray(key_size)
        key_data[:5] = b'empty'
        struct.pack_into('<I', key_data, 16, 0)
        struct.pack_into('<H', key_data, 20, 2017)
        
        # Resource with 0 size
        resource_entry = struct.pack('<II', offset_to_data, 0)
        
        erf_data = bytes(header_data + key_data + resource_entry)
        
        with tempfile.NamedTemporaryFile(suffix='.mod', delete=False) as f:
            f.write(erf_data)
            f.flush()
            
            parser = ERFParser()
            parser.read(f.name)
            
            # Should successfully read
            assert len(parser.keys) == 1
            assert parser.resources[0].resource_size == 0
            
            # Should extract empty data
            data = parser.extract_resource('empty.2da')
            assert data == b''
            
            os.unlink(f.name)
    
    def test_unicode_handling(self):
        """Test that parser handles only ASCII names properly"""
        parser = ERFParser()
        parser._file_path = "test.erf"  # Set file path to avoid None error
        parser.header = ERFHeader(
            file_type='MOD ',
            version='V1.0',
            localized_string_count=0,
            localized_string_size=0,
            entry_count=1,
            offset_to_localized_string=160,
            offset_to_key_list=0,  # Set to 0 for BytesIO
            offset_to_resource_list=24,
            build_year=2024,
            build_day=100,
            description_strref=0xFFFFFFFF,
            reserved=b'\x00' * 120
        )
        
        # Create key with high-bit characters (invalid ASCII)
        key_data = BytesIO()
        key_data.write(b'\xFF\xFE\xFD\xFC' + b'\x00' * 12)  # Invalid ASCII
        key_data.write(struct.pack('<I', 0))
        key_data.write(struct.pack('<H', 2017))
        key_data.write(struct.pack('<H', 0))
        
        key_data.seek(0)
        # Should raise on invalid ASCII (now wrapped in ValueError)
        with pytest.raises(ValueError, match="Invalid ASCII in resource name"):
            parser._parse_keys(key_data)