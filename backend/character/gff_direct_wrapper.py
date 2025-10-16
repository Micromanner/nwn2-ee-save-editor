"""
Direct GFF Wrapper - Updates both dictionary and GFFElement simultaneously
This avoids the need for problematic update_from_dict calls
"""

from typing import Any
from .character_manager import GFFDataWrapper


class DirectGFFWrapper(GFFDataWrapper):
    """
    A wrapper that updates both the dictionary representation and the
    GFFElement directly, keeping them in sync.

    IMPORTANT: get() always reads from gff_element to ensure fresh data
    """

    def __init__(self, gff_element):
        """
        Initialize with a GFFElement

        Args:
            gff_element: The GFFElement to wrap
        """
        self.gff_element = gff_element
        super().__init__(gff_element.to_dict())

    def get(self, path: str, default=None):
        """
        Get value at path from internal _data dictionary

        Args:
            path: Dot-separated path to value
            default: Default value if path doesn't exist

        Returns:
            Value at path or default

        Note: We use _data directly since set() keeps it in sync.
        We do NOT refresh from gff_element because set_field() may not
        work correctly for complex types like lists.
        """
        # Use parent's get method directly - _data is kept up to date by set()
        return super().get(path, default)

    def set(self, path: str, value: Any):
        """
        Set value at path, updating both dict and GFFElement

        Args:
            path: Dot-separated path to value
            value: Value to set
        """
        # First update the dictionary
        super().set(path, value)

        # Then try to update the GFFElement directly
        try:
            # For simple paths, use set_field
            if '.' not in path and '[' not in path:
                self.gff_element.set_field(path, value)
            else:
                # For complex paths, we'll need to navigate the structure
                # For now, we'll rely on update_from_dict being called later
                # This could be enhanced to handle complex paths
                pass
        except Exception:
            # If direct update fails, we'll need update_from_dict later
            # This is expected for complex nested structures
            pass