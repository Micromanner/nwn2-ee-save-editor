"""
Data router - Raw GFF data access and field structure exploration
Provides low-level access to character data for debugging and advanced editing
"""

import logging
from typing import Dict, Any, List, Optional
from fastapi import APIRouter, Depends, HTTPException, status, Query
from parsers.gff import GFFFieldType
from fastapi_routers.dependencies import (
    get_character_manager,
    CharacterManagerDep
)
# from fastapi_models import (...) - moved to lazy loading

logger = logging.getLogger(__name__)
router = APIRouter(prefix="/data", tags=["data"])


@router.get("/characters/{character_id}/raw")
def get_raw_character_data(
    character_id: int,
    manager: CharacterManagerDep,
    path: Optional[str] = Query(None, description="GFF path to specific data (e.g., 'ClassList.0.Class')")
):
    """
    Get raw GFF data for a character
    
    Optionally specify a path to get specific nested data.
    Examples:
    - 'Str' - Get strength value
    - 'ClassList' - Get all classes
    - 'ClassList.0' - Get first class
    - 'ClassList.0.Class' - Get class ID of first class
    """
    from fastapi_models import RawDataResponse
    
    try:
        if path:
            # Get specific field using path
            value = manager.gff.get(path)
            if value is None:
                raise HTTPException(
                    status_code=status.HTTP_404_NOT_FOUND,
                    detail=f"Field not found at path: {path}"
                )
            
            # Use helper function - no duplicated logic
            field_type = _determine_gff_type(value)
            
            return RawDataResponse(
                path=path,
                value=value,
                field_type=field_type,
                raw_data={path: value}
            )
        else:
            # Return full character data
            return RawDataResponse(
                path="/",
                value=manager.character_data,
                field_type="Struct",
                raw_data=manager.character_data
            )
            
    except Exception as e:
        logger.error(f"Failed to get raw data for character {character_id}: {e}")
        if isinstance(e, HTTPException):
            raise
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get raw data: {str(e)}"
        )


@router.get("/characters/{character_id}/structure")
def get_field_structure(
    character_id: int,
    manager: CharacterManagerDep,
    path: Optional[str] = Query(None, description="GFF path to analyze structure"),
    max_depth: int = Query(2, description="Maximum depth to explore", ge=1, le=5)
):
    """
    Get the field structure of character data
    
    Returns information about field types, sizes, and nested structures.
    Useful for understanding the GFF data organization.
    """
    from fastapi_models import FieldStructureResponse
    
    try:
        # Get the data at the specified path
        if path:
            data = manager.gff.get(path)
            if data is None:
                raise HTTPException(
                    status_code=status.HTTP_404_NOT_FOUND,
                    detail=f"Field not found at path: {path}"
                )
        else:
            data = manager.character_data
            path = "/"
        
        # Analyze structure
        structure = analyze_structure(data, max_depth=max_depth)
        
        return FieldStructureResponse(
            path=path,
            structure=structure,
            total_fields=count_fields(structure),
            max_depth_analyzed=max_depth
        )
        
    except Exception as e:
        logger.error(f"Failed to get field structure for character {character_id}: {e}")
        if isinstance(e, HTTPException):
            raise
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get field structure: {str(e)}"
        )


def _determine_gff_type(value: Any) -> str:
    """Determine GFF field type from Python value"""
    if isinstance(value, bool):
        return "BYTE"
    elif isinstance(value, int):
        return "INT"
    elif isinstance(value, float):
        return "FLOAT"
    elif isinstance(value, str):
        return "CExoString"
    elif isinstance(value, dict):
        return "Struct"
    elif isinstance(value, list):
        return "List"
    else:
        return "unknown"


def analyze_structure(data: Any, current_depth: int = 0, max_depth: int = 2) -> Dict[str, Any]:
    """
    Recursively analyze the structure of GFF data
    
    Args:
        data: The data to analyze
        current_depth: Current recursion depth
        max_depth: Maximum depth to explore
        
    Returns:
        Dictionary describing the structure
    """
    if current_depth >= max_depth:
        return {
            "type": type(data).__name__,
            "truncated": True,
            "reason": "max_depth_reached"
        }
    
    if data is None:
        return {"type": "null", "value": None}
    
    elif isinstance(data, bool):
        return {"type": "bool", "value": data}
    
    elif isinstance(data, int):
        return {"type": "int", "value": data}
    
    elif isinstance(data, float):
        return {"type": "float", "value": data}
    
    elif isinstance(data, str):
        return {
            "type": "string",
            "length": len(data),
            "value": data if len(data) <= 100 else data[:100] + "..."
        }
    
    elif isinstance(data, bytes):
        return {
            "type": "bytes",
            "length": len(data),
            "preview": data[:20].hex() if len(data) > 0 else ""
        }
    
    elif isinstance(data, list):
        result = {
            "type": "list",
            "length": len(data),
            "items": []
        }
        
        # Analyze first few items
        for i, item in enumerate(data[:5]):  # Limit to first 5 items
            result["items"].append({
                "index": i,
                "structure": analyze_structure(item, current_depth + 1, max_depth)
            })
        
        if len(data) > 5:
            result["truncated"] = True
            result["total_items"] = len(data)
        
        return result
    
    elif isinstance(data, dict):
        result = {
            "type": "struct",
            "field_count": len(data),
            "fields": {}
        }
        
        # Analyze all fields (but respect max_depth)
        for key, value in data.items():
            result["fields"][key] = analyze_structure(value, current_depth + 1, max_depth)
        
        return result
    
    else:
        # Unknown type
        return {
            "type": type(data).__name__,
            "value": str(data)[:100] if hasattr(data, '__str__') else "unknown"
        }


def count_fields(structure: Dict[str, Any]) -> int:
    """
    Count total number of fields in a structure
    
    Args:
        structure: The structure dictionary from analyze_structure
        
    Returns:
        Total field count
    """
    if not isinstance(structure, dict):
        return 0
    
    count = 1  # Count this field
    
    if structure.get("type") == "struct" and "fields" in structure:
        for field_structure in structure["fields"].values():
            count += count_fields(field_structure)
    
    elif structure.get("type") == "list" and "items" in structure:
        for item in structure["items"]:
            if "structure" in item:
                count += count_fields(item["structure"])
    
    return count


@router.post("/characters/{character_id}/raw")
def update_raw_field(
    character_id: int,
    request,  # Type removed for lazy loading
    manager: CharacterManagerDep
):
    """
    Update a raw GFF field value
    
    WARNING: This is a low-level operation that bypasses validation.
    Use with caution as it can corrupt save files if used incorrectly.
    """
    from fastapi_models import RawFieldUpdateRequest, RawFieldUpdateResponse
    
    try:
        # Check if field exists
        current_value = manager.gff.get(request.path)
        if current_value is None:
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"Field not found at path: {request.path}"
            )
        
        # Set the new value
        manager.gff.set(request.path, request.value)
        
        logger.info(
            f"Updated raw field for character {character_id}: "
            f"path={request.path}, old_value={current_value}, new_value={request.value}"
        )
        
        return RawFieldUpdateResponse(
            success=True,
            path=request.path,
            old_value=current_value,
            new_value=request.value,
            message="Field updated successfully"
        )
        
    except Exception as e:
        logger.error(f"Failed to update raw field for character {character_id}: {e}")
        if isinstance(e, HTTPException):
            raise
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update raw field: {str(e)}"
        )