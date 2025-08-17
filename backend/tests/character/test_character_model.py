"""
Comprehensive tests for the Character model
Tests all aspects including get_game_rules(), module caching, campaign fields,
validation logic, and database operations.
"""
import pytest
import os
import sys

from django.contrib.auth.models import User
from django.core.exceptions import ValidationError
from unittest.mock import Mock, patch, MagicMock
from datetime import datetime
import json
import time

from character.models import (
    Character, CharacterClass, CharacterFeat, 
    CharacterSkill, CharacterSpell, CharacterItem
)
from gamedata.game_rules_service import GameRulesService
from parsers.resource_manager import ResourceManager

# Mark all tests to use Django DB
pytestmark = pytest.mark.django_db


class TestCharacterModel:
    """Test the Character model core functionality"""
    
    @pytest.fixture
    def user(self, db):
        """Create a test user"""
        return User.objects.create_user(
            username='testuser',
            email='test@example.com',
            password='testpass123'
        )
    
    @pytest.fixture
    def basic_character(self, db, user):
        """Create a basic character for testing"""
        return Character.objects.create(
            owner=user,
            file_name='test_character.bic',
            file_path='/path/to/test_character.bic',
            first_name='Test',
            last_name='Character',
            race_id=1,
            race_name='Human',
            character_level=5,
            strength=16,
            dexterity=14,
            constitution=15,
            intelligence=12,
            wisdom=13,
            charisma=10,
            hit_points=42,
            max_hit_points=42,
            experience=10000
        )
    
    @pytest.fixture
    def campaign_character(self, db, user):
        """Create a character from an official campaign"""
        return Character.objects.create(
            owner=user,
            file_name='campaign_char.bic',
            file_path='/saves/motb/campaign_char.bic',
            first_name='Campaign',
            last_name='Hero',
            race_id=4,
            race_name='Elf',
            character_level=20,
            module_name='2100_mulsantir',
            campaign_name='NWN2 Mask of the Betrayer Campaign',
            campaign_path='/path/to/motb',
            campaign_modules=['2000_motb', '2100_mulsantir', '2200_thayred'],
            campaign_level_cap=30,
            uses_custom_content=True,
            custom_content_ids={'classes': [100], 'feats': [2000]}
        )
    
    @pytest.fixture
    def module_character(self, db, user):
        """Create a character with custom module content"""
        return Character.objects.create(
            owner=user,
            file_name='module_char.ros',
            file_path='/modules/custom_module/module_char.ros',
            is_companion=True,
            first_name='Custom',
            last_name='Companion',
            race_id=3,
            race_name='Halfling',
            character_level=10,
            module_name='CustomAdventure',
            uses_custom_content=True,
            module_hakpaks=['custom_classes.hak', 'custom_items.hak'],
            custom_content_ids={
                'classes': [50, 51],
                'feats': [1000, 1001],
                'spells': [3000],
                'items': [5000]
            }
        )
    
    # === Basic Model Tests ===
    
    def test_character_creation(self, basic_character):
        """Test basic character creation and field storage"""
        assert basic_character.id is not None
        assert basic_character.first_name == 'Test'
        assert basic_character.last_name == 'Character'
        assert basic_character.race_name == 'Human'
        assert basic_character.character_level == 5
        assert basic_character.experience == 10000
        assert basic_character.created_at is not None
        assert basic_character.updated_at is not None
    
    def test_character_str_representation(self, basic_character):
        """Test string representation of character"""
        assert str(basic_character) == "Test Character (Level 5)"
        
        # Test unnamed character
        unnamed = Character(first_name='', last_name='', character_level=1)
        assert str(unnamed) == "Unnamed (Level 1)"
    
    def test_alignment_property(self, basic_character):
        """Test alignment calculation from law_chaos and good_evil"""
        # Test neutral alignment (default)
        basic_character.law_chaos = 50
        basic_character.good_evil = 50
        assert basic_character.alignment == "True Neutral"
        
        # Test lawful good
        basic_character.law_chaos = 75
        basic_character.good_evil = 80
        assert basic_character.alignment == "Lawful Good"
        
        # Test chaotic evil
        basic_character.law_chaos = 20
        basic_character.good_evil = 15
        assert basic_character.alignment == "Chaotic Evil"
        
        # Test edge cases
        basic_character.law_chaos = 70  # Exactly at threshold
        basic_character.good_evil = 30  # Exactly at threshold
        assert basic_character.alignment == "Lawful Evil"
    
    def test_field_validators(self, db):
        """Test field validators work correctly"""
        # Test attribute validators
        with pytest.raises(ValidationError) as exc_info:
            char = Character(strength=2)  # Below minimum
            char.full_clean()
        assert 'strength' in str(exc_info.value)
        
        with pytest.raises(ValidationError) as exc_info:
            char = Character(dexterity=51)  # Above maximum
            char.full_clean()
        assert 'dexterity' in str(exc_info.value)
        
        # Test alignment validators
        with pytest.raises(ValidationError) as exc_info:
            char = Character(law_chaos=101)  # Above maximum
            char.full_clean()
        assert 'law_chaos' in str(exc_info.value)
        
        # Test level validators
        with pytest.raises(ValidationError) as exc_info:
            char = Character(character_level=0)  # Below minimum
            char.full_clean()
        assert 'character_level' in str(exc_info.value)
        
        # Test hit points validators
        with pytest.raises(ValidationError) as exc_info:
            char = Character(hit_points=0)  # Below minimum
            char.full_clean()
        assert 'hit_points' in str(exc_info.value)
    
    # === get_game_rules() Tests ===
    
    @patch('gamedata.game_rules_service_generated.GameRulesService')
    @patch('parsers.resource_manager.ResourceManager')
    def test_get_game_rules_base_game(self, mock_rm_class, mock_grs_class, basic_character):
        """Test get_game_rules returns base game rules when no custom content"""
        mock_service = Mock()
        mock_grs_class.return_value = mock_service
        
        # Character has no custom content
        basic_character.uses_custom_content = False
        
        rules = basic_character.get_game_rules()
        
        # Should create service with default resource manager
        mock_grs_class.assert_called_once_with()
        assert rules == mock_service
        
        # Test caching - second call should return same instance
        # With the improvement, base game rules are now cached
        rules2 = basic_character.get_game_rules()
        assert rules2 == mock_service
        assert mock_grs_class.call_count == 1  # Not called again, uses cache
    
    @patch('gamedata.game_rules_service_generated.GameRulesService')
    @patch('parsers.resource_manager.ResourceManager')
    def test_get_game_rules_with_module(self, mock_rm_class, mock_grs_class, module_character):
        """Test get_game_rules loads module-specific rules"""
        mock_rm = Mock()
        mock_rm_class.return_value = mock_rm
        mock_rm.find_module.return_value = '/path/to/module'
        mock_rm.set_module.return_value = True
        
        mock_service = Mock()
        mock_grs_class.return_value = mock_service
        
        rules = module_character.get_game_rules()
        
        # Should create resource manager with module
        mock_rm_class.assert_called_once_with(suppress_warnings=True)
        mock_rm.find_module.assert_called_once_with('CustomAdventure')
        mock_rm.set_module.assert_called_once_with('/path/to/module')
        
        # Should create service with module-aware resource manager
        mock_grs_class.assert_called_once_with(resource_manager=mock_rm)
        assert rules == mock_service
    
    def test_module_game_rules_caching(self, module_character):
        """Test that module game rules are cached properly"""
        with patch('gamedata.game_rules_service_generated.GameRulesService') as mock_grs:
            with patch('parsers.resource_manager.ResourceManager') as mock_rm:
                mock_service = Mock()
                mock_grs.return_value = mock_service
                
                # First call
                rules1 = module_character.get_game_rules()
                assert module_character._module_game_rules == mock_service
                
                # Second call should use cache
                rules2 = module_character.get_game_rules()
                assert rules2 == rules1
                
                # Resource manager should only be created once
                mock_rm.assert_called_once()
                mock_grs.assert_called_once()
    
    @patch('gamedata.game_rules_service_generated.GameRulesService')
    @patch('parsers.resource_manager.ResourceManager')
    def test_get_game_rules_module_not_found(self, mock_rm_class, mock_grs_class, module_character):
        """Test get_game_rules handles missing module gracefully"""
        mock_rm = Mock()
        mock_rm_class.return_value = mock_rm
        mock_rm.find_module.return_value = None  # Module not found
        
        mock_service = Mock()
        mock_grs_class.return_value = mock_service
        
        rules = module_character.get_game_rules()
        
        # Should still create service even if module not found
        mock_grs_class.assert_called_once_with(resource_manager=mock_rm)
        assert rules == mock_service
    
    # === Campaign Support Tests ===
    
    def test_is_from_campaign(self, basic_character, campaign_character):
        """Test is_from_campaign method"""
        assert not basic_character.is_from_campaign()
        assert campaign_character.is_from_campaign()
    
    def test_get_campaign_display_name(self, campaign_character):
        """Test campaign display name mapping"""
        # Test known campaign
        assert campaign_character.get_campaign_display_name() == 'Mask of the Betrayer'
        
        # Test original campaign
        campaign_character.campaign_name = 'Neverwinter Nights 2 Campaign'
        assert campaign_character.get_campaign_display_name() == 'Original Campaign'
        
        # Test unknown campaign (returns as-is)
        campaign_character.campaign_name = 'Custom Campaign Name'
        assert campaign_character.get_campaign_display_name() == 'Custom Campaign Name'
    
    def test_get_campaign_progress(self, campaign_character):
        """Test campaign progress calculation"""
        progress = campaign_character.get_campaign_progress()
        
        assert progress is not None
        assert progress['current_module'] == '2100_mulsantir'
        assert progress['current_index'] == 2  # Second module (0-indexed + 1)
        assert progress['total_modules'] == 3
        assert progress['progress_percent'] == 66  # 2/3 * 100
        assert progress['modules_completed'] == 1
        assert progress['modules_remaining'] == 1
    
    def test_get_campaign_progress_no_module(self, campaign_character):
        """Test campaign progress when module not set"""
        campaign_character.module_name = ''
        assert campaign_character.get_campaign_progress() is None
        
        campaign_character.module_name = 'unknown_module'
        assert campaign_character.get_campaign_progress() is None
    
    def test_get_campaign_progress_no_campaign(self, basic_character):
        """Test campaign progress for non-campaign character"""
        assert basic_character.get_campaign_progress() is None
    
    # === Character Class Methods Tests ===
    
    def test_get_primary_class_name(self, basic_character):
        """Test getting primary class name"""
        # Create character classes
        CharacterClass.objects.create(
            character=basic_character,
            class_id=0,
            class_name='Fighter',
            class_level=3
        )
        CharacterClass.objects.create(
            character=basic_character,
            class_id=6,
            class_name='Ranger',
            class_level=2
        )
        
        with patch.object(basic_character, 'get_game_rules') as mock_rules:
            # Mock the rules service
            mock_fighter = Mock()
            mock_fighter.name = 'Fighter'
            mock_ranger = Mock()
            mock_ranger.name = 'Ranger'
            
            mock_rules.return_value.classes = {
                0: mock_fighter,
                6: mock_ranger
            }
            
            # Should return highest level class
            assert basic_character.get_primary_class_name() == 'Fighter'
    
    def test_get_primary_class_name_no_classes(self, basic_character):
        """Test getting primary class name when no classes exist"""
        with patch.object(basic_character, 'get_game_rules') as mock_rules:
            assert basic_character.get_primary_class_name() == 'Unknown'
    
    # === Validation Tests ===
    
    def test_validate_character_data_valid(self, basic_character):
        """Test character validation with valid data"""
        # Add some valid character data
        CharacterClass.objects.create(
            character=basic_character,
            class_id=0,
            class_name='Fighter',
            class_level=5
        )
        CharacterFeat.objects.create(
            character=basic_character,
            feat_id=1,
            feat_name='Weapon Focus'
        )
        CharacterSkill.objects.create(
            character=basic_character,
            skill_id=5,
            skill_name='Intimidate',
            rank=8
        )
        CharacterSpell.objects.create(
            character=basic_character,
            spell_id=100,
            spell_name='Magic Missile',
            spell_level=1
        )
        CharacterItem.objects.create(
            character=basic_character,
            base_item_id=10,
            base_item_name='Longsword',
            location='RIGHT_HAND'
        )
        
        with patch.object(basic_character, 'get_game_rules') as mock_rules:
            # Mock valid IDs in game rules
            mock_rules.return_value.classes = {0: Mock()}
            mock_rules.return_value.feats = {1: Mock()}
            mock_rules.return_value.races = {1: Mock()}
            mock_rules.return_value.skills = {5: Mock()}
            mock_rules.return_value.spells = {100: Mock()}
            mock_rules.return_value.base_items = {10: Mock()}
            
            errors = basic_character.validate_character_data()
            assert errors == []
    
    def test_validate_character_data_invalid(self, basic_character):
        """Test character validation with invalid data"""
        # Add invalid character data
        CharacterClass.objects.create(
            character=basic_character,
            class_id=999,  # Invalid ID
            class_name='CustomClass',
            class_level=5
        )
        CharacterFeat.objects.create(
            character=basic_character,
            feat_id=9999,  # Invalid ID
            feat_name='CustomFeat'
        )
        basic_character.race_id = 888  # Invalid race
        
        with patch.object(basic_character, 'get_game_rules') as mock_rules:
            # Mock empty game rules
            mock_rules.return_value.classes = {}
            mock_rules.return_value.feats = {}
            mock_rules.return_value.races = {}
            mock_rules.return_value.skills = {}
            mock_rules.return_value.spells = {}
            mock_rules.return_value.base_items = {}
            
            errors = basic_character.validate_character_data()
            
            assert len(errors) == 3
            assert any('Unknown class ID: 999' in e for e in errors)
            assert any('Unknown feat ID: 9999' in e for e in errors)
            assert any('Unknown race ID: 888' in e for e in errors)
    
    def test_validate_character_with_custom_content(self, module_character):
        """Test validation correctly uses module-specific rules"""
        # Add custom content
        CharacterClass.objects.create(
            character=module_character,
            class_id=50,  # Custom class from module
            class_name='CustomPrestige',
            class_level=5
        )
        
        with patch.object(module_character, 'get_game_rules') as mock_rules:
            # Mock module includes the custom class
            mock_rules.return_value.classes = {50: Mock()}
            mock_rules.return_value.feats = {}
            mock_rules.return_value.races = {3: Mock()}  # Halfling
            mock_rules.return_value.skills = {}
            mock_rules.return_value.spells = {}
            mock_rules.return_value.base_items = {}
            
            errors = module_character.validate_character_data()
            # Should be valid because module rules include class 50
            assert errors == []
    
    # === create_from_file Tests ===
    
    def test_create_from_file_bic(self, user):
        """Test creating character from .bic file - this functionality has moved to FastAPI"""
        # Note: Character.create_from_file() was Django-specific functionality
        # In FastAPI, character creation is handled by session registry and savegame router
        # This test is kept for legacy compatibility but the actual method would need to be removed
        
        with pytest.raises(AttributeError):
            # The create_from_file method should no longer exist on the Character model
            # as character import is now handled by FastAPI routers
            Character.create_from_file('/path/to/char.bic', owner=user)
    
    def test_create_from_file_savegame_dir(self, user):
        """Test creating character from save game directory - this functionality has moved to FastAPI"""
        # Note: Character.create_from_file() was Django-specific functionality
        # In FastAPI, savegame import is handled by fastapi_routers.savegame.import_savegame()
        # This test is kept for legacy compatibility but the actual method would need to be removed
        
        with pytest.raises(AttributeError):
            # The create_from_file method should no longer exist on the Character model
            Character.create_from_file('/path/to/savegame/', owner=user)
    
    def test_create_from_file_savegame_zip(self, user):
        """Test creating character from save game zip - this functionality has moved to FastAPI"""
        # Note: Character.create_from_file() was Django-specific functionality
        # In FastAPI, savegame import is handled by fastapi_routers.savegame.import_savegame()
        # This test is kept for legacy compatibility but the actual method would need to be removed
        
        with pytest.raises(AttributeError):
            # The create_from_file method should no longer exist on the Character model
            Character.create_from_file('/path/to/savegame.zip', owner=user)
    
    # === Database Operation Tests ===
    
    def test_character_deletion_cascades(self, basic_character):
        """Test that deleting character cascades to related models"""
        # Create related objects
        char_class = CharacterClass.objects.create(
            character=basic_character,
            class_id=0,
            class_name='Fighter',
            class_level=5
        )
        char_feat = CharacterFeat.objects.create(
            character=basic_character,
            feat_id=1,
            feat_name='Power Attack'
        )
        char_skill = CharacterSkill.objects.create(
            character=basic_character,
            skill_id=1,
            skill_name='Concentration',
            rank=5
        )
        char_spell = CharacterSpell.objects.create(
            character=basic_character,
            spell_id=1,
            spell_name='Burning Hands',
            spell_level=1
        )
        char_item = CharacterItem.objects.create(
            character=basic_character,
            base_item_id=1,
            base_item_name='Dagger'
        )
        
        # Delete character
        basic_character.delete()
        
        # Check all related objects are deleted
        assert not CharacterClass.objects.filter(id=char_class.id).exists()
        assert not CharacterFeat.objects.filter(id=char_feat.id).exists()
        assert not CharacterSkill.objects.filter(id=char_skill.id).exists()
        assert not CharacterSpell.objects.filter(id=char_spell.id).exists()
        assert not CharacterItem.objects.filter(id=char_item.id).exists()
    
    def test_character_ordering(self, user, db):
        """Test characters are ordered by updated_at descending"""
        char1 = Character.objects.create(
            owner=user,
            file_name='char1.bic',
            file_path='/char1.bic',
            first_name='First'
        )
        
        # Create second character later
        import time
        time.sleep(0.1)
        
        char2 = Character.objects.create(
            owner=user,
            file_name='char2.bic',
            file_path='/char2.bic',
            first_name='Second'
        )
        
        # Get all characters
        chars = list(Character.objects.all())
        
        # Most recently updated should be first
        assert chars[0] == char2
        assert chars[1] == char1
    
    def test_character_indexes(self, basic_character):
        """Test database indexes are created properly"""
        # This test verifies the Meta.indexes configuration
        # The actual index creation is handled by Django migrations
        meta = Character._meta
        
        # Check indexes are defined
        assert len(meta.indexes) > 0
        
        # Check for owner, updated_at index
        index_fields = [idx.fields for idx in meta.indexes]
        assert ['owner', '-updated_at'] in index_fields
    
    # === JSON Field Tests ===
    
    def test_json_field_defaults(self, basic_character):
        """Test JSON fields have proper defaults"""
        assert basic_character.armor_tint == {}
        assert basic_character.model_scale == {}
        assert basic_character.action_list == {}
        assert basic_character.combat_info == {}
        assert basic_character.module_hakpaks == []
        assert basic_character.custom_content_ids == {}
        assert basic_character.campaign_modules == []
    
    def test_json_field_storage(self, db, user):
        """Test JSON fields store complex data correctly"""
        complex_data = {
            'armor_tint': {'r': 255, 'g': 128, 'b': 0},
            'model_scale': {'x': 1.0, 'y': 1.0, 'z': 1.2},
            'action_list': [
                {'action': 'attack', 'target': 123},
                {'action': 'cast', 'spell': 456}
            ],
            'custom_content_ids': {
                'classes': [100, 101],
                'feats': [2000, 2001, 2002],
                'nested': {'data': 'test'}
            }
        }
        
        char = Character.objects.create(
            owner=user,
            file_name='json_test.bic',
            file_path='/json_test.bic',
            **complex_data
        )
        
        # Reload from database
        char.refresh_from_db()
        
        # Verify data integrity
        assert char.armor_tint == complex_data['armor_tint']
        assert char.model_scale == complex_data['model_scale']
        assert char.action_list == complex_data['action_list']
        assert char.custom_content_ids == complex_data['custom_content_ids']
    
    # === Edge Cases and Error Handling ===
    
    def test_character_with_long_names(self, db, user):
        """Test character with very long names"""
        long_name = 'A' * 100  # Max length
        char = Character.objects.create(
            owner=user,
            file_name='long.bic',
            file_path='/long.bic',
            first_name=long_name,
            last_name=long_name
        )
        
        assert char.first_name == long_name
        assert char.last_name == long_name
    
    def test_character_without_owner(self, db):
        """Test character can be created without owner"""
        char = Character.objects.create(
            file_name='ownerless.bic',
            file_path='/ownerless.bic',
            first_name='Ownerless',
            owner=None  # Explicitly no owner
        )
        
        assert char.owner is None
        assert char.first_name == 'Ownerless'
    
    def test_companion_flag(self, db, user):
        """Test is_companion flag based on file extension"""
        # .ros file should be companion
        ros_char = Character.objects.create(
            owner=user,
            file_name='companion.ros',
            file_path='/companion.ros',
            is_companion=True
        )
        assert ros_char.is_companion
        
        # .bic file should not be companion
        bic_char = Character.objects.create(
            owner=user,
            file_name='player.bic',
            file_path='/player.bic',
            is_companion=False
        )
        assert not bic_char.is_companion
    
    def test_module_name_edge_cases(self, db, user):
        """Test edge cases for module name handling"""
        # Empty module name
        char1 = Character.objects.create(
            owner=user,
            file_name='test1.bic',
            file_path='/test1.bic',
            module_name='',
            uses_custom_content=False
        )
        assert char1.module_name == ''
        assert not char1.uses_custom_content
        
        # Very long module name
        long_module = 'VeryLongModuleNameThatExceedsNormalLength' * 10  # Make it longer to ensure 255+ chars
        char2 = Character.objects.create(
            owner=user,
            file_name='test2.bic',
            file_path='/test2.bic',
            module_name=long_module[:255],  # Truncate to field max
            uses_custom_content=True
        )
        assert len(char2.module_name) == 255
    
    def test_special_characters_in_fields(self, db, user):
        """Test handling of special characters in text fields"""
        special_chars = "Test'Name\"With<Special>Chars&Symbols"
        char = Character.objects.create(
            owner=user,
            file_name='special.bic',
            file_path='/special.bic',
            first_name=special_chars,
            deity=special_chars,
            tag=special_chars[:32]  # Tag field is limited to 32 chars
        )
        
        assert char.first_name == special_chars
        assert char.deity == special_chars
        assert char.tag == special_chars[:32]
    
    def test_float_field_precision(self, db, user):
        """Test float fields maintain precision"""
        char = Character.objects.create(
            owner=user,
            file_name='float_test.bic',
            file_path='/float_test.bic',
            challenge_rating=12.75,
            xp_mod=1.25,
            x_position=123.456789,
            y_position=-987.654321,
            x_orientation=0.123456789
        )
        
        char.refresh_from_db()
        
        assert char.challenge_rating == 12.75
        assert char.xp_mod == 1.25
        # Float fields might lose some precision
        assert abs(char.x_position - 123.456789) < 0.0001
        assert abs(char.y_position - (-987.654321)) < 0.0001
    
    # === New Model Methods Tests ===
    
    def test_get_ability_modifier(self, basic_character):
        """Test ability modifier calculation"""
        # Test various ability scores
        basic_character.strength = 16
        basic_character.dexterity = 14
        basic_character.constitution = 15
        basic_character.intelligence = 12
        basic_character.wisdom = 10
        basic_character.charisma = 8
        
        # Test modifiers
        assert basic_character.get_ability_modifier('STR') == 3  # (16-10)//2 = 3
        assert basic_character.get_ability_modifier('strength') == 3  # lowercase
        assert basic_character.get_ability_modifier('STRENGTH') == 3  # uppercase
        assert basic_character.get_ability_modifier('DEX') == 2  # (14-10)//2 = 2
        assert basic_character.get_ability_modifier('CON') == 2  # (15-10)//2 = 2.5 -> 2
        assert basic_character.get_ability_modifier('INT') == 1  # (12-10)//2 = 1
        assert basic_character.get_ability_modifier('WIS') == 0  # (10-10)//2 = 0
        assert basic_character.get_ability_modifier('CHA') == -1  # (8-10)//2 = -1
        
        # Test edge cases
        basic_character.strength = 3  # Minimum
        assert basic_character.get_ability_modifier('STR') == -4  # (3-10)//2 = -3.5 -> -4
        
        basic_character.strength = 50  # Maximum
        assert basic_character.get_ability_modifier('STR') == 20  # (50-10)//2 = 20
    
    def test_calculate_total_level(self, basic_character):
        """Test total level calculation from classes"""
        # No classes yet
        assert basic_character.calculate_total_level() == 5  # Falls back to character_level
        
        # Add classes
        CharacterClass.objects.create(
            character=basic_character,
            class_id=0,
            class_name='Fighter',
            class_level=3
        )
        CharacterClass.objects.create(
            character=basic_character,
            class_id=1,
            class_name='Wizard',
            class_level=2
        )
        
        assert basic_character.calculate_total_level() == 5  # 3 + 2
    
    def test_calculate_saves(self, basic_character):
        """Test save calculation with ability modifiers"""
        # Set abilities
        basic_character.constitution = 16  # +3 bonus
        basic_character.dexterity = 14    # +2 bonus
        basic_character.wisdom = 13        # +1 bonus
        
        # Set base saves
        basic_character.fortitude_save = 5
        basic_character.reflex_save = 3
        basic_character.will_save = 4
        
        # Set save bonuses from items/feats
        basic_character.fortbonus = 2
        basic_character.refbonus = 1
        basic_character.willbonus = 0
        
        saves = basic_character.calculate_saves()
        
        assert saves['fortitude'] == 10  # 5 (base) + 3 (CON) + 2 (bonus)
        assert saves['reflex'] == 6      # 3 (base) + 2 (DEX) + 1 (bonus)
        assert saves['will'] == 5        # 4 (base) + 1 (WIS) + 0 (bonus)
    
    def test_can_take_feat(self, basic_character):
        """Test feat prerequisite checking"""
        # Mock game rules
        with patch.object(basic_character, 'get_game_rules') as mock_rules:
            mock_feat = Mock(id=100, label='Test Feat')
            mock_rules.return_value.feats = {100: mock_feat}
            
            # Test can take new feat
            can_take, errors = basic_character.can_take_feat(100)
            assert can_take is True
            assert errors == []
            
            # Add the feat
            CharacterFeat.objects.create(
                character=basic_character,
                feat_id=100,
                feat_name='Test Feat'
            )
            
            # Test already has feat
            can_take, errors = basic_character.can_take_feat(100)
            assert can_take is False
            assert 'Character already has this feat' in errors[0]
            
            # Test unknown feat
            can_take, errors = basic_character.can_take_feat(999)
            assert can_take is False
            assert 'Unknown feat ID: 999' in errors[0]
    
    def test_get_available_skill_points(self, basic_character):
        """Test available skill point calculation"""
        # Set total skill points
        basic_character.skill_points = 50
        
        # No skills allocated yet
        assert basic_character.get_available_skill_points() == 50
        
        # Add some skills
        CharacterSkill.objects.create(
            character=basic_character,
            skill_id=1,
            skill_name='Concentration',
            rank=10
        )
        CharacterSkill.objects.create(
            character=basic_character,
            skill_id=2,
            skill_name='Hide',
            rank=15
        )
        
        # 50 - 10 - 15 = 25 remaining
        assert basic_character.get_available_skill_points() == 25
    
    def test_get_spell_slots(self, basic_character):
        """Test spell slot calculation (placeholder)"""
        slots = basic_character.get_spell_slots(0)
        
        # Currently returns placeholder data
        assert isinstance(slots, dict)
        assert all(level in slots for level in range(10))
        assert all(slots[level] == 0 for level in range(10))
    
    def test_clean_method_alignment_validation(self, basic_character):
        """Test model clean method validates alignment restrictions"""
        # Add a paladin class
        CharacterClass.objects.create(
            character=basic_character,
            class_id=3,  # Paladin
            class_name='Paladin',
            class_level=5
        )
        
        # Set non-lawful good alignment
        basic_character.law_chaos = 50  # Neutral
        basic_character.good_evil = 50  # Neutral
        
        # Should raise validation error
        with pytest.raises(ValidationError) as exc_info:
            basic_character.clean()
        assert 'Paladins must be Lawful Good' in str(exc_info.value)
        
        # Fix alignment
        basic_character.law_chaos = 75  # Lawful
        basic_character.good_evil = 75  # Good
        
        # Should pass validation
        basic_character.clean()  # No exception
    
    def test_clean_method_level_sync(self, basic_character):
        """Test model clean method syncs character level with class levels"""
        # Store original level
        original_level = basic_character.character_level
        assert original_level == 5
        
        # Add classes that don't match character_level
        CharacterClass.objects.create(
            character=basic_character,
            class_id=0,
            class_name='Fighter',
            class_level=7
        )
        CharacterClass.objects.create(
            character=basic_character,
            class_id=1,
            class_name='Wizard',
            class_level=3
        )
        
        # The signal should have already synced it
        basic_character.refresh_from_db()
        assert basic_character.character_level == 10  # Signal auto-synced
        
        # Manually set incorrect level
        basic_character.character_level = 15
        basic_character.save()
        
        # Clean should fix it
        basic_character.clean()
        assert basic_character.character_level == 10  # Fixed by clean
    
    def test_export_to_gff(self, basic_character):
        """Test exporting character data to GFF format"""
        # Add some character data
        CharacterClass.objects.create(
            character=basic_character,
            class_id=0,
            class_name='Fighter',
            class_level=5
        )
        CharacterFeat.objects.create(
            character=basic_character,
            feat_id=1,
            feat_name='Power Attack'
        )
        CharacterSkill.objects.create(
            character=basic_character,
            skill_id=1,
            skill_name='Concentration',
            rank=8
        )
        
        # Mock GFF components
        with patch('parsers.gff.GFFWriter') as mock_writer_class:
            with patch('parsers.gff.GFFElement') as mock_element_class:
                mock_element = Mock()
                mock_element_class.return_value = mock_element
                
                # Export without output path
                result = basic_character.export_to_gff()
                
                # Should return GFF element
                assert result == mock_element
                
                # Verify fields were added
                assert mock_element.add_field.called
                
                # Check some key fields
                calls = mock_element.add_field.call_args_list
                field_names = [call[0][0] for call in calls]
                
                assert 'FirstName' in field_names
                assert 'LastName' in field_names
                assert 'Race' in field_names
                assert 'Str' in field_names
                assert 'ClassList' in field_names
                assert 'FeatList' in field_names
                
        # Test with output path
        with patch('parsers.gff.GFFWriter') as mock_writer_class:
            with patch('parsers.gff.GFFElement'):
                mock_writer = Mock()
                mock_writer_class.return_value = mock_writer
                
                output_path = '/tmp/test_export.bic'
                result = basic_character.export_to_gff(output_path)
                
                # Should return output path
                assert result == output_path
                
                # Should write to file
                mock_writer.write.assert_called_once()
                call_args = mock_writer.write.call_args[0]
                assert call_args[0] == output_path
                assert call_args[2] == 'BIC V3.2'
    
    # === Custom Manager Tests ===
    
    def test_character_manager_owned_by(self, user, db):
        """Test CharacterManager.owned_by method"""
        user2 = User.objects.create_user(username='user2')
        
        # Create characters for different users
        char1 = Character.objects.create(
            owner=user,
            file_name='char1.bic',
            file_path='/char1.bic'
        )
        char2 = Character.objects.create(
            owner=user,
            file_name='char2.bic',
            file_path='/char2.bic'
        )
        char3 = Character.objects.create(
            owner=user2,
            file_name='char3.bic',
            file_path='/char3.bic'
        )
        
        # Test owned_by
        user_chars = list(Character.objects.owned_by(user))
        assert len(user_chars) == 2
        assert char1 in user_chars
        assert char2 in user_chars
        assert char3 not in user_chars
    
    def test_character_manager_player_characters(self, db):
        """Test CharacterManager.player_characters method"""
        char1 = Character.objects.create(
            file_name='player.bic',
            file_path='/player.bic',
            is_companion=False
        )
        char2 = Character.objects.create(
            file_name='companion.ros',
            file_path='/companion.ros',
            is_companion=True
        )
        
        players = list(Character.objects.player_characters())
        assert len(players) == 1
        assert char1 in players
        assert char2 not in players
    
    def test_character_manager_companions(self, db):
        """Test CharacterManager.companions method"""
        char1 = Character.objects.create(
            file_name='player.bic',
            file_path='/player.bic',
            is_companion=False
        )
        char2 = Character.objects.create(
            file_name='companion.ros',
            file_path='/companion.ros',
            is_companion=True
        )
        
        companions = list(Character.objects.companions())
        assert len(companions) == 1
        assert char2 in companions
        assert char1 not in companions
    
    def test_character_manager_from_campaign(self, db):
        """Test CharacterManager.from_campaign method"""
        char1 = Character.objects.create(
            file_name='oc.bic',
            file_path='/oc.bic',
            campaign_name='Neverwinter Nights 2 Campaign'
        )
        char2 = Character.objects.create(
            file_name='motb.bic',
            file_path='/motb.bic',
            campaign_name='NWN2 Mask of the Betrayer Campaign'
        )
        char3 = Character.objects.create(
            file_name='custom.bic',
            file_path='/custom.bic',
            campaign_name=''
        )
        
        oc_chars = list(Character.objects.from_campaign('Neverwinter Nights 2 Campaign'))
        assert len(oc_chars) == 1
        assert char1 in oc_chars
    
    def test_character_manager_with_custom_content(self, db):
        """Test CharacterManager.with_custom_content method"""
        char1 = Character.objects.create(
            file_name='vanilla.bic',
            file_path='/vanilla.bic',
            uses_custom_content=False
        )
        char2 = Character.objects.create(
            file_name='custom.bic',
            file_path='/custom.bic',
            uses_custom_content=True
        )
        
        custom_chars = list(Character.objects.with_custom_content())
        assert len(custom_chars) == 1
        assert char2 in custom_chars
        assert char1 not in custom_chars
    
    def test_character_manager_high_level(self, db):
        """Test CharacterManager.high_level method"""
        char1 = Character.objects.create(
            file_name='low.bic',
            file_path='/low.bic',
            character_level=5
        )
        char2 = Character.objects.create(
            file_name='mid.bic',
            file_path='/mid.bic',
            character_level=15
        )
        char3 = Character.objects.create(
            file_name='high.bic',
            file_path='/high.bic',
            character_level=25
        )
        
        # Default min_level=20
        high_chars = list(Character.objects.high_level())
        assert len(high_chars) == 1
        assert char3 in high_chars
        
        # Custom min_level
        mid_high_chars = list(Character.objects.high_level(min_level=15))
        assert len(mid_high_chars) == 2
        assert char2 in mid_high_chars
        assert char3 in mid_high_chars
    
    def test_character_manager_by_class(self, db):
        """Test CharacterManager.by_class method"""
        char1 = Character.objects.create(
            file_name='fighter.bic',
            file_path='/fighter.bic'
        )
        char2 = Character.objects.create(
            file_name='wizard.bic',
            file_path='/wizard.bic'
        )
        char3 = Character.objects.create(
            file_name='multi.bic',
            file_path='/multi.bic'
        )
        
        # Add classes
        CharacterClass.objects.create(
            character=char1,
            class_id=0,  # Fighter
            class_name='Fighter',
            class_level=5
        )
        CharacterClass.objects.create(
            character=char2,
            class_id=1,  # Wizard
            class_name='Wizard',
            class_level=5
        )
        CharacterClass.objects.create(
            character=char3,
            class_id=0,  # Fighter
            class_name='Fighter',
            class_level=3
        )
        CharacterClass.objects.create(
            character=char3,
            class_id=1,  # Wizard
            class_name='Wizard',
            class_level=2
        )
        
        # Find all fighters
        fighters = list(Character.objects.by_class(0))
        assert len(fighters) == 2
        assert char1 in fighters
        assert char3 in fighters
        assert char2 not in fighters
    
    def test_character_manager_with_related(self, db):
        """Test CharacterManager.with_related method"""
        char = Character.objects.create(
            file_name='test.bic',
            file_path='/test.bic'
        )
        
        # Add related objects
        CharacterClass.objects.create(
            character=char,
            class_id=0,
            class_name='Fighter',
            class_level=5
        )
        CharacterFeat.objects.create(
            character=char,
            feat_id=1,
            feat_name='Power Attack'
        )
        
        # Query with prefetch - test that it doesn't cause extra queries
        from django.test.utils import override_settings
        from django.db import connection
        from django.test import TestCase
        
        # Track queries
        initial_queries = len(connection.queries)
        
        char_with_related = Character.objects.with_related().get(id=char.id)
        # Access related objects (should not cause additional queries due to prefetch)
        classes = list(char_with_related.classes.all())
        feats = list(char_with_related.feats.all())
        
        # Should only have made the initial queries (character + prefetch queries)
        total_queries = len(connection.queries) - initial_queries
        assert total_queries <= 3  # 1 for character, up to 2 for prefetch
        
        assert len(classes) == 1
        assert len(feats) == 1
    
    # === Signal Tests ===
    
    def test_character_level_sync_signal(self, basic_character):
        """Test signal that syncs character level with class levels"""
        # Character starts at level 5
        assert basic_character.character_level == 5
        
        # Add a class
        CharacterClass.objects.create(
            character=basic_character,
            class_id=0,
            class_name='Fighter',
            class_level=8
        )
        
        # Refresh from DB
        basic_character.refresh_from_db()
        
        # Character level should be updated
        assert basic_character.character_level == 8
        
        # Add another class
        CharacterClass.objects.create(
            character=basic_character,
            class_id=1,
            class_name='Wizard',
            class_level=2
        )
        
        # Refresh from DB
        basic_character.refresh_from_db()
        
        # Character level should be sum
        assert basic_character.character_level == 10  # 8 + 2
    
    def test_character_validation_on_save_signal(self, basic_character):
        """Test pre_save signal runs validation"""
        # Add a paladin class
        CharacterClass.objects.create(
            character=basic_character,
            class_id=3,  # Paladin
            class_name='Paladin',
            class_level=5
        )
        
        # Set invalid alignment for paladin
        basic_character.law_chaos = 30  # Not lawful
        basic_character.good_evil = 30  # Not good
        
        # Should raise validation error on save
        with pytest.raises(ValidationError) as exc_info:
            basic_character.save()
        assert 'Paladins must be Lawful Good' in str(exc_info.value)
    
    # === Database Constraint Tests ===
    
    def test_model_constraints(self):
        """Test database constraints are defined"""
        meta = Character._meta
        
        # Check constraints are defined
        assert len(meta.constraints) > 0
        
        # Check specific constraints
        constraint_names = [c.name for c in meta.constraints]
        assert 'valid_law_chaos' in constraint_names
        assert 'valid_good_evil' in constraint_names
        assert 'valid_character_level' in constraint_names
        assert 'valid_strength' in constraint_names
        assert 'valid_dexterity' in constraint_names
        assert 'valid_constitution' in constraint_names
        assert 'valid_intelligence' in constraint_names
        assert 'valid_wisdom' in constraint_names
        assert 'valid_charisma' in constraint_names


