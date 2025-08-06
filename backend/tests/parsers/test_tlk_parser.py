# LEGACY REPLCAED BY RUST TDA PARSER


# import pytest
# import os
# import struct
# import tempfile
# from pathlib import Path
# from unittest.mock import Mock, patch, mock_open

# from parsers.tlk import TLKParser


# class TestTLKParser:
#     """Test suite for TLK file parser"""
    
#     @pytest.fixture
#     def parser(self):
#         """Create a TLKParser instance"""
#         return TLKParser()
        
#     @pytest.fixture
#     def test_tlk_path(self):
#         """Path to test TLK file"""
#         return Path(__file__).parent.parent / 'fixtures' / 'tlk' / 'test_dialog.tlk'
        
#     @pytest.fixture
#     def fixtures_dir(self):
#         """Path to TLK fixtures directory"""
#         return Path(__file__).parent.parent / 'fixtures' / 'tlk'
        
#     @pytest.fixture
#     def valid_tlk_data(self):
#         """Create valid TLK file data in memory"""
#         # TLK Header (20 bytes)
#         header = b'TLK '  # File type (4 bytes)
#         header += b'V3.0'  # Version (4 bytes)
#         header += struct.pack('<I', 0)  # Language ID - English (4 bytes)
#         header += struct.pack('<I', 3)  # String count (4 bytes)
#         header += struct.pack('<I', 140)  # String data offset: 20 (header) + 3*40 (entries) = 140
        
#         # String table entries (40 bytes each)
#         entries = b''
        
#         # Entry 0: "Hello, world!"
#         entries += struct.pack('<I', 0x01)  # Flags (present)
#         entries += b'\x00' * 16  # Sound ResRef (empty)
#         entries += struct.pack('<I', 0)  # Volume variance
#         entries += struct.pack('<I', 0)  # Pitch variance
#         entries += struct.pack('<I', 0)  # Data offset
#         entries += struct.pack('<I', 13)  # String size
#         entries += b'\x00' * 4  # Reserved
        
#         # Entry 1: Empty string (present but length 0)
#         entries += struct.pack('<I', 0x01)  # Flags (present)
#         entries += b'\x00' * 16  # Sound ResRef
#         entries += struct.pack('<I', 0)  # Volume variance
#         entries += struct.pack('<I', 0)  # Pitch variance
#         entries += struct.pack('<I', 13)  # Data offset
#         entries += struct.pack('<I', 0)  # String size
#         entries += b'\x00' * 4  # Reserved
        
#         # Entry 2: Not present
#         entries += struct.pack('<I', 0x00)  # Flags (not present)
#         entries += b'\x00' * 16  # Sound ResRef
#         entries += struct.pack('<I', 0)  # Volume variance
#         entries += struct.pack('<I', 0)  # Pitch variance
#         entries += struct.pack('<I', 13)  # Data offset
#         entries += struct.pack('<I', 0)  # String size
#         entries += b'\x00' * 4  # Reserved
        
#         # String data
#         string_data = b'Hello, world!'
        
#         return header + entries + string_data
        
#     @pytest.fixture
#     def invalid_header_data(self):
#         """Create TLK data with invalid header"""
#         return b'INVALID_HEADER' + b'\x00' * 100
        
#     @pytest.fixture
#     def wrong_version_data(self):
#         """Create TLK data with wrong version"""
#         header = b'TLK '  # File type
#         header += b'V2.0'  # Wrong version
#         header += b'\x00' * 12  # Rest of header
#         return header
        
#     # Test TLK file loading
    
#     def test_read_valid_tlk_file(self, parser, test_tlk_path):
#         """Test loading a valid TLK file"""
#         if test_tlk_path.exists():
#             parser.read(str(test_tlk_path))
#             assert parser.file_path == str(test_tlk_path)
#             assert parser.header['file_type'] == 'TLK '
#             assert parser.header['version'] == 'V3.0'
#             assert parser.header['string_count'] > 0
            
#     def test_read_nonexistent_file(self, parser):
#         """Test loading a non-existent file"""
#         with pytest.raises(FileNotFoundError):
#             parser.read('/path/to/nonexistent/file.tlk')
            
#     def test_read_empty_file(self, parser):
#         """Test loading an empty file"""
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             temp_path = f.name
            
#         try:
#             with pytest.raises(ValueError, match="header too short"):
#                 parser.read(temp_path)
#         finally:
#             os.unlink(temp_path)
            
#     # Test header parsing
    
#     def test_parse_valid_header(self, parser, valid_tlk_data):
#         """Test parsing a valid TLK header"""
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(valid_tlk_data)
#             temp_path = f.name
            
