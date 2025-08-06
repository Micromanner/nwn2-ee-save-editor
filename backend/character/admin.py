from django.contrib import admin
from .models import Character, CharacterClass, CharacterFeat, CharacterSkill, CharacterSpell, CharacterItem


class CharacterClassInline(admin.TabularInline):
    model = CharacterClass
    extra = 0
    fields = ['class_name', 'class_level', 'domain1_name', 'domain2_name']


class CharacterItemInline(admin.TabularInline):
    model = CharacterItem
    extra = 0
    fields = ['location', 'display_name', 'stack_size']
    readonly_fields = ['display_name']


@admin.register(Character)
class CharacterAdmin(admin.ModelAdmin):
    list_display = ['__str__', 'first_name', 'last_name', 'race_name', 'alignment', 'character_level', 'updated_at']
    list_filter = ['is_companion', 'gender', 'race_name']
    search_fields = ['first_name', 'last_name', 'file_name']
    readonly_fields = ['created_at', 'updated_at', 'alignment']
    
    fieldsets = [
        ('Metadata', {
            'fields': ['owner', 'file_name', 'file_path', 'is_companion', 'created_at', 'updated_at']
        }),
        ('Basic Info', {
            'fields': ['first_name', 'last_name', 'age', 'gender', 'deity']
        }),
        ('Race', {
            'fields': [('race_id', 'race_name'), ('subrace_id', 'subrace_name')]
        }),
        ('Alignment', {
            'fields': ['alignment', ('law_chaos', 'good_evil')]
        }),
        ('Level & Experience', {
            'fields': ['character_level', 'experience']
        }),
        ('Abilities', {
            'fields': [
                ('strength', 'dexterity'),
                ('constitution', 'intelligence'),
                ('wisdom', 'charisma')
            ]
        }),
        ('Combat', {
            'fields': [
                ('hit_points', 'max_hit_points'),
                'armor_class',
                ('fortitude_save', 'reflex_save', 'will_save')
            ]
        }),
        ('Wealth', {
            'fields': ['gold']
        }),
    ]
    
    inlines = [CharacterClassInline, CharacterItemInline]


@admin.register(CharacterFeat)
class CharacterFeatAdmin(admin.ModelAdmin):
    list_display = ['feat_name', 'character']
    list_filter = ['character']
    search_fields = ['feat_name']


@admin.register(CharacterSkill)
class CharacterSkillAdmin(admin.ModelAdmin):
    list_display = ['skill_name', 'rank', 'character']
    list_filter = ['character', 'rank']
    search_fields = ['skill_name']


@admin.register(CharacterSpell)
class CharacterSpellAdmin(admin.ModelAdmin):
    list_display = ['spell_name', 'spell_level', 'is_memorized', 'character']
    list_filter = ['spell_level', 'is_memorized', 'character']
    search_fields = ['spell_name']


@admin.register(CharacterItem)
class CharacterItemAdmin(admin.ModelAdmin):
    list_display = ['display_name', 'location', 'stack_size', 'character']
    list_filter = ['location', 'character']
    search_fields = ['localized_name', 'base_item_name']
