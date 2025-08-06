from rest_framework import serializers
from character.models import Character, CharacterClass, CharacterFeat, CharacterSkill, CharacterSpell, CharacterItem


class CharacterClassSerializer(serializers.ModelSerializer):
    class Meta:
        model = CharacterClass
        fields = ['id', 'class_id', 'class_name', 'class_level', 
                  'domain1_id', 'domain1_name', 'domain2_id', 'domain2_name']


class CharacterFeatSerializer(serializers.ModelSerializer):
    class Meta:
        model = CharacterFeat
        fields = ['id', 'feat_id', 'feat_name']


class CharacterSkillSerializer(serializers.ModelSerializer):
    class Meta:
        model = CharacterSkill
        fields = ['id', 'skill_id', 'skill_name', 'rank']


class CharacterSpellSerializer(serializers.ModelSerializer):
    class Meta:
        model = CharacterSpell
        fields = ['id', 'spell_id', 'spell_name', 'spell_level', 
                  'class_index', 'is_memorized']


class CharacterItemSerializer(serializers.ModelSerializer):
    display_name = serializers.ReadOnlyField()
    
    class Meta:
        model = CharacterItem
        fields = ['id', 'base_item_id', 'base_item_name', 'localized_name',
                  'display_name', 'stack_size', 'location', 'inventory_slot', 
                  'properties']


class CharacterListSerializer(serializers.ModelSerializer):
    """Lightweight serializer for character lists"""
    alignment = serializers.ReadOnlyField()
    primary_class = serializers.SerializerMethodField()
    
    class Meta:
        model = Character
        fields = ['id', 'first_name', 'last_name', 'race_name', 'alignment',
                  'character_level', 'primary_class', 'is_companion', 
                  'updated_at', 'file_name']
                  
    def get_primary_class(self, obj):
        """Get the highest level class"""
        primary = obj.classes.order_by('-class_level').first()
        return f"{primary.class_name} {primary.class_level}" if primary else "No Class"


class CharacterDetailSerializer(serializers.ModelSerializer):
    """Full character details with all fields"""
    alignment = serializers.ReadOnlyField()
    classes = CharacterClassSerializer(many=True, read_only=True)
    feats = CharacterFeatSerializer(many=True, read_only=True)
    skills = CharacterSkillSerializer(many=True, read_only=True)
    spells = CharacterSpellSerializer(many=True, read_only=True)
    items = CharacterItemSerializer(many=True, read_only=True)
    
    class Meta:
        model = Character
        fields = [
            # Core identity & stats
            'id',
            'first_name', 'last_name', 'age', 'gender', 'deity',
            'race_id', 'race_name', 'subrace_id', 'subrace_name',
            'law_chaos', 'good_evil', 'alignment',
            'experience', 'character_level',
            'strength', 'dexterity', 'constitution',
            'intelligence', 'wisdom', 'charisma',
            
            # Appearance fields (IMPORTANT FOR FRONTEND!)
            'appearance_type', 'appearance_head', 'appearance_hair', 'appearance_f_hair',
            'tail', 'wings',  # Key fields for customization
            'color_tattoo1', 'color_tattoo2',
            'portrait', 'custom_portrait',
            'never_draw_helmet',
            'model_scale', 'tint_hair', 'tint_head', 'tintable',
            'appearance_sef', 'armor_tint',
            
            # Combat & defense
            'hit_points', 'max_hit_points', 'current_hit_points',
            'armor_class', 'base_attack_bonus',
            'fortitude_save', 'reflex_save', 'will_save',
            'natural_ac', 'faction_id',
            'damage_min', 'damage_max',
            
            # Position & location
            'area_id',
            'x_position', 'y_position', 'z_position',
            'x_orientation', 'y_orientation', 'z_orientation',
            
            # Flags & states
            'is_pc', 'is_commandable', 'is_dm', 'is_immortal',
            'is_companion', 'is_destroyable', 'is_raiseable',
            
            # Module & campaign
            'mod_is_primary_plr', 'mod_last_mod_id',
            'mod_commnty_id', 'mod_commnty_name',
            'module_name', 'campaign_name', 'campaign_path',
            'campaign_modules', 'campaign_level_cap',
            
            # Quest and story progress
            'completed_quests_count', 'active_quests_count',
            'companion_influence', 'unlocked_locations', 'current_area',
            
            # Enhanced campaign overview
            'game_act', 'difficulty_level', 'last_saved_timestamp',
            'companion_status', 'hidden_statistics', 'story_milestones', 'quest_details',
            
            # Other important fields
            'gold', 'skill_points',
            'sound_set_file', 'movement_rate',
            
            # Metadata
            'file_name', 'file_path', 'created_at', 'updated_at',
            
            # Related data
            'classes', 'feats', 'skills', 'spells', 'items'
        ]
        read_only_fields = ['alignment', 'created_at', 'updated_at']


class CharacterUpdateSerializer(serializers.ModelSerializer):
    """Serializer for updating character data"""
    class Meta:
        model = Character
        fields = [
            # Basic info
            'first_name', 'last_name', 'age', 'gender', 'deity',
            'law_chaos', 'good_evil', 'experience',
            
            # Abilities
            'strength', 'dexterity', 'constitution',
            'intelligence', 'wisdom', 'charisma',
            
            # Combat stats
            'hit_points', 'max_hit_points', 'current_hit_points',
            'armor_class', 'base_attack_bonus',
            'fortitude_save', 'reflex_save', 'will_save',
            
            # Appearance (KEY FIELDS FOR EDITING!)
            'appearance_type', 'appearance_head', 'appearance_hair',
            'tail', 'wings',  # MOST IMPORTANT!
            'color_tattoo1', 'color_tattoo2',
            'portrait', 'custom_portrait', 'never_draw_helmet',
            
            # Other
            'gold', 'faction_id', 'skill_points'
        ]
        
    def validate_law_chaos(self, value):
        if not 0 <= value <= 100:
            raise serializers.ValidationError("Law/Chaos must be between 0 and 100")
        return value
        
    def validate_good_evil(self, value):
        if not 0 <= value <= 100:
            raise serializers.ValidationError("Good/Evil must be between 0 and 100")
        return value
        
    def validate(self, data):
        """Additional cross-field validation"""
        # Validate ability scores
        for ability in ['strength', 'dexterity', 'constitution', 
                       'intelligence', 'wisdom', 'charisma']:
            if ability in data:
                value = data[ability]
                if value < 3 or value > 50:
                    raise serializers.ValidationError(
                        f"{ability.capitalize()} must be between 3 and 50"
                    )
                    
        # Validate hit points
        if 'hit_points' in data and 'max_hit_points' in data:
            if data['hit_points'] > data['max_hit_points']:
                raise serializers.ValidationError(
                    "Hit points cannot exceed maximum hit points"
                )
                
        return data


class FileUploadSerializer(serializers.Serializer):
    """Serializer for file upload"""
    file = serializers.FileField()
    
    def validate_file(self, value):
        # Check file extension
        valid_extensions = ['.bic', '.ros']
        ext = value.name.lower()[-4:]
        if ext not in valid_extensions:
            raise serializers.ValidationError(
                f"Invalid file type. Allowed types: {', '.join(valid_extensions)}"
            )
        # Check file size (10MB max)
        if value.size > 10 * 1024 * 1024:
            raise serializers.ValidationError("File size cannot exceed 10MB")
        return value