#         try:
#             parser.read(temp_path)
#             assert parser.header['file_type'] == 'TLK '
#             assert parser.header['version'] == 'V3.0'
#             assert parser.header['language_id'] == 0
#             assert parser.header['string_count'] == 3
#             assert parser.header['string_data_offset'] == 140
#         finally:
#             os.unlink(temp_path)
            
#     def test_invalid_file_type(self, parser, invalid_header_data):
#         """Test parsing file with invalid file type"""
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(invalid_header_data)
#             temp_path = f.name
            
#         try:
#             with pytest.raises(ValueError, match="Invalid TLK file"):
#                 parser.read(temp_path)
#         finally:
#             os.unlink(temp_path)
            
#     def test_wrong_version(self, parser, wrong_version_data):
#         """Test parsing file with wrong version"""
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(wrong_version_data)
#             temp_path = f.name
            
#         try:
#             with pytest.raises(ValueError, match="Invalid TLK file"):
#                 parser.read(temp_path)
#         finally:
#             os.unlink(temp_path)
            
#     # Test string lookup
    
#     def test_get_string_valid_id(self, parser, valid_tlk_data):
#         """Test retrieving string by valid ID"""
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(valid_tlk_data)
#             temp_path = f.name
            
#         try:
#             parser.read(temp_path)
#             assert parser.get_string(0) == "Hello, world!"
#             assert parser.get_string(1) == ""  # Present but empty
#             assert parser.get_string(2) == ""  # Not present
#         finally:
#             os.unlink(temp_path)
            
#     def test_get_string_negative_id(self, parser, valid_tlk_data):
#         """Test retrieving string with negative ID"""
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(valid_tlk_data)
#             temp_path = f.name
            
#         try:
#             parser.read(temp_path)
#             assert parser.get_string(-1) is None
#         finally:
#             os.unlink(temp_path)
            
#     def test_get_string_out_of_range(self, parser, valid_tlk_data):
#         """Test retrieving string with out of range ID"""
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(valid_tlk_data)
#             temp_path = f.name
            
#         try:
#             parser.read(temp_path)
#             assert parser.get_string(1000) is None
#         finally:
#             os.unlink(temp_path)
            
#     def test_get_string_no_file_loaded(self, parser):
#         """Test retrieving string when no file is loaded"""
#         assert parser.get_string(0) is None
        
#     def test_get_all_strings(self, parser, valid_tlk_data):
#         """Test batch string retrieval"""
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(valid_tlk_data)
#             temp_path = f.name
            
#         try:
#             parser.read(temp_path)
#             strings = parser.get_all_strings(start=0, count=5)
#             assert len(strings) == 3  # Only 3 strings in file
#             assert strings[0] == "Hello, world!"
#             assert strings[1] == ""
#             assert strings[2] == ""
#         finally:
#             os.unlink(temp_path)
            
#     # Test language support
    
#     def test_language_id_english(self, parser, valid_tlk_data):
#         """Test English language ID (0)"""
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(valid_tlk_data)
#             temp_path = f.name
            
#         try:
#             parser.read(temp_path)
#             assert parser.header['language_id'] == 0  # English
#         finally:
#             os.unlink(temp_path)
            
#     def test_language_id_other(self, parser):
#         """Test non-English language IDs"""
#         # Create TLK with French language ID (2)
#         header = b'TLK '
#         header += b'V3.0'
#         header += struct.pack('<I', 2)  # French
#         header += struct.pack('<I', 0)  # No strings
#         header += struct.pack('<I', 20)  # String data offset
        
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(header)
#             temp_path = f.name
            
#         try:
#             parser.read(temp_path)
#             assert parser.header['language_id'] == 2  # French
#         finally:
#             os.unlink(temp_path)
            
#     # Test malformed TLK handling
    
#     def test_truncated_header(self, parser):
#         """Test handling of truncated header"""
#         truncated_data = b'TLK V3.0'  # Only 8 bytes instead of 20
        
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(truncated_data)
#             temp_path = f.name
            
#         try:
#             with pytest.raises(ValueError, match="header too short"):
#                 parser.read(temp_path)
#         finally:
#             os.unlink(temp_path)
            
#     def test_truncated_string_table(self, parser):
#         """Test handling of truncated string table"""
#         # Header says 5 strings but only provide data for 1
#         header = b'TLK '
#         header += b'V3.0'
#         header += struct.pack('<I', 0)  # Language ID
#         header += struct.pack('<I', 5)  # Claims 5 strings
#         header += struct.pack('<I', 60)  # String data offset
        
