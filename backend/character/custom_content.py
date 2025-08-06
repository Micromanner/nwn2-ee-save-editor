"""
Custom content detection and protection for NWN2 characters
Identifies and protects non-vanilla content to prevent data loss
"""

from typing import Dict, List, Set, Tuple, Any, Optional
import json
import os
import logging

logger = logging.getLogger(__name__)


class CustomContentDetector:
    """Detects and manages custom content in character files"""
    
    # Standard vanilla ID ranges
    VANILLA_FEAT_MAX = 10000
    VANILLA_SPELL_MAX = 10000
    VANILLA_ITEM_MAX = 10000
    
    # Known epithet feat ranges (these are special and should be protected)
    EPITHET_FEAT_RANGES = [
        (3785, 3834),  # OC epithet feats
        (11509, 11994),  # MotB epithet feats
    ]
    
    def __init__(self, vanilla_rules=None):
        """
        Initialize the detector
        
        Args:
            vanilla_rules: Game rules service with vanilla data
        """
        self.vanilla_rules = vanilla_rules
        self._load_epithet_feats()
        
    def _load_epithet_feats(self):
        """Load known epithet feats from JSON file"""
        self.epithet_feats: Set[int] = set()
        
        # Try to load from the analysis scripts directory
        epithet_file = os.path.join(
            os.path.dirname(__file__), 
            '..', 'scripts', 'analysis', 'epithet_feats.json'
        )
        
        if os.path.exists(epithet_file):
            try:
                with open(epithet_file, 'r') as f:
                    data = json.load(f)
                    self.epithet_feats = set(data.get('epithet_feat_ids', []))
                logger.info(f"Loaded {len(self.epithet_feats)} epithet feats")
            except Exception as e:
                logger.warning(f"Could not load epithet feats: {e}")
        
        # Add known ranges as fallback
        for start, end in self.EPITHET_FEAT_RANGES:
            self.epithet_feats.update(range(start, end + 1))
    
    def detect_custom_content(self, character_data: Dict[str, Any]) -> Dict[str, Dict[str, Any]]:
        """
        Detect all custom content in a character
        
        Args:
            character_data: Raw GFF character data
            
        Returns:
            Dict mapping content IDs to protection info
        """
        custom_content = {}
        
        # Check feats
        custom_content.update(self._detect_custom_feats(character_data))
        
        # Check spells
        custom_content.update(self._detect_custom_spells(character_data))
        
        # Check items
        custom_content.update(self._detect_custom_items(character_data))
        
        return custom_content
    
    def _detect_custom_feats(self, character_data: Dict[str, Any]) -> Dict[str, Dict[str, Any]]:
        """Detect custom or special feats that need protection"""
        custom_feats = {}
        feat_list = character_data.get('FeatList', [])
        
        for i, feat in enumerate(feat_list):
            feat_id = feat.get('Feat', 0)
            protection_info = self._check_feat_protection(feat_id)
            
            if protection_info['protected']:
                custom_feats[f'feat_{feat_id}'] = {
                    'type': 'feat',
                    'id': feat_id,
                    'index': i,
                    'protected': True,
                    'reason': protection_info['reason'],
                    'description': protection_info.get('description', ''),
                    'removable': protection_info.get('removable', False)
                }
        
        return custom_feats
    
    def _check_feat_protection(self, feat_id: int) -> Dict[str, Any]:
        """
        Check if a feat should be protected
        
        Returns:
            Dict with protection info
        """
        # Epithet feats - always protected
        if feat_id in self.epithet_feats:
            return {
                'protected': True,
                'reason': 'epithet',
                'description': 'Epithet feat (story reward)',
                'removable': False
            }
        
        # Check if in epithet ranges
        for start, end in self.EPITHET_FEAT_RANGES:
            if start <= feat_id <= end:
                return {
                    'protected': True,
                    'reason': 'epithet_range',
                    'description': 'In epithet feat range',
                    'removable': False
                }
        
        # Custom content (high ID)
        if feat_id > self.VANILLA_FEAT_MAX:
            return {
                'protected': True,
                'reason': 'custom',
                'description': 'Custom module feat',
                'removable': True  # Can be removed if needed
            }
        
        # Unknown vanilla feat
        if self.vanilla_rules and feat_id not in self.vanilla_rules.feats:
            return {
                'protected': True,
                'reason': 'unknown',
                'description': 'Unknown feat ID',
                'removable': True
            }
        
        # Not protected
        return {'protected': False}
    
    def _detect_custom_spells(self, character_data: Dict[str, Any]) -> Dict[str, Dict[str, Any]]:
        """Detect custom spells"""
        custom_spells = {}
        
        # Check all spell lists (0-9 for spell levels)
        for level in range(10):
            spell_list_name = f'KnownList{level}'
            spell_list = character_data.get(spell_list_name, [])
            
            for i, spell in enumerate(spell_list):
                spell_id = spell.get('Spell', 0)
                
                # Custom spell check
                is_custom = (
                    spell_id > self.VANILLA_SPELL_MAX or
                    (self.vanilla_rules and spell_id not in self.vanilla_rules.spells)
                )
                
                if is_custom:
                    custom_spells[f'spell_{spell_id}'] = {
                        'type': 'spell',
                        'id': spell_id,
                        'level': level,
                        'list': spell_list_name,
                        'index': i,
                        'protected': True,
                        'reason': 'custom' if spell_id > self.VANILLA_SPELL_MAX else 'unknown',
                        'removable': True
                    }
        
        return custom_spells
    
    def _detect_custom_items(self, character_data: Dict[str, Any]) -> Dict[str, Dict[str, Any]]:
        """Detect custom items in inventory"""
        custom_items = {}
        
        # Check equipped items
        for slot in ['Head', 'Chest', 'Boots', 'Arms', 'RightHand', 'LeftHand', 
                     'Cloak', 'LeftRing', 'RightRing', 'Neck', 'Belt', 'Arrows', 
                     'Bullets', 'Bolts']:
            item = character_data.get(slot)
            if item and isinstance(item, dict):
                custom_info = self._check_item_custom(item)
                if custom_info:
                    custom_items[f'equipped_{slot}'] = custom_info
        
        # Check inventory items
        item_list = character_data.get('ItemList', [])
        for i, item in enumerate(item_list):
            if isinstance(item, dict):
                custom_info = self._check_item_custom(item)
                if custom_info:
                    custom_items[f'inventory_{i}'] = {
                        **custom_info,
                        'index': i
                    }
        
        return custom_items
    
    def _check_item_custom(self, item: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        """Check if an item is custom"""
        base_item = item.get('BaseItem', 0)
        
        if base_item > self.VANILLA_ITEM_MAX or (
            self.vanilla_rules and base_item not in self.vanilla_rules.base_items
        ):
            return {
                'type': 'item',
                'id': base_item,
                'protected': True,
                'reason': 'custom' if base_item > self.VANILLA_ITEM_MAX else 'unknown',
                'tag': item.get('Tag', ''),
                'resref': item.get('TemplateResRef', ''),
                'removable': True
            }
        
        return None
    
    def get_protection_summary(self, custom_content: Dict[str, Dict[str, Any]]) -> Dict[str, Any]:
        """Get a summary of protected content"""
        summary = {
            'total': len(custom_content),
            'by_type': {},
            'by_reason': {},
            'epithet_feats': [],
            'custom_feats': [],
            'custom_spells': [],
            'custom_items': []
        }
        
        for key, info in custom_content.items():
            content_type = info['type']
            reason = info['reason']
            
            # Count by type
            summary['by_type'][content_type] = summary['by_type'].get(content_type, 0) + 1
            
            # Count by reason
            summary['by_reason'][reason] = summary['by_reason'].get(reason, 0) + 1
            
            # Categorize
            if content_type == 'feat':
                if reason in ['epithet', 'epithet_range']:
                    summary['epithet_feats'].append(info['id'])
                else:
                    summary['custom_feats'].append(info['id'])
            elif content_type == 'spell':
                summary['custom_spells'].append({
                    'id': info['id'],
                    'level': info.get('level', 0)
                })
            elif content_type == 'item':
                summary['custom_items'].append({
                    'id': info['id'],
                    'tag': info.get('tag', '')
                })
        
        return summary
    
    def should_protect_content(self, content_type: str, content_id: int) -> bool:
        """
        Quick check if specific content should be protected
        
        Args:
            content_type: 'feat', 'spell', or 'item'
            content_id: The content ID
            
        Returns:
            True if content should be protected
        """
        if content_type == 'feat':
            return self._check_feat_protection(content_id)['protected']
        elif content_type == 'spell':
            return (content_id > self.VANILLA_SPELL_MAX or 
                   (self.vanilla_rules and content_id not in self.vanilla_rules.spells))
        elif content_type == 'item':
            return (content_id > self.VANILLA_ITEM_MAX or
                   (self.vanilla_rules and content_id not in self.vanilla_rules.base_items))
        
        return False