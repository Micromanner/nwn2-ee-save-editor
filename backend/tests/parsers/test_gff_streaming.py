"""
Tests for streaming GFF parser
"""
import pytest
import io
import struct
import tempfile
from pathlib import Path

from parsers.gff import GFFParser, GFFWriter, GFFElement, GFFFieldType, LocalizedString, LocalizedSubstring
from parsers.gff_streaming import (
    StreamingGFFParser, StreamingOptions, LazyGFFElement,
    extract_character_name, count_module_areas
)


class TestStreamingGFFParser:
    """Test streaming GFF parser functionality"""
    
    @pytest.fixture
    def simple_gff_file(self, tmp_path):
        """Create a simple GFF file for testing"""
        # Create a simple BIC-like structure
        fields = [
            GFFElement(GFFFieldType.LOCSTRING, 0, "FirstName",
                      LocalizedString(-1, [LocalizedSubstring("Test", 0, 0)])),
            GFFElement(GFFFieldType.LOCSTRING, 0, "LastName",
                      LocalizedString(-1, [LocalizedSubstring("Character", 0, 0)])),
            GFFElement(GFFFieldType.BYTE, 0, "Str", 16),
            GFFElement(GFFFieldType.BYTE, 0, "Dex", 14),
            GFFElement(GFFFieldType.INT, 0, "Experience", 5000),
            GFFElement(GFFFieldType.STRING, 0, "Description", "A test character for streaming"),
        ]
        root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        # Write to file
        test_file = tmp_path / "test.bic"
        writer = GFFWriter("BIC ", "V3.2")
        writer.write(str(test_file), root)
        
        return test_file
    
    @pytest.fixture
    def large_string_gff_file(self, tmp_path):
        """Create a GFF file with large strings for memory testing"""
        # Create a large string (1MB)
        large_text = "X" * (1024 * 1024)
        
        fields = [
            GFFElement(GFFFieldType.STRING, 0, "LargeField1", large_text),
            GFFElement(GFFFieldType.STRING, 0, "LargeField2", large_text),
            GFFElement(GFFFieldType.INT, 0, "SmallField", 42),
        ]
        root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        test_file = tmp_path / "large.gff"
        writer = GFFWriter("TEST", "V3.2")
        writer.write(str(test_file), root)
        
        return test_file
    
    @pytest.fixture  
    def module_like_file(self, tmp_path):
        """Create a module-like GFF structure"""
        # Create area list
        area_structs = []
        for i in range(5):
            area_fields = [
                GFFElement(GFFFieldType.RESREF, 0, "Area_Name", f"area{i:03d}"),
                GFFElement(GFFFieldType.INT, 0, "Area_ID", i),
            ]
            area_structs.append(GFFElement(GFFFieldType.STRUCT, 1, "", area_fields))
        
        fields = [
            GFFElement(GFFFieldType.LOCSTRING, 0, "Mod_Name",
                      LocalizedString(-1, [LocalizedSubstring("Test Module", 0, 0)])),
            GFFElement(GFFFieldType.STRING, 0, "Mod_Tag", "test_module"),
            GFFElement(GFFFieldType.LIST, 0, "Mod_Area_list", area_structs),
        ]
        root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        test_file = tmp_path / "module.ifo"
        writer = GFFWriter("IFO ", "V3.2")
        writer.write(str(test_file), root)
        
        return test_file
    
    def test_basic_streaming(self, simple_gff_file):
        """Test basic streaming functionality"""
        parser = StreamingGFFParser()
        
        elements = list(parser.parse_file(str(simple_gff_file)))
        assert len(elements) == 1
        
        root = elements[0]
        assert root.type == GFFFieldType.STRUCT
        assert len(root.value) == 6
        
        # Check field values
        field_map = {field.label: field for field in root.value}
        assert "FirstName" in field_map
        assert "Str" in field_map
        assert field_map["Str"].value == 16
        assert field_map["Experience"].value == 5000
    
    def test_filtered_parsing(self, simple_gff_file):
        """Test parsing only specific fields"""
        parser = StreamingGFFParser()
        
        # Parse only name fields
        result = parser.parse_file_filtered(str(simple_gff_file), {"FirstName", "LastName"})
        
        assert len(result) == 2
        assert "FirstName" in result
        assert "LastName" in result
        assert isinstance(result["FirstName"], LocalizedString)
        assert isinstance(result["LastName"], LocalizedString)
    
    def test_field_filter_callback(self, simple_gff_file):
        """Test field filtering with callback"""
        # Only load string fields
        def string_fields_only(label, field_type):
            return field_type in [GFFFieldType.STRING, GFFFieldType.LOCSTRING]
        
        options = StreamingOptions(field_filter=string_fields_only)
        parser = StreamingGFFParser(options)
        
        elements = list(parser.parse_file(str(simple_gff_file)))
        root = elements[0]
        
        # Should only have string fields
        for field in root.value:
            assert field.type in [GFFFieldType.STRING, GFFFieldType.LOCSTRING]
    
    def test_lazy_loading(self, large_string_gff_file):
        """Test lazy loading of large fields"""
        options = StreamingOptions(
            lazy_fields={"LargeField1", "LargeField2"}
        )
        parser = StreamingGFFParser(options)
        
        elements = list(parser.parse_file(str(large_string_gff_file)))
        root = elements[0]
        
        # Find lazy fields
        lazy_fields = [f for f in root.value if isinstance(f, LazyGFFElement)]
        assert len(lazy_fields) == 2
        
        # Verify lazy field hasn't loaded yet
        lazy_field = lazy_fields[0]
        assert not lazy_field._loaded
        
        # Access value triggers load
        value = lazy_field.value
        assert lazy_field._loaded
        assert len(value) == 1024 * 1024
    
    def test_struct_processor_callback(self, module_like_file):
        """Test struct processing callback"""
        processed_structs = []
        
        def process_struct(element):
            processed_structs.append(element.label or "root")
        
        options = StreamingOptions(struct_processor=process_struct)
        parser = StreamingGFFParser(options)
        
        list(parser.parse_file(str(module_like_file)))
        
        # Should have processed root + 5 area structs
        assert len(processed_structs) >= 6
        assert "root" in processed_structs
    
    def test_extract_character_name(self, simple_gff_file):
        """Test the example character name extraction function"""
        name = extract_character_name(str(simple_gff_file))
        assert name == "Test Character"
    
    def test_count_module_areas(self, module_like_file):
        """Test the example area counting function"""
        count = count_module_areas(str(module_like_file))
        assert count == 5
    
    def test_memory_limits(self, tmp_path):
        """Test memory limit enforcement"""
        # Create file with string exceeding limit
        fields = [
            GFFElement(GFFFieldType.STRING, 0, "BigString", "X" * 2_000_000),
        ]
        root = GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
        test_file = tmp_path / "too_big.gff"
        writer = GFFWriter("TEST", "V3.2")
        writer.write(str(test_file), root)
        
        # Parse with 1MB string limit
        options = StreamingOptions(max_string_length=1024 * 1024)
        parser = StreamingGFFParser(options)
        
        with pytest.raises(ValueError, match="String too long"):
            list(parser.parse_file(str(test_file)))
    
    def test_streaming_vs_regular_parser(self, simple_gff_file):
        """Verify streaming parser produces same results as regular parser"""
        # Parse with regular parser
        regular_parser = GFFParser()
        regular_result = regular_parser.read(str(simple_gff_file))
        
        # Parse with streaming parser
        streaming_parser = StreamingGFFParser()
        streaming_results = list(streaming_parser.parse_file(str(simple_gff_file)))
        streaming_result = streaming_results[0]
        
        # Compare results
        assert regular_result.type == streaming_result.type
        assert regular_result.id == streaming_result.id
        assert len(regular_result.value) == len(streaming_result.value)
        
        # Compare field values
        regular_fields = {f.label: f for f in regular_result.value}
        streaming_fields = {f.label: f for f in streaming_result.value}
        
        assert set(regular_fields.keys()) == set(streaming_fields.keys())
        
        for label in regular_fields:
            reg_field = regular_fields[label]
            stream_field = streaming_fields[label]
            assert reg_field.type == stream_field.type
            assert reg_field.value == stream_field.value
    
    def test_invalid_file_handling(self, tmp_path):
        """Test handling of invalid files"""
        # Create invalid file
        test_file = tmp_path / "invalid.gff"
        test_file.write_bytes(b"Not a GFF file")
        
        parser = StreamingGFFParser()
        with pytest.raises(ValueError, match="header"):
            list(parser.parse_file(str(test_file)))
    
    def test_label_caching(self, simple_gff_file):
        """Test that label caching works correctly"""
        parser = StreamingGFFParser()
        
        # Parse file which should trigger label caching
        with open(simple_gff_file, 'rb') as f:
            list(parser.parse_stream(f))
            
        # Labels should be cached
        assert parser._label_cache is not None
        assert len(parser._label_cache) == parser.label_count * 16