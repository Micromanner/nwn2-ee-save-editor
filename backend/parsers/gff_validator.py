"""
GFF File Validation Framework

Provides validation for NWN2 GFF V3.2 binary format files with support for:
- Structure validation based on file type
- Field type and value validation
- Required field checking
- Reference validation (e.g. TLK string refs)
- Corrupted file detection and recovery hints
"""

from typing import Dict, List, Optional, Set, Tuple, Any, Union
from dataclasses import dataclass, field
from enum import Enum, auto
import struct
import io
from pathlib import Path

from .gff import GFFParser, GFFElement, GFFFieldType, LocalizedString


class ValidationLevel(Enum):
    """Validation strictness levels"""
    STRICT = auto()    # All fields must match schema exactly
    NORMAL = auto()    # Required fields must be present, extra fields allowed
    LENIENT = auto()   # Missing required fields produce warnings, not errors


class ValidationSeverity(Enum):
    """Severity levels for validation issues"""
    ERROR = auto()     # Critical issue, file may not work correctly
    WARNING = auto()   # Non-critical issue, file should work but may have problems
    INFO = auto()      # Informational message


@dataclass
class ValidationIssue:
    """Represents a validation issue found in a GFF file"""
    severity: ValidationSeverity
    field_path: str  # e.g. "ClassList[0].Class"
    message: str
    expected_value: Optional[Any] = None
    actual_value: Optional[Any] = None
    recovery_hint: Optional[str] = None


@dataclass
class FieldSchema:
    """Schema definition for a GFF field"""
    label: str
    field_type: GFFFieldType
    required: bool = True
    min_value: Optional[Union[int, float]] = None
    max_value: Optional[Union[int, float]] = None
    allowed_values: Optional[Set[Union[int, str]]] = None
    min_length: Optional[int] = None
    max_length: Optional[int] = None
    sub_schema: Optional['StructSchema'] = None  # For STRUCT fields
    list_item_schema: Optional['StructSchema'] = None  # For LIST fields
    validate_tlk_ref: bool = False  # For LOCSTRING fields
    custom_validator: Optional[callable] = None


@dataclass
class StructSchema:
    """Schema definition for a GFF struct"""
    struct_id: Optional[int] = None
    fields: Dict[str, FieldSchema] = field(default_factory=dict)
    allow_extra_fields: bool = True
    
    def add_field(self, field_schema: FieldSchema) -> 'StructSchema':
        """Add a field to the schema"""
        self.fields[field_schema.label] = field_schema
        return self


