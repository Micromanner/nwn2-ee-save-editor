"""
Gamedata endpoints router - NWN2 paths, game configuration
"""

import logging
from fastapi import APIRouter, HTTPException, status
from pathlib import Path

from config.nwn2_settings import nwn2_paths
# from fastapi_models import (...) - moved to lazy loading

logger = logging.getLogger(__name__)

router = APIRouter()


@router.get("/paths")
def get_nwn2_paths():
    """Get NWN2 installation paths"""
    from fastapi_models import PathInfo, CustomFolderInfo, PathConfig, NWN2PathsResponse
    try:
        # Helper to create PathInfo objects
        def create_path_info(path, auto_detected=True) -> PathInfo:
            if not path:
                return PathInfo(path=None, exists=False, auto_detected=False)
            path_obj = Path(path)
            return PathInfo(
                path=str(path_obj),
                exists=path_obj.exists(),
                auto_detected=auto_detected
            )
        
        # Helper to create CustomFolderInfo objects
        def create_custom_folder_info(path) -> CustomFolderInfo:
            path_obj = Path(path)
            return CustomFolderInfo(
                path=str(path_obj),
                exists=path_obj.exists()
            )
        
        # Get custom folders
        custom_override_folders = []
        custom_hak_folders = []
        
        if hasattr(nwn2_paths, 'custom_override_folders'):
            for folder in nwn2_paths.custom_override_folders:
                custom_override_folders.append(create_custom_folder_info(folder))
        
        if hasattr(nwn2_paths, 'custom_hak_folders'):
            for folder in nwn2_paths.custom_hak_folders:
                custom_hak_folders.append(create_custom_folder_info(folder))
        
        # Create path config
        path_config = PathConfig(
            game_folder=create_path_info(nwn2_paths.game_folder),
            documents_folder=create_path_info(nwn2_paths.user_folder),
            steam_workshop_folder=PathInfo(path=None, exists=False, auto_detected=False),  # Not available
            custom_override_folders=custom_override_folders,
            custom_module_folders=[],  # Not tracking custom module folders yet
            custom_hak_folders=custom_hak_folders
        )
        
        return NWN2PathsResponse(paths=path_config)
        
    except Exception as e:
        logger.error(f"Failed to get NWN2 paths: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get NWN2 paths: {str(e)}"
        )


@router.get("/config")
def get_gamedata_config():
    """Get gamedata configuration"""
    from fastapi_models import GameDataConfigResponse
    try:
        # Use path string conversion helper - no duplicated logic
        def _path_str(path):
            return str(path) if path else ""
        
        return GameDataConfigResponse(
            nwn2_install_path=_path_str(nwn2_paths.game_folder),
            nwn2_user_path=_path_str(nwn2_paths.user_folder),
            saves_path=_path_str(nwn2_paths.saves),
            data_path=_path_str(nwn2_paths.data),
            dialog_tlk_path=_path_str(nwn2_paths.dialog_tlk)
        )
        
    except Exception as e:
        logger.error(f"Failed to get gamedata config: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get gamedata config: {str(e)}"
        )


@router.get("/table/")
def get_table_data(name: str, search: str = None, limit: int = None, offset: int = 0):
    """Dynamic table access - uses GameRulesService"""
    from fastapi_models import GameDataTableResponse
    from gamedata.services.game_rules_service import GameRulesService
    try:
        # Use GameRulesService to get table data
        grs = GameRulesService()
        table_data = grs.get_table(name)
        
        if not table_data:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"Table '{name}' not found"
            )
        
        # Convert table data to dictionary format
        converted_data = []
        for row_index, data_obj in enumerate(table_data):
            if hasattr(data_obj, 'to_dict'):
                raw_dict = data_obj.to_dict(use_original_names=True)
            else:
                # Fallback: use available attributes
                raw_dict = {}
                for attr in dir(data_obj):
                    if not attr.startswith('_') and not callable(getattr(data_obj, attr)):
                        raw_dict[attr] = getattr(data_obj, attr)
            
            # Create standardized output
            row_data = {
                'id': row_index,
                'raw_data': raw_dict
            }
            
            # Add name if available
            name_fields = ['Name', 'Label', 'name', 'label']
            for field in name_fields:
                if field in raw_dict and raw_dict[field] and str(raw_dict[field]).lower() != '****':
                    row_data['name'] = raw_dict[field]
                    break
                    
            converted_data.append(row_data)
        
        # Apply search filter
        if search:
            search = search.lower()
            converted_data = [item for item in converted_data if search in item.get('name', '').lower()]
        
        # Apply pagination
        total_count = len(converted_data)
        if offset:
            converted_data = converted_data[offset:]
        if limit:
            converted_data = converted_data[:limit]
        
        return GameDataTableResponse(
            table_name=name,
            data=converted_data,
            count=len(converted_data)
        )
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to get table data: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get table data: {str(e)}"
        )


