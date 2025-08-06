#!/usr/bin/env python3
"""
Django management command to extract comprehensive appearance data from cached
.2da files. This creates properly categorized JSON structures for frontend use.
"""
import json
import os
from typing import Dict, List, Any, Optional

from django.conf import settings
from django.core.management.base import BaseCommand, CommandError

# --- ASSUMPTIONS ---
# The following modules are assumed to be available in the Django project's path.
# The original script's sys.path manipulation is removed in favor of a proper
# project structure.
from config.nwn2_settings import nwn2_paths
from parsers.rust_tlk_parser import TLKParser
from gamedata.safe_cache import SafeCache


class Command(BaseCommand):
    """
    Extracts game appearance data from cached .2da files into structured JSON.
    """
    help = 'Extracts game appearance data from cached .2da files into structured JSON.'

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        # Define paths based on Django's settings.BASE_DIR for robustness
        self.cache_dir = os.path.join(settings.BASE_DIR, 'cache')
        self.output_dir = os.path.join(settings.BASE_DIR, 'gamedata', 'appearance_data_v2')
        self.tlk_path = str(nwn2_paths.dialog_tlk)
        self.tlk = None

    def handle(self, *args, **options):
        """Main entry point for the management command."""
        self.stdout.write(self.style.SUCCESS("Starting appearance data extraction..."))

        # Load the TLK file for string lookups
        self.tlk = self._load_tlk()
        if not self.tlk:
            raise CommandError(f"Could not load TLK file from: {self.tlk_path}")

        # Extract all data categories
        all_data = {
            'appearance_types': self._extract_appearance_types(),
            'body_parts': self._extract_body_parts(),
            'colors': self._extract_colors(),
            'portraits': self._extract_portraits(),
            'soundsets': self._extract_soundsets(),
            'genders': self._extract_gender_data()
        }

        # Print summary of extracted data
        self._print_summary(all_data)

        # Save the extracted data to categorized JSON files
        self._save_categorized_data(all_data)

        self.stdout.write(self.style.SUCCESS("\nExtraction complete!"))

    def _load_cached_2da(self, filename: str) -> Optional[Dict[str, Any]]:
        """Loads a single cached .2da file from the cache directory."""
        filepath = os.path.join(self.cache_dir, f'{filename}.2da.msgpack')
        if not os.path.exists(filepath):
            self.stdout.write(self.style.WARNING(f"Warning: Cache file not found: {filepath}"))
            return None
        
        return SafeCache.load(filepath.with_suffix(''))

    def _load_tlk(self):
        """Loads the TLK file for string lookups."""
        self.stdout.write(f"Loading TLK file from: {self.tlk_path}")
        try:
            tlk = TLKParser()
            tlk.read(self.tlk_path)
            return tlk
        except FileNotFoundError:
            return None

    def _get_string_from_ref(self, str_ref: str) -> Optional[str]:
        """Gets a localized string from a string reference using the loaded TLK."""
        if not str_ref or str_ref == '****':
            return None
        
        try:
            ref = int(str_ref)
            return self.tlk.get_string(ref)
        except (ValueError, TypeError):
            return str_ref

    def _extract_appearance_types(self):
        """Extracts base appearance types (races, creatures, etc.)."""
        appearance_table = self._load_cached_2da('appearance')
        if not appearance_table:
            return {}
        
        appearance_types = {'player_races': {}, 'creatures': {}, 'npcs': {}, 'other': {}}
        count = appearance_table.get_resource_count()
        self.stdout.write(f"Processing appearance.2da with {count} entries...")
        
        for i in range(count):
            appearance_id = i
            data = {
                'id': appearance_id,
                'label': appearance_table.get_string(i, 'LABEL') or appearance_table.get_string(i, 0),
                'string_ref': appearance_table.get_string(i, 'STRING_REF'),
                'race': appearance_table.get_string(i, 'RACE'),
                'racialtype': appearance_table.get_string(i, 'RACIALTYPE'),
                'model_type': appearance_table.get_string(i, 'NWN2_Model_Type'),
            }
            data['name'] = self._get_string_from_ref(data['string_ref']) or data['label']
            
            # Categorize based on index, model type, and racial type
            if i < 7:  # First 7 are player races
                appearance_types['player_races'][str(appearance_id)] = data
            elif data['model_type'] in ['F', 'S']:  # Full body or simple model
                if data['racialtype'] in ['1', '2', '3', '4', '5', '6', '7']:  # PC racial types
                    appearance_types['npcs'][str(appearance_id)] = data
                else:
                    appearance_types['creatures'][str(appearance_id)] = data
            else:
                appearance_types['other'][str(appearance_id)] = data
        
        return appearance_types

    def _extract_body_parts(self):
        """Extracts all body part variations with armor type context."""
        body_parts = {
            'chest': [], 'legs': [], 'belt': [], 'foot': [], 'hand': [], 'neck': [],
            'pelvis': [], 'bicep': [], 'forearm': [], 'shin': [], 'shoulder': [], 'robe': []
        }
        
        # Load armor types for context
        armor_types = {}
        armor_table = self._load_cached_2da('armor')
        if armor_table:
            self.stdout.write("Processing armor.2da for armor type names...")
            for i in range(armor_table.get_resource_count()):
                name_ref = armor_table.get_string(i, 'Name')
                toolset_name_ref = armor_table.get_string(i, 'ToolsetName')
                label = armor_table.get_string(i, 'Label') or armor_table.get_string(i, 0)
                name = self._get_string_from_ref(toolset_name_ref) or self._get_string_from_ref(name_ref) or label
                armor_types[i] = {'id': i, 'label': label, 'name': name}

        # Load armorvisualdata for additional context
        armorvisual_names = {}
        armorvisual_table = self._load_cached_2da('armorvisualdata')
        if armorvisual_table:
            self.stdout.write("Processing armorvisualdata.2da for visual names...")
            for i in range(armorvisual_table.get_resource_count()):
                label = armorvisual_table.get_string(i, 'Label') or armorvisual_table.get_string(i, 0)
                toolset_name_ref = armorvisual_table.get_string(i, 'ToolsetName')
                prefix = armorvisual_table.get_string(i, 'Prefix')
                name = self._get_string_from_ref(toolset_name_ref) or label
                armorvisual_names[i] = {'label': label, 'name': name, 'prefix': prefix}
        
        part_files = list(body_parts.keys())
        for part in part_files:
            parts_table = self._load_cached_2da(f'parts_{part}')
            if not parts_table:
                continue
            
            count = parts_table.get_resource_count()
            self.stdout.write(f"Processing parts_{part}.2da with {count} entries...")
            
            for i in range(count):
                ac_bonus = parts_table.get_string(i, 'ACBONUS') or '0'
                cost_mod = parts_table.get_string(i, 'COSTMODIFIER') or '1.0'
                entry = {
                    'id': i,
                    'ac_bonus': float(ac_bonus) if ac_bonus and ac_bonus != '****' else 0.0,
                    'cost_modifier': float(cost_mod) if cost_mod and cost_mod != '****' else 1.0,
                    'part_type': part,
                    'name': f"{part.capitalize()} {i}",
                }
                
                if part in ['chest', 'legs', 'bicep', 'forearm', 'shin', 'shoulder'] and i in armorvisual_names:
                    entry['body_model_type'] = armorvisual_names[i]['name']
                
                body_parts[part].append(entry)
        
        body_parts['body_model_types'] = list(armor_types.values())
        body_parts['visual_armor_categories'] = list(armorvisual_names.values())
        
        return body_parts

    def _extract_colors(self):
        """Extracts all color data organized by race and type."""
        colors = {'race_palettes': {}, 'master_colors': {}, 'item_colors': {}}
        race_color_files = [
            'human', 'dwarf', 'elf', 'gnome', 'halfling', 'halforc', 'halfelf',
            'aasimar', 'tiefling', 'drow', 'yuanti', 'grayorc', 'graydwarf',
            'golddwarf', 'wildelf', 'woodelf', 'sunelf', 'moonelf', 'halfdrow',
            'shielddwarf', 'deepgnome', 'rockgnome', 'strongheart', 'lightfoot',
            'ghostwise', 'earthgen', 'airgen', 'watergen', 'firegen'
        ]
        
        self.stdout.write("Processing race color palettes...")
        for race in race_color_files:
            color_table = self._load_cached_2da(f'color_{race}')
            if not color_table:
                continue
            
            race_colors = []
            for i in range(color_table.get_resource_count()):
                race_colors.append({
                    'id': i,
                    'hair': color_table.get_string(i, 'hair_2') or '',
                    'hair_accessory': color_table.get_string(i, 'hair_acc') or '',
                    'skin': color_table.get_string(i, 'skin') or '',
                    'eyes': color_table.get_string(i, 'eyes') or '',
                    'body_hair': color_table.get_string(i, 'body_hair') or '',
                })
            colors['race_palettes'][race] = race_colors
        
        master_colors_table = self._load_cached_2da('nwn2_colors')
        if master_colors_table:
            self.stdout.write("Processing nwn2_colors.2da...")
            for i in range(master_colors_table.get_resource_count()):
                color_name = master_colors_table.get_string(i, 'LABEL') or master_colors_table.get_string(i, 0)
                if color_name:
                    colors['master_colors'][str(i)] = {'id': i, 'name': color_name}
        
        iprp_colors_table = self._load_cached_2da('iprp_color')
        if iprp_colors_table:
            self.stdout.write("Processing iprp_color.2da...")
            for i in range(iprp_colors_table.get_resource_count()):
                str_ref = iprp_colors_table.get_string(i, 'Name')
                name = self._get_string_from_ref(str_ref) or f"Color {i}"
                colors['item_colors'][str(i)] = {'id': i, 'name': name}
                
        return colors

    def _extract_portraits(self):
        """Extracts portrait data from portraits.2da."""
        portraits_table = self._load_cached_2da('portraits')
        if not portraits_table:
            return {}
        
        portraits = {'player': {}, 'npc': {}, 'custom': {}}
        count = portraits_table.get_resource_count()
        self.stdout.write(f"Processing portraits.2da with {count} entries...")
        
        for i in range(count):
            base_resref = portraits_table.get_string(i, 'BaseResRef')
            if not base_resref or base_resref == '****':
                continue

            sex = portraits_table.get_string(i, 'Sex')
            race = portraits_table.get_string(i, 'Race')
            data = {
                'id': i,
                'filename': base_resref,
                'sex': sex if sex != '****' else None,
                'race': race if race != '****' else None
            }
            
            # Generate a clean display name from the filename
            display_name = base_resref.replace('_', ' ').strip().title()
            data['name'] = display_name
            
            if base_resref.startswith('po_'):
                portraits['player'][i] = data
            elif base_resref.startswith('n_'):
                portraits['npc'][i] = data
            else:
                portraits['custom'][i] = data
                
        return portraits

    def _extract_soundsets(self):
        """Extracts soundset data."""
        soundset_table = self._load_cached_2da('soundset') or self._load_cached_2da('soundsets')
        if not soundset_table:
            self.stdout.write(self.style.WARNING("Warning: soundset.2da not found."))
            return {}
        
        soundsets = {'male': [], 'female': [], 'other': []}
        count = soundset_table.get_resource_count()
        self.stdout.write(f"Processing soundset.2da with {count} entries...")

        for i in range(count):
            label = soundset_table.get_string(i, 'LABEL') or soundset_table.get_string(i, 0)
            if not label or label == '****':
                continue

            str_ref = soundset_table.get_string(i, 'STRREF')
            name = self._get_string_from_ref(str_ref) or label
            resref = soundset_table.get_string(i, 'RESREF')
            type_col = soundset_table.get_string(i, 'TYPE')

            data = {
                'id': i,
                'label': label,
                'resref': resref if resref != '****' else None,
                'name': name,
                'type': type_col if type_col and type_col != '****' else None
            }
            
            label_lower = (label or '').lower()
            name_lower = (name or '').lower()
            if 'female' in name_lower or '_f_' in label_lower:
                soundsets['female'].append(data)
            elif 'male' in name_lower or '_m_' in label_lower:
                soundsets['male'].append(data)
            else:
                soundsets['other'].append(data)
                
        return soundsets

    def _extract_gender_data(self):
        """Extracts gender data from gender.2da."""
        gender_table = self._load_cached_2da('gender')
        if not gender_table:
            return []
        
        count = gender_table.get_resource_count()
        self.stdout.write(f"Processing gender.2da with {count} entries...")
        
        genders = []
        for i in range(count):
            name_ref = gender_table.get_string(i, 'NAME')
            gender_code = gender_table.get_string(i, 'GENDER')
            constant = gender_table.get_string(i, 'CONSTANT')
            
            name = self._get_string_from_ref(name_ref)
            if not name:
                if constant and constant != '****':
                    name = constant.replace('GENDER_', '').replace('_', ' ').title()
                elif gender_code:
                    code_map = {'M': 'Male', 'F': 'Female', 'B': 'Both', 'O': 'Other', 'N': 'None'}
                    name = code_map.get(gender_code, f'Gender {i}')
            
            genders.append({
                'id': i,
                'name': name,
                'code': gender_code if gender_code != '****' else None,
                'constant': constant if constant and constant != '****' else None
            })
        return genders
    
    def _print_summary(self, all_data: Dict[str, Any]):
        """Prints a summary of the extracted data counts."""
        self.stdout.write(self.style.SUCCESS("\n=== Extraction Summary ==="))
        for category, data in all_data.items():
            if isinstance(data, dict):
                self.stdout.write(f"\n{category}:")
                for subcat, items in data.items():
                    if isinstance(items, (dict, list)):
                        self.stdout.write(f"  {subcat}: {len(items)} entries")
            elif isinstance(data, list):
                 self.stdout.write(f"\n{category}: {len(data)} entries")

    def _save_categorized_data(self, all_data: Dict[str, Any]):
        """Saves all extracted data into a structured directory of JSON files."""
        self.stdout.write("\nSaving categorized data...")
        os.makedirs(self.output_dir, exist_ok=True)
        
        subdirs = ['body_parts', 'colors', 'portraits', 'base_data']
        for subdir in subdirs:
            os.makedirs(os.path.join(self.output_dir, subdir), exist_ok=True)

        # Save body parts data
        if 'body_parts' in all_data:
            body_parts_data = all_data['body_parts']
            for part_type, parts in body_parts_data.items():
                if isinstance(parts, list) and part_type not in ['body_model_types', 'visual_armor_categories']:
                    path = os.path.join(self.output_dir, 'body_parts', f'{part_type}.json')
                    with open(path, 'w') as f:
                        json.dump({part_type: parts}, f, indent=2)
            ref_data = {
                'body_model_types': body_parts_data.get('body_model_types', []),
                'visual_armor_categories': body_parts_data.get('visual_armor_categories', [])
            }
            path = os.path.join(self.output_dir, 'body_parts', 'reference.json')
            with open(path, 'w') as f:
                json.dump(ref_data, f, indent=2)
            self.stdout.write(self.style.HTTP_INFO(f"Saved body parts data to {os.path.join(self.output_dir, 'body_parts')}"))

        # Save color data
        if 'colors' in all_data:
            colors_data = all_data['colors']
            if 'race_palettes' in colors_data:
                for race, palette in colors_data['race_palettes'].items():
                    path = os.path.join(self.output_dir, 'colors', f'{race}_colors.json')
                    with open(path, 'w') as f:
                        json.dump({race: palette}, f, indent=2)
            other_colors = {
                'master_colors': colors_data.get('master_colors', {}),
                'item_colors': colors_data.get('item_colors', {})
            }
            path = os.path.join(self.output_dir, 'colors', 'reference.json')
            with open(path, 'w') as f:
                json.dump(other_colors, f, indent=2)
            self.stdout.write(self.style.HTTP_INFO(f"Saved color data to {os.path.join(self.output_dir, 'colors')}"))

        # Save portrait data in batches
        if 'portraits' in all_data:
            batch_size = 100
            for category, portraits in all_data['portraits'].items():
                if isinstance(portraits, dict):
                    portrait_list = list(portraits.values())
                    for i in range(0, len(portrait_list), batch_size):
                        batch = portrait_list[i:i + batch_size]
                        batch_num = i // batch_size
                        path = os.path.join(self.output_dir, 'portraits', f'{category}_batch_{batch_num}.json')
                        with open(path, 'w') as f:
                            json.dump({'category': category, 'batch': batch_num, 'portraits': batch}, f, indent=2)
            self.stdout.write(self.style.HTTP_INFO(f"Saved portrait data to {os.path.join(self.output_dir, 'portraits')}"))

        # Save other base data
        base_data_categories = ['appearance_types', 'soundsets', 'genders']
        for category in base_data_categories:
            if category in all_data:
                path = os.path.join(self.output_dir, 'base_data', f'{category}.json')
                with open(path, 'w') as f:
                    json.dump(all_data[category], f, indent=2)
        self.stdout.write(self.style.HTTP_INFO(f"Saved base data to {os.path.join(self.output_dir, 'base_data')}"))
        
        # Finally, create an index file for easy navigation of the data
        self._create_index_file(all_data)


    def _create_index_file(self, all_data: Dict[str, Any]):
        """Creates an index.json file that maps out the data structure and provides stats."""
        index = {'structure': {}, 'statistics': {}}
        
        # Define the structure for the index file
        if 'body_parts' in all_data:
            index['structure']['body_parts'] = {
                'directory': 'body_parts/',
                'files': [f'{part}.json' for part in all_data['body_parts'] if isinstance(all_data['body_parts'][part], list)],
                'reference': 'body_parts/reference.json'
            }
        if 'colors' in all_data:
            index['structure']['colors'] = {
                'directory': 'colors/',
                'race_files': [f'{race}_colors.json' for race in all_data.get('colors', {}).get('race_palettes', {})],
                'reference': 'colors/reference.json'
            }
        if 'portraits' in all_data:
            index['structure']['portraits'] = {
                'directory': 'portraits/',
                'description': 'Split into batches of 100 entries.'
            }
        index['structure']['base_data'] = {
            'directory': 'base_data/',
            'files': [f'{cat}.json' for cat in ['appearance_types', 'soundsets', 'genders'] if cat in all_data]
        }
        
        # Gather statistics
        for category, data in all_data.items():
            if isinstance(data, dict):
                index['statistics'][category] = {
                    'total_entries': sum(len(v) if isinstance(v, (list, dict)) else 0 for v in data.values()),
                    'subcategories': list(data.keys())
                }
            elif isinstance(data, list):
                index['statistics'][category] = {'total_entries': len(data)}
        
        index_path = os.path.join(self.output_dir, 'index.json')
        with open(index_path, 'w') as f:
            json.dump(index, f, indent=2)
        self.stdout.write(self.style.SUCCESS(f"\nSaved index file to {index_path}"))