class TestCharacterClass:
    """Test the CharacterClass model"""
    
    @pytest.fixture
    def character(self, db):
        """Create a basic character for testing"""
        return Character.objects.create(
            file_name='test.bic',
            file_path='/test.bic',
            first_name='Test'
        )
    
    def test_character_class_creation(self, character):
        """Test creating character class"""
        char_class = CharacterClass.objects.create(
            character=character,
            class_id=0,
            class_name='Fighter',
            class_level=5
        )
        
        assert char_class.character == character
        assert char_class.class_id == 0
        assert char_class.class_name == 'Fighter'
        assert char_class.class_level == 5
        assert str(char_class) == 'Fighter 5'
    
    def test_character_class_with_domains(self, character):
        """Test character class with divine caster domains"""
        char_class = CharacterClass.objects.create(
            character=character,
            class_id=2,
            class_name='Cleric',
            class_level=10,
            domain1_id=1,
            domain1_name='Good',
            domain2_id=5,
            domain2_name='Healing'
        )
        
        assert char_class.domain1_name == 'Good'
        assert char_class.domain2_name == 'Healing'
    
    def test_character_class_ordering(self, character):
        """Test character classes are ordered by level descending, then name"""
        # Create classes in specific order
        fighter = CharacterClass.objects.create(
            character=character,
            class_id=0,
            class_name='Fighter',
            class_level=5
        )
        wizard = CharacterClass.objects.create(
            character=character,
            class_id=10,
            class_name='Wizard',
            class_level=5
        )
        rogue = CharacterClass.objects.create(
            character=character,
            class_id=7,
            class_name='Rogue',
            class_level=10
        )
        
        classes = list(character.classes.all())
        
        # Should be ordered by level desc, then name
        assert classes[0] == rogue  # Level 10
        assert classes[1] == fighter  # Level 5, alphabetically first
        assert classes[2] == wizard  # Level 5, alphabetically second
    
    def test_character_class_auto_populate_name(self, character):
        """Test class name is auto-populated from game rules if not set"""
        with patch.object(character, 'get_game_rules') as mock_get_rules:
            mock_rules = Mock()
            mock_class = Mock()
            mock_class.name = 'Warlock'
            mock_rules.classes = {
                1: mock_class
            }
            mock_get_rules.return_value = mock_rules
            
            char_class = CharacterClass(
                character=character,
                class_id=1,  # Wizard - no alignment restrictions
                class_level=7
                # No class_name provided
            )
            char_class.save()
            
            assert char_class.class_name == 'Warlock'
    
    @patch.object(Character, 'get_game_rules')
    def test_character_class_unknown_class(self, mock_get_rules, character):
        """Test handling of unknown class ID"""
        mock_rules = Mock()
        mock_rules.classes = {}  # No classes
        mock_get_rules.return_value = mock_rules
        
        char_class = CharacterClass(
            character=character,
            class_id=999,
            class_level=1
        )
        char_class.save()
        
        assert char_class.class_name == 'Unknown_999'
    
    def test_character_class_level_validation(self, character):
        """Test class level validators"""
        with pytest.raises(ValidationError) as exc_info:
            char_class = CharacterClass(
                character=character,
                class_id=0,
                class_name='Fighter',
                class_level=0  # Below minimum
            )
            char_class.full_clean()
        assert 'class_level' in str(exc_info.value)
        
        with pytest.raises(ValidationError) as exc_info:
            char_class = CharacterClass(
                character=character,
                class_id=0,
                class_name='Fighter',
                class_level=31  # Above maximum
            )
            char_class.full_clean()
        assert 'class_level' in str(exc_info.value)


