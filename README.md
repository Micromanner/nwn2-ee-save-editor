<h1 align="center">NWN2 Enhanced Edition Save Editor</h1>

<p align="center">
  A desktop application for editing Neverwinter Nights 2 Enhanced Edition saves.<br/>
  Built with Tauri, Rust, and Vite. Fully offline, available as a portable <code>.exe</code> or installer.
</p>

<p align="center">
  <a href="https://github.com/Micromanner/nwn2-ee-save-editor/releases/latest"><img src="https://img.shields.io/github/v/release/Micromanner/nwn2-ee-save-editor?style=flat-square&color=a0522d" alt="Latest release"/></a>
  <a href="https://github.com/Micromanner/nwn2-ee-save-editor/releases"><img src="https://img.shields.io/github/downloads/Micromanner/nwn2-ee-save-editor/total?style=flat-square&color=a0522d" alt="Downloads"/></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/Micromanner/nwn2-ee-save-editor?style=flat-square&color=a0522d" alt="License"/></a>
  <img src="https://img.shields.io/badge/platform-windows%20%7C%20linux-a0522d?style=flat-square" alt="Platform"/>
  <a href="https://github.com/Micromanner/nwn2-ee-save-editor/stargazers"><img src="https://img.shields.io/github/stars/Micromanner/nwn2-ee-save-editor?style=flat-square&color=a0522d" alt="Stars"/></a>
</p>

<p align="center">
  <a href="https://github.com/Micromanner/nwn2-ee-save-editor/releases/latest"><img src="https://img.shields.io/badge/Download-Windows-0078D6?style=for-the-badge&logo=windows&logoColor=white" alt="Download for Windows"/></a>
  <a href="https://github.com/Micromanner/nwn2-ee-save-editor/releases/latest"><img src="https://img.shields.io/badge/Download-Linux-FCC624?style=for-the-badge&logo=linux&logoColor=black" alt="Download for Linux"/></a>
</p>

<p align="center">
  <img src=".github/screenshots/Overview.png" alt="Character overview" width="90%"/>
</p>

## Contents

- [Features](#features)
- [Screenshots](#screenshots)
- [Requirements](#requirements)
- [Getting Started](#getting-started)
- [Building from Source](#building-from-source)
- [Roadmap](#roadmap)

## Features

### Character Editing
- **Ability Scores** - Edit base scores with point-buy tracking, view racial modifiers, level bonuses, and equipment bonuses
- **Classes & Levels** - Multiclass management, level history tracking and experience points
- **Feats** - Add/remove feats
- **Skills** - Edit skills with rank allocation, class skill detection, and skill point budgets
- **Spells** - Manage known/memorized spells, domain spells, and spell slot calculations
- **Inventory** - Full equipment management, inventory items, gold, encumbrance, item property editing
- **Race** - Change race with subrace support and automatic stat adjustments
- **Appearance** - Customize body parts, phenotype, wings, tails, and colors with live 3D preview
- **Identity** - Name, biography, alignment, deity, age and gender
- **Combat Stats** - View attack bonuses, armor class, saving throws, and initiative
- **Companions** - View and edit party companions

### Game State
- **Campaign Variables and Settings** - Edit global campaign settings and variables
- **Module Variables** - View and modify module-level state

### Save Management
- Automatic backup creation before every save
- Backup restore with safety snapshots

### Mod Support
- HAK pack override chain
- Steam Workshop integration
- Custom override directories

### 3D Model Viewer
- Browse and preview all in-game 3D models

## Screenshots

<table>
  <tr>
    <td width="50%"><img src=".github/screenshots/Appearance.png" alt="Appearance"/><br/><sub><b>Appearance</b> — live 3D preview with body parts, tints, wings, tails</sub></td>
    <td width="50%"><img src=".github/screenshots/Inventory.png" alt="Inventory"/><br/><sub><b>Inventory</b> — equipment, item properties, encumbrance</sub></td>
  </tr>
  <tr>
    <td><img src=".github/screenshots/ClassesandLevel.png" alt="Classes and levels"/><br/><sub><b>Classes & Levels</b> — multiclass with level history</sub></td>
    <td><img src=".github/screenshots/Gamestate.png" alt="Game state"/><br/><sub><b>Game state</b> — campaign and module variables</sub></td>
  </tr>
  <tr>
    <td><img src=".github/screenshots/Backups.png" alt="Backups"/><br/><sub><b>Backups</b> — automatic safety snapshots and restore</sub></td>
    <td><img src=".github/screenshots/Feats.png" alt="Feats"/><br/><sub><b>Feats</b> — add/remove with prerequisite checks</sub></td>
  </tr>
</table>

## Requirements

- Windows 10+ / Ubuntu
- Neverwinter Nights 2 Enhanced Edition

## Getting Started

1. Download the latest release
2. Run the `.exe` - game paths are auto-detected
3. Open a save file and start editing

## Building from Source

```bash
# Install dependencies
npm install

# Development
npm run tauri:dev

# Production build
npm run tauri:build
```

Requires:
- Node.js 18+
- Rust toolchain (stable)

## Roadmap

- Full equipment rendering in the 3D character preview (some equipped items don't yet display correctly)
- Quest journal editor — browse each quest with its branches and outcomes, see current progress, and jump to any state (currently read-only, Windows only)
