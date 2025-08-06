"""
Companion models for NWN2 .ros files
Auto-generated and then manually refined
"""

from django.db import models
from django.contrib.auth import get_user_model
from django.core.validators import MinValueValidator, MaxValueValidator

User = get_user_model()


class Companion(models.Model):
    """Model for NWN2 companion characters (.ros files)"""
    
    # Metadata fields
    owner = models.ForeignKey(User, on_delete=models.CASCADE, related_name='companions')
    file_name = models.CharField(max_length=255)
    file_path = models.CharField(max_length=500)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    
    # Core identification fields - matches Character model
    first_name = models.CharField(max_length=255, db_index=True)
    last_name = models.CharField(max_length=255, blank=True, default='')
    tag = models.CharField(max_length=32, blank=True, default='', help_text='Creature tag')
    roster_tag = models.CharField(max_length=32, blank=True, default='', help_text='Roster identifier')
    
    # Basic attributes - matches Character model
    age = models.IntegerField(default=0)
    gender = models.IntegerField(default=0)
    race_id = models.IntegerField(default=0)
    subrace_id = models.IntegerField(default=0)
    deity = models.CharField(max_length=32, blank=True, default='')
    
    # Alignment
    law_chaos = models.IntegerField(default=50, validators=[MinValueValidator(0), MaxValueValidator(100)])
    good_evil = models.IntegerField(default=50, validators=[MinValueValidator(0), MaxValueValidator(100)])
    
    # Core stats
    strength = models.IntegerField(default=10)
    dexterity = models.IntegerField(default=10)
    constitution = models.IntegerField(default=10)
    intelligence = models.IntegerField(default=10)
    wisdom = models.IntegerField(default=10)
    charisma = models.IntegerField(default=10)
    
    # Combat stats
    hit_points = models.IntegerField(default=1)
    max_hit_points = models.IntegerField(default=1)
    current_hit_points = models.IntegerField(default=1)
    armor_class = models.IntegerField(default=10)
    base_attack_bonus = models.IntegerField(default=0)
    
    # Saves
    fortitude_save = models.IntegerField(default=0)
    reflex_save = models.IntegerField(default=0)
    will_save = models.IntegerField(default=0)
    
    # NPC-specific fields
    action_list = models.JSONField(null=True, blank=True, default=list, help_text='ActionList')
    effect_list = models.JSONField(null=True, blank=True, default=list, help_text='EffectList')
    personal_rep_list = models.JSONField(null=True, blank=True, default=list, help_text='PersonalRepList')
    var_table = models.JSONField(null=True, blank=True, default=list, help_text='VarTable')
    
    # All other fields stored as JSON for flexibility
    additional_data = models.JSONField(default=dict, blank=True)
    
    class Meta:
        db_table = 'companion'
        ordering = ['first_name', 'last_name']
        
    def __str__(self):
        name = self.first_name
        if self.last_name:
            name += f" {self.last_name}"
        return f"{name} ({self.roster_tag})" if self.roster_tag else name
        
    @property
    def total_level(self):
        """Calculate total character level from all classes"""
        return sum(cls.class_level for cls in self.classes.all())
        
    @property
    def primary_class(self):
        """Get the primary (highest level) class"""
        primary = self.classes.order_by('-class_level').first()
        if primary:
            return self._get_class_name(primary.class_id)
        return "No Class"
        
    def _get_class_name(self, class_id):
        """Get class name from ID"""
        class_names = {
            0: "Barbarian", 1: "Bard", 2: "Cleric", 3: "Druid",
            4: "Fighter", 5: "Monk", 6: "Paladin", 7: "Ranger",
            8: "Rogue", 9: "Sorcerer", 10: "Wizard", 11: "Warlock",
            # Add prestige classes as needed
        }
        return class_names.get(class_id, f"Class {class_id}")


class CompanionClass(models.Model):
    """Character class information for companions"""
    companion = models.ForeignKey(Companion, on_delete=models.CASCADE, related_name='classes')
    class_id = models.IntegerField()
    class_level = models.IntegerField(default=1)
    
    class Meta:
        db_table = 'companion_class'
        ordering = ['-class_level', 'class_id']
        

class CompanionFeat(models.Model):
    """Feats for companions"""
    companion = models.ForeignKey(Companion, on_delete=models.CASCADE, related_name='feats')
    feat_id = models.IntegerField()
    
    class Meta:
        db_table = 'companion_feat'
        

class CompanionSkill(models.Model):
    """Skills for companions"""
    companion = models.ForeignKey(Companion, on_delete=models.CASCADE, related_name='skills')
    skill_id = models.IntegerField()
    rank = models.IntegerField(default=0)
    
    class Meta:
        db_table = 'companion_skill'
        unique_together = [['companion', 'skill_id']]