#         # Only one entry instead of 5
#         entry = struct.pack('<I', 0x01) + b'\x00' * 36
        
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(header + entry)
#             temp_path = f.name
            
#         try:
#             parser.read(temp_path)
#             # Should handle gracefully, only load what's available
#             assert len(parser.string_entries) == 5
#             # But accessing missing entries should return None
#             assert parser.get_string(4) is None
#         finally:
#             os.unlink(temp_path)
            
#     def test_corrupted_string_data(self, parser):
#         """Test handling of corrupted string data"""
#         # Create TLK with string offset beyond file size
#         header = b'TLK '
#         header += b'V3.0'
#         header += struct.pack('<I', 0)
#         header += struct.pack('<I', 1)
#         header += struct.pack('<I', 60)
        
#         entry = struct.pack('<I', 0x01)  # Present
#         entry += b'\x00' * 16  # Sound ResRef
#         entry += struct.pack('<I', 0)  # Volume
#         entry += struct.pack('<I', 0)  # Pitch
#         entry += struct.pack('<I', 1000)  # Offset way beyond file
#         entry += struct.pack('<I', 10)  # Size
#         entry += b'\x00' * 4
        
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(header + entry)
#             temp_path = f.name
            
#         try:
#             parser.read(temp_path)
#             # Should return None for corrupted data
#             assert parser.get_string(0) is None
#         finally:
#             os.unlink(temp_path)
            
#     # Test search functionality
    
#     def test_search_strings_case_sensitive(self, parser, valid_tlk_data):
#         """Test case-sensitive string search"""
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(valid_tlk_data)
#             temp_path = f.name
            
#         try:
#             parser.read(temp_path)
#             matches = parser.search_strings("Hello", case_sensitive=True)
#             assert matches == [0]
            
#             matches = parser.search_strings("hello", case_sensitive=True)
#             assert matches == []
#         finally:
#             os.unlink(temp_path)
            
#     def test_search_strings_case_insensitive(self, parser, valid_tlk_data):
#         """Test case-insensitive string search"""
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(valid_tlk_data)
#             temp_path = f.name
            
#         try:
#             parser.read(temp_path)
#             matches = parser.search_strings("hello", case_sensitive=False)
#             assert matches == [0]
            
#             matches = parser.search_strings("WORLD", case_sensitive=False)
#             assert matches == [0]
#         finally:
#             os.unlink(temp_path)
            
#     def test_search_no_matches(self, parser, valid_tlk_data):
#         """Test search with no matches"""
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(valid_tlk_data)
#             temp_path = f.name
            
#         try:
#             parser.read(temp_path)
#             matches = parser.search_strings("nonexistent")
#             assert matches == []
#         finally:
#             os.unlink(temp_path)
            
#     # Test caching behavior
    
#     def test_string_caching(self, parser, valid_tlk_data):
#         """Test that strings are cached after first access"""
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(valid_tlk_data)
#             temp_path = f.name
            
#         try:
#             parser.read(temp_path)
            
#             # First access - not in cache
#             assert 0 not in parser._string_data_cache
#             result1 = parser.get_string(0)
            
#             # Should now be in cache
#             assert 0 in parser._string_data_cache
#             assert parser._string_data_cache[0] == result1
            
#             # Second access should return same result
#             result2 = parser.get_string(0)
#             assert result1 == result2
#         finally:
#             os.unlink(temp_path)
            
#     def test_cache_size_limit(self, parser):
#         """Test cache size limiting"""
#         # Create TLK with many strings
#         string_count = 1500  # More than cache size (1000)
        
#         header = b'TLK '
#         header += b'V3.0'
#         header += struct.pack('<I', 0)
#         header += struct.pack('<I', string_count)
#         header += struct.pack('<I', 20 + string_count * 40)
        
#         entries = b''
#         for i in range(string_count):
#             entries += struct.pack('<I', 0x01)  # Present
#             entries += b'\x00' * 16
#             entries += struct.pack('<I', 0)
#             entries += struct.pack('<I', 0)
#             entries += struct.pack('<I', i * 10)
#             entries += struct.pack('<I', 9)  # "String XX"
#             entries += b'\x00' * 4
            
#         string_data = b''
#         for i in range(string_count):
#             string_data += f"String {i:02d}".encode('utf-8')
            
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(header + entries + string_data)
#             temp_path = f.name
            
#         try:
#             parser.read(temp_path)
            
#             # Access more strings than cache size
#             for i in range(1100):
#                 parser.get_string(i)
                
#             # Cache should not exceed limit
#             assert len(parser._string_data_cache) <= parser._cache_size
#         finally:
#             os.unlink(temp_path)
            
#     # Test file info
    
