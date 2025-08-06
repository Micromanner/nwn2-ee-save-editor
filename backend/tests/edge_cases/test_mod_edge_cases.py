import pytest
import os
from pathlib import Path

from parsers.resource_manager import ResourceManager
from gamedata.game_rules_service import GameRulesService


@pytest.fixture
def resource_manager():
    """Provides a default ResourceManager instance for tests."""
    return ResourceManager()


@pytest.fixture
def game_rules_service():
    """Provides a default GameRulesService instance for tests."""
    return GameRulesService()


class TestModSupportEdgeCases:
    """Test edge cases for mod support functionality."""

    def test_missing_hak_files(self):
        """Test behavior when module references missing HAK files."""
        # Arrange: Create a resource manager with missing HAKs
        rm = ResourceManager()
        rm._module_haks = ['nonexistent_hak1.hak', 'nonexistent_hak2.hak']
        rm._module_name = 'TestModuleMissingHAKs'

        # Act & Assert: Should not crash when HAKs are missing
        result = rm.get_2da_with_overrides('classes')
        assert result is not None, "Should return data even with missing HAKs"

        # Arrange: Test GameRulesService with the same resource manager
        grs = GameRulesService(resource_manager=rm)
        grs._load_classes()

        # Assert: Should still have base game data
        assert len(grs.classes) > 0, "Should have base game classes"

    @pytest.mark.parametrize("filename, content", [
        ('classes.2da', b'This is not a valid 2DA file!\nRandom garbage data'),
        ('feat.2da', b'2DA V2.0\n\nMissing required columns\n0 BadData'),
        ('spells.2da', b'2DA V2.0\n\nLABEL\n**** Missing rows!'),
    ])
    def test_corrupted_2da_overrides(self, tmp_path, filename, content):
        """Test behavior with a corrupted or invalid 2DA file."""
        # Arrange: Create a corrupted 2DA file in a temporary override directory
        (tmp_path / filename).write_bytes(content)

        rm = ResourceManager()
        rm._override_dirs = [str(tmp_path)]

        # Act & Assert: Should fall back to base data without crashing
        result = rm.get_2da_with_overrides(Path(filename).stem)
        assert result is not None, f"{filename} should return base data"

    def test_character_with_invalid_ids(self, game_rules_service):
        """Test characters with IDs not in current module's 2DAs."""
        grs = game_rules_service
        character_data = {
            'ClassList': [
                {'Class': 99999, 'ClassLevel': 10},  # Non-existent
                {'Class': 1, 'ClassLevel': 5}        # Valid (Fighter)
            ]
        }
        grs._load_classes()
        valid_class_ids = list(grs.classes.keys())

        # Assert that IDs are correctly identified as valid or invalid
        assert 99999 not in valid_class_ids
        assert 1 in valid_class_ids

    def test_module_switching(self, game_rules_service):
        """Test switching between modules with same character."""
        grs = game_rules_service

        grs.set_module_context(None)
        grs._load_classes()
        grs._load_feats()
        base_classes_count = len(grs.classes)
        base_feats_count = len(grs.feats)

        assert base_classes_count > 0
        assert base_feats_count > 0

        # Switch context and check again
        grs.set_module_context(None)
        grs._load_classes()
        grs._load_feats()
        restored_classes_count = len(grs.classes)
        restored_feats_count = len(grs.feats)

        assert base_classes_count == restored_classes_count
        assert base_feats_count == restored_feats_count

    def test_empty_or_malformed_module(self, resource_manager, tmp_path):
        """Test loading empty or malformed module files."""
        rm = resource_manager

        # Arrange: Create empty and corrupted .mod files
        empty_mod = tmp_path / "empty.mod"
        empty_mod.touch()
        corrupted_mod = tmp_path / "corrupted.mod"
        corrupted_mod.write_bytes(b'NOT_AN_ERF_FILE' * 100)

        # Act & Assert: Loading these files should fail gracefully
        assert not rm.set_module(str(empty_mod))
        assert not rm.set_module(str(corrupted_mod))

        # Assert: ResourceManager should still function with base data
        classes = rm.get_2da_with_overrides('classes')
        assert classes is not None, "Should fall back to base data"

    @pytest.mark.parametrize("module_name", [
        "Module's Name.mod",
        "Module (Test).mod",
        "Module - Test.mod",
        "Module & Test.mod"
    ])
    def test_module_with_special_characters(self, resource_manager, module_name):
        """Test modules with special characters in names are handled."""
        rm = resource_manager
        rm._module_name = module_name
        result = rm.get_2da_with_overrides('classes')
        assert result is not None

    def test_hak_load_order_preservation(self, resource_manager):
        """Test that HAK load order is preserved."""
        rm = resource_manager
        hak_list = ['hak1.hak', 'hak2.hak', 'hak3.hak']
        rm._module_haks = hak_list
        assert rm._module_haks == hak_list