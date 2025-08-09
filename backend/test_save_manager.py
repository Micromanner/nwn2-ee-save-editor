#!/usr/bin/env python3
"""
Quick test script to validate SaveManager fixes
"""

import os
import sys
import django

# Add the backend directory to the Python path
backend_dir = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, backend_dir)

# Setup Django
os.environ.setdefault('DJANGO_SETTINGS_MODULE', 'config.settings')
django.setup()

from character.managers.save_manager import SaveManager
from gamedata.dynamic_loader.dynamic_game_data_loader import DynamicGameDataLoader
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

def test_save_manager():
    """Test the SaveManager functionality"""
    
    # Mock character manager with minimal required interface
    class MockCharacterManager:
        def __init__(self):
            # Create a sample GFF structure
            self.gff = {
                'Str': 16, 'Dex': 14, 'Con': 15, 'Int': 12, 'Wis': 13, 'Cha': 11,
                'FortSave': 3, 'RefSave': 2, 'WillSave': 1,
                'Race': 6,  # Human
                'ClassList': [
                    {'Class': 0, 'ClassLevel': 5}  # Fighter level 5
                ],
                'FeatList': [
                    {'Feat': 14}  # Great Fortitude
                ]
            }
            self.game_data_loader = DynamicGameDataLoader()
            self.character_data = {}
            self.custom_content = {}
            self._managers = {}
            
        def get_manager(self, manager_type):
            return self._managers.get(manager_type)
        
        def on(self, event_type, handler):
            pass
        
        def emit(self, event):
            pass
    
    # Create mock character manager
    char_manager = MockCharacterManager()
    
    try:
        # Create SaveManager instance
        save_manager = SaveManager(char_manager)
        logger.info("SaveManager created successfully")
        
        # Test calculation
        saves = save_manager.calculate_saving_throws()
        logger.info(f"Calculated saves: {saves}")
        
        # Test individual save calculations
        fort_save = save_manager.calculate_fortitude_save()
        ref_save = save_manager.calculate_reflex_save()
        will_save = save_manager.calculate_will_save()
        
        logger.info(f"Individual saves - Fort: {fort_save}, Ref: {ref_save}, Will: {will_save}")
        
        logger.info("SaveManager test completed successfully!")
        return True
        
    except Exception as e:
        logger.error(f"SaveManager test failed: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == '__main__':
    success = test_save_manager()
    sys.exit(0 if success else 1)