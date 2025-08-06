#!/usr/bin/env python3
"""
Test data generator library for NWN2 parser tests.
Provides easy-to-use functions for creating test data.
"""
import os
import struct
from pathlib import Path
from typing import Dict, List, Optional, Tuple, Any
from .enhanced_gff_creator import create_module_ifo, create_campaign_cam

# Since create_test_data.py is deleted, define the needed functions here
class ERFResourceType:
    """ERF resource type constants"""
    TDA = 2017  # 2DA files
    IFO = 2014  # Module info files


def create_test_erf(filename: str, file_type: str, resources: list):
    """Create a minimal ERF file with given resources"""
    with open(filename, 'wb') as f:
        # Write ERF header
        f.write(file_type.encode('ascii'))  # File type (4 bytes)
        f.write(b'V1.0')  # Version (4 bytes)
        
        # Calculate offsets
        header_size = 160  # Standard ERF header size
        localized_string_count = 0
        localized_string_size = 0
        entry_count = len(resources) if resources else 0
        
        # Offsets
        offset_to_localized_string = header_size
        offset_to_key_list = offset_to_localized_string + localized_string_size
        offset_to_resource_list = offset_to_key_list + (entry_count * 24)  # Each key is 24 bytes
        
        # Write header fields
        f.write(struct.pack('<I', localized_string_count))
        f.write(struct.pack('<I', localized_string_size))
        f.write(struct.pack('<I', entry_count))
        f.write(struct.pack('<I', offset_to_localized_string))
        f.write(struct.pack('<I', offset_to_key_list))
        f.write(struct.pack('<I', offset_to_resource_list))
        
        # Build year/day
        f.write(struct.pack('<I', 2024))  # Build year
        f.write(struct.pack('<I', 1))     # Build day
        
        # Description strref and padding
        f.write(struct.pack('<I', 0xFFFFFFFF))  # No description
        f.write(b'\x00' * 116)  # Reserved bytes to reach 160 bytes header
        
        if resources:
            # Write key list
            for i, (resref, res_type, _) in enumerate(resources):
                # Pad resref to 16 bytes
                resref_bytes = resref.encode('ascii')[:16]
                resref_bytes += b'\x00' * (16 - len(resref_bytes))
                f.write(resref_bytes)
                f.write(struct.pack('<I', i))  # Resource ID
                f.write(struct.pack('<H', res_type))  # Resource type
                f.write(struct.pack('<H', 0))  # Reserved
            
            # Calculate resource offsets
            resource_data_offset = offset_to_resource_list + (entry_count * 8)
            
            # Write resource list
            current_offset = resource_data_offset
            for _, _, data in resources:
                f.write(struct.pack('<I', current_offset))
                f.write(struct.pack('<I', len(data)))
                current_offset += len(data)
            
            # Write resource data
            for _, _, data in resources:
                f.write(data)
    
    return filename


def create_test_tlk(filename: str, strings: dict):
    """Create a minimal valid TLK file."""
    with open(filename, 'wb') as f:
        # TLK Header
        f.write(b'TLK ')
        f.write(b'V3.0')
        
        # Language ID (0 = English)
        f.write(struct.pack('<I', 0))
        
        # String count
        string_count = max(strings.keys()) + 1 if strings else 0
        f.write(struct.pack('<I', string_count))
        
        # String entries offset
        string_entries_offset = 20  # Header size
        f.write(struct.pack('<I', string_entries_offset))
        
        # String data starts after all entries
        string_data_offset = string_entries_offset + (string_count * 40)
        
        # Write string entries
        current_string_offset = 0
        for i in range(string_count):
            if i in strings:
                text = strings[i]
                # Flags (32 bits) - 0x01 for has text, 0x00 for no sound
                f.write(struct.pack('<I', 0x01))
                # Sound ResRef (16 bytes) - empty
                f.write(b'\x00' * 16)
                # Volume Variance and Pitch Variance (4 bytes each)
                f.write(struct.pack('<I', 0))
                f.write(struct.pack('<I', 0)) 
                # Offset to string relative to string data section
                f.write(struct.pack('<I', current_string_offset))
                # String length
                f.write(struct.pack('<I', len(text)))
                # Sound length
                f.write(struct.pack('<I', 0))
                
                current_string_offset += len(text) + 1  # +1 for null terminator
            else:
                # Empty entry
                f.write(b'\x00' * 40)
        
        # Write string data
        for i in range(string_count):
            if i in strings:
                text = strings[i]
                f.write(text.encode('utf-8') + b'\x00')


