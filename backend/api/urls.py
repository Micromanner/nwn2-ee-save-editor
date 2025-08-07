from django.urls import path, include
from django.views.decorators.csrf import csrf_exempt
from rest_framework.routers import DefaultRouter
from .views import (
    CharacterViewSet, AttributeViewSet, SkillViewSet, FeatViewSet,
    SpellViewSet, CombatViewSet, InventoryViewSet, ClassViewSet,
    RaceViewSet, SaveViewSet
)
from .views.session_views import CharacterSessionViewSet
from . import savegame_views
from . import character_creation_views
from . import character_export_views
from .system_views import health_check, ready_check, cache_status, rebuild_cache, get_config, update_config, nwn2_data_path_config, auto_discover_nwn2, trigger_background_loading, background_loading_status_endpoint, initialization_status_endpoint

# Main router
router = DefaultRouter()
router.register(r'characters', CharacterViewSet)

urlpatterns = [
    path('', include(router.urls)),
    
    # Character subsystem endpoints (nested manually)
    path('characters/<int:character_pk>/attributes/', include([
        path('state/', AttributeViewSet.as_view({'get': 'attributes_state'}), name='character-attributes-state'),
        path('update/', AttributeViewSet.as_view({'post': 'change_attributes'}), name='character-attributes-update'),
        path('<str:pk>/set/', AttributeViewSet.as_view({'post': 'set_attribute'}), name='character-attribute-set'),
        path('point-buy/', AttributeViewSet.as_view({'post': 'set_point_buy'}), name='character-attributes-point-buy'),
        path('roll/', AttributeViewSet.as_view({'post': 'roll_attributes'}), name='character-attributes-roll'),
        path('modifiers/', AttributeViewSet.as_view({'get': 'get_modifiers'}), name='character-attributes-modifiers'),
    ])),
    
    path('characters/<int:character_pk>/skills/', include([
        path('state/', SkillViewSet.as_view({'get': 'skills_state'}), name='character-skills-state'),
        path('update/', SkillViewSet.as_view({'post': 'update_skills'}), name='character-skills-update'),
        path('batch/', SkillViewSet.as_view({'post': 'batch_update'}), name='character-skills-batch'),
        path('reset/', SkillViewSet.as_view({'post': 'reset_skills'}), name='character-skills-reset'),
        path('<int:pk>/check/', SkillViewSet.as_view({'get': 'skill_check'}), name='character-skill-check'),
        path('<int:pk>/prerequisites/', SkillViewSet.as_view({'get': 'skill_prerequisites'}), name='character-skill-prerequisites'),
        path('export/', SkillViewSet.as_view({'get': 'export_build'}), name='character-skills-export'),
        path('import/', SkillViewSet.as_view({'post': 'import_build'}), name='character-skills-import'),
    ])),
    
    path('characters/<int:character_pk>/feats/', include([
        path('state/', FeatViewSet.as_view({'get': 'feats_state'}), name='character-feats-state'),
        path('available/', FeatViewSet.as_view({'get': 'available_feats'}), name='character-feats-available'),
        path('legitimate/', FeatViewSet.as_view({'get': 'legitimate_feats'}), name='character-feats-legitimate'),
        path('add/', FeatViewSet.as_view({'post': 'add_feat'}), name='character-feats-add'),
        path('remove/', FeatViewSet.as_view({'post': 'remove_feat'}), name='character-feats-remove'),
        path('<int:pk>/prerequisites/', FeatViewSet.as_view({'get': 'feat_prerequisites'}), name='character-feat-prerequisites'),
        path('<int:pk>/check/', FeatViewSet.as_view({'get': 'check_prerequisites'}), name='character-feat-check'),
        path('<int:pk>/details/', FeatViewSet.as_view({'get': 'feat_details'}), name='character-feat-details'),
        path('<int:pk>/validate/', FeatViewSet.as_view({'get': 'validate_feat'}), name='character-feat-validate'),
        path('by-category/', FeatViewSet.as_view({'get': 'feats_by_category'}), name='character-feats-by-category'),
    ])),
    
    path('characters/<int:character_pk>/combat/', include([
        path('state/', CombatViewSet.as_view({'get': 'combat_state'}), name='character-combat-state'),
        path('bab/', CombatViewSet.as_view({'get': 'base_attack_bonus'}), name='character-combat-bab'),
        path('ac/', CombatViewSet.as_view({'get': 'armor_class'}), name='character-combat-ac'),
        path('update-ac/', CombatViewSet.as_view({'post': 'update_natural_armor'}), name='character-combat-update-ac'),
        path('attacks/', CombatViewSet.as_view({'get': 'attack_bonuses'}), name='character-combat-attacks'),
        path('damage/', CombatViewSet.as_view({'get': 'damage_bonuses'}), name='character-combat-damage'),
        path('weapons/', CombatViewSet.as_view({'get': 'equipped_weapons'}), name='character-combat-weapons'),
        path('simulate/', CombatViewSet.as_view({'post': 'simulate_attack'}), name='character-combat-simulate'),
        path('defensive/', CombatViewSet.as_view({'get': 'defensive_stats'}), name='character-combat-defensive'),
    ])),
    
    path('characters/<int:character_pk>/saves/', include([
        path('state/', SaveViewSet.as_view({'get': 'saves_state'}), name='character-saves-state'),
        path('breakdown/', SaveViewSet.as_view({'get': 'save_breakdown'}), name='character-saves-breakdown'),
        path('update/', SaveViewSet.as_view({'post': 'update_save_bonuses'}), name='character-saves-update'),
        path('simulate/', SaveViewSet.as_view({'post': 'simulate_save'}), name='character-saves-simulate'),
        path('resistances/', SaveViewSet.as_view({'get': 'damage_resistances'}), name='character-saves-resistances'),
    ])),
    
    path('characters/<int:character_pk>/spells/', include([
        path('state/', SpellViewSet.as_view({'get': 'spells_state'}), name='character-spells-state'),
        path('available/', SpellViewSet.as_view({'get': 'available_spells'}), name='character-spells-available'),
        path('all/', SpellViewSet.as_view({'get': 'all_spells'}), name='character-spells-all'),
        path('manage/', SpellViewSet.as_view({'post': 'manage_spells'}), name='character-spells-manage'),
    ])),
    
    path('characters/<int:character_pk>/inventory/', include([
        path('state/', InventoryViewSet.as_view({'get': 'inventory_state'}), name='character-inventory-state'),
    ])),
    
    path('characters/<int:character_pk>/classes/', include([
        path('state/', ClassViewSet.as_view({'get': 'classes_state'}), name='character-classes-state'),
        path('change/', ClassViewSet.as_view({'post': 'change_class'}), name='character-class-change'),
        path('level-up/', ClassViewSet.as_view({'post': 'level_up'}), name='character-classes-level-up'),
        path('categorized/', ClassViewSet.as_view({'get': 'get_categorized_classes'}), name='character-classes-categorized'),
        path('features/<int:class_id>/', ClassViewSet.as_view({'get': 'get_class_features'}), name='character-class-features'),
    ])),
    
    # Standalone class endpoints (don't require character context)
    path('classes/categorized/', ClassViewSet.as_view({'get': 'get_categorized_classes_standalone'}), name='classes-categorized'),
    path('classes/features/<int:class_id>/', ClassViewSet.as_view({'get': 'get_class_features'}), name='class-features'),
    
    path('characters/<int:character_pk>/race/', include([
        path('change/', RaceViewSet.as_view({'post': 'change_race'}), name='character-race-change'),
    ])),
    
    # Character session management endpoints
    path('characters/<int:character_pk>/session/', include([
        path('start/', CharacterSessionViewSet.as_view({'post': 'start_session'}), name='character-session-start'),
        path('stop/', CharacterSessionViewSet.as_view({'delete': 'stop_session'}), name='character-session-stop'),
        path('status/', CharacterSessionViewSet.as_view({'get': 'session_status'}), name='character-session-status'),
        path('save/', CharacterSessionViewSet.as_view({'post': 'save_session'}), name='character-session-save'),
    ])),
    
    # Global session list endpoint
    path('characters/session/list/', CharacterSessionViewSet.as_view({'get': 'list_active_sessions'}), name='character-sessions-list'),
    
    # Save game specific endpoints
    path('savegames/import/', csrf_exempt(savegame_views.import_savegame), name='import-savegame'),
    path('savegames/<int:character_id>/info/', savegame_views.get_savegame_info, name='get-savegame-info'),
    path('savegames/<int:character_id>/companions/', savegame_views.list_savegame_companions, name='list-savegame-companions'),
    path('savegames/<int:character_id>/update/', savegame_views.update_savegame_character, name='update-savegame-character'),
    path('savegames/<int:character_id>/restore-backup/', savegame_views.restore_savegame_backup, name='restore-savegame-backup'),
    
    # Health check endpoint
    path('health/', health_check, name='health-check'),
    path('ready/', ready_check, name='ready-check'),
    
    # System endpoints
    path('system/cache/status/', cache_status, name='cache-status'),
    path('system/cache/rebuild/', rebuild_cache, name='rebuild-cache'),
    path('system/config/', get_config, name='get-config'),
    path('system/config/update/', update_config, name='update-config'),
    path('system/nwn2-data-path/', nwn2_data_path_config, name='nwn2-data-path-config'),
    path('system/auto-discover-nwn2/', auto_discover_nwn2, name='auto-discover-nwn2'),
    path('system/background-loading/trigger/', trigger_background_loading, name='trigger-background-loading'),
    path('system/background-loading/status/', background_loading_status_endpoint, name='background-loading-status'),
    path('system/initialization/status/', initialization_status_endpoint, name='initialization-status'),
    
    
    # Character creation endpoints
    path('characters/create/', character_creation_views.create_character, name='create-character'),
    path('characters/templates/', character_creation_views.get_character_templates, name='get-character-templates'),
    path('characters/validate/', character_creation_views.validate_character_build, name='validate-character-build'),
    
    # Character export endpoints
    path('characters/export/localvault/', character_export_views.export_to_localvault, name='export-to-localvault'),
    path('characters/export/module/', character_export_views.export_for_module, name='export-for-module'),
]