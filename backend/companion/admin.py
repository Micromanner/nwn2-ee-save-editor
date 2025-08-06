"""
Admin configuration for companions
"""

from django.contrib import admin
from .models import Companion, CompanionClass, CompanionFeat, CompanionSkill


class CompanionClassInline(admin.TabularInline):
    model = CompanionClass
    extra = 0


class CompanionFeatInline(admin.TabularInline):
    model = CompanionFeat
    extra = 0
    

class CompanionSkillInline(admin.TabularInline):
    model = CompanionSkill
    extra = 0


@admin.register(Companion)
class CompanionAdmin(admin.ModelAdmin):
    list_display = ['first_name', 'last_name', 'roster_tag', 'primary_class', 
                    'total_level', 'owner', 'created_at']
    list_filter = ['race_id', 'gender']
    search_fields = ['first_name', 'last_name', 'roster_tag', 'tag']
    readonly_fields = ['created_at', 'updated_at']
    
    inlines = [CompanionClassInline, CompanionFeatInline, CompanionSkillInline]
    
    fieldsets = (
        ('Metadata', {
            'fields': ('owner', 'file_name', 'file_path', 'created_at', 'updated_at')
        }),
        ('Identity', {
            'fields': ('first_name', 'last_name', 'tag', 'roster_tag')
        }),
        ('Basic Info', {
            'fields': ('age', 'gender', 'race_id', 'subrace_id', 'deity')
        }),
        ('Alignment', {
            'fields': ('law_chaos', 'good_evil')
        }),
        ('Abilities', {
            'fields': ('strength', 'dexterity', 'constitution', 
                      'intelligence', 'wisdom', 'charisma')
        }),
        ('Combat Stats', {
            'fields': ('hit_points', 'max_hit_points', 'current_hit_points',
                      'armor_class', 'base_attack_bonus')
        }),
        ('Saves', {
            'fields': ('fortitude_save', 'reflex_save', 'will_save')
        }),
        ('NPC Data', {
            'fields': ('action_list', 'effect_list', 'personal_rep_list', 'var_table'),
            'classes': ('collapse',)
        }),
        ('Additional Data', {
            'fields': ('additional_data',),
            'classes': ('collapse',)
        })
    )
