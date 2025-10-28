"""
FastAPI router for character creation and export operations.
Handles creating new characters and exporting them to NWN2.
Useless AF right now, will be changed later.
"""

import os
import shutil
import tempfile
from datetime import datetime
from typing import List, Optional
from fastapi import APIRouter, HTTPException, status, Depends
from pydantic import BaseModel
from loguru import logger

from config.nwn2_settings import nwn2_paths
from character.character_creation_service import CharacterCreationService
from parsers.resource_manager import ResourceManager
from .dependencies import check_system_ready

router = APIRouter(tags=["Character Creation"])


# Pydantic models for character creation
class CharacterClassInfo(BaseModel):
    classId: int
    level: int

class CreateCharacterRequest(BaseModel):
    firstName: str
    lastName: str
    age: int = 25
    gender: int  # 0 = Male, 1 = Female
    deity: Optional[str] = ""
    raceId: int
    classes: List[CharacterClassInfo]
    strength: int
    dexterity: int
    constitution: int
    intelligence: int
    wisdom: int
    charisma: int
    lawChaos: int = 50  # 0-100 scale
    goodEvil: int = 50  # 0-100 scale
    appearance: Optional[dict] = None
    portrait: Optional[str] = None
    voiceset: Optional[int] = None
    soundset: Optional[int] = None
    subrace: Optional[str] = ""
    biography: Optional[str] = ""
    saveToLocalVault: bool = False
    autoLevel: bool = False
    maxLevel: int = 1

class CreateCharacterResponse(BaseModel):
    success: bool
    message: str
    character_path: Optional[str] = None
    character_id: Optional[str] = None
    warnings: List[str] = []

class CharacterTemplate(BaseModel):
    name: str
    description: str
    classes: List[CharacterClassInfo]
    race: str
    raceId: int
    attributes: dict
    portrait: Optional[str] = None

class ExportToLocalVaultRequest(BaseModel):
    source_path: str
    backup_existing: bool = True

class ExportToLocalVaultResponse(BaseModel):
    success: bool
    message: str
    exported_path: str
    backup_path: Optional[str] = None

class ExportForModuleRequest(BaseModel):
    source_path: str
    module_name: str
    character_name: Optional[str] = None

class ExportForModuleResponse(BaseModel):
    success: bool
    message: str
    exported_path: str


@router.post("/characters/create", response_model=CreateCharacterResponse)
def create_character(
    request: CreateCharacterRequest,
    _: None = Depends(check_system_ready)
):
    """
    Create a new character from character builder data.
    
    This endpoint creates a new NWN2 character file (.bic) with the specified
    attributes, race, classes, and other properties.
    
    - **firstName/lastName**: Character name
    - **age**: Character age (cosmetic)
    - **gender**: 0 for male, 1 for female
    - **deity**: Character's deity (optional)
    - **raceId**: Race ID from racialtypes.2da
    - **classes**: List of classes with levels
    - **attributes**: STR, DEX, CON, INT, WIS, CHA
    - **alignment**: lawChaos (0-100) and goodEvil (0-100)
    - **saveToLocalVault**: Automatically save to NWN2 localvault
    - **autoLevel**: Automatically level up to specified level
    """
    try:
        # Create temporary directory for character file
        temp_dir = tempfile.mkdtemp(prefix="nwn2_char_")
        character_path = os.path.join(temp_dir, "player.bic")
        
        # Initialize creation service
        creation_service = CharacterCreationService()
        
        # Convert request to dict for service
        character_data = request.dict()
        
        # Create the character
        result = creation_service.create_character(
            character_data,
            output_path=character_path
        )
        
        if not result['success']:
            return CreateCharacterResponse(
                success=False,
                message=result.get('error', 'Character creation failed'),
                warnings=result.get('warnings', [])
            )
        
        # Optionally save to localvault
        if request.saveToLocalVault:
            localvault_path = os.path.join(
                nwn2_paths.get('nwn2_docs', ''),
                'localvault',
                f"{request.firstName}_{request.lastName}.bic"
            )
            
            if os.path.exists(localvault_path) and request.backup_existing:
                backup_path = f"{localvault_path}.backup_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
                shutil.copy2(localvault_path, backup_path)
                logger.info(f"Backed up existing character to {backup_path}")
            
            shutil.copy2(character_path, localvault_path)
            character_path = localvault_path
        
        return CreateCharacterResponse(
            success=True,
            message="Character created successfully",
            character_path=character_path,
            character_id=result.get('character_id'),
            warnings=result.get('warnings', [])
        )
        
    except Exception as e:
        logger.error(f"Failed to create character: {e}", exc_info=True)
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to create character: {str(e)}"
        )