#     def test_get_info(self, parser, valid_tlk_data):
#         """Test getting TLK file information"""
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(valid_tlk_data)
#             temp_path = f.name
            
#         try:
#             parser.read(temp_path)
#             info = parser.get_info()
            
#             assert info['file_path'] == temp_path
#             assert info['language_id'] == 0
#             assert info['string_count'] == 3
#             assert info['cache_size'] >= 0
#             assert info['file_size'] == len(valid_tlk_data)
#         finally:
#             os.unlink(temp_path)
            
#     def test_get_info_no_file(self, parser):
#         """Test getting info when no file is loaded"""
#         info = parser.get_info()
#         assert info['file_path'] is None
#         assert info['file_size'] == 0
        
#     # Test special characters and encoding
    
#     def test_unicode_strings(self, parser):
#         """Test handling of Unicode strings"""
#         # Create TLK with Unicode content
#         header = b'TLK '
#         header += b'V3.0'
#         header += struct.pack('<I', 0)
#         header += struct.pack('<I', 1)
#         header += struct.pack('<I', 60)
        
#         entry = struct.pack('<I', 0x01)
#         entry += b'\x00' * 16
#         entry += struct.pack('<I', 0)
#         entry += struct.pack('<I', 0)
#         entry += struct.pack('<I', 0)
        
#         unicode_string = "Hello 世界! 🌍"
#         string_bytes = unicode_string.encode('utf-8')
#         entry += struct.pack('<I', len(string_bytes))
#         entry += b'\x00' * 4
        
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(header + entry + string_bytes)
#             temp_path = f.name
            
#         try:
#             parser.read(temp_path)
#             result = parser.get_string(0)
#             assert result == unicode_string
#         finally:
#             os.unlink(temp_path)
            
#     def test_sound_resref_handling(self, parser):
#         """Test handling of sound resource references"""
#         header = b'TLK '
#         header += b'V3.0'
#         header += struct.pack('<I', 0)
#         header += struct.pack('<I', 1)
#         header += struct.pack('<I', 60)
        
#         entry = struct.pack('<I', 0x01)
#         entry += b'vo_hello\x00' + b'\x00' * 8  # Sound ResRef
#         entry += struct.pack('<I', 5)  # Volume variance
#         entry += struct.pack('<I', 3)  # Pitch variance
#         entry += struct.pack('<I', 0)
#         entry += struct.pack('<I', 5)
#         entry += b'\x00' * 4
        
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(header + entry + b'Hello')
#             temp_path = f.name
            
#         try:
#             parser.read(temp_path)
#             assert parser.string_entries[0]['sound_resref'] == 'vo_hello'
#         finally:
#             os.unlink(temp_path)
            
#     # Test error recovery
    
#     def test_file_read_error_recovery(self, parser, valid_tlk_data):
#         """Test recovery from file read errors"""
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(valid_tlk_data)
#             temp_path = f.name
            
#         try:
#             parser.read(temp_path)
            
#             # Simulate file being deleted/inaccessible
#             os.unlink(temp_path)
            
#             # Should return None instead of crashing
#             assert parser.get_string(0) is None
#         except:
#             # Clean up if test fails
#             if os.path.exists(temp_path):
#                 os.unlink(temp_path)
                
#     def test_multiple_file_loads(self, parser):
#         """Test loading multiple files in sequence"""
#         # First file
#         tlk1_data = self._create_simple_tlk(["First", "Second"])
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(tlk1_data)
#             temp_path1 = f.name
            
#         # Second file  
#         tlk2_data = self._create_simple_tlk(["Third", "Fourth", "Fifth"])
#         with tempfile.NamedTemporaryFile(suffix='.tlk', delete=False) as f:
#             f.write(tlk2_data)
#             temp_path2 = f.name
            
#         try:
#             # Load first file
#             parser.read(temp_path1)
#             assert parser.get_string(0) == "First"
#             assert parser.get_string(1) == "Second"
#             assert len(parser.string_entries) == 2
            
#             # Load second file - should replace first
#             parser.read(temp_path2)
#             assert parser.get_string(0) == "Third"
#             assert parser.get_string(1) == "Fourth"
#             assert parser.get_string(2) == "Fifth"
#             assert len(parser.string_entries) == 3
            
#             # Cache should be cleared
#             assert 0 not in parser._string_data_cache or parser._string_data_cache[0] == "Third"
#         finally:
#             os.unlink(temp_path1)
#             os.unlink(temp_path2)
            
#     def _create_simple_tlk(self, strings):
#         """Helper to create a simple TLK file with given strings"""
#         string_count = len(strings)
        