class TestDataGenerator:
    """Main test data generator class"""
    
    def __init__(self, base_path: Path):
        self.base_path = Path(base_path)
        self._ensure_directories()
        
    def _ensure_directories(self):
        """Ensure all required directories exist"""
        for subdir in ['2da', 'hak', 'modules', 'campaigns', 'tlk', 'override', 'workshop']:
            (self.base_path / subdir).mkdir(exist_ok=True)
            
    def create_2da(self, filename: str, columns: List[str], rows: List[List[str]], 
                   directory: str = '2da') -> Path:
        """Create a 2DA file with given columns and rows"""
        content = "2DA V2.0\n\n"
        content += "\t".join([""] + columns) + "\n"
        
        for i, row in enumerate(rows):
            content += f"{i}\t" + "\t".join(row) + "\n"
            
        filepath = self.base_path / directory / filename
        filepath.write_text(content)
        return filepath
        
    def create_standard_2das(self) -> Dict[str, Path]:
        """Create standard 2DA files used in most tests"""
        files = {}
        
        # classes.2da
        files['classes.2da'] = self.create_2da(
            'classes.2da',
            ['Name', 'HitDie', 'AttackBonusTable', 'SkillPointBase'],
            [
                ['5001', '10', 'CLS_ATK_1', '4'],  # Fighter
                ['5002', '6', 'CLS_ATK_3', '2'],   # Wizard
                ['5003', '8', 'CLS_ATK_2', '8'],   # Rogue
            ]
        )
        
        # racialtypes.2da
        files['racialtypes.2da'] = self.create_2da(
            'racialtypes.2da',
            ['Name', 'ConverName', 'Appearance'],
            [
                ['6001', '6501', '0'],  # Human
                ['6002', '6502', '1'],  # Elf
                ['6003', '6503', '2'],  # Dwarf
            ]
        )
        
        # feat.2da
        files['feat.2da'] = self.create_2da(
            'feat.2da',
            ['LABEL', 'FEAT', 'DESCRIPTION', 'ICON'],
            [
                ['FEAT_ALERTNESS', '0', '1000', 'ir_alertness'],
                ['FEAT_ARMOR_PROF_LIGHT', '1', '1001', 'ir_light'],
                ['FEAT_ARMOR_PROF_MEDIUM', '2', '1002', 'ir_medium'],
            ]
        )
        
        # appearances.2da
        files['appearances.2da'] = self.create_2da(
            'appearances.2da',
            ['Name', 'SIZECATEGORY'],
            [
                ['1015', '3'],  # Human appearance
                ['1016', '3'],  # Elf appearance
                ['1017', '3'],  # Dwarf appearance
            ]
        )
        
        # cls_atk_1.2da (BAB progression)
        files['cls_atk_1.2da'] = self.create_2da(
            'cls_atk_1.2da',
            ['BAB'],
            [[str(i)] for i in range(21)]  # 0-20
        )
        
        return files
        
    def create_hak(self, name: str, contents: Dict[str, bytes]) -> Path:
        """Create a HAK file with given contents"""
        resources = []
        for filename, data in contents.items():
            resref = filename.rsplit('.', 1)[0]  # Remove extension
            ext = filename.rsplit('.', 1)[1] if '.' in filename else ''
            
            # Determine resource type
            res_type = ERFResourceType.TDA if ext == '2da' else 0
            
            resources.append((resref, res_type, data))
            
        filepath = self.base_path / 'hak' / f'{name}.hak'
        create_test_erf(str(filepath), 'HAK ', resources)
        return filepath
        
    def create_module(self, name: str, hak_list: List[str] = None,
                     custom_tlk: str = "", custom_2das: Dict[str, str] = None) -> Path:
        """Create a module file with given properties"""
        resources = []
        
        # Create module.ifo
        ifo_data = create_module_ifo(name, hak_list, custom_tlk)
        resources.append(('module', ERFResourceType.IFO, ifo_data))
        
        # Add any custom 2DAs
        if custom_2das:
            for filename, content in custom_2das.items():
                resref = filename.rsplit('.', 1)[0]
                resources.append((resref, ERFResourceType.TDA, content.encode('utf-8')))
                
        filepath = self.base_path / 'modules' / f'{name.lower().replace(" ", "_")}.mod'
        create_test_erf(str(filepath), 'MOD ', resources)
        return filepath
        
    def create_campaign(self, name: str, modules: List[Dict[str, str]]) -> Path:
        """Create a campaign file"""
        campaign_data = create_campaign_cam(name, modules)
        
        filepath = self.base_path / 'campaigns' / f'{name.lower().replace(" ", "_")}.cam'
        filepath.write_bytes(campaign_data)
        return filepath
        
    def create_tlk(self, name: str, strings: Dict[int, str]) -> Path:
        """Create a TLK file with given strings"""
        filepath = self.base_path / 'tlk' / f'{name}.tlk'
        create_test_tlk(str(filepath), strings)
        return filepath
        
    def create_standard_tlk(self) -> Path:
        """Create a standard dialog.tlk with common strings"""
        strings = {
            # Class names
            5001: "Fighter",
            5002: "Wizard", 
            5003: "Rogue",
            5004: "Ranger",
            5005: "Cleric",
            
            # Race names
            6001: "Human",
            6002: "Elf",
            6003: "Dwarf",
            6004: "Half-Orc",
            6005: "Halfling",
            
            # Feat names
            1000: "Alertness",
            1001: "Light Armor Proficiency",
            1002: "Medium Armor Proficiency",
            
            # Appearance names
            1015: "Human Male",
            1016: "Elf Male",
            1017: "Dwarf Male",
        }
        
        return self.create_tlk('dialog', strings)
        
    def create_override_structure(self, files: Dict[str, str]) -> Dict[str, Path]:
        """Create files in override directory"""
        created = {}
        for filename, content in files.items():
            filepath = self.base_path / 'override' / filename
            filepath.write_text(content)
            created[filename] = filepath
        return created
        
    def create_workshop_structure(self, item_id: str, files: Dict[str, str]) -> Dict[str, Path]:
        """Create workshop directory structure with files"""
        workshop_dir = self.base_path / 'workshop' / 'content' / '2738630' / item_id / 'override'
        workshop_dir.mkdir(parents=True, exist_ok=True)
        
        created = {}
        for filename, content in files.items():
            filepath = workshop_dir / filename
            filepath.write_text(content)
            created[filename] = filepath
        return created
        
    def clean_all(self):
        """Remove all generated test data"""
        import shutil
        for subdir in ['2da', 'hak', 'modules', 'campaigns', 'tlk', 'override', 'workshop']:
            path = self.base_path / subdir
            if path.exists():
                shutil.rmtree(path)
                path.mkdir()


