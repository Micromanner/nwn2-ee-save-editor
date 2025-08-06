"""
Comprehensive tests for CharacterManager class and related functionality.
Tests cover initialization, transactions, manager registration, custom content detection,
import from files/savegames, module detection, and campaign association.
"""
import pytest
import tempfile
import os
import json
import shutil
from unittest.mock import Mock, MagicMock, patch, PropertyMock
from pathlib import Path
from io import BytesIO

import sys

from character.character_manager import (
    CharacterManager, Transaction, GFFDataWrapper
)
from character.services import CharacterImportService
from character.events import EventEmitter, EventData, EventType
from parsers.gff import GFFParser, GFFElement, GFFFieldType
from gamedata.services.game_rules_service import GameRulesService


@pytest.fixture
def mock_game_rules():
    """Create a mock GameRulesService with sample data"""
    mock_rules = Mock(spec=GameRulesService)
    
    # Mock feats
    mock_rules.feats = {
        1: Mock(id=1, label="Alertness"),
        2: Mock(id=2, label="Weapon Focus"),
        3: Mock(id=3, label="Power Attack"),
        # Custom feat IDs > 10000
    }
    
    # Mock spells
    mock_rules.spells = {
        0: Mock(id=0, label="Acid Splash"),
        1: Mock(id=1, label="Magic Missile"),
        2: Mock(id=2, label="Fireball"),
    }
    
    # Mock classes
    mock_rules.classes = {
        0: Mock(id=0, label="Fighter"),
        1: Mock(id=1, label="Wizard"),
        2: Mock(id=2, label="Rogue"),
    }
    
    # Mock races
    mock_rules.races = {
        0: {"name": "Human"},
        1: {"name": "Elf"},
        2: {"name": "Dwarf"},
    }
    
    return mock_rules


@pytest.fixture
def sample_character_data():
    """Create sample character data for testing"""
    return {
        "FirstName": {
            "type": "locstring",
            "substrings": [{"string": "Test", "language": 0, "gender": 0}]
        },
        "LastName": {
            "type": "locstring",
            "substrings": [{"string": "Character", "language": 0, "gender": 0}]
        },
        "Race": 0,  # Human
        "LawfulChaotic": 50,
        "GoodEvil": 50,
        "Str": 16,
        "Dex": 14,
        "Con": 15,
        "Int": 12,
        "Wis": 10,
        "Cha": 8,
        "ClassList": [
            {"Class": 0, "ClassLevel": 5},  # Fighter 5
            {"Class": 1, "ClassLevel": 3}   # Wizard 3
        ],
        "FeatList": [
            {"Feat": 1},  # Alertness (vanilla)
            {"Feat": 2},  # Weapon Focus (vanilla)
            {"Feat": 10001}  # Custom feat
        ],
        "KnownList0": [
            {"Spell": 0},  # Acid Splash (vanilla)
            {"Spell": 10002}  # Custom spell
        ],
        "Mod_Name": "test_module",
        "Mod_LastModId": "test_module_id"
    }


@pytest.fixture
def mock_game_data_loader():
    """Create a mock DynamicGameDataLoader"""
    mock_loader = Mock()
    
    # Mock get_by_id to return data for vanilla content, None for custom
    def get_by_id_side_effect(table_name, content_id):
        if table_name == 'feat':
            if content_id <= 1000:  # Vanilla feats
                mock_data = Mock()
                mock_data.name = f"Feat {content_id}"
                mock_data.label = f"Feat {content_id}"
                return mock_data
            return None  # Custom feats
        elif table_name == 'spells':
            if content_id <= 1000:  # Vanilla spells
                mock_data = Mock()
                mock_data.name = f"Spell {content_id}"
                mock_data.spellname = f"Spell {content_id}"
                return mock_data
            return None  # Custom spells
        elif table_name == 'classes':
            if content_id <= 10:  # Vanilla classes
                mock_data = Mock()
                mock_data.name = f"Class {content_id}"
                mock_data.label = f"Class {content_id}"
                return mock_data
            return None  # Custom classes
        elif table_name == 'racialtypes':
            if content_id <= 10:  # Vanilla races
                mock_data = Mock()
                mock_data.name = f"Race {content_id}"
                mock_data.label = f"Race {content_id}"
                return mock_data
            return None  # Custom races
        return None
    
    mock_loader.get_by_id.side_effect = get_by_id_side_effect
    return mock_loader

@pytest.fixture  
def character_manager(sample_character_data, mock_game_data_loader):
    """Create a CharacterManager instance for testing"""
    mock_rules_service = Mock()
    mock_rules_service.validate_character.return_value = []
    return CharacterManager(sample_character_data, game_data_loader=mock_game_data_loader, rules_service=mock_rules_service)


