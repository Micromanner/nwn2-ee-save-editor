
import { useState } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { Card, CardContent } from '@/components/ui/Card';
import { useAlignment, Alignment } from '@/hooks/useAlignment';
import AlignmentGrid from './AlignmentGrid';

interface AlignmentSectionProps {
  alignment?: Alignment;
  onAlignmentChange?: (alignment: Alignment) => void;
}

export default function AlignmentSection({ 
  alignment: externalAlignment,
  onAlignmentChange 
}: AlignmentSectionProps) {
  const t = useTranslations();
  
  const [internalAlignment, setInternalAlignment] = useState<Alignment>({
    lawChaos: 50,
    goodEvil: 50
  });

  const currentAlignment = externalAlignment || internalAlignment;
  const { getAlignmentColor } = useAlignment(currentAlignment);
  const alignmentColor = getAlignmentColor(currentAlignment.lawChaos, currentAlignment.goodEvil);

  const updateAlignment = (updates: Partial<Alignment>) => {
    const newAlignment = { ...currentAlignment, ...updates };
    
    if (onAlignmentChange) {
      onAlignmentChange(newAlignment);
    } else {
      setInternalAlignment(newAlignment);
    }
  };

  const setAlignmentFromGrid = (lawChaosValue: number, goodEvilValue: number) => {
    updateAlignment({ lawChaos: lawChaosValue, goodEvil: goodEvilValue });
  };

  return (
    <Card variant="container">
      <CardContent className="p-6">
        <h3 className="section-title">{t('character.alignment')}</h3>
        <div className="max-w-3xl mx-auto">
          <div className="flex flex-col items-center justify-center space-y-6">
            <div className="grid grid-cols-2 gap-6 max-w-sm mx-auto">
              <div className="bg-[rgb(var(--color-surface-2))] rounded-lg p-4 border border-[rgb(var(--color-surface-border))] text-center">
                <div className="text-xs font-medium text-[rgb(var(--color-text-muted))] uppercase tracking-wide mb-2">
                  Law ↔ Chaos
                </div>
                <input
                  type="number"
                  min="0"
                  max="100"
                  value={currentAlignment.lawChaos}
                  onChange={(e) => updateAlignment({ lawChaos: parseInt(e.target.value) || 0 })}
                  className="text-2xl font-bold text-[rgb(var(--color-text-primary))] mb-1 bg-[rgb(var(--color-surface-3))] text-center border rounded px-2 py-1 w-20 mx-auto focus:outline-none"
                  style={{
                    borderColor: 'rgb(var(--color-surface-border))',
                    transition: 'border-color 0.2s ease'
                  }}
                  onMouseEnter={(e) => e.currentTarget.style.setProperty('border-color', alignmentColor, 'important')}
                  onMouseLeave={(e) => e.currentTarget.style.setProperty('border-color', 'rgb(var(--color-surface-border))', 'important')}
                  onFocus={(e) => e.currentTarget.style.setProperty('border-color', alignmentColor, 'important')}
                  onBlur={(e) => e.currentTarget.style.setProperty('border-color', 'rgb(var(--color-surface-border))', 'important')}
                />
                <div className="text-sm text-[rgb(var(--color-text-secondary))]">
                  {currentAlignment.lawChaos <= 30 ? 'Chaotic' : 
                   (currentAlignment.lawChaos >= 70 ? 'Lawful' : 'Neutral')}
                </div>
              </div>
              
              <div className="bg-[rgb(var(--color-surface-2))] rounded-lg p-4 border border-[rgb(var(--color-surface-border))] text-center">
                <div className="text-xs font-medium text-[rgb(var(--color-text-muted))] uppercase tracking-wide mb-2">
                  Good ↔ Evil
                </div>
                <input
                  type="number"
                  min="0"
                  max="100"
                  value={currentAlignment.goodEvil}
                  onChange={(e) => updateAlignment({ goodEvil: parseInt(e.target.value) || 0 })}
                  className="text-2xl font-bold text-[rgb(var(--color-text-primary))] mb-1 bg-[rgb(var(--color-surface-3))] text-center border rounded px-2 py-1 w-20 mx-auto focus:outline-none"
                  style={{
                    borderColor: 'rgb(var(--color-surface-border))',
                    transition: 'border-color 0.2s ease'
                  }}
                  onMouseEnter={(e) => e.currentTarget.style.setProperty('border-color', alignmentColor, 'important')}
                  onMouseLeave={(e) => e.currentTarget.style.setProperty('border-color', 'rgb(var(--color-surface-border))', 'important')}
                  onFocus={(e) => e.currentTarget.style.setProperty('border-color', alignmentColor, 'important')}
                  onBlur={(e) => e.currentTarget.style.setProperty('border-color', 'rgb(var(--color-surface-border))', 'important')}
                />
                <div className="text-sm text-[rgb(var(--color-text-secondary))]">
                  {currentAlignment.goodEvil <= 30 ? 'Evil' : 
                   (currentAlignment.goodEvil >= 70 ? 'Good' : 'Neutral')}
                </div>
              </div>
            </div>
            
            <AlignmentGrid 
              onAlignmentSelect={setAlignmentFromGrid}
              currentAlignment={currentAlignment}
            />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}