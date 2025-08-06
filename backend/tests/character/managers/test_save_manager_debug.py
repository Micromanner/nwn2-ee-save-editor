import pytest
from unittest.mock import Mock
from character.managers.save_manager import SaveManager


def test_debug_feat_bonus():
    """Debug feat bonus calculation - tests that feat cache is populated correctly"""
    # Create minimal mocks
    mock_gff = Mock()
    mock_gff.get = Mock(return_value=[])
    
    mock_manager = Mock()
    mock_manager.gff = mock_gff
    mock_manager.custom_content = {}
    
    # Create mock feat with various field combinations
    class MockFeat:
        def __init__(self):
            self.id = 22
            self.label = 'IronWill'
            self.save_modifier = 2
    
    mock_loader = Mock()
    mock_loader.get_table = Mock(return_value=[MockFeat()])
    
    mock_manager.game_data_loader = mock_loader
    mock_manager.get_racial_saves = Mock(return_value={'fortitude': 0, 'reflex': 0, 'will': 0})
    
    save_manager = SaveManager(mock_manager)
    
    # Mock the class-related methods to avoid dependency issues
    save_manager._has_class_by_name = Mock(return_value=False)
    save_manager._get_class_level_by_name = Mock(return_value=0)
    save_manager._get_ability_modifier = Mock(return_value=0)
    
    # Test that feat cache is populated correctly
    assert save_manager._save_affecting_feats is not None
    assert 'will' in save_manager._save_affecting_feats
    assert len(save_manager._save_affecting_feats['will']) == 1
    
    # Check the cached feat info
    will_feat = save_manager._save_affecting_feats['will'][0]
    assert will_feat['id'] == 22
    assert will_feat['label'] == 'IronWill'
    assert will_feat['bonus'] == 2
    
    # Test that feat bonuses are calculated correctly when character has the feat
    mock_gff.get = Mock(return_value=[{'Feat': 22}])  # Character has Iron Will feat
    bonuses = save_manager._calculate_feat_bonuses()
    
    assert bonuses['will'] == 2  # Should get the Iron Will bonus
    assert bonuses['fortitude'] == 0  # Should not affect fortitude
    assert bonuses['reflex'] == 0  # Should not affect reflex


if __name__ == "__main__":
    test_debug_feat_bonus()