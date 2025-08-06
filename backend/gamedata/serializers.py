from rest_framework import serializers


class RaceSerializer(serializers.Serializer):
    id = serializers.IntegerField()
    name = serializers.CharField()
    ecl_modifier = serializers.IntegerField()
    playable = serializers.BooleanField()


class SubraceSerializer(serializers.Serializer):
    id = serializers.IntegerField()
    name = serializers.CharField()
    base_race = serializers.IntegerField()
    ecl_modifier = serializers.IntegerField()
    playable = serializers.BooleanField()


class ClassSerializer(serializers.Serializer):
    id = serializers.IntegerField()
    name = serializers.CharField()
    hit_die = serializers.IntegerField()
    skill_points = serializers.IntegerField()
    playable = serializers.BooleanField()
    spellcaster = serializers.BooleanField()
    primary_ability = serializers.CharField()


class FeatSerializer(serializers.Serializer):
    id = serializers.IntegerField()
    name = serializers.CharField()
    description = serializers.CharField()
    all_classes_can_use = serializers.BooleanField()
    category = serializers.CharField(allow_null=True)
    max_times = serializers.IntegerField()


class SkillSerializer(serializers.Serializer):
    id = serializers.IntegerField()
    name = serializers.CharField()
    description = serializers.CharField()
    key_ability = serializers.CharField()
    armor_check_penalty = serializers.BooleanField()
    untrained = serializers.BooleanField()


class SpellSerializer(serializers.Serializer):
    id = serializers.IntegerField()
    name = serializers.CharField()
    school = serializers.CharField(allow_null=True)
    range = serializers.CharField(allow_null=True)
    innate_level = serializers.IntegerField(allow_null=True)


class AlignmentSerializer(serializers.Serializer):
    id = serializers.CharField()
    name = serializers.CharField()
    law_chaos = serializers.IntegerField()
    good_evil = serializers.IntegerField()


class DomainSerializer(serializers.Serializer):
    id = serializers.IntegerField()
    name = serializers.CharField()
    description = serializers.CharField()


class DeitySerializer(serializers.Serializer):
    id = serializers.IntegerField()
    name = serializers.CharField()


class BaseItemSerializer(serializers.Serializer):
    id = serializers.IntegerField()
    name = serializers.CharField()
    category = serializers.CharField(allow_null=True)
    stackable = serializers.BooleanField(allow_null=True)
    base_cost = serializers.FloatField(allow_null=True)
    weight = serializers.FloatField(allow_null=True)


class SpellSchoolSerializer(serializers.Serializer):
    id = serializers.CharField()
    name = serializers.CharField()


class GenderSerializer(serializers.Serializer):
    id = serializers.IntegerField()
    name = serializers.CharField()


class FeatCategorySerializer(serializers.Serializer):
    id = serializers.CharField()
    name = serializers.CharField()