class TestCharacterFeat:
    """Test the CharacterFeat model"""
    
    @pytest.fixture
    def character(self, db):
        """Create a basic character for testing"""
        return Character.objects.create(
            file_name='test.bic',
            file_path='/test.bic',
            first_name='Test'
        )
    
    def test_character_feat_creation(self, character):
        """Test creating character feat"""
        feat = CharacterFeat.objects.create(
            character=character,
            feat_id=1,
            feat_name='Power Attack'
        )
        
        assert feat.character == character
        assert feat.feat_id == 1
        assert feat.feat_name == 'Power Attack'
        assert str(feat) == 'Power Attack'
    
    def test_character_feat_ordering(self, character):
        """Test feats are ordered by name"""
        # Create feats in random order
        feats = [
            CharacterFeat.objects.create(
                character=character,
                feat_id=3,
                feat_name='Weapon Focus'
            ),
            CharacterFeat.objects.create(
                character=character,
                feat_id=1,
                feat_name='Power Attack'
            ),
            CharacterFeat.objects.create(
                character=character,
                feat_id=2,
                feat_name='Cleave'
            )
        ]
        
        ordered_feats = list(character.feats.all())
        
        assert ordered_feats[0].feat_name == 'Cleave'
        assert ordered_feats[1].feat_name == 'Power Attack'
        assert ordered_feats[2].feat_name == 'Weapon Focus'


