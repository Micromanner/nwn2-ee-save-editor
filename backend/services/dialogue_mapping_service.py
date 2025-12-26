"""
Dialogue Mapping Service - Extracts quest variable mappings from dialogue files

FEATURE ON HOLD
===============
This service is currently disabled in the frontend. The quest mapping feature produces
duplicate and incorrect mappings due to fundamental limitations in NWN2's architecture:

1. Many quest variables (e.g., "CallumState") are set via area scripts or creature
   death scripts, NOT dialogue files - so they never co-occur with journal updates.

2. The same variable can map to multiple journal entries, causing duplicate quests
   in the UI (e.g., "Getting Back in the Fight" appearing 6 times).

3. Only ~7% of NWN2 scripts contain AddJournalQuestEntry() calls - the rest use
   generic ga_journal which receives parameters from dialogue, not the script itself.

The dialogue parsing approach IS the correct one (vs parsing .nss scripts), but it
cannot achieve complete coverage. See QUEST_MAPPING_UX_PROBLEM.md for full analysis.

TO RE-ENABLE: Uncomment the QuestsEditor in GameStateEditor.tsx

---

This service parses .dlg dialogue files from NWN2 campaigns to find co-occurrences
of variable-setting scripts (ga_global_int, etc.) and journal-updating scripts
(ga_journal) on the same dialogue nodes, providing high-confidence variable-to-quest
mappings.
"""

import os
import hashlib
import tempfile
import lzma
from pathlib import Path
from typing import Dict, List, Any, Optional, Tuple, Set
from dataclasses import dataclass, field, asdict
from collections import defaultdict
from io import BytesIO
import msgpack
from nwn2_rust import GffParser, ErfParser

from utils.paths import get_writable_dir


@dataclass
class QuestVariableMapping:
    variable_name: str
    variable_value: int
    journal_tag: str
    journal_entry_id: int
    confidence: float
    source_dialogues: List[str] = field(default_factory=list)
    co_occurrence_count: int = 1


@dataclass
class DialogueMappingCache:
    version: str = "1.0"
    campaign_guid: str = ""
    campaign_name: str = ""
    generated_at: str = ""
    source_hash: str = ""
    mappings: List[Dict[str, Any]] = field(default_factory=list)
    dialogue_count: int = 0
    mapping_count: int = 0


