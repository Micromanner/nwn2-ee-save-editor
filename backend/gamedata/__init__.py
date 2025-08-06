# Gamedata module
# Submodules should be imported directly:
#   from gamedata.services import GameRulesService
#   from gamedata.dynamic_loader import DynamicGameDataLoader
#   from gamedata.cache import SafeCache
#   etc.

# Ensure Django settings are configured
import os
os.environ.setdefault('DJANGO_SETTINGS_MODULE', 'config.settings')