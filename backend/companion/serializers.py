"""
Serializers for companion API
"""

from rest_framework import serializers
from .models import Companion, CompanionClass, CompanionFeat, CompanionSkill


class CompanionClassSerializer(serializers.ModelSerializer):
    class_name = serializers.SerializerMethodField()
    
    class Meta:
        model = CompanionClass
        fields = ['id', 'class_id', 'class_level', 'class_name']
        
    def get_class_name(self, obj):
        return obj.companion._get_class_name(obj.class_id)


class CompanionFeatSerializer(serializers.ModelSerializer):
    class Meta:
        model = CompanionFeat
        fields = ['id', 'feat_id']


class CompanionSkillSerializer(serializers.ModelSerializer):
    class Meta:
        model = CompanionSkill
        fields = ['id', 'skill_id', 'rank']


class CompanionListSerializer(serializers.ModelSerializer):
    """Serializer for companion list view"""
    primary_class = serializers.ReadOnlyField()
    total_level = serializers.ReadOnlyField()
    
    class Meta:
        model = Companion
        fields = ['id', 'first_name', 'last_name', 'roster_tag', 
                  'primary_class', 'total_level', 'race_id', 'gender',
                  'created_at', 'updated_at']


class CompanionDetailSerializer(serializers.ModelSerializer):
    """Detailed companion serializer"""
    classes = CompanionClassSerializer(many=True, read_only=True)
    feats = CompanionFeatSerializer(many=True, read_only=True)
    skills = CompanionSkillSerializer(many=True, read_only=True)
    primary_class = serializers.ReadOnlyField()
    total_level = serializers.ReadOnlyField()
    
    class Meta:
        model = Companion
        fields = '__all__'


class CompanionImportSerializer(serializers.Serializer):
    """Serializer for companion file import"""
    file = serializers.FileField()
    
    def validate_file(self, value):
        if not value.name.endswith('.ros'):
            raise serializers.ValidationError("Only .ros files are supported")
        return value
