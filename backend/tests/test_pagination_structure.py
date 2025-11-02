"""Test the pagination response structure from feat_manager"""
import pytest
from unittest.mock import Mock, MagicMock


def test_pagination_response_builder():
    """Test that _build_pagination_response creates correct structure"""
    from character.managers.feat_manager import FeatManager

    mock_char_manager = MagicMock()
    mock_char_manager.custom_content = {}
    mock_char_manager.get_data.return_value = {}
    mock_char_manager.game_rules_service = MagicMock()
    mock_char_manager.tlk_service = MagicMock()

    feat_manager = FeatManager(mock_char_manager)

    result = feat_manager._build_pagination_response(
        feats=[{'id': 1}, {'id': 2}],
        page=1,
        limit=10,
        total=25
    )

    assert 'feats' in result
    assert 'pagination' in result

    pagination = result['pagination']
    assert pagination['page'] == 1
    assert pagination['limit'] == 10
    assert pagination['total'] == 25
    assert pagination['pages'] == 3
    assert pagination['has_next'] == True
    assert pagination['has_previous'] == False


def test_pagination_response_last_page():
    """Test pagination on last page"""
    from character.managers.feat_manager import FeatManager

    mock_char_manager = MagicMock()
    mock_char_manager.custom_content = {}
    mock_char_manager.get_data.return_value = {}
    mock_char_manager.game_rules_service = MagicMock()
    mock_char_manager.tlk_service = MagicMock()

    feat_manager = FeatManager(mock_char_manager)

    result = feat_manager._build_pagination_response(
        feats=[{'id': 1}],
        page=3,
        limit=10,
        total=25
    )

    pagination = result['pagination']
    assert pagination['page'] == 3
    assert pagination['pages'] == 3
    assert pagination['has_next'] == False
    assert pagination['has_previous'] == True


def test_pagination_response_single_page():
    """Test pagination with single page"""
    from character.managers.feat_manager import FeatManager

    mock_char_manager = MagicMock()
    mock_char_manager.custom_content = {}
    mock_char_manager.get_data.return_value = {}
    mock_char_manager.game_rules_service = MagicMock()
    mock_char_manager.tlk_service = MagicMock()

    feat_manager = FeatManager(mock_char_manager)

    result = feat_manager._build_pagination_response(
        feats=[{'id': 1}, {'id': 2}],
        page=1,
        limit=10,
        total=2
    )

    pagination = result['pagination']
    assert pagination['pages'] == 1
    assert pagination['has_next'] == False
    assert pagination['has_previous'] == False


def test_pagination_response_empty():
    """Test pagination with no results"""
    from character.managers.feat_manager import FeatManager

    mock_char_manager = MagicMock()
    mock_char_manager.custom_content = {}
    mock_char_manager.get_data.return_value = {}
    mock_char_manager.game_rules_service = MagicMock()
    mock_char_manager.tlk_service = MagicMock()

    feat_manager = FeatManager(mock_char_manager)

    result = feat_manager._build_pagination_response(
        feats=[],
        page=1,
        limit=10,
        total=0
    )

    pagination = result['pagination']
    assert pagination['total'] == 0
    assert pagination['pages'] == 1
    assert pagination['has_next'] == False
    assert pagination['has_previous'] == False