@pytest.fixture
def mock_resource_manager():
    """Create a mock ResourceManager for testing imports"""
    mock_rm = Mock()
    
    # Mock 2DA data
    mock_2da = Mock()
    mock_2da.get_resource_count.return_value = 100
    mock_2da.get_int.return_value = 1000  # String reference
    
    mock_rm.get_2da.return_value = mock_2da
    mock_rm.get_2da_with_overrides.return_value = mock_2da
    
    # Mock string lookup
    mock_rm.get_string.return_value = "Test String"
    
    # Mock race name lookup
    mock_rm.get_race_name.return_value = "Human"
    
    # Mock module finding
    mock_rm.find_module.return_value = "/path/to/module"
    mock_rm.set_module.return_value = True
    mock_rm._module_info = {"Mod_HakList": ["test.hak"]}
    
    # Mock campaign finding
    mock_rm.find_campaign.return_value = {
        "name": "Test Campaign",
        "modules": ["test_module", "test_module2"],
        "level_cap": 20
    }
    
    return mock_rm


@pytest.fixture
def import_service(mock_resource_manager):
    """Create a CharacterImportService instance for testing"""
    return CharacterImportService(mock_resource_manager)


@pytest.fixture
def temp_save_dir():
    """Create a temporary save game directory structure"""
    with tempfile.TemporaryDirectory() as tmpdir:
        save_dir = Path(tmpdir) / "test_save"
        save_dir.mkdir()
        
        # Create currentmodule.txt
        (save_dir / "currentmodule.txt").write_text("test_module")
        
        # Create empty save files
        (save_dir / "globals.xml").touch()
        (save_dir / "playerinfo.bin").touch()
        (save_dir / "resgff.zip").touch()
        
        yield str(save_dir)