@router.get("/tables")
def get_tables_list():
    """Get list of available tables - uses GameRulesService"""
    from fastapi_models import GameDataTablesResponse
    from gamedata.services.game_rules_service import GameRulesService
    try:
        grs = GameRulesService()
        stats = grs.get_stats()
        
        # Get available tables from stats or fallback to common tables
        if 'table_data' in stats:
            available_tables = list(stats['table_data'].keys())
        else:
            # Fallback to common tables
            available_tables = ['classes', 'racialtypes', 'feat', 'skills', 'spells', 
                              'baseitems', 'gender', 'appearance']
        
        table_info = []
        for table_name in available_tables:
            try:
                table_data = grs.get_table(table_name)
                count = len(table_data) if table_data else 0
                
                # Get column information if available
                columns = []
                if table_data and len(table_data) > 0:
                    first_item = table_data[0]
                    if hasattr(first_item, 'to_dict'):
                        sample_dict = first_item.to_dict(use_original_names=True)
                        columns = list(sample_dict.keys())[:10]  # Limit to first 10 columns
                
                table_info.append({
                    'name': table_name,
                    'count': count,
                    'columns': columns
                })
            except Exception as e:
                table_info.append({
                    'name': table_name,
                    'count': 0,
                    'error': str(e)
                })
        
        return GameDataTablesResponse(
            tables=table_info,
            total_tables=len(table_info)
        )
        
    except Exception as e:
        logger.error(f"Failed to get tables list: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get tables list: {str(e)}"
        )


@router.get("/schema/")
def get_table_schema(name: str):
    """Get table schema information - uses GameRulesService"""
    from fastapi_models import GameDataSchemaResponse
    from gamedata.services.game_rules_service import GameRulesService
    try:
        grs = GameRulesService()
        table_data = grs.get_table(name)
        
        if not table_data or len(table_data) == 0:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"Table '{name}' not found or empty"
            )
        
        # Get schema from first item
        first_item = table_data[0]
        columns = []
        
        if hasattr(first_item, 'to_dict'):
            sample_dict = first_item.to_dict(use_original_names=True)
            for original_name, sample_value in sample_dict.items():
                columns.append({
                    'original_name': original_name,
                    'safe_name': original_name,
                    'sample_value': sample_value,
                    'type': type(sample_value).__name__ if sample_value is not None else 'unknown'
                })
        else:
            # Fallback to using available attributes
            for attr in dir(first_item):
                if not attr.startswith('_') and not callable(getattr(first_item, attr)):
                    sample_value = getattr(first_item, attr)
                    columns.append({
                        'original_name': attr,
                        'safe_name': attr,
                        'sample_value': sample_value,
                        'type': type(sample_value).__name__ if sample_value is not None else 'unknown'
                    })
        
        return GameDataSchemaResponse(
            table_name=name,
            row_count=len(table_data),
            columns=columns
        )
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to get table schema: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get table schema: {str(e)}"
        )


# Convenience endpoints for common tables
@router.get("/races")
def get_races(search: str = None, limit: int = None, offset: int = 0):
    """Get races - convenience wrapper for table endpoint"""
    return get_table_data("racialtypes", search, limit, offset)


@router.get("/subraces")
def get_subraces(search: str = None, limit: int = None, offset: int = 0):
    """Get subraces from racialsubtypes.2da - includes Aasimar, Tiefling, etc."""
    return get_table_data("racialsubtypes", search, limit, offset)