#         header = b'TLK '
#         header += b'V3.0'
#         header += struct.pack('<I', 0)  # Language
#         header += struct.pack('<I', string_count)
#         header += struct.pack('<I', 20 + string_count * 40)  # String data offset
        
#         entries = b''
#         offset = 0
#         for s in strings:
#             entries += struct.pack('<I', 0x01)  # Present
#             entries += b'\x00' * 16  # No sound
#             entries += struct.pack('<I', 0)  # Volume
#             entries += struct.pack('<I', 0)  # Pitch
#             entries += struct.pack('<I', offset)
#             entries += struct.pack('<I', len(s.encode('utf-8')))
#             entries += b'\x00' * 4
#             offset += len(s.encode('utf-8'))
            
#         string_data = b''.join(s.encode('utf-8') for s in strings)
        
#         return header + entries + string_data
        
#     # Additional tests using generated fixtures
    
#     def test_real_dialog_files(self, parser, fixtures_dir):
#         """Test loading various real dialog files"""
#         test_files = [
#             ('dialog_english.tlk', 10, 0),  # 10 strings, English
#             ('dialog_french.tlk', 4, 1),     # 4 strings, French
#             ('dialog_single.tlk', 1, 0),     # 1 string
#             ('dialog_empty.tlk', 0, 0),      # No strings
#         ]
        
#         for filename, expected_count, expected_lang in test_files:
#             file_path = fixtures_dir / filename
#             if file_path.exists():
#                 parser.read(str(file_path))
#                 assert len(parser.string_entries) == expected_count
#                 assert parser.header['language_id'] == expected_lang
                
#     def test_dialog_with_sound(self, parser, fixtures_dir):
#         """Test dialog file with sound references"""
#         file_path = fixtures_dir / 'dialog_with_sound.tlk'
#         if file_path.exists():
#             parser.read(str(file_path))
            
#             # Check that sound references are loaded
#             for i in range(min(3, len(parser.string_entries))):
#                 entry = parser.string_entries[i]
#                 if entry['present']:
#                     assert entry['sound_resref'] == f'vo_{i:04d}'
                    
#     def test_large_file_performance(self, parser, fixtures_dir):
#         """Test performance with large TLK file"""
#         import time
        
#         file_path = fixtures_dir / 'dialog_large.tlk'
#         if file_path.exists():
#             start_time = time.time()
#             parser.read(str(file_path))
#             load_time = time.time() - start_time
            
#             # Should load reasonably fast (under 1 second)
#             assert load_time < 1.0
#             assert len(parser.string_entries) == 10000
            
#             # Test random access performance
#             start_time = time.time()
#             for i in [0, 1000, 5000, 9999]:
#                 parser.get_string(i)
#             access_time = time.time() - start_time
            
#             # Random access should be fast
#             assert access_time < 0.1
            
#     def test_all_missing_strings(self, parser, fixtures_dir):
#         """Test file where all strings are marked as not present"""
#         file_path = fixtures_dir / 'dialog_all_missing.tlk'
#         if file_path.exists():
#             parser.read(str(file_path))
            
#             # All strings should return empty string
#             for i in range(len(parser.string_entries)):
#                 assert parser.get_string(i) == ""
                
#     def test_malformed_files_from_fixtures(self, parser, fixtures_dir):
#         """Test handling of various malformed files"""
#         malformed_files = [
#             'malformed_truncated.tlk',
#             'malformed_version.tlk',
#             'malformed_type.tlk',
#         ]
        
#         for filename in malformed_files:
#             file_path = fixtures_dir / filename
#             if file_path.exists():
#                 with pytest.raises(ValueError):
#                     parser.read(str(file_path))
                    
#     def test_unicode_content_from_fixture(self, parser, fixtures_dir):
#         """Test Unicode handling with real fixture"""
#         file_path = fixtures_dir / 'dialog_english.tlk'
#         if file_path.exists():
#             parser.read(str(file_path))
            
#             # Check Cyrillic string
#             cyrillic = parser.get_string(7)
#             if cyrillic:
#                 assert "Продолжить" in cyrillic
                
#             # Check Japanese string  
#             japanese = parser.get_string(8)
#             if japanese:
#                 assert "日本語" in japanese
                
#     def test_special_characters_from_fixture(self, parser, fixtures_dir):
#         """Test special character handling"""
#         file_path = fixtures_dir / 'dialog_english.tlk'
#         if file_path.exists():
#             parser.read(str(file_path))
            
#             # Check special chars string
#             special = parser.get_string(9)
#             if special:
#                 assert "\n" in special
#                 assert "\t" in special
#                 assert "\r" in special