class TestCharacterSkill:
    """Test the CharacterSkill model"""
    
    @pytest.fixture
    def character(self, db):
        """Create a basic character for testing"""
        return Character.objects.create(
            file_name='test.bic',
            file_path='/test.bic',
            first_name='Test'
        )
    
    def test_character_skill_creation(self, character):
        """Test creating character skill"""
        skill = CharacterSkill.objects.create(
            character=character,
            skill_id=5,
            skill_name='Concentration',
            rank=10
        )
        
        assert skill.character == character
        assert skill.skill_id == 5
        assert skill.skill_name == 'Concentration'
        assert skill.rank == 10
        assert str(skill) == 'Concentration: 10'
    
    def test_character_skill_rank_validation(self, character):
        """Test skill rank validators"""
        with pytest.raises(ValidationError) as exc_info:
            skill = CharacterSkill(
                character=character,
                skill_id=1,
                skill_name='Test',
                rank=-1  # Below minimum
            )
            skill.full_clean()
        assert 'rank' in str(exc_info.value)
        
        with pytest.raises(ValidationError) as exc_info:
            skill = CharacterSkill(
                character=character,
                skill_id=1,
                skill_name='Test',
                rank=51  # Above maximum
            )
            skill.full_clean()
        assert 'rank' in str(exc_info.value)