@router.get("/races/{race_id}/subraces")
def get_subraces_for_race(race_id: int):
    """Get available subraces for a specific base race"""
    from fastapi_models import GameDataTableResponse
    from gamedata.services.game_rules_service import GameRulesService
    
    try:
        rules_service = GameRulesService()
        
        # Get all subraces
        all_subraces_list = rules_service.get_table('racialsubtypes')
        # Convert list to dict with indices as keys
        all_subraces = {i: subrace for i, subrace in enumerate(all_subraces_list)}
        
        # Filter by base race
        filtered_subraces = {}
        for subrace_id, subrace_data in all_subraces.items():
            # Check BaseRace field
            base_race = getattr(subrace_data, 'BaseRace', None)
            if base_race is not None:
                try:
                    if int(base_race) == race_id:
                        # Check if it's player accessible
                        player_race = getattr(subrace_data, 'PlayerRace', 1)
                        if int(player_race) == 1:
                            filtered_subraces[subrace_id] = subrace_data
                except (ValueError, TypeError):
                    continue
        
        # Convert to the same format as table data
        result_data = []
        for subrace_id, subrace_data in filtered_subraces.items():
            subrace_dict = {'id': subrace_id}
            # Add all attributes
            for attr in dir(subrace_data):
                if not attr.startswith('_') and not callable(getattr(subrace_data, attr)):
                    value = getattr(subrace_data, attr)
                    subrace_dict[attr] = value
            result_data.append(subrace_dict)
        
        return GameDataTableResponse(
            table_name="racialsubtypes",
            data=result_data,
            total_count=len(result_data),
            limit=None,
            offset=0
        )
        
    except Exception as e:
        logger.error(f"Failed to get subraces for race {race_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get subraces: {str(e)}"
        )


@router.get("/classes") 
def get_classes(search: str = None, limit: int = None, offset: int = 0, playable_only: bool = False):
    """Get classes - convenience wrapper for table endpoint with playable filter"""
    from fastapi_models import GameDataTableResponse
    from gamedata.services.game_rules_service import GameRulesService
    try:
        # Get base table data
        result = get_table_data("classes", search, limit, offset)
        
        # Apply playable filter if requested
        if playable_only:
            filtered_data = []
            for item in result.data:
                raw_data = item.get('raw_data', {})
                playable = raw_data.get('PlayerClass', raw_data.get('playerclass', 0))
                if playable == 1 or playable == '1':
                    filtered_data.append(item)
            
            result.data = filtered_data
            result.count = len(filtered_data)
        
        return result
        
    except Exception as e:
        logger.error(f"Failed to get classes: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get classes: {str(e)}"
        )


@router.get("/feats")
def get_feats(search: str = None, limit: int = None, offset: int = 0):
    """Get feats - convenience wrapper for table endpoint"""
    return get_table_data("feat", search, limit, offset)


@router.get("/skills")
def get_skills(search: str = None, limit: int = None, offset: int = 0):
    """Get skills - convenience wrapper for table endpoint"""
    return get_table_data("skills", search, limit, offset)


@router.get("/spells")
def get_spells(search: str = None, limit: int = None, offset: int = 0):
    """Get spells - convenience wrapper for table endpoint"""
    return get_table_data("spells", search, limit, offset)


@router.get("/base_items")
def get_base_items(search: str = None, limit: int = None, offset: int = 0):
    """Get base items - convenience wrapper for table endpoint"""
    return get_table_data("baseitems", search, limit, offset)


@router.get("/modules")
def get_modules():
    """Get modules list - uses ResourceManager directly"""
    from fastapi_models import GameDataModulesResponse
    from parsers.resource_manager import ResourceManager
    try:
        rm = ResourceManager(suppress_warnings=True)
        
        # Ensure module index is built
        if not hasattr(rm, '_modules_indexed') or not rm._modules_indexed:
            if hasattr(rm, '_build_module_hak_index'):
                rm._build_module_hak_index()
        
        # Get module data
        modules = []
        if hasattr(rm, '_module_to_haks'):
            for module_name, module_info in rm._module_to_haks.items():
                modules.append({
                    'name': module_name,
                    'mod_file': module_info.get('mod_file', ''),
                    'hak_count': len(module_info.get('haks', [])),
                    'haks': module_info.get('haks', []),
                    'custom_tlk': module_info.get('custom_tlk', '')
                })
        
        return GameDataModulesResponse(
            modules=modules,
            count=len(modules)
        )
        
    except Exception as e:
        logger.error(f"Failed to get modules: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get modules: {str(e)}"
        )


@router.get("/modules/stats")
def get_modules_stats():
    """Get module index statistics - uses ResourceManager directly"""
    from parsers.resource_manager import ResourceManager
    try:
        rm = ResourceManager(suppress_warnings=True)
        
        if hasattr(rm, 'get_module_index_stats'):
            stats = rm.get_module_index_stats()
            return stats
        else:
            return {"error": "Module index stats not available"}
        
    except Exception as e:
        logger.error(f"Failed to get module stats: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get module stats: {str(e)}"
        )