"""
NWN2 Description Parser - Converts NWN2's HTML-like descriptions into structured data

TODO: Integrate this parser to display rich class descriptions in the frontend.
      1. Fetch DescriptionStrRef from classes.2da
      2. Look up the string in TLK to get raw NWN2 markup
      3. Use this parser to convert to structured data or clean HTML
      4. Return in the /classes/categorized API endpoint
      5. Display in ClassSelectorModal.tsx when user hovers/clicks a class
"""
import re
from typing import Dict, List, Optional, Any
from dataclasses import dataclass

@dataclass
class ParsedDescription:
    """Structured class description data"""
    title: str
    class_type: str  # "BASE" or "PRESTIGE"
    summary: str
    restrictions: List[str]
    requirements: Dict[str, Any]
    features: Dict[str, Any]
    abilities: List[Dict[str, str]]
    raw_html: str

class DescriptionParser:
    """Parses NWN2 class descriptions into structured data"""
    
    # NWN2 color mappings
    COLOR_MAP = {
        'Gold': '#FFD700',
        'Red': '#FF4444',
        'Blue': '#4488FF',
        'Green': '#44AA44',
        'Yellow': '#FFFF44',
        'White': '#FFFFFF'
    }
    
    def parse_class_description(self, description: str) -> ParsedDescription:
        """Parse a class description into structured components"""
        if not description:
            return self._empty_description()
        
        # Clean up the text
        text = description.strip()
        
        # Extract title
        title = self._extract_title(text)
        
        # Determine class type
        class_type = "PRESTIGE" if "(PRESTIGE CLASS:" in text else "BASE"
        
        # Extract main sections
        summary = self._extract_summary(text)
        restrictions = self._extract_restrictions(text)
        requirements = self._extract_requirements(text)
        features = self._extract_features(text)
        abilities = self._extract_abilities(text)
        
        # Convert to clean HTML
        html = self._convert_to_html(text)
        
        return ParsedDescription(
            title=title,
            class_type=class_type,
            summary=summary,
            restrictions=restrictions,
            requirements=requirements,
            features=features,
            abilities=abilities,
            raw_html=html
        )
    
    def _extract_title(self, text: str) -> str:
        """Extract class title from description"""
        # Look for <color=Gold><b>Title</b></color> pattern
        match = re.search(r'<color=Gold><b>([^<]+)</b></color>', text)
        if match:
            return match.group(1).strip()
        
        # Fallback to first line
        first_line = text.split('\n')[0].strip()
        return re.sub(r'<[^>]+>', '', first_line)
    
    def _extract_summary(self, text: str) -> str:
        """Extract the main description/summary"""
        lines = text.split('\n')
        summary_lines = []
        in_summary = False
        
        for line in lines:
            line = line.strip()
            if not line:
                continue
                
            # Skip title and prestige class notice
            if '<color=Gold><b>' in line or '(PRESTIGE CLASS:' in line:
                continue
                
            # Stop at requirements section
            if '<color=Gold>Requirements:</color>' in line:
                break
                
            # Stop at class features section
            if '<color=Gold>Class Features:</color>' in line:
                break
                
            # Clean HTML tags and add to summary
            clean_line = re.sub(r'<[^>]+>', '', line)
            if clean_line and not clean_line.startswith('Requirements:') and not clean_line.startswith('Class Features:'):
                summary_lines.append(clean_line)
        
        return '\n'.join(summary_lines).strip()
    
    def _extract_restrictions(self, text: str) -> List[str]:
        """Extract any class restrictions (red text)"""
        restrictions = []
        red_pattern = r'<color=Red>([^<]+)</color>'
        matches = re.findall(red_pattern, text)
        
        for match in matches:
            # Skip the prestige class notice
            if not match.startswith('(PRESTIGE CLASS:'):
                restrictions.append(match.strip())
        
        return restrictions
    
    def _extract_requirements(self, text: str) -> Dict[str, Any]:
        """Extract structured requirements"""
        requirements = {}
        
        # Find requirements section - more flexible matching
        req_patterns = [
            r'<color=Gold>Requirements:</color>(.*?)(?=<color=Gold>Class Features:</color>|<color=Gold>.*?:</color>|$)',
            r'Requirements:(.*?)(?=Class Features:|$)'
        ]
        
        req_text = None
        for pattern in req_patterns:
            req_match = re.search(pattern, text, re.DOTALL | re.IGNORECASE)
            if req_match:
                req_text = req_match.group(1)
                break
        
        if not req_text:
            return requirements
        
        # Parse individual requirements
        req_lines = req_text.split('\n')
        for line in req_lines:
            line = line.strip()
            if not line:
                continue
                
            # Look for <b>Category:</b> Value pattern
            bold_match = re.search(r'<b>([^:]+):</b>\s*(.+)', line)
            if bold_match:
                category = bold_match.group(1).strip()
                value = re.sub(r'<[^>]+>', '', bold_match.group(2)).strip()
                requirements[category.lower()] = value
            # Also look for plain Category: Value pattern
            elif ':' in line and not line.startswith('<'):
                parts = line.split(':', 1)
                if len(parts) == 2:
                    category = re.sub(r'<[^>]+>', '', parts[0]).strip()
                    value = re.sub(r'<[^>]+>', '', parts[1]).strip()
                    if category and value:
                        requirements[category.lower()] = value
        
        return requirements
    
    def _extract_features(self, text: str) -> Dict[str, Any]:
        """Extract class features section"""
        features = {}
        
        # Find class features section
        feat_match = re.search(r'<color=Gold>Class Features:</color>(.*?)(?=<color=Gold>Class Abilities:</color>|<color=Gold>.*?:</color>|$)', text, re.DOTALL)
        if not feat_match:
            return features
        
        feat_text = feat_match.group(1)
        
        # Parse feature lines
        feat_lines = feat_text.split('\n')
        current_feature = None
        
        for line in feat_lines:
            line = line.strip()
            if not line:
                continue
                
            # Look for - <b>Feature Name:</b> Description pattern
            if line.startswith('- <b>'):
                bold_match = re.search(r'- <b>([^:]+):</b>\s*(.+)', line)
                if bold_match:
                    feature_name = bold_match.group(1).strip()
                    description = re.sub(r'<[^>]+>', '', bold_match.group(2)).strip()
                    features[feature_name.lower()] = description
                    current_feature = feature_name.lower()
            # Look for simple - Feature: Description pattern
            elif line.startswith('- ') and ':' in line:
                parts = line[2:].split(':', 1)
                if len(parts) == 2:
                    feature_name = re.sub(r'<[^>]+>', '', parts[0]).strip()
                    description = re.sub(r'<[^>]+>', '', parts[1]).strip()
                    features[feature_name.lower()] = description
        
        return features
    
    def _extract_abilities(self, text: str) -> List[Dict[str, str]]:
        """Extract class abilities with levels"""
        abilities = []
        
        # Look for level-based abilities
        level_pattern = r'Level (\d+):\s*([^\n]+)'
        matches = re.findall(level_pattern, text)
        
        for level, ability in matches:
            abilities.append({
                'level': level,
                'ability': re.sub(r'<[^>]+>', '', ability).strip()
            })
        
        return abilities
    
    def _convert_to_html(self, text: str) -> str:
        """Convert NWN2 tags to proper HTML"""
        html = text
        
        # Convert color tags
        for nwn_color, css_color in self.COLOR_MAP.items():
            html = re.sub(
                rf'<color={nwn_color}>([^<]+)</color>',
                rf'<span style="color: {css_color}; font-weight: bold;">\1</span>',
                html,
                flags=re.IGNORECASE
            )
        
        # Convert bold tags
        html = re.sub(r'<b>([^<]+)</b>', r'<strong>\1</strong>', html)
        
        # Convert line breaks
        html = html.replace('\n\n', '<br><br>').replace('\n', '<br>')
        
        # Convert bullet points
        html = re.sub(r'^- ', '• ', html, flags=re.MULTILINE)
        
        return html
    
    def _empty_description(self) -> ParsedDescription:
        """Return empty description structure"""
        return ParsedDescription(
            title="",
            class_type="BASE",
            summary="",
            restrictions=[],
            requirements={},
            features={},
            abilities=[],
            raw_html=""
        )