class TestCharacterSpell:
    """Test the CharacterSpell model"""
    
    @pytest.fixture
    def character(self, db):
        """Create a basic character for testing"""
        return Character.objects.create(
            file_name='test.bic',
            file_path='/test.bic',
            first_name='Test'
        )
    
    def test_character_spell_creation(self, character):
        """Test creating character spell"""
        spell = CharacterSpell.objects.create(
            character=character,
            spell_id=100,
            spell_name='Fireball',
            spell_level=3,
            class_index=0,
            is_memorized=True
        )
        
        assert spell.character == character
        assert spell.spell_id == 100
        assert spell.spell_name == 'Fireball'
        assert spell.spell_level == 3
        assert spell.is_memorized
        assert str(spell) == 'L3: Fireball'
    
    def test_character_spell_ordering(self, character):
        """Test spells are ordered by level then name"""
        spells = [
            CharacterSpell.objects.create(
                character=character,
                spell_id=1,
                spell_name='Magic Missile',
                spell_level=1
            ),
            CharacterSpell.objects.create(
                character=character,
                spell_id=2,
                spell_name='Fireball',
                spell_level=3
            ),
            CharacterSpell.objects.create(
                character=character,
                spell_id=3,
                spell_name='Burning Hands',
                spell_level=1
            )
        ]
        
        ordered_spells = list(character.spells.all())
        
        # Level 1 spells first, alphabetically
        assert ordered_spells[0].spell_name == 'Burning Hands'
        assert ordered_spells[1].spell_name == 'Magic Missile'
        # Level 3 spell last
        assert ordered_spells[2].spell_name == 'Fireball'