class DialogueMappingService:
    """Service for extracting quest variable mappings from dialogue files"""

    CACHE_VERSION = "2.2"
    CACHE_DIR = get_writable_dir("cache/quest_mappings")

    VARIABLE_SCRIPTS = {
        'ga_global_int',
        'ga_global_string',
        'ga_global_float',
        'ga_set_int',
        'ga_set_globalint',
    }

    CONDITIONAL_SCRIPTS = {
        'gc_global_int',
        'gc_global_string',
        'gc_check_int',
        'gc_check_globalint',
    }

    JOURNAL_SCRIPTS = {
        'ga_journal',
        'ga_journal_entry',
        'ga_journalentry',
        'gc_journal_entry',
        'gc_journal',
        'ga_give_quest_xp',
    }

    JOURNAL_PATTERNS = [
        'journal',
        '_journal_entry',
        'jrl_',
        'quest_xp',
    ]

    def __init__(self, campaign_guid: str, campaign_name: str = ""):
        """
        Initialize the dialogue mapping service for a specific campaign

        Args:
            campaign_guid: The Campaign_ID GUID from module.ifo
            campaign_name: Optional campaign name for logging
        """
        self.campaign_guid = campaign_guid
        self.campaign_name = campaign_name
        self._mappings: Dict[str, List[QuestVariableMapping]] = {}
        self._cache_loaded = False

        self.CACHE_DIR.mkdir(parents=True, exist_ok=True)

    @property
    def cache_path(self) -> Path:
        """Get cache file path for this campaign"""
        safe_guid = self.campaign_guid.replace('-', '').replace(' ', '_')[:32]
        return self.CACHE_DIR / f"{safe_guid}.msgpack"

    def get_mapping(self, variable_name: str, value: Optional[int] = None) -> Optional[QuestVariableMapping]:
        """
        Get quest mapping for a variable

        Args:
            variable_name: Variable name (e.g., "SimmyState")
            value: Optional specific value to match

        Returns:
            QuestVariableMapping if found, None otherwise
        """
        self._ensure_mappings_loaded()

        mappings = self._mappings.get(variable_name, [])
        if not mappings:
            return None

        if value is not None:
            for mapping in mappings:
                if mapping.variable_value == value:
                    return mapping

        return mappings[0] if mappings else None

    def get_all_mappings_for_variable(self, variable_name: str) -> List[QuestVariableMapping]:
        """Get all known value mappings for a variable"""
        self._ensure_mappings_loaded()
        return self._mappings.get(variable_name, [])

    def get_all_mappings(self) -> Dict[str, List[QuestVariableMapping]]:
        """Get all quest variable mappings"""
        self._ensure_mappings_loaded()
        return self._mappings.copy()

    def get_cache_info(self) -> Dict[str, Any]:
        """Get information about the current cache"""
        if self.cache_path.exists():
            try:
                with open(self.cache_path, 'rb') as f:
                    cache_data = msgpack.unpack(f, raw=False)
                return {
                    'cached': True,
                    'version': cache_data.get('version', 'unknown'),
                    'generated_at': cache_data.get('generated_at', 'unknown'),
                    'dialogue_count': cache_data.get('dialogue_count', 0),
                    'mapping_count': cache_data.get('mapping_count', 0),
                    'campaign_name': cache_data.get('campaign_name', ''),
                }
            except Exception as e:
                logger.warning(f"Failed to read cache info: {e}")

        return {
            'cached': False,
            'version': self.CACHE_VERSION,
            'generated_at': None,
            'dialogue_count': 0,
            'mapping_count': 0,
        }

    def _ensure_mappings_loaded(self) -> None:
        """Ensure mappings are loaded from cache or generated"""
        if self._cache_loaded:
            return

        if self._try_load_cache():
            self._cache_loaded = True
            return

        logger.info(f"Building dialogue mappings for campaign: {self.campaign_name or self.campaign_guid}")
        self._build_mappings()
        self._save_cache()
        self._cache_loaded = True

    def _try_load_cache(self) -> bool:
        """Try to load mappings from cache file"""
        if not self.cache_path.exists():
            return False

        try:
            with open(self.cache_path, 'rb') as f:
                cache_data = msgpack.unpack(f, raw=False)

            if cache_data.get('version') != self.CACHE_VERSION:
                logger.info(f"Cache version mismatch, rebuilding")
                return False

            if cache_data.get('campaign_guid') != self.campaign_guid:
                logger.info(f"Cache campaign GUID mismatch, rebuilding")
                return False

            for mapping_dict in cache_data.get('mappings', []):
                mapping = QuestVariableMapping(
                    variable_name=mapping_dict['variable_name'],
                    variable_value=mapping_dict['variable_value'],
                    journal_tag=mapping_dict['journal_tag'],
                    journal_entry_id=mapping_dict['journal_entry_id'],
                    confidence=mapping_dict['confidence'],
                    source_dialogues=mapping_dict.get('source_dialogues', []),
                    co_occurrence_count=mapping_dict.get('co_occurrence_count', 1),
                )

                if mapping.variable_name not in self._mappings:
                    self._mappings[mapping.variable_name] = []
                self._mappings[mapping.variable_name].append(mapping)

            logger.info(f"Loaded {len(cache_data.get('mappings', []))} mappings from cache")
            return True

        except Exception as e:
            logger.warning(f"Failed to load cache: {e}")
            return False

    def _save_cache(self) -> None:
        """Save mappings to cache file"""
        try:
            from datetime import datetime

            all_mappings = []
            for var_mappings in self._mappings.values():
                for mapping in var_mappings:
                    all_mappings.append(asdict(mapping))

            cache = {
                'version': self.CACHE_VERSION,
                'campaign_guid': self.campaign_guid,
                'campaign_name': self.campaign_name,
                'generated_at': datetime.now().isoformat(),
                'source_hash': '',
                'mappings': all_mappings,
                'dialogue_count': getattr(self, '_dialogue_count', 0),
                'mapping_count': len(all_mappings),
            }

            with open(self.cache_path, 'wb') as f:
                msgpack.pack(cache, f)

            logger.info(f"Saved {len(all_mappings)} mappings to cache")

        except Exception as e:
            logger.error(f"Failed to save cache: {e}")

    def _build_mappings(self) -> None:
        """Build quest variable mappings by parsing dialogue files with expanded correlation"""
        self._mappings = {}

        campaign_folder = self._find_campaign_folder()
        if not campaign_folder:
            logger.warning(f"Campaign folder not found for GUID: {self.campaign_guid}")
            return

        module_files = self._get_campaign_module_files(campaign_folder)
        if not module_files:
            logger.warning(f"No module files found in campaign folder")
            return

        logger.info(f"Found {len(module_files)} module files to parse")

        file_correlations = defaultdict(lambda: defaultdict(int))
        node_correlations = defaultdict(lambda: defaultdict(int))
        dialogue_sources = defaultdict(lambda: defaultdict(set))
        dialogue_count = 0

        for module_path in module_files:
            module_name = module_path.stem
            logger.debug(f"Processing module: {module_name}")

            dialogues = self._extract_dialogues_from_module(module_path)
            dialogue_count += len(dialogues)

            for dlg_name, dlg_data in dialogues:
                try:
                    node_scripts = self._parse_dialogue_scripts(dlg_data)

                    file_vars = set()
                    file_journals = set()

                    for node_id, scripts in node_scripts.items():
                        node_vars = []
                        node_journals = []

                        for script in scripts:
                            script_name = script.get('script', '').lower()
                            params = script.get('params', [])
                            script_type = script.get('type', '')

                            if any(vs in script_name for vs in ['global_int', 'set_int', 'globalint', 'check_int']):
                                if len(params) >= 1:
                                    var_name = str(params[0])
                                    val = None
                                    if len(params) > 1 and str(params[1]).lstrip('-').isdigit():
                                        val = int(params[1])
                                    node_vars.append((var_name, val))
                                    file_vars.add(var_name)
                            
                            # Handle direct journal links from Quest/QuestEntry fields
                            elif script_type == 'journal_direct':
                                if len(params) >= 1:
                                    tag = str(params[0])
                                    entry = int(params[1]) if len(params) > 1 and str(params[1]).isdigit() else 0
                                    node_journals.append((tag, entry))
                                    file_journals.add(tag)

                            # Handle journal scripts (ga_journal, gc_journal_entry, ga_give_quest_xp, etc.)
                            elif 'journal' in script_name or 'quest_xp' in script_name:
                                if len(params) >= 1:
                                    tag = str(params[0])
                                    entry = int(params[1]) if len(params) > 1 and str(params[1]).isdigit() else 0
                                    node_journals.append((tag, entry))
                                    file_journals.add(tag)

                        for v_name, v_val in node_vars:
                            for j_tag, j_entry in node_journals:
                                key_var = (v_name, v_val if v_val is not None else -1)
                                key_jrl = (j_tag, j_entry)
                                node_correlations[key_var][key_jrl] += 1
                                dialogue_sources[v_name][j_tag].add(f"{module_name}/{dlg_name}")

                    for f_var in file_vars:
                        for f_jrl in file_journals:
                            file_correlations[f_var][f_jrl] += 1

                except Exception as e:
                    logger.debug(f"Failed to parse dialogue {dlg_name}: {e}")

        self._dialogue_count = dialogue_count

        processed_vars = set()

        for var_key, jrl_counts in node_correlations.items():
            var_name, var_val = var_key
            for jrl_key, count in jrl_counts.items():
                j_tag, j_entry = jrl_key

                mapping = QuestVariableMapping(
                    variable_name=var_name,
                    variable_value=var_val,
                    journal_tag=j_tag,
                    journal_entry_id=j_entry,
                    confidence=0.95,
                    source_dialogues=list(dialogue_sources[var_name][j_tag]),
                    co_occurrence_count=count,
                )

                if var_name not in self._mappings:
                    self._mappings[var_name] = []
                self._mappings[var_name].append(mapping)
                processed_vars.add(var_name)

        for var_name, jrl_counts in file_correlations.items():
            if not jrl_counts:
                continue

            best_tag = max(jrl_counts, key=jrl_counts.get)
            count = jrl_counts[best_tag]

            # NWN2 naming convention: variables ending in "State" or starting with "q_"/"Q_"
            # are almost always quest state trackers - trust singleton matches for these
            is_likely_quest_state = (
                var_name.endswith('State') or
                var_name.startswith('q_') or
                var_name.startswith('Q_')
            )
            threshold = 1 if is_likely_quest_state else 2

            if count >= threshold and var_name not in processed_vars:
                base_confidence = 0.65 if count < 4 else 0.80
                final_confidence = min(base_confidence + (0.10 if is_likely_quest_state else 0.0), 0.95)

                mapping = QuestVariableMapping(
                    variable_name=var_name,
                    variable_value=0,
                    journal_tag=best_tag,
                    journal_entry_id=0,
                    confidence=final_confidence,
                    source_dialogues=[],
                    co_occurrence_count=count,
                )

                if var_name not in self._mappings:
                    self._mappings[var_name] = []

                exists = any(m.journal_tag == best_tag for m in self._mappings[var_name])
                if not exists:
                    self._mappings[var_name].append(mapping)

        journal_tags = self._load_journal_tags(campaign_folder)
        if journal_tags:
            all_file_vars = set()
            for var_name in file_correlations.keys():
                all_file_vars.add(var_name)

            prefix_mappings = self._match_by_prefix(all_file_vars, journal_tags, processed_vars)
            for var_name, mapping in prefix_mappings.items():
                if var_name not in self._mappings:
                    self._mappings[var_name] = []
                self._mappings[var_name].append(mapping)

        total_mappings = sum(len(m) for m in self._mappings.values())
        logger.info(f"Built {total_mappings} mappings from {dialogue_count} dialogue files")

    def _find_campaign_folder(self) -> Optional[Path]:
        """Find the campaign folder by GUID"""
        campaigns_dir = nwn2_paths.campaigns

        if not campaigns_dir.exists():
            logger.warning(f"Campaigns directory not found: {campaigns_dir}")
            return None

        for campaign_name in os.listdir(campaigns_dir):
            campaign_path = campaigns_dir / campaign_name

            if not campaign_path.is_dir():
                continue

            campaign_file = campaign_path / 'campaign.cam'
            if not campaign_file.exists():
                continue

            try:
                campaign_data = GffParser(str(campaign_file)).to_dict()

                file_guid = campaign_data.get('GUID', '')
                if file_guid == self.campaign_guid:
                    logger.info(f"Found campaign folder: {campaign_name}")
                    return campaign_path

            except Exception as e:
                logger.debug(f"Failed to parse {campaign_file}: {e}")

        return None

    def _get_campaign_module_files(self, campaign_folder: Path) -> List[Path]:
        """Get list of .mod files for the campaign"""
        modules = []

        campaign_file = campaign_folder / 'campaign.cam'
        if campaign_file.exists():
            try:
                campaign_data = GffParser(str(campaign_file)).to_dict()

                mod_names = campaign_data.get('ModNames', [])
                modules_dir = nwn2_paths.modules

                for mod_entry in mod_names:
                    if isinstance(mod_entry, dict):
                        mod_name = mod_entry.get('ModuleName', '')
                    else:
                        mod_name = str(mod_entry)

                    if mod_name:
                        mod_path = modules_dir / f"{mod_name}.mod"
                        if mod_path.exists():
                            modules.append(mod_path)
                        else:
                            logger.debug(f"Module file not found: {mod_path}")

            except Exception as e:
                logger.warning(f"Failed to read campaign module list: {e}")

        if not modules:
            modules_dir = nwn2_paths.modules
            if modules_dir.exists():
                modules = list(modules_dir.glob("*.mod"))

        return modules

    def _load_journal_tags(self, campaign_folder: Path) -> Set[str]:
        """Load all journal tags from module.jrl"""
        journal_tags = set()

        jrl_path = campaign_folder / 'module.jrl'
        if not jrl_path.exists():
            return journal_tags

        try:
            jrl_data = GffParser(str(jrl_path)).to_dict()

            for category in jrl_data.get('Categories', []):
                if isinstance(category, dict):
                    tag = category.get('Tag', '')
                    if tag:
                        journal_tags.add(tag)

            logger.debug(f"Loaded {len(journal_tags)} journal tags from module.jrl")

        except Exception as e:
            logger.warning(f"Failed to load journal tags: {e}")

        return journal_tags

    def _match_by_prefix(
        self,
        variables: Set[str],
        journal_tags: Set[str],
        already_mapped: Set[str]
    ) -> Dict[str, QuestVariableMapping]:
        """Match variables to journal tags by shared prefix (e.g., 11_)"""
        import re

        mappings = {}

        prefix_pattern = re.compile(r'^(\d+)_')

        tags_by_prefix = defaultdict(list)
        for tag in journal_tags:
            match = prefix_pattern.match(tag)
            if match:
                tags_by_prefix[match.group(1)].append(tag)

        for var_name in variables:
            if var_name in already_mapped:
                continue

            match = prefix_pattern.match(var_name)
            if not match:
                continue

            prefix = match.group(1)
            candidate_tags = tags_by_prefix.get(prefix, [])

            if not candidate_tags:
                continue

            var_clean = re.sub(r'^\d+_b?', '', var_name)
            var_parts = re.split(r'_', var_clean)
            var_words = set()
            for part in var_parts:
                camel_parts = re.findall(r'[A-Z]?[a-z]+|[A-Z]+(?=[A-Z]|$)', part)
                var_words.update(w.lower() for w in camel_parts if len(w) >= 3)

            best_tag = None
            best_score = 0

            for tag in candidate_tags:
                tag_clean = re.sub(r'^\d+_', '', tag)
                tag_parts = re.split(r'_', tag_clean)
                tag_words = set()
                for part in tag_parts:
                    tag_words.add(part.lower())

                common = var_words & tag_words
                score = len(common)

                if not common:
                    for vw in var_words:
                        for tw in tag_words:
                            if len(vw) >= 4 and len(tw) >= 4:
                                if vw in tw or tw in vw:
                                    score += 0.5
                                elif vw[:4] == tw[:4]:
                                    score += 0.3

                if score > best_score:
                    best_score = score
                    best_tag = tag

            if best_tag and best_score >= 0.3:
                mappings[var_name] = QuestVariableMapping(
                    variable_name=var_name,
                    variable_value=0,
                    journal_tag=best_tag,
                    journal_entry_id=0,
                    confidence=0.40 + (0.10 * min(best_score, 3)),
                    source_dialogues=[],
                    co_occurrence_count=0,
                )
            elif candidate_tags:
                mappings[var_name] = QuestVariableMapping(
                    variable_name=var_name,
                    variable_value=0,
                    journal_tag=candidate_tags[0],
                    journal_entry_id=0,
                    confidence=0.30,
                    source_dialogues=[],
                    co_occurrence_count=0,
                )

        logger.info(f"Prefix matching added {len(mappings)} additional mappings")
        return mappings

    def _extract_dialogues_from_module(self, module_path: Path) -> List[Tuple[str, bytes]]:
        """Extract all .dlg files from a module"""
        dialogues = []

        try:
            parser = ErfParser()
            parser.read(str(module_path))

            resources = parser.list_resources()
            dlg_resources = [r for r in resources if r.get('name', '').lower().endswith('.dlg')]

            for resource in dlg_resources:
                res_name = resource.get('name', '')
                try:
                    dlg_data = parser.extract_resource(res_name)
                    if dlg_data:
                        dialogues.append((res_name, dlg_data))
                except Exception as e:
                    logger.debug(f"Failed to extract {res_name}: {e}")

        except Exception as e:
            logger.warning(f"Failed to parse module {module_path.name}: {e}")

        return dialogues

    def _parse_dialogue_scripts(self, dlg_data: bytes) -> Dict[int, List[Dict[str, Any]]]:
        """Parse dialogue GFF and extract script actions from each node"""
        node_scripts = {}

        try:
            dlg_dict = GffParser.from_bytes(dlg_data).to_dict()

            node_id = 0

            # Process StartingList (dialogue entry points with conditions)
            for starting in dlg_dict.get('StartingList', []):
                if isinstance(starting, dict):
                    scripts = self._extract_scripts_from_node(starting)
                    if scripts:
                        node_scripts[node_id] = scripts
                node_id += 1

            for entry in dlg_dict.get('EntryList', []):
                if isinstance(entry, dict):
                    scripts = self._extract_scripts_from_node(entry)
                    if scripts:
                        node_scripts[node_id] = scripts
                node_id += 1

            for reply in dlg_dict.get('ReplyList', []):
                if isinstance(reply, dict):
                    scripts = self._extract_scripts_from_node(reply)
                    if scripts:
                        node_scripts[node_id] = scripts
                node_id += 1

        except Exception as e:
            logger.debug(f"Failed to parse dialogue GFF: {e}")

        return node_scripts

    def _extract_scripts_from_node(self, node: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Extract both action scripts and conditional scripts from a dialogue node"""
        scripts = []

        script_list = node.get('ScriptList', [])
        if not isinstance(script_list, list):
            script_list = [script_list] if script_list else []

        for script_entry in script_list:
            if not isinstance(script_entry, dict):
                continue

            script_ref = script_entry.get('Script', '')
            if not script_ref:
                continue

            params = []
            parameters = script_entry.get('Parameters', [])
            if isinstance(parameters, list):
                for param_obj in parameters:
                    if isinstance(param_obj, dict):
                        params.append(param_obj.get('Parameter', ''))

            scripts.append({
                'script': script_ref,
                'params': params,
                'type': 'action',
            })

        # Check multiple condition list keys (NWN2 uses different keys in different contexts)
        for cond_key in ['ConditionList', 'ActiveConditiona', 'ActiveCondition']:
            condition_list = node.get(cond_key, [])
            if not isinstance(condition_list, list):
                condition_list = [condition_list] if condition_list else []

            for cond_entry in condition_list:
                if not isinstance(cond_entry, dict):
                    continue

                script_ref = cond_entry.get('Script', '')
                if not script_ref:
                    continue

                params = []
                parameters = cond_entry.get('Parameters', [])
                if isinstance(parameters, list):
                    for param_obj in parameters:
                        if isinstance(param_obj, dict):
                            params.append(param_obj.get('Parameter', ''))

                scripts.append({
                    'script': script_ref,
                    'params': params,
                    'type': 'condition',
                })

        active_script = node.get('Script', '')
        if active_script:
            scripts.append({
                'script': active_script,
                'params': [],
                'type': 'condition',
            })

        # Extract direct Quest/QuestEntry references (NWN2 native quest tracking)
        quest_tag = node.get('Quest', '')
        quest_entry = node.get('QuestEntry', 0)
        if quest_tag and isinstance(quest_tag, str) and quest_tag.strip():
            scripts.append({
                'script': '_direct_journal_link',
                'params': [quest_tag.strip(), str(quest_entry)],
                'type': 'journal_direct',
            })

        return scripts

    def invalidate_cache(self) -> bool:
        """Delete the cache file to force rebuild on next access"""
        try:
            if self.cache_path.exists():
                self.cache_path.unlink()
                self._cache_loaded = False
                self._mappings = {}
                logger.info(f"Cache invalidated for campaign: {self.campaign_guid}")
                return True
        except Exception as e:
            logger.error(f"Failed to invalidate cache: {e}")
        return False


def get_dialogue_mapping_service(content_manager) -> Optional[DialogueMappingService]:
    """
    Factory function to create DialogueMappingService from ContentManager

    Args:
        content_manager: ContentManager instance with campaign info

    Returns:
        DialogueMappingService instance or None if no campaign info available
    """
    campaign_id = content_manager.module_info.get('campaign_id', '')
    if not campaign_id:
        logger.warning("No campaign_id available for dialogue mapping")
        return None

    campaign_name = content_manager.module_info.get('campaign', '')

    return DialogueMappingService(campaign_id, campaign_name)