# Convenience functions for quick test setup
def setup_basic_test_data(base_path: Path) -> TestDataGenerator:
    """Set up basic test data structure"""
    gen = TestDataGenerator(base_path)
    gen.create_standard_2das()
    gen.create_standard_tlk()
    return gen


def create_full_override_chain(base_path: Path) -> Dict[str, Any]:
    """Create a full override chain for testing priority"""
    gen = TestDataGenerator(base_path)
    
    # Base game files
    base_files = gen.create_standard_2das()
    
    # Override directory
    gen.create_override_structure({
        'classes.2da': '2DA V2.0\n\n\tLABEL Name HitDie\n0\tOverrideClass 12\n'
    })
    
    # Workshop
    gen.create_workshop_structure('123456', {
        'classes.2da': '2DA V2.0\n\n\tLABEL Name HitDie\n0\tWorkshopClass 8\n'
    })
    
    # HAK with override
    hak_2da = '2DA V2.0\n\n\tLABEL Name HitDie\n0\tHAKClass 10\n'
    gen.create_hak('custom', {'classes.2da': hak_2da.encode('utf-8')})
    
    # Module with HAK and its own override
    gen.create_module(
        'TestModule',
        hak_list=['custom'],
        custom_2das={'classes.2da': '2DA V2.0\n\n\tLABEL Name HitDie\n0\tModuleClass 6\n'}
    )
    
    return {
        'generator': gen,
        'base_files': base_files,
        'expected_chain': ['ModuleClass', 'HAKClass', 'WorkshopClass', 'OverrideClass', '5001']
    }