class TestCharacterItem:
    """Test the CharacterItem model"""
    
    @pytest.fixture
    def character(self, db):
        """Create a basic character for testing"""
        return Character.objects.create(
            file_name='test.bic',
            file_path='/test.bic',
            first_name='Test'
        )
    
    def test_character_item_creation(self, character):
        """Test creating character item"""
        item = CharacterItem.objects.create(
            character=character,
            base_item_id=10,
            base_item_name='Longsword',
            localized_name='Longsword +1',
            stack_size=1,
            location='RIGHT_HAND'
        )
        
        assert item.character == character
        assert item.base_item_id == 10
        assert item.base_item_name == 'Longsword'
        assert item.localized_name == 'Longsword +1'
        assert item.location == 'RIGHT_HAND'
        assert str(item) == 'Longsword +1'
    
    def test_character_item_stack_display(self, character):
        """Test item string representation with stacks"""
        item = CharacterItem.objects.create(
            character=character,
            base_item_id=100,
            base_item_name='Arrow',
            stack_size=99,
            location='ARROWS'
        )
        
        assert str(item) == 'Arrow x99'
    
    def test_character_item_display_name(self, character):
        """Test display_name property"""
        # With localized name
        item1 = CharacterItem.objects.create(
            character=character,
            base_item_id=1,
            base_item_name='Generic Sword',
            localized_name='Flaming Sword of Doom',
            location='RIGHT_HAND'
        )
        assert item1.display_name == 'Flaming Sword of Doom'
        
        # Without localized name
        item2 = CharacterItem.objects.create(
            character=character,
            base_item_id=2,
            base_item_name='Shield',
            localized_name='',
            location='LEFT_HAND'
        )
        assert item2.display_name == 'Shield'
    
    def test_character_item_locations(self, character):
        """Test all equipment locations"""
        locations = [
            'INVENTORY', 'HEAD', 'CHEST', 'BOOTS', 'ARMS',
            'RIGHT_HAND', 'LEFT_HAND', 'CLOAK', 'LEFT_RING',
            'RIGHT_RING', 'NECK', 'BELT', 'ARROWS', 'BULLETS', 'BOLTS'
        ]
        
        for loc in locations:
            item = CharacterItem.objects.create(
                character=character,
                base_item_id=1,
                base_item_name='Test Item',
                location=loc
            )
            assert item.location == loc
    
    def test_character_item_ordering(self, character):
        """Test items are ordered by location then inventory slot"""
        items = [
            CharacterItem.objects.create(
                character=character,
                base_item_id=1,
                base_item_name='Helmet',
                location='HEAD'
            ),
            CharacterItem.objects.create(
                character=character,
                base_item_id=2,
                base_item_name='Sword',
                location='RIGHT_HAND'
            ),
            CharacterItem.objects.create(
                character=character,
                base_item_id=3,
                base_item_name='Item1',
                location='INVENTORY',
                inventory_slot=5
            ),
            CharacterItem.objects.create(
                character=character,
                base_item_id=4,
                base_item_name='Item2',
                location='INVENTORY',
                inventory_slot=2
            )
        ]
        
        ordered_items = list(character.items.all())
        
        # Items should be ordered by location string, then slot
        # Check that inventory items are together and ordered by slot
        inv_items = [i for i in ordered_items if i.location == 'INVENTORY']
        assert inv_items[0].inventory_slot == 2
        assert inv_items[1].inventory_slot == 5
    
    def test_character_item_properties_json(self, character):
        """Test item properties JSON field"""
        properties = [
            {'type': 'enhancement', 'value': 2},
            {'type': 'damage_bonus', 'damage_type': 'fire', 'value': '1d6'}
        ]
        
        item = CharacterItem.objects.create(
            character=character,
            base_item_id=1,
            base_item_name='Magic Sword',
            properties=properties
        )
        
        item.refresh_from_db()
        assert item.properties == properties
    
    def test_character_item_stack_validation(self, character):
        """Test stack size validation"""
        with pytest.raises(ValidationError) as exc_info:
            item = CharacterItem(
                character=character,
                base_item_id=1,
                base_item_name='Test',
                stack_size=0  # Below minimum
            )
            item.full_clean()
        assert 'stack_size' in str(exc_info.value)