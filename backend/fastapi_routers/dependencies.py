"""
FastAPI dependencies for character management and common functionality
"""

import logging
from typing import Optional
from fastapi import Depends, HTTPException, Request, status
from fastapi.responses import JSONResponse
from fastapi.exceptions import RequestValidationError
from starlette.exceptions import HTTPException as StarletteHTTPException

# Removed character_info dependency - using session registry directly
from fastapi_core.session_registry import get_character_session, get_path_from_id
from fastapi_core.exceptions import CharacterNotFoundException, CharacterSessionException, SystemNotReadyException
from gamedata.dynamic_loader.singleton import is_loader_ready
from fastapi_models import ErrorResponse

logger = logging.getLogger(__name__)


def check_system_ready():
    """
    Dependency to check if the system is ready to handle requests.
    
    Raises:
        SystemNotReadyException: If system is not ready
    """
    if not is_loader_ready():
        logger.info("Request received but DynamicGameDataLoader not ready yet")
        raise SystemNotReadyException(50)  # Assume 50% progress if not ready


def _get_character_session(character_id: int):
    """
    Helper function to get character session - no duplicate session creation
    
    Args:
        character_id: Character ID (integer)
        
    Returns:
        session: Character session object
        
    Raises:
        HTTPException: If character not found or can't be loaded
    """
    try:
        # Convert integer ID to file path
        file_path = get_path_from_id(character_id)
        if not file_path:
            raise CharacterNotFoundException(character_id)
        
        # Get session (no duplicate session creation)
        session = get_character_session(file_path)
        return session
        
    except (HTTPException, Exception) as e:
        if hasattr(e, 'status_code'):  # Already an HTTP exception
            raise e
        logger.error(f"Failed to get character session: {e}")
        raise CharacterSessionException(f"Unable to load character: {str(e)}", character_id)


def get_character_manager(character_id: int, ready_check: None = Depends(check_system_ready)):
    """
    Dependency to get character manager for read operations.
    
    Args:
        character_id: Character ID (integer)
        ready_check: System readiness check dependency
        
    Returns:
        character_manager: Character manager object
        
    Raises:
        HTTPException: If character not found or can't be loaded
    """
    session = _get_character_session(character_id)
    return session.character_manager


def get_character_session_dep(character_id: int, ready_check: None = Depends(check_system_ready)):
    """
    Dependency to get character in-memory session.
    
    Args:
        character_id: Character ID (integer)
        ready_check: System readiness check dependency
        
    Returns:
        character_session: Character session object
        
    Raises:
        HTTPException: If character not found or can't be loaded
    """
    return _get_character_session(character_id)


def handle_character_error(character_id: int, error: Exception, operation: str = "operation"):
    """
    Standard error handling for character operations.
    
    Args:
        character_id: Character ID for logging
        error: Exception that occurred
        operation: Operation being performed
        
    Returns:
        HTTPException: Appropriate HTTP exception
    """
    logger.exception(f"Error in {operation} for character {character_id}: {str(error)}")
    
    error_str = str(error).lower()
    
    # Check if system not ready
    if "system not ready" in error_str:
        return HTTPException(
            status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
            detail="System is still initializing, please try again in a few seconds",
            headers={'Retry-After': '5'}
        )
    # Character not found
    elif "not found" in error_str:
        return HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Character {character_id} not found or save files missing"
        )
    # Operation not supported
    elif "not supported" in error_str:
        return HTTPException(
            status_code=status.HTTP_501_NOT_IMPLEMENTED,
            detail=str(error)
        )
    # Generic server error
    else:
        return HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to {operation}: {str(error)}"
        )


def setup_exception_handlers(app):
    """
    Setup global exception handlers for the FastAPI app.
    
    Args:
        app: FastAPI application instance
    """
    
    @app.exception_handler(StarletteHTTPException)
    async def http_exception_handler(request: Request, exc: StarletteHTTPException):
        """Handle HTTP exceptions"""
        return JSONResponse(
            status_code=exc.status_code,
            content={"error": exc.detail},
            headers=getattr(exc, 'headers', None)
        )
    
    @app.exception_handler(RequestValidationError)
    async def validation_exception_handler(request: Request, exc: RequestValidationError):
        """Handle request validation errors"""
        errors = []
        for error in exc.errors():
            field_path = " -> ".join(str(loc) for loc in error["loc"])
            errors.append(f"{field_path}: {error['msg']}")
        
        return JSONResponse(
            status_code=status.HTTP_422_UNPROCESSABLE_ENTITY,
            content={
                "error": "Validation error",
                "details": errors
            }
        )
    
    @app.exception_handler(Exception)
    async def global_exception_handler(request: Request, exc: Exception):
        """Handle all other exceptions"""
        logger.error(f"Unhandled exception on {request.url}: {exc}", exc_info=True)
        return JSONResponse(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            content={"error": f"Internal server error: {str(exc)}"}
        )


# Commonly used dependencies that can be imported  
# Note: These now return the manager/session directly (no CharacterInfo tuple)
from typing import Annotated
from character.character_manager import CharacterManager
from character.in_memory_save_manager import InMemoryCharacterSession

CharacterManagerDep = Annotated[CharacterManager, Depends(get_character_manager)]
CharacterSessionDep = Annotated[InMemoryCharacterSession, Depends(get_character_session_dep)]