
import sys
import os
import unittest
from unittest.mock import MagicMock, patch

# Add backend to path
sys.path.append(os.path.join(os.getcwd(), 'backend'))

from character.managers.feat_manager import FeatManager
from character.services.rules_service import RulesService

class TestFeatLoading(unittest.TestCase):
    def setUp(self):
        self.rules_service = MagicMock(spec=RulesService)
        self.character_manager = MagicMock()
        self.character_manager.rules_service = self.rules_service
        self.character_manager.gff = MagicMock()
        self.character_manager.gff.get.return_value = [] # Default empty list
        
        self.feat_manager = FeatManager(self.character_manager)

    def test_parse_feat_type_domain(self):
        # Mock feat data with type 8192 (Domain)
        feat_data = MagicMock()
        # Mock field_mapper response
        with patch('gamedata.dynamic_loader.field_mapping_utility.field_mapper.get_field_value') as mock_get_val:
            def side_effect(obj, field, default=None):
                if field == 'type':
                    return '8192'
                if field == 'description':
                    return ''
                return default
            mock_get_val.side_effect = side_effect
            
            feat_type = self.feat_manager._parse_feat_type(feat_data)
            print(f"Parsed Type 8192: {feat_type}")
            self.assertEqual(feat_type, 8192, "Should parse type 8192 correctly")

    def test_parse_feat_type_background(self):
        # Mock feat data with type 128 (Background)
        feat_data = MagicMock()
        with patch('gamedata.dynamic_loader.field_mapping_utility.field_mapper.get_field_value') as mock_get_val:
            def side_effect(obj, field, default=None):
                if field == 'type':
                    return '128'
                return default
            mock_get_val.side_effect = side_effect
            
            feat_type = self.feat_manager._parse_feat_type(feat_data)
            print(f"Parsed Type 128: {feat_type}")
            self.assertEqual(feat_type, 128, "Should parse type 128 correctly")

    def test_get_feat_summary_fast_categorization(self):
        # Setup mocked get_feat_info_display
        self.feat_manager.get_feat_info_display = MagicMock()
        
        # Test Case 1: Background Feat
        self.feat_manager.get_feat_info_display.side_effect = [
            {'id': 1, 'type': 128, 'protected': False, 'custom': False}, # Background
            {'id': 2, 'type': 8192, 'protected': False, 'custom': False}, # Domain
            {'id': 3, 'type': 1, 'protected': False, 'custom': False}     # General
        ]
        
        self.character_manager.gff.get.return_value = [{'Feat': 1}, {'Feat': 2}, {'Feat': 3}]
        
        summary = self.feat_manager.get_feat_summary_fast()
        
        print("Summary keys:", summary.keys())
        print("Background Feats:", len(summary.get('background_feats', [])))
        print("Domain Feats:", len(summary.get('domain_feats', [])))
        print("General Feats:", len(summary.get('general_feats', [])))

        self.assertIn('background_feats', summary)
        self.assertIn('domain_feats', summary)
        self.assertEqual(len(summary['background_feats']), 1)
        self.assertEqual(len(summary['domain_feats']), 1)
        self.assertEqual(len(summary['general_feats']), 1)

if __name__ == '__main__':
    unittest.main()