class TestCharacterManager:
    """Test CharacterManager core functionality"""
    
    def test_initialization(self, sample_character_data, mock_game_data_loader):
        """Test CharacterManager initialization"""
        manager = CharacterManager(sample_character_data, game_data_loader=mock_game_data_loader)
        
        assert manager.character_data == sample_character_data
        assert manager.game_data_loader == mock_game_data_loader
        assert isinstance(manager.gff, GFFDataWrapper)
        assert manager.gff_element is None
        assert len(manager._managers) == 0
        assert manager._current_transaction is None
        assert len(manager.custom_content) > 0  # Should detect custom feat
    
    def test_initialization_with_gff_element(self, sample_character_data, mock_game_data_loader):
        """Test CharacterManager initialization with GFFElement"""
        mock_element = Mock()
        
        with patch('character.gff_direct_wrapper.DirectGFFWrapper') as mock_wrapper_class:
            mock_wrapper = Mock()
            # Mock the get method to return appropriate values for custom content detection
            mock_wrapper.get.side_effect = lambda key, default=None: {
                'FeatList': sample_character_data.get('FeatList', []),
                'KnownList0': sample_character_data.get('KnownList0', []),
                'KnownList1': [], 'KnownList2': [], 'KnownList3': [], 'KnownList4': [],
                'KnownList5': [], 'KnownList6': [], 'KnownList7': [], 'KnownList8': [], 'KnownList9': [],
                'ClassList': sample_character_data.get('ClassList', [])
            }.get(key, default)
            mock_wrapper_class.return_value = mock_wrapper
            
            manager = CharacterManager(sample_character_data, game_data_loader=mock_game_data_loader, gff_element=mock_element)
            
            assert manager.gff_element == mock_element
            assert manager.gff == mock_wrapper
            mock_wrapper_class.assert_called_once_with(mock_element)
    
    def test_register_manager(self, character_manager):
        """Test manager registration"""
        mock_manager_class = Mock()
        mock_manager_instance = Mock()
        mock_manager_class.return_value = mock_manager_instance
        
        character_manager.register_manager("test", mock_manager_class)
        
        assert "test" in character_manager._manager_classes
        assert character_manager._managers["test"] == mock_manager_instance
        mock_manager_class.assert_called_once_with(character_manager)
    
    def test_get_manager(self, character_manager):
        """Test manager retrieval"""
        mock_manager = Mock()
        character_manager._managers["test"] = mock_manager
        
        assert character_manager.get_manager("test") == mock_manager
        assert character_manager.get_manager("nonexistent") is None
    
    def test_custom_content_detection(self, character_manager):
        """Test detection of custom content"""
        # Should detect feat 10001 as custom
        assert "feat_10001" in character_manager.custom_content
        assert character_manager.custom_content["feat_10001"]["type"] == "feat"
        assert character_manager.custom_content["feat_10001"]["id"] == 10001
        assert character_manager.custom_content["feat_10001"]["protected"] is True
        assert character_manager.custom_content["feat_10001"]["source"] == "custom-mod"
        
        # Should detect spell 10002 as custom
        assert "spell_10002" in character_manager.custom_content
        assert character_manager.custom_content["spell_10002"]["type"] == "spell"
        assert character_manager.custom_content["spell_10002"]["id"] == 10002
        assert character_manager.custom_content["spell_10002"]["source"] == "custom-mod"
    
    def test_get_character_name(self, character_manager):
        """Test character name extraction"""
        name = character_manager._get_character_name()
        assert name == "Test Character"
    
    def test_get_character_name_non_localized(self, mock_game_data_loader):
        """Test character name extraction with non-localized strings"""
        data = {
            "FirstName": "Simple",
            "LastName": "Name"
        }
        manager = CharacterManager(data, game_data_loader=mock_game_data_loader)
        name = manager._get_character_name()
        assert name == "Simple Name"
    
    def test_get_character_summary(self, character_manager):
        """Test character summary generation"""
        summary = character_manager.get_character_summary()
        
        assert summary["name"] == "Test Character"
        assert summary["level"] == 8  # 5 + 3
        assert len(summary["classes"]) == 2
        assert summary["classes"][0]["class_id"] == 0
        assert summary["classes"][0]["level"] == 5
        assert summary["classes"][0]["name"] == "Class 0"  # From mock
        assert summary["race"] == "Race 0"  # From mock
        assert summary["alignment"]["law_chaos"] == 50
        assert summary["alignment"]["good_evil"] == 50
        # Updated to use dynamic ability access
        assert summary["abilities"]["strength"] == 16
        assert summary["abilities"]["dexterity"] == 14
        assert summary["custom_content_count"] == 2  # feat 10001 and spell 10002 are both custom (>1000)
    
    def test_validate_changes_no_managers(self, character_manager):
        """Test validation with no registered managers"""
        is_valid, errors = character_manager.validate_changes()
        assert is_valid is True
        assert len(errors) == 0
    
    def test_validate_changes_with_managers(self, character_manager):
        """Test validation with registered managers"""
        # Mock manager with validate method
        mock_manager = Mock()
        mock_manager.validate.return_value = (False, ["Error 1", "Error 2"])
        character_manager._managers["test"] = mock_manager
        
        is_valid, errors = character_manager.validate_changes()
        assert is_valid is False
        assert len(errors) == 2
        assert errors[0] == "test: Error 1"
        assert errors[1] == "test: Error 2"
    
    def test_export_changes(self, character_manager):
        """Test export functionality"""
        # Add some event history
        event_data = EventData(
            event_type=EventType.ABILITY_CHANGED,
            source_manager="test",
            timestamp=1234567890
        )
        character_manager.emit(event_data)
        
        export = character_manager.export_changes()
        
        assert "summary" in export
        assert export["summary"]["name"] == "Test Character"
        assert "transactions" in export
        assert "custom_content" in export
        assert len(export["custom_content"]) == 2
        assert "event_history" in export
        assert len(export["event_history"]) > 0
    
    def test_notify_managers(self, character_manager):
        """Test internal notification system"""
        # Mock manager with notification handler
        mock_manager = Mock()
        mock_manager.on_test_notification = Mock()
        character_manager._managers["test"] = mock_manager
        
        character_manager._notify_managers("test_notification", {"data": "value"})
        
        mock_manager.on_test_notification.assert_called_once_with({"data": "value"})
    
    def test_notify_managers_transaction_rollback(self, character_manager):
        """Test transaction rollback notification updates gff references"""
        mock_manager = Mock()
        character_manager._managers["test"] = mock_manager
        
        character_manager._notify_managers("transaction_rollback", {"transaction_id": "123"})
        
        assert mock_manager.gff == character_manager.gff


class TestTransaction:
    """Test Transaction functionality"""
    
    def test_transaction_creation(self, character_manager):
        """Test transaction creation"""
        txn = character_manager.begin_transaction()
        
        assert isinstance(txn, Transaction)
        assert txn.manager == character_manager
        assert txn.id.startswith("txn_")
        assert len(txn.changes) == 0
        assert txn.original_state == character_manager.character_data
        assert character_manager._current_transaction == txn
    
    def test_transaction_already_in_progress(self, character_manager):
        """Test error when transaction already in progress"""
        character_manager.begin_transaction()
        
        with pytest.raises(RuntimeError, match="Transaction already in progress"):
            character_manager.begin_transaction()
    
    def test_add_change(self, character_manager):
        """Test adding changes to transaction"""
        txn = character_manager.begin_transaction()
        
        txn.add_change("test_change", {"field": "value", "old": 1, "new": 2})
        
        assert len(txn.changes) == 1
        assert txn.changes[0]["type"] == "test_change"
        assert txn.changes[0]["details"]["field"] == "value"
        assert "timestamp" in txn.changes[0]
    
    def test_commit_transaction(self, character_manager):
        """Test committing a transaction"""
        txn = character_manager.begin_transaction()
        txn.add_change("test", {"data": "value"})
        
        result = character_manager.commit_transaction()
        
        assert result["transaction_id"] == txn.id
        assert len(result["changes"]) == 1
        assert "duration" in result
        assert character_manager._current_transaction is None
        assert txn in character_manager._transaction_history
    
    def test_commit_no_transaction(self, character_manager):
        """Test error when committing without transaction"""
        with pytest.raises(RuntimeError, match="No transaction in progress"):
            character_manager.commit_transaction()
    
    def test_rollback_transaction(self, character_manager):
        """Test rolling back a transaction"""
        # Store original value
        original_str = character_manager.character_data["Str"]
        
        # Modify character data before transaction
        character_manager.character_data["Str"] = 20
        
        # Start transaction - this captures current state (Str=20)
        txn = character_manager.begin_transaction()
        
        # Further modify after transaction started
        character_manager.character_data["Str"] = 25
        
        # Rollback should restore to state when transaction began (20, not original 16)
        character_manager.rollback_transaction()
        
        assert character_manager.character_data["Str"] == 20  # State at transaction start
        assert character_manager._current_transaction is None
        # GFF wrapper should be recreated
        assert isinstance(character_manager.gff, GFFDataWrapper)
    
    def test_rollback_no_transaction(self, character_manager):
        """Test error when rolling back without transaction"""
        with pytest.raises(RuntimeError, match="No transaction in progress"):
            character_manager.rollback_transaction()


