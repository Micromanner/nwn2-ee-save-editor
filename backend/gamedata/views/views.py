from rest_framework import viewsets
from rest_framework.decorators import action
from rest_framework.response import Response
from rest_framework.views import APIView
from rest_framework.pagination import PageNumberPagination
from django.core.cache import cache
from django.conf import settings
import os
import json

from parsers.resource_manager import ResourceManager
from gamedata.middleware import get_resource_manager, get_game_rules_service, set_module_for_session, get_current_module
from config.nwn2_settings import nwn2_paths
from pathlib import Path


class GameDataViewSet(viewsets.ViewSet):
    """
    Completely data-driven API endpoints for game data.
    
    This viewset automatically adapts to any 2DA structure without hardcoding
    attribute names, making it compatible with all mods and custom content.
    """
    
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._rm = None
        self._grs = None
    
    @property
    def rm(self):
        """Get resource manager from middleware or create default"""
        rm = get_resource_manager()
        if rm:
            return rm
        # Fallback to creating one if middleware not active
        if self._rm is None:
            self._rm = ResourceManager(suppress_warnings=True)
        return self._rm
    
    @property
    def grs(self):
        """Get GameRulesService from middleware or create default"""
        grs = get_game_rules_service()
        if grs:
            return grs
        # Fallback to using ResourceManager-based approach
        if self._grs is None:
            from gamedata.services.game_rules_service import GameRulesService
            self._grs = GameRulesService(self.rm, load_mode='priority_only')
        return self._grs
    
    def _get_cached_or_load(self, cache_key, loader_func):
        """Get data from cache or load it"""
        data = cache.get(cache_key)
        if data is None:
            data = loader_func()
            cache.set(cache_key, data, 3600)  # Cache for 1 hour
        return data
    
    def _get_module_aware_cache_key(self, base_key):
        """Get cache key that includes module context"""
        module_path = get_current_module() or 'base'
        return f'{base_key}_{module_path}'
    
    def _convert_data_object_to_dict(self, data_obj, table_name=None, row_index=None):
        """
        Convert a dynamic data object to a dictionary using its introspection methods.
        This works with any 2DA structure without hardcoding field names.
        """
        if not data_obj:
            return {}
        
        # Get the raw data using the to_dict method
        if hasattr(data_obj, 'to_dict'):
            raw_dict = data_obj.to_dict(use_original_names=True)
        else:
            # Fallback: use available attributes
            raw_dict = {}
            for attr in dir(data_obj):
                if not attr.startswith('_') and not callable(getattr(data_obj, attr)):
                    raw_dict[attr] = getattr(data_obj, attr)
        
        # Create standardized output with row index as ID
        id_value = self._get_row_id(data_obj, raw_dict)
        # Use row_index if provided and no valid ID was found
        if id_value == 0 and row_index is not None:
            id_value = row_index
        
        result = {
            'id': id_value,
            'raw_data': raw_dict
        }
        
        # Add commonly useful fields if they exist (but don't require them)
        name_fields = ['Name', 'Label', 'name', 'label']
        for field in name_fields:
            if field in raw_dict and raw_dict[field] and str(raw_dict[field]).lower() != '****':
                result['name'] = raw_dict[field]
                break
        
        # Add description if available
        desc_fields = ['Description', 'description', 'Desc']
        for field in desc_fields:
            if field in raw_dict and raw_dict[field]:
                result['description'] = raw_dict[field]
                break
        
        # Add icon if available
        icon_fields = ['Icon', 'icon', 'IconResRef', 'iconresref']
        for field in icon_fields:
            if field in raw_dict and raw_dict[field] and str(raw_dict[field]).lower() != '****':
                result['icon'] = raw_dict[field]
                # Generate enhanced icon URL
                result['icon_url'] = f"/api/gamedata/icons/{raw_dict[field]}/"
                break
        
        # Add table-specific processing
        if table_name == 'spells':
            result = self._process_spell_data(result, raw_dict, data_obj)
        
        return result
    
    def _process_spell_data(self, result, raw_dict, data_obj):
        """
        Process spell-specific data to extract class levels, school, and metadata
        """
        # Define class-specific spell level columns from spells.2da
        class_columns = {
            'bard': 'Bard',
            'cleric': 'Cleric', 
            'druid': 'Druid',
            'paladin': 'Paladin',
            'ranger': 'Ranger',
            'wizard': 'Wiz_Sorc',  # Wizard and Sorcerer share this column
            'sorcerer': 'Wiz_Sorc',
            'warlock': 'Warlock',
            'innate': 'Innate'  # For innate abilities
        }
        
        # Extract class-specific spell levels
        spell_levels = {}
        for class_name, column_name in class_columns.items():
            level_value = raw_dict.get(column_name)
            if level_value is not None and level_value != '****' and level_value != '':
                try:
                    level_int = int(level_value)
                    # -1 means not available for this class, 0-9 are valid levels
                    if level_int >= 0:
                        spell_levels[class_name] = level_int
                except (ValueError, TypeError):
                    pass
        
        result['spell_levels'] = spell_levels
        
        # Determine the "innate level" - use the lowest available spell level
        # This fixes the frontend issue where everything was treated as cantrip
        if spell_levels:
            result['innate_level'] = min(spell_levels.values())
        else:
            result['innate_level'] = None  # Not available to any standard class
        
        # Extract spell school
        school_id = raw_dict.get('School')
        if school_id is not None and school_id != '****' and school_id != '':
            try:
                school_int = int(school_id)
                # TODO: Look up school names from spellschools.2da if needed
                result['school_id'] = school_int
            except (ValueError, TypeError):
                pass
        
        # Extract other spell metadata
        spell_metadata = {}
        
        # Range
        range_value = raw_dict.get('Range')
        if range_value is not None and range_value != '****':
            spell_metadata['range'] = range_value
        
        # Metamagic flags
        metamagic = raw_dict.get('MetaMagic')
        if metamagic is not None and metamagic != '****':
            spell_metadata['metamagic'] = metamagic
        
        # Target type
        target_type = raw_dict.get('TargetType')
        if target_type is not None and target_type != '****':
            spell_metadata['target_type'] = target_type
        
        # Casting time
        cast_time = raw_dict.get('CastTime')
        if cast_time is not None and cast_time != '****':
            spell_metadata['cast_time'] = cast_time
        
        # Conjuration time
        conj_time = raw_dict.get('ConjTime')
        if conj_time is not None and conj_time != '****':
            spell_metadata['conjuration_time'] = conj_time
        
        # Verbal/Somatic components
        vs_components = raw_dict.get('VS')
        if vs_components is not None and vs_components != '****':
            spell_metadata['components'] = vs_components
        
        # Spell description (may be a string reference)
        spell_desc = raw_dict.get('SpellDesc')
        if spell_desc is not None and spell_desc != '****':
            spell_metadata['spell_desc'] = spell_desc
        
        # Add metadata to result
        if spell_metadata:
            result['spell_metadata'] = spell_metadata
        
        return result
    
    def _get_row_id(self, data_obj, raw_dict):
        """Extract a row ID from the data object or raw dictionary"""
        # Try common ID fields
        id_fields = ['id', 'ID', 'Index', 'index', 'Row', 'row']
        for field in id_fields:
            if hasattr(data_obj, field):
                return getattr(data_obj, field)
            if field in raw_dict:
                return raw_dict[field]
        
        # If no ID field found, try to find it in the raw data by looking for numeric first column
        if raw_dict:
            first_key = next(iter(raw_dict))
            first_value = raw_dict[first_key]
            if isinstance(first_value, (int, str)) and str(first_value).isdigit():
                return int(first_value)
        
        # Last resort: return 0
        return 0
    
    def _filter_valid_entries(self, data_list, table_name=None):
        """Filter out invalid entries (like **** names) from data"""
        valid_entries = []
        
        for item in data_list:
            # Skip if no name or invalid name
            if 'name' not in item or not item['name'] or str(item['name']).lower() in ['****', 'padding', '']:
                continue
            
            valid_entries.append(item)
        
        return valid_entries
    
    def _get_table_data(self, table_name, search=None, limit=None, offset=0):
        """
        Internal method to get table data with filtering and pagination.
        Returns a dict with 'data', 'count', and 'table_name' keys, or an error dict.
        """
        def load_table_data():
            try:
                table_data = self.grs.get_table(table_name)
                if not table_data:
                    return []
                
                # Convert all objects to standardized dictionaries
                converted_data = []
                for row_index, data_obj in enumerate(table_data):
                    converted = self._convert_data_object_to_dict(data_obj, table_name, row_index)
                    converted_data.append(converted)
                
                # Filter out invalid entries
                valid_data = self._filter_valid_entries(converted_data, table_name)
                
                return valid_data
                
            except Exception as e:
                return {'error': f'Failed to load table {table_name}: {str(e)}'}
        
        cache_key = self._get_module_aware_cache_key(f'gamedata_table_{table_name}')
        data = self._get_cached_or_load(cache_key, load_table_data)
        
        # Handle errors
        if isinstance(data, dict) and 'error' in data:
            return data
        
        # Apply filters
        if search:
            search = search.lower()
            data = [item for item in data if search in item.get('name', '').lower()]
        
        # Apply pagination
        try:
            offset = int(offset) if offset is not None else 0
            if limit is not None:
                limit = int(limit)
                data = data[offset:offset + limit]
            else:
                data = data[offset:]
        except (ValueError, TypeError):
            pass
        
        return {
            'table_name': table_name,
            'data': data,
            'count': len(data)
        }

    @action(detail=False, methods=['get'])
    def table(self, request):
        """
        Get data from any 2DA table dynamically.
        Usage: /api/gamedata/table/?name=classes&limit=10&search=fighter
        """
        # Handle both DRF and Django request objects
        params = getattr(request, 'query_params', request.GET)
        table_name = params.get('name')
        if not table_name:
            return Response({'error': 'table name is required'}, status=400)
        
        # Get parameters
        search = params.get('search')
        limit = params.get('limit')
        offset = params.get('offset', 0)
        
        # Get table data
        result = self._get_table_data(table_name, search, limit, offset)
        
        # Return appropriate response
        if 'error' in result:
            return Response(result, status=404)
        
        return Response(result)
    
    @action(detail=False, methods=['get'])
    def tables(self, request):
        """Get list of available tables"""
        try:
            # Get available tables from the loader
            if hasattr(self.grs, 'table_data') and self.grs.table_data:
                available_tables = list(self.grs.table_data.keys())
            else:
                # Fallback to common tables
                available_tables = ['classes', 'racialtypes', 'feat', 'skills', 'spells', 
                                  'baseitems', 'gender', 'appearance']
            
            table_info = []
            for table_name in available_tables:
                try:
                    table_data = self.grs.get_table(table_name)
                    count = len(table_data) if table_data else 0
                    
                    # Get column information if available
                    columns = []
                    if table_data and len(table_data) > 0:
                        first_item = table_data[0]
                        if hasattr(first_item, 'get_column_mapping'):
                            columns = list(first_item.get_column_mapping().keys())
                        elif hasattr(first_item, '__slots__'):
                            columns = [slot[1:] for slot in first_item.__slots__ if slot != '_resource_manager']
                    
                    table_info.append({
                        'name': table_name,
                        'count': count,
                        'columns': columns[:10]  # Limit to first 10 columns for overview
                    })
                except Exception as e:
                    table_info.append({
                        'name': table_name,
                        'count': 0,
                        'error': str(e)
                    })
            
            return Response({
                'tables': table_info,
                'total_tables': len(table_info)
            })
            
        except Exception as e:
            return Response({'error': f'Failed to get table list: {str(e)}'}, status=500)
    
    @action(detail=False, methods=['get'])
    def schema(self, request):
        """Get schema information for a specific table"""
        params = getattr(request, 'query_params', request.GET)
        table_name = params.get('name')
        if not table_name:
            return Response({'error': 'table name is required'}, status=400)
        
        try:
            table_data = self.grs.get_table(table_name)
            if not table_data or len(table_data) == 0:
                return Response({'error': f'Table {table_name} not found or empty'}, status=404)
            
            # Get schema from first item
            first_item = table_data[0]
            schema_info = {
                'table_name': table_name,
                'row_count': len(table_data),
                'columns': []
            }
            
            # Get column mapping if available
            if hasattr(first_item, 'get_column_mapping'):
                column_mapping = first_item.get_column_mapping()
                for original_name, safe_name in column_mapping.items():
                    # Try to get a sample value
                    sample_value = getattr(first_item, safe_name, None)
                    schema_info['columns'].append({
                        'original_name': original_name,
                        'safe_name': safe_name,
                        'sample_value': sample_value,
                        'type': type(sample_value).__name__ if sample_value is not None else 'unknown'
                    })
            else:
                # Fallback to using available attributes
                for attr in dir(first_item):
                    if not attr.startswith('_') and not callable(getattr(first_item, attr)):
                        sample_value = getattr(first_item, attr)
                        schema_info['columns'].append({
                            'original_name': attr,
                            'safe_name': attr,
                            'sample_value': sample_value,
                            'type': type(sample_value).__name__ if sample_value is not None else 'unknown'
                        })
            
            return Response(schema_info)
            
        except Exception as e:
            return Response({'error': f'Failed to get schema for {table_name}: {str(e)}'}, status=500)
    
    # Convenience methods for common tables (backwards compatibility)
    @action(detail=False, methods=['get'])
    def races(self, request):
        """Get races - convenience wrapper for table endpoint"""
        # Handle both DRF and Django request objects
        params = getattr(request, 'query_params', request.GET)
        
        # Get parameters
        search = params.get('search')
        limit = params.get('limit')
        offset = params.get('offset', 0)
        
        # Get table data
        result = self._get_table_data('racialtypes', search, limit, offset)
        
        # Return appropriate response
        if 'error' in result:
            return Response(result, status=404)
        
        return Response(result)
    
    @action(detail=False, methods=['get'])
    def classes(self, request):
        """Get classes - convenience wrapper for table endpoint"""
        # Handle both DRF and Django request objects
        params = getattr(request, 'query_params', request.GET)
        
        # Get parameters
        search = params.get('search')
        limit = params.get('limit')
        offset = params.get('offset', 0)
        playable_only = params.get('playable_only', '').lower() == 'true'
        
        # Get table data
        result = self._get_table_data('classes', search, limit, offset)
        
        # Handle errors
        if 'error' in result:
            return Response(result, status=404)
        
        # Apply playable filter if requested
        if playable_only and 'data' in result:
            filtered_data = []
            for item in result['data']:
                raw_data = item.get('raw_data', {})
                playable = raw_data.get('PlayerClass', raw_data.get('playerclass', 0))
                if playable == 1 or playable == '1':
                    filtered_data.append(item)
            result['data'] = filtered_data
            result['count'] = len(filtered_data)
        
        return Response(result)
    
    @action(detail=False, methods=['get'])
    def feats(self, request):
        """Get feats - convenience wrapper for table endpoint"""
        # Handle both DRF and Django request objects
        params = getattr(request, 'query_params', request.GET)
        
        # Get parameters
        search = params.get('search')
        limit = params.get('limit')
        offset = params.get('offset', 0)
        
        # Get table data
        result = self._get_table_data('feat', search, limit, offset)
        
        # Return appropriate response
        if 'error' in result:
            return Response(result, status=404)
        
        return Response(result)
    
    @action(detail=False, methods=['get'])
    def skills(self, request):
        """Get skills - convenience wrapper for table endpoint"""
        # Handle both DRF and Django request objects
        params = getattr(request, 'query_params', request.GET)
        
        # Get parameters
        search = params.get('search')
        limit = params.get('limit')
        offset = params.get('offset', 0)
        
        # Get table data
        result = self._get_table_data('skills', search, limit, offset)
        
        # Return appropriate response
        if 'error' in result:
            return Response(result, status=404)
        
        return Response(result)
    
    @action(detail=False, methods=['get'])
    def spells(self, request):
        """Get spells - convenience wrapper for table endpoint"""
        # Handle both DRF and Django request objects
        params = getattr(request, 'query_params', request.GET)
        
        # Get parameters
        search = params.get('search')
        limit = params.get('limit')
        offset = params.get('offset', 0)
        
        # Get table data
        result = self._get_table_data('spells', search, limit, offset)
        
        # Return appropriate response
        if 'error' in result:
            return Response(result, status=404)
        
        return Response(result)
    
    @action(detail=False, methods=['get'])
    def base_items(self, request):
        """Get base items - convenience wrapper for table endpoint"""
        # Handle both DRF and Django request objects
        params = getattr(request, 'query_params', request.GET)
        
        # Get parameters
        search = params.get('search')
        limit = params.get('limit')
        offset = params.get('offset', 0)
        
        # Get table data
        result = self._get_table_data('baseitems', search, limit, offset)
        
        # Return appropriate response
        if 'error' in result:
            return Response(result, status=404)
        
        return Response(result)
    
    @action(detail=False, methods=['get'])
    def test_data_loading(self, request):
        """Test endpoint to verify data loading works"""
        try:
            result = {
                'grs_available': self.grs is not None,
                'rm_available': self.rm is not None,
                'available_tables': []
            }
            
            # Test getting a simple table
            try:
                if hasattr(self.grs, 'table_data'):
                    result['available_tables'] = list(self.grs.table_data.keys())
                
                # Test loading gender table
                gender_table = self.grs.get_table('gender')
                result['gender_table'] = {
                    'count': len(gender_table) if gender_table else 0,
                    'sample_data': []
                }
                
                if gender_table and len(gender_table) > 0:
                    # Convert first few entries
                    for i, item in enumerate(gender_table[:3]):
                        converted = self._convert_data_object_to_dict(item, 'gender', i)
                        result['gender_table']['sample_data'].append(converted)
                
            except Exception as e:
                result['error'] = str(e)
                import traceback
                result['traceback'] = traceback.format_exc()
            
            return Response(result)
            
        except Exception as e:
            return Response({
                'error': str(e),
                'type': type(e).__name__
            }, status=500)