@router.get("/characters/templates", response_model=List[CharacterTemplate])
def get_character_templates(
    _: None = Depends(check_system_ready)
):
    """
    Get available character templates for quick creation.
    
    Returns a list of pre-configured character templates with
    recommended attribute distributions and class combinations.
    """
    templates = [
        CharacterTemplate(
            name="Fighter",
            description="A strong warrior focused on melee combat",
            classes=[CharacterClassInfo(classId=4, level=1)],
            race="Human",
            raceId=6,
            attributes={
                "strength": 16,
                "dexterity": 13,
                "constitution": 14,
                "intelligence": 10,
                "wisdom": 12,
                "charisma": 8
            },
            portrait="po_hu_m_01_"
        ),
        CharacterTemplate(
            name="Wizard",
            description="A scholar of the arcane arts",
            classes=[CharacterClassInfo(classId=11, level=1)],
            race="Elf",
            raceId=1,
            attributes={
                "strength": 8,
                "dexterity": 14,
                "constitution": 12,
                "intelligence": 18,
                "wisdom": 10,
                "charisma": 10
            },
            portrait="po_el_m_01_"
        ),
        CharacterTemplate(
            name="Rogue",
            description="A stealthy and skilled infiltrator",
            classes=[CharacterClassInfo(classId=8, level=1)],
            race="Halfling",
            raceId=3,
            attributes={
                "strength": 10,
                "dexterity": 18,
                "constitution": 12,
                "intelligence": 14,
                "wisdom": 10,
                "charisma": 12
            },
            portrait="po_ha_m_01_"
        ),
        CharacterTemplate(
            name="Cleric",
            description="A divine spellcaster and healer",
            classes=[CharacterClassInfo(classId=2, level=1)],
            race="Dwarf",
            raceId=0,
            attributes={
                "strength": 14,
                "dexterity": 10,
                "constitution": 14,
                "intelligence": 10,
                "wisdom": 16,
                "charisma": 10
            },
            portrait="po_dw_m_01_"
        )
    ]
    
    return templates


@router.post("/characters/export/localvault", response_model=ExportToLocalVaultResponse)
def export_to_localvault(
    request: ExportToLocalVaultRequest,
    _: None = Depends(check_system_ready)
):
    """
    Export a created character directly to NWN2's localvault.
    
    - **source_path**: Path to the character file to export
    - **backup_existing**: Whether to backup existing character with same name
    
    The character will be available for selection in NWN2 game.
    """
    try:
        if not os.path.exists(request.source_path):
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Source character file not found"
            )
        
        # Get NWN2 localvault path
        localvault_path = os.path.join(
            nwn2_paths.get('nwn2_docs', ''),
            'localvault'
        )
        
        if not os.path.exists(localvault_path):
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="NWN2 localvault not found. Is NWN2 installed?"
            )
        
        # Get filename from source
        filename = os.path.basename(request.source_path)
        if not filename.endswith('.bic'):
            filename = 'player.bic'
        
        dest_path = os.path.join(localvault_path, filename)
        backup_path = None
        
        # Backup existing if requested
        if request.backup_existing and os.path.exists(dest_path):
            backup_path = f"{dest_path}.backup_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
            shutil.copy2(dest_path, backup_path)
            logger.info(f"Backed up existing character to {backup_path}")
        
        # Copy character to localvault
        shutil.copy2(request.source_path, dest_path)
        
        return ExportToLocalVaultResponse(
            success=True,
            message="Character exported to localvault successfully",
            exported_path=dest_path,
            backup_path=backup_path
        )
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to export to localvault: {e}", exc_info=True)
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to export character: {str(e)}"
        )


@router.post("/characters/export/module", response_model=ExportForModuleResponse)
def export_for_module(
    request: ExportForModuleRequest,
    _: None = Depends(check_system_ready)
):
    """
    Export a character for use with a specific module.
    
    - **source_path**: Path to the character file to export
    - **module_name**: Name of the module (without .mod extension)
    - **character_name**: Optional custom name for the exported character
    
    The character will be placed in the module's override directory.
    """
    try:
        if not os.path.exists(request.source_path):
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Source character file not found"
            )
        
        # Get module directory
        modules_path = os.path.join(
            nwn2_paths.get('nwn2_docs', ''),
            'modules',
            request.module_name
        )
        
        if not os.path.exists(modules_path):
            # Try to create module directory
            os.makedirs(modules_path, exist_ok=True)
            logger.info(f"Created module directory: {modules_path}")
        
        # Determine character filename
        if request.character_name:
            filename = f"{request.character_name}.bic"
        else:
            filename = os.path.basename(request.source_path)
            if not filename.endswith('.bic'):
                filename = 'player.bic'
        
        dest_path = os.path.join(modules_path, filename)
        
        # Copy character to module directory
        shutil.copy2(request.source_path, dest_path)
        
        return ExportForModuleResponse(
            success=True,
            message=f"Character exported for module '{request.module_name}'",
            exported_path=dest_path
        )
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to export for module: {e}", exc_info=True)
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to export character for module: {str(e)}"
        )