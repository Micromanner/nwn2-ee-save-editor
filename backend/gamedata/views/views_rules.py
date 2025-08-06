"""
API views for game rules
"""

from rest_framework.views import APIView
from rest_framework.response import Response
from rest_framework import status
from rest_framework.permissions import IsAuthenticated

from gamedata.middleware import get_game_rules_service


class GameRulesAPIView(APIView):
    """Base class for game rules API views"""
    
    @property
    def game_rules(self):
        """Get game rules service from middleware or create default"""
        grs = get_game_rules_service()
        if grs:
            return grs
        # Fallback if middleware not active
        from gamedata.services.game_rules_service import GameRulesService
        return GameRulesService()


class GameRulesView(GameRulesAPIView):
    """Get all game rules for frontend"""
    
    def get(self, request):
        """Get game rules summary"""
        rules_summary = self.self.game_rules.get_all_rules_summary()
        return Response(rules_summary)


class ValidateFeatView(GameRulesAPIView):
    """Validate feat selection"""
    permission_classes = [IsAuthenticated]
    
    def post(self, request):
        """
        Validate if a character can select a feat
        
        Request body:
        {
            "character_data": {
                "level": 10,
                "feat_ids": [1, 2, 3],
                "strength": 16,
                "dexterity": 14,
                ...
            },
            "feat_id": 42,
            "cheat_mode": false
        }
        """
        character_data = request.data.get('character_data', {})
        feat_id = request.data.get('feat_id')
        cheat_mode = request.data.get('cheat_mode', False)
        
        if feat_id is None:
            return Response(
                {'error': 'feat_id is required'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        is_valid, errors = self.self.game_rules.validate_feat_selection(
            character_data, feat_id, cheat_mode
        )
        
        feat = self.game_rules.feats.get(feat_id)
        feat_info = {
            'id': feat_id,
            'name': feat.name if feat else f'Feat {feat_id}',
            'valid': is_valid,
            'errors': errors,
            'cheat_mode': cheat_mode
        }
        
        return Response(feat_info)


class AvailableFeatsView(GameRulesAPIView):
    """Get available feats for a character"""
    permission_classes = [IsAuthenticated]
    
    def post(self, request):
        """
        Get list of available feats
        
        Request body same as ValidateFeatView
        """
        character_data = request.data.get('character_data', {})
        cheat_mode = request.data.get('cheat_mode', False)
        
        available_feats = self.game_rules.get_available_feats(
            character_data, cheat_mode
        )
        
        return Response({
            'count': len(available_feats),
            'cheat_mode': cheat_mode,
            'feats': available_feats
        })


class ValidateClassChangeView(GameRulesAPIView):
    """Validate class change"""
    permission_classes = [IsAuthenticated]
    
    def post(self, request):
        """
        Validate if a character can change to a class
        
        Request body:
        {
            "character_data": {
                "alignment": {"law_chaos": 50, "good_evil": 80},
                ...
            },
            "new_class_id": 6,  # Paladin
            "cheat_mode": false
        }
        """
        character_data = request.data.get('character_data', {})
        new_class_id = request.data.get('new_class_id')
        cheat_mode = request.data.get('cheat_mode', False)
        
        if new_class_id is None:
            return Response(
                {'error': 'new_class_id is required'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        is_valid, errors = self.game_rules.validate_class_change(
            character_data, new_class_id, cheat_mode
        )
        
        class_info = self.game_rules.classes.get(new_class_id)
        
        return Response({
            'class_id': new_class_id,
            'class_name': class_info.name if class_info else f'Class {new_class_id}',
            'valid': is_valid,
            'errors': errors,
            'cheat_mode': cheat_mode
        })


class CalculateStatsView(GameRulesAPIView):
    """Calculate character stats based on race, class, level"""
    permission_classes = [IsAuthenticated]
    
    def post(self, request):
        """
        Calculate stats for a character
        
        Request body:
        {
            "race_id": 0,
            "class_id": 4,
            "level": 10,
            "base_abilities": {
                "strength": 16,
                "dexterity": 14,
                ...
            }
        }
        """
        race_id = request.data.get('race_id', 0)
        class_id = request.data.get('class_id', 0)
        level = request.data.get('level', 1)
        base_abilities = request.data.get('base_abilities', {})
        
        # Build character data
        character_data = {
            'race_id': race_id,
            'class_id': class_id,
            'level': level,
            **base_abilities
        }
        
        # Calculate ability modifiers
        modifiers = self.game_rules.calculate_ability_modifiers(character_data)
        
        # Get class data
        class_data = self.game_rules.classes.get(class_id)
        if not class_data:
            return Response(
                {'error': f'Invalid class ID: {class_id}'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        # Calculate derived stats
        level_idx = min(level - 1, 19)  # Cap at 20
        
        # BAB
        bab = self.game_rules.BAB_PROGRESSION[class_data.bab_type][level_idx]
        
        # Saves
        fort_base = self.game_rules.SAVE_PROGRESSION[class_data.fort_save][level_idx]
        ref_base = self.game_rules.SAVE_PROGRESSION[class_data.ref_save][level_idx]
        will_base = self.game_rules.SAVE_PROGRESSION[class_data.will_save][level_idx]
        
        # Add ability modifiers
        fort_total = fort_base + modifiers['CON']
        ref_total = ref_base + modifiers['DEX']
        will_total = will_base + modifiers['WIS']
        
        # Hit points
        con_mod = modifiers['CON']
        base_hp = class_data.hit_die  # Max at level 1
        if level > 1:
            avg_roll = (class_data.hit_die + 1) // 2
            base_hp += avg_roll * (level - 1)
        total_hp = base_hp + (con_mod * level)
        total_hp = max(1, total_hp)
        
        # Skill points
        int_mod = modifiers['INT']
        skill_points = self.game_rules.calculate_skill_points(class_id, level, int_mod)
        
        return Response({
            'abilities_with_race': {
                ability: base_abilities.get(ability.lower(), 10) + 
                        getattr(self.game_rules.races.get(race_id), f'{ability.lower()}_adjust', 0)
                for ability in ['STR', 'DEX', 'CON', 'INT', 'WIS', 'CHA']
            },
            'ability_modifiers': modifiers,
            'base_attack_bonus': bab,
            'saves': {
                'fortitude': {'base': fort_base, 'total': fort_total},
                'reflex': {'base': ref_base, 'total': ref_total},
                'will': {'base': will_base, 'total': will_total},
            },
            'hit_points': total_hp,
            'skill_points': skill_points,
            'class_skills': self.game_rules.get_class_skills(class_id)
        })


class ClassSkillsView(GameRulesAPIView):
    """Get class skills for a specific class"""
    
    def get(self, request, class_id):
        """Get class skills"""
        class_skills = self.game_rules.get_class_skills(class_id)
        
        # Get skill details
        skills_data = []
        for skill_id in class_skills:
            skill = self.game_rules.skills.get(skill_id)
            if skill:
                skills_data.append({
                    'id': skill_id,
                    'name': skill.name,
                    'key_ability': skill.key_ability,
                    'armor_check': skill.armor_check_penalty,
                    'untrained': skill.untrained
                })
        
        return Response({
            'class_id': class_id,
            'class_name': self.game_rules.classes.get(class_id, {}).name,
            'skills': skills_data
        })