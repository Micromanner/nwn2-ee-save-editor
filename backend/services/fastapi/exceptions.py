"""
Custom exceptions for FastAPI application
Centralized exception definitions following 2025 FastAPI best practices
"""

from typing import Optional


class NWN2SaveEditorException(Exception):
    """Base exception for NWN2 Save Editor."""
    def __init__(self, message: str, status_code: int = 500):
        self.message = message
        self.status_code = status_code
        super().__init__(self.message)


class CharacterNotFoundException(NWN2SaveEditorException):
    """Character not found exception."""
    def __init__(self, character_id: int):
        super().__init__(f"Character {character_id} not found", 404)
        self.character_id = character_id


class CharacterSessionException(NWN2SaveEditorException):
    """Character session related exception."""
    def __init__(self, message: str, character_id: Optional[int] = None):
        super().__init__(message, 500)
        self.character_id = character_id


class SystemNotReadyException(NWN2SaveEditorException):
    """System initialization not complete."""
    def __init__(self, progress: int):
        super().__init__("System is still initializing, please try again", 503)
        self.progress = progress


class ValidationException(NWN2SaveEditorException):
    """Data validation exception."""
    def __init__(self, message: str, field: Optional[str] = None):
        super().__init__(message, 422)
        self.field = field


class SaveFileException(NWN2SaveEditorException):
    """Save file related exception."""
    def __init__(self, message: str, file_path: Optional[str] = None):
        super().__init__(message, 500)
        self.file_path = file_path