class GFFValidator:
    """Validates GFF files against predefined schemas"""
    
    def __init__(self, validation_level: ValidationLevel = ValidationLevel.NORMAL):
        self.validation_level = validation_level
        self.schemas: Dict[str, StructSchema] = {}
        self._init_schemas()
        
    def _init_schemas(self):
        """Initialize validation schemas for known file types"""
        # BIC (character) file schema
        self._init_bic_schema()
        
        # IFO (module info) file schema
        self._init_ifo_schema()
        
        # ARE (area) file schema
        self._init_are_schema()
        
        # ROS (roster) file schema
        self._init_ros_schema()
        
        # More schemas can be added as needed
        
    def _init_bic_schema(self):
        """Initialize BIC file validation schema"""
        # Basic character fields
        bic_schema = StructSchema()
        
        # Character basics
        bic_schema.add_field(FieldSchema("FirstName", GFFFieldType.LOCSTRING, required=True))
        bic_schema.add_field(FieldSchema("LastName", GFFFieldType.LOCSTRING, required=False))
        
        # Abilities
        for ability in ["Str", "Dex", "Con", "Int", "Wis", "Cha"]:
            bic_schema.add_field(FieldSchema(ability, GFFFieldType.BYTE, required=True, min_value=3, max_value=50))
        
        # Level and XP
        bic_schema.add_field(FieldSchema("HitDice", GFFFieldType.BYTE, required=True, min_value=1, max_value=30))
        bic_schema.add_field(FieldSchema("Experience", GFFFieldType.DWORD, required=True, min_value=0))
        
        # Hit Points
        bic_schema.add_field(FieldSchema("MaxHitPoints", GFFFieldType.SHORT, required=True, min_value=1))
        bic_schema.add_field(FieldSchema("CurrentHitPoints", GFFFieldType.SHORT, required=True, min_value=0))
        bic_schema.add_field(FieldSchema("PregameCurrent", GFFFieldType.SHORT, required=True, min_value=0))
        
        # Alignment
        bic_schema.add_field(FieldSchema("LawfulChaotic", GFFFieldType.BYTE, required=True, min_value=0, max_value=100))
        bic_schema.add_field(FieldSchema("GoodEvil", GFFFieldType.BYTE, required=True, min_value=0, max_value=100))
        
        # Race and appearance
        bic_schema.add_field(FieldSchema("Race", GFFFieldType.BYTE, required=True, min_value=0))
        bic_schema.add_field(FieldSchema("Subrace", GFFFieldType.STRING, required=False, max_length=32))
        bic_schema.add_field(FieldSchema("Appearance_Type", GFFFieldType.WORD, required=True))
        bic_schema.add_field(FieldSchema("Gender", GFFFieldType.BYTE, required=True, min_value=0, max_value=1))
        
        # Saves
        bic_schema.add_field(FieldSchema("fortbonus", GFFFieldType.BYTE, required=True))
        bic_schema.add_field(FieldSchema("refbonus", GFFFieldType.BYTE, required=True))
        bic_schema.add_field(FieldSchema("willbonus", GFFFieldType.BYTE, required=True))
        
        # Class list
        class_schema = StructSchema(struct_id=2)
        class_schema.add_field(FieldSchema("Class", GFFFieldType.INT, required=True, min_value=0))
        class_schema.add_field(FieldSchema("ClassLevel", GFFFieldType.SHORT, required=True, min_value=1, max_value=30))
        
        bic_schema.add_field(FieldSchema("ClassList", GFFFieldType.LIST, required=True, 
                                        list_item_schema=class_schema))
        
        # Feat list
        feat_schema = StructSchema(struct_id=1)
        feat_schema.add_field(FieldSchema("Feat", GFFFieldType.WORD, required=True))
        
        bic_schema.add_field(FieldSchema("FeatList", GFFFieldType.LIST, required=False,
                                        list_item_schema=feat_schema))
        
        # Skill list
        skill_schema = StructSchema(struct_id=0)
        skill_schema.add_field(FieldSchema("Rank", GFFFieldType.BYTE, required=True, min_value=0))
        
        bic_schema.add_field(FieldSchema("SkillList", GFFFieldType.LIST, required=True,
                                        list_item_schema=skill_schema))
        
        self.schemas["BIC"] = bic_schema
        
    def _init_ifo_schema(self):
        """Initialize IFO file validation schema"""
        ifo_schema = StructSchema()
        
        # Module info
        ifo_schema.add_field(FieldSchema("Mod_Name", GFFFieldType.LOCSTRING, required=True))
        ifo_schema.add_field(FieldSchema("Mod_Tag", GFFFieldType.STRING, required=True, max_length=32))
        ifo_schema.add_field(FieldSchema("Mod_Description", GFFFieldType.LOCSTRING, required=False))
        
        # Module properties
        ifo_schema.add_field(FieldSchema("Mod_IsSaveGame", GFFFieldType.BYTE, required=False))
        ifo_schema.add_field(FieldSchema("Mod_Version", GFFFieldType.DWORD, required=False))
        
        # Entry info
        ifo_schema.add_field(FieldSchema("Mod_Entry_Area", GFFFieldType.RESREF, required=True))
        ifo_schema.add_field(FieldSchema("Mod_Entry_X", GFFFieldType.FLOAT, required=True))
        ifo_schema.add_field(FieldSchema("Mod_Entry_Y", GFFFieldType.FLOAT, required=True))
        ifo_schema.add_field(FieldSchema("Mod_Entry_Z", GFFFieldType.FLOAT, required=True))
        
        self.schemas["IFO"] = ifo_schema
        
    def _init_are_schema(self):
        """Initialize ARE file validation schema"""
        are_schema = StructSchema()
        
        # Area properties
        are_schema.add_field(FieldSchema("Name", GFFFieldType.LOCSTRING, required=True))
        are_schema.add_field(FieldSchema("Tag", GFFFieldType.STRING, required=True, max_length=32))
        are_schema.add_field(FieldSchema("ResRef", GFFFieldType.RESREF, required=True))
        
        # Area settings
        are_schema.add_field(FieldSchema("Width", GFFFieldType.INT, required=True, min_value=1))
        are_schema.add_field(FieldSchema("Height", GFFFieldType.INT, required=True, min_value=1))
        
        self.schemas["ARE"] = are_schema
        
    def _init_ros_schema(self):
        """Initialize ROS file validation schema"""
        ros_schema = StructSchema()
        
        # Roster entry fields
        ros_schema.add_field(FieldSchema("RosName", GFFFieldType.STRING, required=True))
        ros_schema.add_field(FieldSchema("FirstName", GFFFieldType.LOCSTRING, required=True))
        ros_schema.add_field(FieldSchema("LastName", GFFFieldType.LOCSTRING, required=False))
        
        self.schemas["ROS"] = ros_schema
        
    def validate_file(self, file_path: Union[str, Path]) -> List[ValidationIssue]:
        """Validate a GFF file from disk"""
        parser = GFFParser()
        try:
            element = parser.read(str(file_path))
            file_type = parser.get_file_type().strip()
            return self.validate_element(element, file_type)
        except Exception as e:
            return [ValidationIssue(
                ValidationSeverity.ERROR,
                "",
                f"Failed to parse GFF file: {str(e)}",
                recovery_hint="Check if file is corrupted or not a valid GFF V3.2 file"
            )]
            
    def validate_stream(self, stream: io.BytesIO, file_type: Optional[str] = None) -> List[ValidationIssue]:
        """Validate a GFF file from a stream"""
        parser = GFFParser()
        try:
            element = parser.load(stream)
            if file_type is None:
                file_type = parser.get_file_type().strip()
            return self.validate_element(element, file_type)
        except Exception as e:
            return [ValidationIssue(
                ValidationSeverity.ERROR,
                "",
                f"Failed to parse GFF stream: {str(e)}",
                recovery_hint="Check if stream contains valid GFF V3.2 data"
            )]
            
    def validate_element(self, element: GFFElement, file_type: str) -> List[ValidationIssue]:
        """Validate a parsed GFF element against its schema"""
        issues = []
        
        # Get schema for file type
        schema = self.schemas.get(file_type)
        if schema is None:
            if self.validation_level == ValidationLevel.STRICT:
                issues.append(ValidationIssue(
                    ValidationSeverity.WARNING,
                    "",
                    f"No validation schema defined for file type '{file_type}'"
                ))
            return issues
            
        # Validate the root struct
        self._validate_struct(element, schema, "", issues)
        
        return issues
        
    def _validate_struct(self, element: GFFElement, schema: StructSchema, 
                        path: str, issues: List[ValidationIssue]):
        """Validate a struct element against its schema"""
        if element.type != GFFFieldType.STRUCT:
            issues.append(ValidationIssue(
                ValidationSeverity.ERROR,
                path,
                f"Expected STRUCT but got {element.type.name}"
            ))
            return
            
        # Check struct ID if specified
        if schema.struct_id is not None and element.id != schema.struct_id:
            issues.append(ValidationIssue(
                ValidationSeverity.WARNING,
                path,
                f"Struct ID mismatch",
                expected_value=schema.struct_id,
                actual_value=element.id
            ))
            
        # Get all field labels in the struct
        present_fields = set()
        if isinstance(element.value, list):
            for field in element.value:
                present_fields.add(field.label)
                
        # Check required fields
        for field_label, field_schema in schema.fields.items():
            if field_schema.required and field_label not in present_fields:
                severity = ValidationSeverity.ERROR
                if self.validation_level == ValidationLevel.LENIENT:
                    severity = ValidationSeverity.WARNING
                    
                issues.append(ValidationIssue(
                    severity,
                    f"{path}.{field_label}" if path else field_label,
                    f"Required field '{field_label}' is missing",
                    recovery_hint=f"Add {field_label} field of type {field_schema.field_type.name}"
                ))
                
        # Validate present fields
        if isinstance(element.value, list):
            for field in element.value:
                field_path = f"{path}.{field.label}" if path else field.label
                
                if field.label in schema.fields:
                    field_schema = schema.fields[field.label]
                    self._validate_field(field, field_schema, field_path, issues)
                elif not schema.allow_extra_fields and self.validation_level == ValidationLevel.STRICT:
                    issues.append(ValidationIssue(
                        ValidationSeverity.WARNING,
                        field_path,
                        f"Unexpected field '{field.label}' not defined in schema"
                    ))
                    
    def _validate_field(self, field: GFFElement, schema: FieldSchema, 
                       path: str, issues: List[ValidationIssue]):
        """Validate a field against its schema"""
        # Check field type
        if field.type != schema.field_type:
            issues.append(ValidationIssue(
                ValidationSeverity.ERROR,
                path,
                f"Field type mismatch",
                expected_value=schema.field_type.name,
                actual_value=field.type.name
            ))
            return
            
        # Type-specific validation
        if field.type in [GFFFieldType.BYTE, GFFFieldType.CHAR, GFFFieldType.WORD,
                         GFFFieldType.SHORT, GFFFieldType.DWORD, GFFFieldType.INT,
                         GFFFieldType.DWORD64, GFFFieldType.INT64]:
            self._validate_numeric(field, schema, path, issues)
            
        elif field.type in [GFFFieldType.FLOAT, GFFFieldType.DOUBLE]:
            self._validate_float(field, schema, path, issues)
            
        elif field.type == GFFFieldType.STRING:
            self._validate_string(field, schema, path, issues)
            
        elif field.type == GFFFieldType.RESREF:
            self._validate_resref(field, schema, path, issues)
            
        elif field.type == GFFFieldType.LOCSTRING:
            self._validate_locstring(field, schema, path, issues)
            
        elif field.type == GFFFieldType.STRUCT:
            if schema.sub_schema:
                self._validate_struct(field, schema.sub_schema, path, issues)
                
        elif field.type == GFFFieldType.LIST:
            self._validate_list(field, schema, path, issues)
            
        # Custom validation
        if schema.custom_validator:
            try:
                schema.custom_validator(field, path, issues)
            except Exception as e:
                issues.append(ValidationIssue(
                    ValidationSeverity.WARNING,
                    path,
                    f"Custom validator failed: {str(e)}"
                ))
                
    def _validate_numeric(self, field: GFFElement, schema: FieldSchema,
                         path: str, issues: List[ValidationIssue]):
        """Validate numeric field values"""
        value = field.value
        
        if value is None:
            issues.append(ValidationIssue(
                ValidationSeverity.ERROR,
                path,
                "Numeric field has null value",
                recovery_hint="Set to 0 or appropriate default value"
            ))
            return
            
        if schema.min_value is not None and value < schema.min_value:
            issues.append(ValidationIssue(
                ValidationSeverity.ERROR,
                path,
                f"Value below minimum",
                expected_value=f">= {schema.min_value}",
                actual_value=value,
                recovery_hint=f"Set to minimum value {schema.min_value}"
            ))
            
        if schema.max_value is not None and value > schema.max_value:
            issues.append(ValidationIssue(
                ValidationSeverity.ERROR,
                path,
                f"Value above maximum",
                expected_value=f"<= {schema.max_value}",
                actual_value=value,
                recovery_hint=f"Set to maximum value {schema.max_value}"
            ))
            
        if schema.allowed_values and value not in schema.allowed_values:
            issues.append(ValidationIssue(
                ValidationSeverity.ERROR,
                path,
                f"Value not in allowed set",
                expected_value=schema.allowed_values,
                actual_value=value
            ))
            
    def _validate_float(self, field: GFFElement, schema: FieldSchema,
                       path: str, issues: List[ValidationIssue]):
        """Validate float field values"""
        value = field.value
        
        if value is None:
            issues.append(ValidationIssue(
                ValidationSeverity.ERROR,
                path,
                "Float field has null value",
                recovery_hint="Set to 0.0 or appropriate default value"
            ))
            return
            
        if schema.min_value is not None and value < schema.min_value:
            issues.append(ValidationIssue(
                ValidationSeverity.ERROR,
                path,
                f"Value below minimum",
                expected_value=f">= {schema.min_value}",
                actual_value=value
            ))
            
        if schema.max_value is not None and value > schema.max_value:
            issues.append(ValidationIssue(
                ValidationSeverity.ERROR,
                path,
                f"Value above maximum",
                expected_value=f"<= {schema.max_value}",
                actual_value=value
            ))
            
    def _validate_string(self, field: GFFElement, schema: FieldSchema,
                        path: str, issues: List[ValidationIssue]):
        """Validate string field values"""
        value = field.value
        
        if value is None:
            issues.append(ValidationIssue(
                ValidationSeverity.ERROR,
                path,
                "String field has null value",
                recovery_hint="Set to empty string"
            ))
            return
            
        if schema.min_length is not None and len(value) < schema.min_length:
            issues.append(ValidationIssue(
                ValidationSeverity.ERROR,
                path,
                f"String too short",
                expected_value=f"length >= {schema.min_length}",
                actual_value=f"length = {len(value)}"
            ))
            
        if schema.max_length is not None and len(value) > schema.max_length:
            issues.append(ValidationIssue(
                ValidationSeverity.WARNING,
                path,
                f"String too long",
                expected_value=f"length <= {schema.max_length}",
                actual_value=f"length = {len(value)}",
                recovery_hint=f"Truncate to {schema.max_length} characters"
            ))
            
        if schema.allowed_values and value not in schema.allowed_values:
            issues.append(ValidationIssue(
                ValidationSeverity.ERROR,
                path,
                f"String value not in allowed set",
                expected_value=schema.allowed_values,
                actual_value=value
            ))
            
    def _validate_resref(self, field: GFFElement, schema: FieldSchema,
                        path: str, issues: List[ValidationIssue]):
        """Validate resource reference field values"""
        value = field.value
        
        if value is None:
            issues.append(ValidationIssue(
                ValidationSeverity.ERROR,
                path,
                "ResRef field has null value",
                recovery_hint="Set to empty string"
            ))
            return
            
        # ResRefs have max length of 32
        if len(value) > 32:
            issues.append(ValidationIssue(
                ValidationSeverity.ERROR,
                path,
                f"ResRef too long",
                expected_value="length <= 32",
                actual_value=f"length = {len(value)}",
                recovery_hint="Truncate to 32 characters"
            ))
            
    def _validate_locstring(self, field: GFFElement, schema: FieldSchema,
                           path: str, issues: List[ValidationIssue]):
        """Validate localized string field values"""
        value = field.value
        
        if value is None:
            issues.append(ValidationIssue(
                ValidationSeverity.ERROR,
                path,
                "LocString field has null value",
                recovery_hint="Create empty LocalizedString with string_ref=-1"
            ))
            return
            
        if not isinstance(value, LocalizedString):
            issues.append(ValidationIssue(
                ValidationSeverity.ERROR,
                path,
                f"LocString field has wrong type",
                expected_value="LocalizedString",
                actual_value=type(value).__name__
            ))
            return
            
        # Validate TLK reference if requested
        if schema.validate_tlk_ref and value.string_ref >= 0:
            # This would require access to TLK file to validate
            # For now just check if it's a reasonable value
            if value.string_ref > 1000000:  # Arbitrary high limit
                issues.append(ValidationIssue(
                    ValidationSeverity.WARNING,
                    path,
                    f"TLK string reference seems too high",
                    actual_value=value.string_ref
                ))
                
    def _validate_list(self, field: GFFElement, schema: FieldSchema,
                      path: str, issues: List[ValidationIssue]):
        """Validate list field values"""
        value = field.value
        
        if value is None:
            issues.append(ValidationIssue(
                ValidationSeverity.ERROR,
                path,
                "List field has null value",
                recovery_hint="Set to empty list"
            ))
            return
            
        if not isinstance(value, list):
            issues.append(ValidationIssue(
                ValidationSeverity.ERROR,
                path,
                f"List field has wrong type",
                expected_value="list",
                actual_value=type(value).__name__
            ))
            return
            
        # Validate list length
        if schema.min_length is not None and len(value) < schema.min_length:
            issues.append(ValidationIssue(
                ValidationSeverity.ERROR,
                path,
                f"List too short",
                expected_value=f"length >= {schema.min_length}",
                actual_value=f"length = {len(value)}"
            ))
            
        if schema.max_length is not None and len(value) > schema.max_length:
            issues.append(ValidationIssue(
                ValidationSeverity.WARNING,
                path,
                f"List too long",
                expected_value=f"length <= {schema.max_length}",
                actual_value=f"length = {len(value)}"
            ))
            
        # Validate list items
        if schema.list_item_schema:
            for i, item in enumerate(value):
                item_path = f"{path}[{i}]"
                self._validate_struct(item, schema.list_item_schema, item_path, issues)


