# Generated manually to fix data before adding constraints

from django.db import migrations

def fix_character_data(apps, schema_editor):
    """Fix existing character data to meet new constraints"""
    Character = apps.get_model('character', 'Character')
    
    # Fix any characters with invalid levels
    Character.objects.filter(character_level__lt=1).update(character_level=1)
    Character.objects.filter(character_level__gt=40).update(character_level=40)
    
    # Fix any characters with invalid attributes
    for attr in ['strength', 'dexterity', 'constitution', 'intelligence', 'wisdom', 'charisma']:
        Character.objects.filter(**{f'{attr}__lt': 3}).update(**{attr: 3})
        Character.objects.filter(**{f'{attr}__gt': 50}).update(**{attr: 50})
    
    # Fix alignment values
    Character.objects.filter(law_chaos__lt=0).update(law_chaos=0)
    Character.objects.filter(law_chaos__gt=100).update(law_chaos=100)
    Character.objects.filter(good_evil__lt=0).update(good_evil=0)
    Character.objects.filter(good_evil__gt=100).update(good_evil=100)

def reverse_fix(apps, schema_editor):
    """No-op reverse migration"""
    pass

class Migration(migrations.Migration):

    dependencies = [
        ('character', '0005_add_campaign_support'),
    ]

    operations = [
        migrations.RunPython(fix_character_data, reverse_fix),
    ]