class TestGFFDataWrapper:
    """Test GFFDataWrapper functionality"""
    
    def test_get_simple_value(self):
        """Test getting simple values"""
        data = {"field": "value", "number": 42}
        wrapper = GFFDataWrapper(data)
        
        assert wrapper.get("field") == "value"
        assert wrapper.get("number") == 42
        assert wrapper.get("missing") is None
        assert wrapper.get("missing", "default") == "default"
    
    def test_get_nested_value(self):
        """Test getting nested values"""
        data = {
            "level1": {
                "level2": {
                    "value": "deep"
                }
            }
        }
        wrapper = GFFDataWrapper(data)
        
        assert wrapper.get("level1.level2.value") == "deep"
        assert wrapper.get("level1.level2") == {"value": "deep"}
        assert wrapper.get("level1.missing.value") is None
    
    def test_get_list_value(self):
        """Test getting values from lists"""
        data = {
            "items": [
                {"id": 1, "name": "first"},
                {"id": 2, "name": "second"},
                {"id": 3, "name": "third"}
            ]
        }
        wrapper = GFFDataWrapper(data)
        
        assert wrapper.get("items.0.name") == "first"
        assert wrapper.get("items.1.id") == 2
        assert wrapper.get("items.2") == {"id": 3, "name": "third"}
        assert wrapper.get("items.5") is None  # Out of range
    
    def test_set_simple_value(self):
        """Test setting simple values"""
        data = {"existing": "old"}
        wrapper = GFFDataWrapper(data)
        
        wrapper.set("existing", "new")
        wrapper.set("new_field", 123)
        
        assert data["existing"] == "new"
        assert data["new_field"] == 123
    
    def test_set_nested_value(self):
        """Test setting nested values"""
        data = {}
        wrapper = GFFDataWrapper(data)
        
        wrapper.set("level1.level2.value", "deep")
        
        assert data["level1"]["level2"]["value"] == "deep"
    
    def test_set_list_value(self):
        """Test setting values in lists"""
        data = {
            "items": [
                {"id": 1},
                {"id": 2}
            ]
        }
        wrapper = GFFDataWrapper(data)
        
        wrapper.set("items.0.name", "first")
        wrapper.set("items.1.id", 99)
        
        assert data["items"][0]["name"] == "first"
        assert data["items"][1]["id"] == 99
    
    def test_set_create_list(self):
        """Test creating lists when setting with numeric index"""
        data = {}
        wrapper = GFFDataWrapper(data)
        
        # The wrapper creates an empty list when the next part is numeric
        # But it doesn't automatically extend the list
        # This test documents the actual behavior
        with pytest.raises(IndexError):
            wrapper.set("new_list.0", {"id": 1})
        
        # To properly use lists, they need to be pre-populated
        data["items"] = [None, None, None]  # Pre-create slots
        wrapper.set("items.0", {"id": 1})
        wrapper.set("items.2", {"id": 3})
        
        assert data["items"][0] == {"id": 1}
        assert data["items"][1] is None
        assert data["items"][2] == {"id": 3}
    
    def test_set_invalid_path(self):
        """Test error handling for invalid paths"""
        data = {"string": "value"}
        wrapper = GFFDataWrapper(data)
        
        # Can't navigate through string
        with pytest.raises(ValueError, match="Cannot set value at"):
            wrapper.set("string.field", "value")
    
    def test_set_index_out_of_range(self):
        """Test error handling for out of range indices"""
        data = {"items": [1, 2, 3]}
        wrapper = GFFDataWrapper(data)
        
        with pytest.raises(IndexError, match="out of range"):
            wrapper.set("items.5", 99)
    
    def test_raw_data_property(self):
        """Test raw_data property"""
        data = {"test": "data"}
        wrapper = GFFDataWrapper(data)
        
        assert wrapper.raw_data is data
        assert wrapper.raw_data == {"test": "data"}