# Keep other views that don't deal with 2DA data unchanged
class HotReloadView(APIView):
    """API endpoint to control hot reload functionality in development"""
    
    def get(self, request):
        """Get hot reload status"""
        if not settings.DEBUG:
            return Response(
                {'error': 'Hot reload is only available in DEBUG mode'}, 
                status=403
            )
        
        return Response({
            'enabled': False,
            'debug_mode': settings.DEBUG,
            'message': 'Hot reload functionality not implemented'
        })


class CustomOverrideDirectoriesView(APIView):
    """API endpoint to manage custom override directories"""
    
    def get(self, request):
        """Get current custom override directories"""
        rm = get_resource_manager()
        if not rm:
            return Response(
                {'error': 'Resource manager not initialized'}, 
                status=500
            )
        
        directories = rm.get_custom_override_directories() if hasattr(rm, 'get_custom_override_directories') else []
        
        # Get details about each directory
        directory_info = []
        for dir_path in directories:
            path = Path(dir_path)
            if path.exists():
                tda_count = len(list(path.glob('**/*.2da')))
                directory_info.append({
                    'path': str(path),
                    'exists': True,
                    'tda_file_count': tda_count
                })
            else:
                directory_info.append({
                    'path': str(path),
                    'exists': False,
                    'tda_file_count': 0
                })
        
        return Response({
            'directories': directory_info,
            'total_count': len(directory_info)
        })


class WorkshopModsView(APIView):
    """API view for Steam Workshop mod management"""
    
    def get(self, request):
        """Get all installed workshop mods"""
        rm = get_resource_manager()
        if not rm:
            return Response(
                {'error': 'Resource manager not initialized'}, 
                status=500
            )
        
        # Placeholder implementation
        return Response({
            'mods': [],
            'count': 0,
            'message': 'Workshop mod integration not yet implemented'
        })


class CacheStatsView(APIView):
    """View for memory cache statistics"""
    
    def get(self, request):
        """Get current cache statistics"""
        rm = get_resource_manager()
        if not rm:
            return Response(
                {"error": "ResourceManager not available"},
                status=500
            )
        
        try:
            stats = rm.get_cache_stats() if hasattr(rm, 'get_cache_stats') else {}
            return Response(stats)
        except Exception as e:
            return Response(
                {"error": str(e)},
                status=500
            )