class GFFRecovery:
    """Provides recovery suggestions for corrupted GFF files"""
    
    @staticmethod
    def analyze_corruption(file_path: Union[str, Path]) -> List[ValidationIssue]:
        """Analyze a potentially corrupted GFF file and suggest recovery steps"""
        issues = []
        
        try:
            with open(file_path, 'rb') as f:
                # Check file size
                f.seek(0, 2)
                file_size = f.tell()
                f.seek(0)
                
                if file_size < 56:
                    issues.append(ValidationIssue(
                        ValidationSeverity.ERROR,
                        "",
                        f"File too small ({file_size} bytes), GFF header requires 56 bytes",
                        recovery_hint="File is likely truncated or not a GFF file"
                    ))
                    return issues
                    
                # Read header
                header = f.read(56)
                
                # Check file type
                file_type = header[0:4].decode('ascii', errors='ignore')
                if not file_type.replace('\x00', '').isalnum():
                    issues.append(ValidationIssue(
                        ValidationSeverity.ERROR,
                        "",
                        f"Invalid file type '{file_type}'",
                        recovery_hint="Header may be corrupted, check if this is a GFF file"
                    ))
                    
                # Check version
                version = header[4:8].decode('ascii', errors='ignore')
                if version != 'V3.2':
                    issues.append(ValidationIssue(
                        ValidationSeverity.ERROR,
                        "",
                        f"Invalid version '{version}', expected 'V3.2'",
                        recovery_hint="File may be from different game or corrupted"
                    ))
                    
                # Check offsets and counts
                struct_offset = struct.unpack('<I', header[8:12])[0]
                struct_count = struct.unpack('<I', header[12:16])[0]
                field_offset = struct.unpack('<I', header[16:20])[0]
                field_count = struct.unpack('<I', header[20:24])[0]
                label_offset = struct.unpack('<I', header[24:28])[0]
                label_count = struct.unpack('<I', header[28:32])[0]
                field_data_offset = struct.unpack('<I', header[32:36])[0]
                field_data_count = struct.unpack('<I', header[36:40])[0]
                field_indices_offset = struct.unpack('<I', header[40:44])[0]
                field_indices_count = struct.unpack('<I', header[44:48])[0]
                list_indices_offset = struct.unpack('<I', header[48:52])[0]
                list_indices_count = struct.unpack('<I', header[52:56])[0]
                
                # Validate offsets don't exceed file size
                sections = [
                    ("Struct array", struct_offset, struct_count * 12),
                    ("Field array", field_offset, field_count * 12),
                    ("Label array", label_offset, label_count * 16),
                    ("Field data", field_data_offset, field_data_count),
                    ("Field indices", field_indices_offset, field_indices_count),
                    ("List indices", list_indices_offset, list_indices_count)
                ]
                
                for name, offset, size in sections:
                    if offset + size > file_size:
                        issues.append(ValidationIssue(
                            ValidationSeverity.ERROR,
                            "",
                            f"{name} extends beyond file end (offset={offset}, size={size}, file_size={file_size})",
                            recovery_hint="File is truncated, try to recover from backup"
                        ))
                        
                # Check for overlapping sections
                section_ranges = [(offset, offset + size, name) 
                                 for name, offset, size in sections if size > 0]
                section_ranges.sort()
                
                for i in range(len(section_ranges) - 1):
                    start1, end1, name1 = section_ranges[i]
                    start2, end2, name2 = section_ranges[i + 1]
                    if end1 > start2:
                        issues.append(ValidationIssue(
                            ValidationSeverity.ERROR,
                            "",
                            f"Sections overlap: {name1} and {name2} (section 1: {start1}-{end1}, section 2: {start2}-{end2})",
                            recovery_hint="Header offsets are corrupted"
                        ))
                        
        except Exception as e:
            issues.append(ValidationIssue(
                ValidationSeverity.ERROR,
                "",
                f"Failed to analyze file: {str(e)}",
                recovery_hint="File may be severely corrupted or not accessible"
            ))
            
        return issues