class TestCharacterImportService:
    """Test CharacterImportService integration with CharacterManager"""
    
    @patch('character.services.CharacterClass')
    @patch('character.services.CharacterFeat')
    @patch('character.services.CharacterSkill')
    @patch('character.services.CharacterItem')
    @patch('character.services.Character')
    def test_import_from_file(self, mock_character_model, mock_item_model, mock_skill_model, 
                            mock_feat_model, mock_class_model, import_service, temp_save_dir):
        """Test importing from a .bic file"""
        # Create a mock .bic file in a subdirectory without currentmodule.txt
        bic_dir = os.path.join(temp_save_dir, "bic_dir")
        os.makedirs(bic_dir, exist_ok=True)
        bic_path = os.path.join(bic_dir, "test.bic")
        
        # Mock GFFParser
        with patch('character.services.GFFParser') as mock_parser_class:
            mock_parser = Mock()
            mock_parser_class.return_value = mock_parser
            
            # Mock parsed data
            mock_parser.top_level_struct.to_dict.return_value = {
                "FirstName": {"substrings": [{"string": "Hero"}]},
                "LastName": {"substrings": [{"string": "Test"}]},
                "Race": 1,
                "ClassList": [{"Class": 0, "ClassLevel": 10}],
                "FeatList": [{"Feat": 1}, {"Feat": 10005}],
                "Mod_Name": "custom_module"
            }
            
            # Mock Character model with id for foreign key relationships
            mock_char_instance = Mock()
            mock_char_instance.id = 1
            mock_char_instance.pk = 1
            mock_character_model.objects.create.return_value = mock_char_instance
            
            # Import character
            result = import_service.import_character(bic_path)
            
            # Verify GFF parser was used
            mock_parser.read.assert_called_once_with(bic_path)
            
            # Verify character was created with module info
            create_call = mock_character_model.objects.create.call_args[1]
            assert create_call["first_name"] == "Hero"
            assert create_call["last_name"] == "Test"
            assert create_call["module_name"] == "custom_module"
            assert create_call["uses_custom_content"] is True  # Has custom feat
    
    @patch('character.services.CharacterClass')
    @patch('character.services.CharacterFeat') 
    @patch('character.services.CharacterSkill')
    @patch('character.services.CharacterItem')
    @patch('parsers.savegame_handler.SaveGameHandler')
    @patch('character.services.Character')
    def test_import_from_savegame(self, mock_character_model, mock_handler_class, mock_item_model,
                                 mock_skill_model, mock_feat_model, mock_class_model, import_service, temp_save_dir):
        """Test importing from a save game directory"""
        # Mock SaveGameHandler
        mock_handler = Mock()
        mock_handler_class.return_value = mock_handler
        mock_handler.extract_player_data.return_value = b"mock_player_data"
        
        # Mock GFFParser for playerlist.ifo
        with patch('character.services.GFFParser') as mock_parser_class:
            mock_parser = Mock()
            mock_parser_class.return_value = mock_parser
            
            # Mock player list structure
            mock_field = Mock()
            mock_field.value = [Mock()]
            mock_field.value[0].to_dict.return_value = {
                "FirstName": {"substrings": [{"string": "SaveGame"}]},
                "LastName": {"substrings": [{"string": "Character"}]},
                "Race": 2,
                "ClassList": [{"Class": 1, "ClassLevel": 15}]
            }
            
            # Mock the parsed GFF structure
            mock_gff = Mock()
            mock_gff.get_field = Mock(return_value=mock_field)
            mock_parser.load.return_value = mock_gff
            
            # Mock Character model with id for foreign key relationships
            mock_char_instance = Mock()
            mock_char_instance.id = 1
            mock_char_instance.pk = 1
            mock_character_model.objects.create.return_value = mock_char_instance
            
            # Import from save game
            result = import_service.import_character(temp_save_dir)
            
            # Verify save game handler was used
            mock_handler_class.assert_called_once_with(temp_save_dir)
            mock_handler.extract_player_data.assert_called_once()
            
            # Verify character was created
            assert mock_character_model.objects.create.called
            create_call = mock_character_model.objects.create.call_args[1]
            assert create_call["is_savegame"] is True
    
    def test_detect_module_from_currentmodule_txt(self, import_service, temp_save_dir):
        """Test module detection from currentmodule.txt"""
        # Use a file path instead of directory to match expected behavior
        data = {}
        file_path = os.path.join(temp_save_dir, "test.bic")
        import_service._detect_module_info(data, file_path)
        
        # Should detect module from currentmodule.txt
        assert data["_module_info"]["module_name"] == "test_module"
        
        # Verify ResourceManager was called to load module
        import_service.rm.find_module.assert_called_with("test_module")
        import_service.rm.set_module.assert_called()
    
    def test_detect_module_from_mod_name(self, import_service):
        """Test module detection from Mod_Name field"""
        data = {"Mod_Name": "my_custom_module"}
        import_service._detect_module_info(data)
        
        assert data["_module_info"]["module_name"] == "my_custom_module"
        import_service.rm.find_module.assert_called_with("my_custom_module")
    
    def test_detect_official_campaign(self, import_service):
        """Test detection of official campaign modules"""
        data = {"Mod_Name": "OfficialCampaign", "Mod_LastModId": "OfficialCampaign"}
        import_service._detect_module_info(data)
        
        assert data["_module_info"]["module_name"] == "OfficialCampaign"
    
    def test_detect_custom_content_classes(self, import_service):
        """Test detection of custom classes"""
        # Mock that base game only has 3 classes (0-2)
        mock_2da = Mock()
        mock_2da.get_resource_count.return_value = 3
        import_service.rm.get_2da.return_value = mock_2da
        
        data = {
            "ClassList": [
                {"Class": 0},  # Vanilla
                {"Class": 5}   # Custom
            ]
        }
        import_service._detect_module_info(data)
        
        assert data["_module_info"]["uses_custom_content"] is True
        assert 5 in data["_module_info"]["custom_content_ids"]["classes"]
    
    def test_detect_custom_content_feats(self, import_service):
        """Test detection of custom feats"""
        mock_2da = Mock()
        mock_2da.get_resource_count.return_value = 1000
        import_service.rm.get_2da.return_value = mock_2da
        
        data = {
            "FeatList": [
                {"Feat": 1},      # Vanilla
                {"Feat": 999},    # Vanilla (edge)
                {"Feat": 1001}    # Custom
            ]
        }
        import_service._detect_module_info(data)
        
        assert data["_module_info"]["uses_custom_content"] is True
        assert 1001 in data["_module_info"]["custom_content_ids"]["feats"]
    
    def test_detect_custom_spells(self, import_service):
        """Test detection of custom spells"""
        mock_2da = Mock()
        mock_2da.get_resource_count.return_value = 500
        import_service.rm.get_2da.return_value = mock_2da
        
        data = {
            "KnownList0": [{"Spell": 501}],  # Custom cantrip
            "SpellLvlMem1": [
                {"MemorizedList": [{"Spell": 1}, {"Spell": 502}]}
            ]
        }
        import_service._detect_module_info(data)
        
        assert data["_module_info"]["uses_custom_content"] is True
        assert 501 in data["_module_info"]["custom_content_ids"]["spells"]
        assert 502 in data["_module_info"]["custom_content_ids"]["spells"]
    
    def test_detect_campaign_by_module_pattern(self, import_service):
        """Test campaign detection by module name patterns"""
        # Test original campaign
        data = {"_module_info": {"module_name": "1200_highcliff", "campaign_name": ""}}
        
        # Mock os.path.exists to return True for campaign path
        with patch('os.path.exists', return_value=True):
            import_service._detect_campaign_info(data)
        
        assert data["_module_info"]["campaign_name"] == "Test Campaign"
        assert "test_module" in data["_module_info"]["campaign_modules"]
        assert data["_module_info"]["campaign_level_cap"] == 20
    
    def test_detect_campaign_motb(self, import_service):
        """Test Mask of the Betrayer campaign detection"""
        data = {"_module_info": {"module_name": "2100_mulsantir", "campaign_name": ""}}
        
        # Mock campaign info
        import_service.rm.find_campaign.return_value = {
            "name": "Mask of the Betrayer",
            "modules": ["2000_motb_start", "2100_mulsantir", "2200_thayred"],
            "level_cap": 30
        }
        
        # Mock os.path.exists to return True for campaign path
        with patch('os.path.exists', return_value=True):
            import_service._detect_campaign_info(data)
        
        assert "Mask of the Betrayer" in data["_module_info"]["campaign_name"]
    
    def test_detect_campaign_by_module_list(self, import_service):
        """Test campaign detection by checking module lists"""
        data = {"_module_info": {"module_name": "some_module", "campaign_name": ""}}
        
        # Mock campaign directory listing
        with patch('os.path.exists', return_value=True), \
             patch('os.listdir', return_value=["Campaign1", "Campaign2"]), \
             patch('os.path.isdir', return_value=True):
            
            # First campaign doesn't contain module
            # Second campaign contains module
            def find_campaign_side_effect(path):
                if "Campaign2" in path:
                    return {
                        "name": "Found Campaign",
                        "modules": ["some_module", "other_module"],
                        "level_cap": 25
                    }
                return {"name": "Other Campaign", "modules": ["different_module"]}
            
            import_service.rm.find_campaign.side_effect = find_campaign_side_effect
            
            import_service._detect_campaign_info(data)
            
            assert data["_module_info"]["campaign_name"] == "Found Campaign"
            assert data["_module_info"]["campaign_level_cap"] == 25


