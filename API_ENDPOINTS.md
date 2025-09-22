# API Quick Reference

## Base URL: `http://localhost:8000`

## Key Patterns
- Character ID = always `1` (single character system)
- Session management handles the actual save file paths
- No URL encoding needed for character ID

## Essential Endpoints

### Sessions
```bash
GET /api/session/characters/session/list           # Get active sessions
POST /api/session/characters/{char_id}/session/start
DELETE /api/session/characters/{char_id}/session/stop
```

### Character Data
```bash
GET /api/characters/{char_id}/state                # Full character state
GET /api/characters/{char_id}/summary              # Basic info
GET /api/characters/{char_id}/validation           # Validate character
```

### Character Systems
```bash
GET /api/characters/{char_id}/abilities            # Abilities & modifiers
POST /api/characters/{char_id}/abilities/update    # Change abilities

GET /api/characters/{char_id}/skills/state         # Skills state
POST /api/characters/{char_id}/skills/update       # Update skills

GET /api/characters/{char_id}/feats                # Character feats
POST /api/characters/{char_id}/feats/add           # Add feat
DELETE /api/characters/{char_id}/feats/{feat_id}   # Remove feat

GET /api/characters/{char_id}/classes              # Character classes
POST /api/characters/{char_id}/classes/add         # Add class level
POST /api/characters/{char_id}/classes/change      # Change class

GET /api/characters/{char_id}/alignment            # Alignment
POST /api/characters/{char_id}/alignment           # Update alignment

GET /api/characters/{char_id}/spells               # Known spells
POST /api/characters/{char_id}/spells/add          # Add spell

GET /api/characters/{char_id}/inventory            # Inventory items
POST /api/characters/{char_id}/inventory/add       # Add item

GET /api/characters/{char_id}/combat               # Combat stats
```

### Raw Data Access
```bash
GET /api/data/characters/{char_id}/raw?path=Str    # Get raw GFF data
POST /api/data/characters/{char_id}/raw            # Update raw field
GET /api/data/characters/{char_id}/structure       # GFF structure
```

### Game Data
```bash
# Core configuration
GET /api/gamedata/paths                            # NWN2 paths config
GET /api/gamedata/config                           # Game config

# Dynamic table data API  
GET /api/gamedata/table/?name=classes&limit=10&search=fighter  # Get any 2DA table
GET /api/gamedata/tables                           # List all available tables
GET /api/gamedata/schema/?name=classes             # Table schema info

# Convenience endpoints (shortcuts to common tables)
GET /api/gamedata/races                            # Racial types
GET /api/gamedata/classes                          # Character classes
GET /api/gamedata/feats                            # Feats
GET /api/gamedata/skills                           # Skills
GET /api/gamedata/spells                           # Spells
GET /api/gamedata/base_items                       # Base items

# Icon system
GET /api/gamedata/icons/{icon_path}/               # Get icon by path
GET /api/gamedata/icons/stats                      # Icon cache stats
GET /api/gamedata/icons/list                       # List available icons

# Module/mod support
GET /api/gamedata/modules                          # Available modules
GET /api/gamedata/modules/stats                    # Module index stats

# Path management
POST /api/gamedata/set-game-folder                 # Set game installation path
POST /api/gamedata/set-documents-folder            # Set documents path
POST /api/gamedata/set-steam-workshop-folder       # Set Steam workshop path
POST /api/gamedata/add-custom-override-folder      # Add custom override
POST /api/gamedata/remove-custom-override-folder   # Remove custom override
POST /api/gamedata/add-custom-module-folder        # Add custom module path
POST /api/gamedata/remove-custom-module-folder     # Remove custom module path
POST /api/gamedata/add-custom-hak-folder           # Add custom HAK path
POST /api/gamedata/remove-custom-hak-folder        # Remove custom HAK path
GET /api/gamedata/auto-detect-paths                # Auto-detect NWN2 paths
```

### Content & Campaign
```bash
GET /api/characters/{char_id}/campaign-info        # Campaign/module info
GET /api/characters/{char_id}/custom-content       # Custom content
```

### System
```bash
GET /api/system/health                             # Health check
GET /api/system/system/cache/status                # Cache status
POST /api/system/cache/rebuild                     # Rebuild cache
```

## Usage Example
```bash
# 1. Get active sessions
curl "http://localhost:8000/api/session/characters/session/list"

# 2. Character ID is always 1
curl "http://localhost:8000/api/characters/1/state"

# 3. All character endpoints use ID 1
curl "http://localhost:8000/api/characters/1/abilities"
curl "http://localhost:8000/api/characters/1/feats"
```