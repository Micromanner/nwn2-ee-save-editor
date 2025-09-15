export interface ParsedSpellInfo {
  casterLevels: string[];
  innateLevel: string;
  school: string;
  descriptors: string[];
  components: string[];
  range: string;
  areaOfEffect: string;
  duration: string;
  save: string;
  spellResistance: string;
  description: string;
}

export function parseSpellDescription(description: string): ParsedSpellInfo {
  const lines = description.split('\n').map(line => line.trim()).filter(line => line);
  
  const parsed: ParsedSpellInfo = {
    casterLevels: [],
    innateLevel: '',
    school: '',
    descriptors: [],
    components: [],
    range: '',
    areaOfEffect: '',
    duration: '',
    save: '',
    spellResistance: '',
    description: ''
  };

  let descriptionStartIndex = -1;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    if (line.startsWith('Caster Level(s):')) {
      const levels = line.replace('Caster Level(s):', '').trim();
      parsed.casterLevels = levels.split(',').map(level => level.trim());
    } else if (line.startsWith('Innate Level:')) {
      parsed.innateLevel = line.replace('Innate Level:', '').trim();
    } else if (line.startsWith('School:')) {
      parsed.school = line.replace('School:', '').trim();
    } else if (line.startsWith('Descriptor(s):')) {
      const descriptors = line.replace('Descriptor(s):', '').trim();
      parsed.descriptors = descriptors ? descriptors.split(',').map(desc => desc.trim()) : [];
    } else if (line.startsWith('Component(s):')) {
      const components = line.replace('Component(s):', '').trim();
      parsed.components = components.split(',').map(comp => comp.trim());
    } else if (line.startsWith('Range:')) {
      parsed.range = line.replace('Range:', '').trim();
    } else if (line.startsWith('Area of Effect / Target:')) {
      parsed.areaOfEffect = line.replace('Area of Effect / Target:', '').trim();
    } else if (line.startsWith('Duration:')) {
      parsed.duration = line.replace('Duration:', '').trim();
    } else if (line.startsWith('Save:')) {
      parsed.save = line.replace('Save:', '').trim();
    } else if (line.startsWith('Spell Resistance:')) {
      parsed.spellResistance = line.replace('Spell Resistance:', '').trim();
    } else {
      // This is the start of the actual description
      if (descriptionStartIndex === -1) {
        descriptionStartIndex = i;
      }
    }
  }

  // Join remaining lines as description
  if (descriptionStartIndex !== -1) {
    parsed.description = lines.slice(descriptionStartIndex).join(' ');
  }

  return parsed;
}

export function getSpellMetaTags(parsed: ParsedSpellInfo): Array<{label: string, value: string, variant?: 'primary' | 'secondary'}> {
  const tags = [];

  if (parsed.range) {
    tags.push({ label: 'Range', value: parsed.range });
  }

  if (parsed.components.length > 0) {
    tags.push({ label: 'Components', value: parsed.components.join(', ') });
  }

  if (parsed.duration) {
    tags.push({ label: 'Duration', value: parsed.duration });
  }

  if (parsed.areaOfEffect) {
    tags.push({ label: 'Target', value: parsed.areaOfEffect });
  }

  if (parsed.save && parsed.save.toLowerCase() !== 'none') {
    tags.push({ label: 'Save', value: parsed.save, variant: 'secondary' as const });
  }

  if (parsed.spellResistance && parsed.spellResistance.toLowerCase() === 'yes') {
    tags.push({ label: 'SR', value: 'Yes', variant: 'secondary' as const });
  }

  return tags;
}