class TestCharacterManagerEdgeCases:
    """Test edge cases and error handling"""
    
    def test_empty_character_data(self, mock_game_data_loader):
        """Test with empty character data"""
        manager = CharacterManager({}, game_data_loader=mock_game_data_loader)
        
        # Empty data returns "" (empty string) now
        name = manager._get_character_name()
        assert name == ""
        summary = manager.get_character_summary()
        assert summary["name"] == ""
        assert summary["level"] == 0
        assert len(summary["classes"]) == 0
    
    def test_missing_fields(self, mock_game_data_loader):
        """Test with missing expected fields"""
        data = {
            "FirstName": "OnlyFirst",
            # Missing LastName
            # Missing ClassList
            # Missing abilities
        }
        manager = CharacterManager(data, game_data_loader=mock_game_data_loader)
        
        assert manager._get_character_name() == "OnlyFirst"  # Missing LastName results in just first name
        summary = manager.get_character_summary()
        assert summary["abilities"]["strength"] == 10  # Default value
    
    def test_invalid_race_id(self, mock_game_data_loader):
        """Test with invalid race ID"""
        data = {"Race": 999}  # High race ID that won't be found
        
        manager = CharacterManager(data, game_data_loader=mock_game_data_loader)
        summary = manager.get_character_summary()
        assert summary["race"] == "Custom Racialtype 999"  # New dynamic fallback
    
    def test_malformed_class_list(self, mock_game_data_loader):
        """Test with malformed ClassList"""
        data = {
            "ClassList": [
                {"Class": 0, "ClassLevel": 5},
                "not_a_dict",  # Invalid entry
                {"Class": 1}  # Missing ClassLevel
            ]
        }
        
        manager = CharacterManager(data, game_data_loader=mock_game_data_loader)
        # The improved implementation handles malformed data gracefully
        summary = manager.get_character_summary()
        # Only valid dict entries are counted
        assert summary["level"] == 5  # Only first entry has ClassLevel
        assert len(summary["classes"]) == 2  # Only dict entries included
    
    def test_transaction_with_gff_element(self, sample_character_data, mock_game_data_loader):
        """Test transaction rollback with DirectGFFWrapper"""
        mock_element = Mock()
        
        with patch('character.gff_direct_wrapper.DirectGFFWrapper') as mock_wrapper_class:
            # Setup mock wrapper to return proper values
            mock_wrapper = Mock()
            mock_wrapper.get.side_effect = lambda key, default=None: {
                'FeatList': sample_character_data.get('FeatList', []),
                'KnownList0': sample_character_data.get('KnownList0', []),
                'KnownList1': [], 'KnownList2': [], 'KnownList3': [],
                'KnownList4': [], 'KnownList5': [], 'KnownList6': [],
                'KnownList7': [], 'KnownList8': [], 'KnownList9': [],
                'ClassList': sample_character_data.get('ClassList', [])
            }.get(key, default)
            mock_wrapper_class.return_value = mock_wrapper
            
            manager = CharacterManager(sample_character_data, game_data_loader=mock_game_data_loader, gff_element=mock_element)
            
            # Start transaction and rollback
            txn = manager.begin_transaction()
            manager.rollback_transaction()
            
            # Should not recreate DirectGFFWrapper, just GFFDataWrapper
            assert mock_wrapper_class.call_count == 1  # Only initial
            # But GFF wrapper should be replaced with GFFDataWrapper
            assert isinstance(manager.gff, GFFDataWrapper)
    
    def test_large_custom_content(self, mock_game_data_loader):
        """Test with many custom content items"""
        # Create data with many custom feats and spells (>1000 means custom in our mock)
        feat_list = [{"Feat": i} for i in range(1001, 1100)]
        spell_lists = {}
        for level in range(10):
            spell_lists[f"KnownList{level}"] = [{"Spell": i} for i in range(1100 + level * 10, 1110 + level * 10)]
        
        data = {
            "FeatList": feat_list,
            **spell_lists
        }
        
        manager = CharacterManager(data, game_data_loader=mock_game_data_loader)
        
        # Should detect all as custom
        assert len([k for k in manager.custom_content.keys() if k.startswith("feat_")]) == 99
        assert len([k for k in manager.custom_content.keys() if k.startswith("spell_")]) == 100
    
    def test_concurrent_transactions(self, character_manager):
        """Test that concurrent transactions are prevented"""
        txn1 = character_manager.begin_transaction()
        
        # Try to start another transaction
        with pytest.raises(RuntimeError):
            character_manager.begin_transaction()
        
        # Complete first transaction
        character_manager.commit_transaction()
        
        # Add small delay to ensure different timestamp
        import time
        time.sleep(0.001)
        
        # Now should be able to start new transaction
        txn2 = character_manager.begin_transaction()
        assert txn2 is not None
        assert txn2.id != txn1.id


