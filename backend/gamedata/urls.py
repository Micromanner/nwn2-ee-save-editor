from django.urls import path, include
from rest_framework.routers import DefaultRouter
from .views.views import GameDataViewSet, HotReloadView, CustomOverrideDirectoriesView, WorkshopModsView, CacheStatsView
from .views.icon_views import (
    IconView, IconStatsView, IconListView,
    ModuleIconView, LegacyIconView
)
from .views.path_views import (
    get_path_config, set_game_folder, set_documents_folder,
    set_steam_workshop_folder, add_custom_override_folder,
    remove_custom_override_folder, add_custom_module_folder,
    remove_custom_module_folder, add_custom_hak_folder,
    remove_custom_hak_folder, auto_detect_paths
)

router = DefaultRouter()
router.register(r'', GameDataViewSet, basename='gamedata')

urlpatterns = [
    # Icon endpoints (enhanced v2 API only)
    path('icons/', IconStatsView.as_view(), name='icon-stats'),
    path('icons/stats/', IconStatsView.as_view(), name='icon-stats-alt'),  # Alternative path
    path('icons/list/', IconListView.as_view(), name='icon-list'),
    path('icons/module/', ModuleIconView.as_view(), name='module-icons'),
    path('icons/<path:icon_path>/', IconView.as_view(), name='icon-detail'),  # Catch-all must be last
    
    
    # Hot reload endpoint (dev only)
    path('hot-reload/', HotReloadView.as_view(), name='hot-reload'),
    
    # Custom override directories management
    path('custom-overrides/', CustomOverrideDirectoriesView.as_view(), name='custom-overrides'),
    
    # Steam Workshop mods
    path('workshop-mods/', WorkshopModsView.as_view(), name='workshop-mods'),
    
    # Cache statistics
    path('cache-stats/', CacheStatsView.as_view(), name='cache-stats'),
    
    # Path configuration endpoints
    path('paths/', get_path_config, name='get-path-config'),
    path('paths/set-game-folder/', set_game_folder, name='set-game-folder'),
    path('paths/set-documents-folder/', set_documents_folder, name='set-documents-folder'),
    path('paths/set-steam-workshop/', set_steam_workshop_folder, name='set-steam-workshop'),
    path('paths/add-override/', add_custom_override_folder, name='add-override'),
    path('paths/remove-override/', remove_custom_override_folder, name='remove-override'),
    path('paths/add-module/', add_custom_module_folder, name='add-module'),
    path('paths/remove-module/', remove_custom_module_folder, name='remove-module'),
    path('paths/add-hak/', add_custom_hak_folder, name='add-hak'),
    path('paths/remove-hak/', remove_custom_hak_folder, name='remove-hak'),
    path('paths/auto-detect/', auto_detect_paths, name='auto-detect-paths'),
    
    # Original gamedata endpoints
    path('', include(router.urls)),
]