class TestCharacterManagerPerformance:
    """Test performance-related scenarios"""
    
    def test_large_transaction_history(self, character_manager):
        """Test performance with many transactions"""
        # Create many transactions
        for i in range(100):
            txn = character_manager.begin_transaction()
            txn.add_change("test", {"iteration": i})
            character_manager.commit_transaction()
        
        assert len(character_manager._transaction_history) == 100
        
        # Export should still work efficiently
        export = character_manager.export_changes()
        assert len(export["transactions"]) == 100
    
    def test_deep_nested_paths(self):
        """Test GFFDataWrapper with deeply nested paths"""
        # Create deeply nested structure
        data = {}
        current = data
        depth = 20
        
        for i in range(depth):
            current[f"level{i}"] = {}
            current = current[f"level{i}"]
        current["value"] = "deep"
        
        wrapper = GFFDataWrapper(data)
        
        # Build path
        path = ".".join([f"level{i}" for i in range(depth)]) + ".value"
        
        # Should handle deep paths
        assert wrapper.get(path) == "deep"
        
        # Set deep value
        wrapper.set(path, "updated")
        assert wrapper.get(path) == "updated"
    
    def test_large_list_operations(self):
        """Test GFFDataWrapper with large lists"""
        # Create large list
        data = {
            "items": [{"id": i, "value": f"item_{i}"} for i in range(1000)]
        }
        wrapper = GFFDataWrapper(data)
        
        # Access various indices
        assert wrapper.get("items.0.id") == 0
        assert wrapper.get("items.500.value") == "item_500"
        assert wrapper.get("items.999.id") == 999
        
        # Update values
        wrapper.set("items.100.updated", True)
        assert data["items"][100]["updated"] is True


class TestCodeImprovements:
    """Tests that highlight potential code improvements"""
    
    def test_gff_wrapper_error_messages(self):
        """Test that GFFDataWrapper provides clear error messages"""
        wrapper = GFFDataWrapper({"list": [1, 2, 3]})
        
        # Current implementation might benefit from more specific error messages
        with pytest.raises(IndexError) as exc_info:
            wrapper.set("list.10", "value")
        
        # Error message should be helpful
        assert "Index 10 out of range" in str(exc_info.value)
    
    def test_transaction_state_validation(self, character_manager):
        """Test that transactions validate state changes"""
        # The improved implementation now validates changes
        
        txn = character_manager.begin_transaction()
        
        # Make invalid change (e.g., negative ability score)
        character_manager.character_data["Str"] = -5
        
        # The improved implementation validates before commit
        with pytest.raises(ValueError) as exc_info:
            character_manager.commit_transaction()
        
        assert "Transaction validation failed" in str(exc_info.value)
        assert "Strength must be between 3 and 50" in str(exc_info.value)
    
    def test_manager_lifecycle_hooks(self, character_manager):
        """Test that managers have proper lifecycle hooks"""
        # Suggestion: Add pre/post hooks for manager operations
        
        mock_manager = Mock()
        mock_manager.on_register = Mock()  # Suggested improvement
        mock_manager.on_unregister = Mock()  # Suggested improvement
        
        character_manager._managers["test"] = mock_manager
        
        # Currently no lifecycle hooks are called
        # Improvement: Add hooks for better manager integration
    
    def test_custom_content_categories(self, character_manager):
        """Test that custom content is properly detected"""
        # The new implementation detects custom content dynamically
        
        # Check that custom content has basic info  
        for content_id, content_info in character_manager.custom_content.items():
            assert "type" in content_info
            assert "id" in content_info
            assert "name" in content_info
            assert "source" in content_info
            assert content_info["source"] == "custom-mod"  # New dynamic detection
    
    def test_event_filtering(self, character_manager):
        """Test event filtering capabilities"""
        # Current implementation doesn't support event filtering
        # Suggestion: Add event filters for performance
        
        # Emit many events
        import time
        for i in range(100):
            event_data = EventData(
                event_type=EventType.ABILITY_CHANGED,
                source_manager="test",
                timestamp=time.time()
            )
            character_manager.emit(event_data)
        
        # Currently no way to filter events
        all_events = character_manager.get_event_history()
        assert len(all_events) == 100
        
        # Improvement: Add filtering like get_event_history(event_type=EventType.ABILITY_CHANGED)
    
    def test_wrapper_type_hints(self):
        """Test that GFFDataWrapper could benefit from type hints"""
        # Current implementation returns Any
        # Suggestion: Add optional type hints for better IDE support
        
        wrapper = GFFDataWrapper({"number": 42, "text": "hello"})
        
        # These currently return Any
        num = wrapper.get("number")  # Could be get[int]("number")
        text = wrapper.get("text")   # Could be get[str]("text")
        
        # Type checking would catch errors earlier
        assert isinstance(num, int)
        assert isinstance(text, str)