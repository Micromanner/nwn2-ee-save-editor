
import { useAlignment } from '@/hooks/useAlignment';
import NWN2Icon from '@/components/ui/NWN2Icon';

interface AlignmentGridProps {
  onAlignmentSelect: (lawChaos: number, goodEvil: number) => void;
  currentAlignment: { lawChaos: number; goodEvil: number };
}

export default function AlignmentGrid({ onAlignmentSelect, currentAlignment }: AlignmentGridProps) {
  const { alignmentGridData, getAlignmentColor } = useAlignment(currentAlignment);

  const isAlignmentActive = (lawChaosRange: [number, number], goodEvilRange: [number, number]): boolean => {
    return currentAlignment.lawChaos >= lawChaosRange[0] && currentAlignment.lawChaos <= lawChaosRange[1] &&
           currentAlignment.goodEvil >= goodEvilRange[0] && currentAlignment.goodEvil <= goodEvilRange[1];
  };

  const getAlignmentIcon = (name: string): string => {
    const iconMap: { [key: string]: string } = {
      'Lawful Good': 'align_lg', 'Neutral Good': 'align_ng', 'Chaotic Good': 'align_cg',
      'Lawful Neutral': 'align_ln', 'True Neutral': 'align_nn', 'Chaotic Neutral': 'align_cn',
      'Lawful Evil': 'align_le', 'Neutral Evil': 'align_ne', 'Chaotic Evil': 'align_ce'
    };
    return iconMap[name] || 'align_nn';
  };

  return (
    <div className="grid grid-cols-3 gap-4 max-w-lg mx-auto">
      {alignmentGridData.map((alignmentInfo, index) => {
        const isActive = isAlignmentActive(
          alignmentInfo.ranges[0] as [number, number], 
          alignmentInfo.ranges[1] as [number, number]
        );
        const color = getAlignmentColor(alignmentInfo.lawChaos, alignmentInfo.goodEvil);

        return (
          <button
            key={index}
            onClick={() => onAlignmentSelect(alignmentInfo.lawChaos, alignmentInfo.goodEvil)}
            className={`
              flex flex-col items-center justify-center gap-3 p-4 rounded-lg 
              border-2 transition-all duration-200 cursor-pointer
              hover:scale-105 hover:shadow-md
              ${isActive 
                ? 'border-current shadow-lg' 
                : 'border-[rgb(var(--color-surface-border))]'
              }
              bg-[rgb(var(--color-surface-2))] hover:bg-[rgb(var(--color-surface-1))]
            `}
            style={isActive ? { 
              borderColor: color,
              backgroundColor: `${color}20`,
              color: color
            } : {}}
            onMouseEnter={(e) => {
              if (!isActive) {
                e.currentTarget.style.setProperty('border-color', color, 'important');
              }
            }}
            onMouseLeave={(e) => {
              if (!isActive) {
                e.currentTarget.style.setProperty('border-color', 'rgb(var(--color-surface-border))', 'important');
              }
            }}
          >
            <NWN2Icon 
              icon={getAlignmentIcon(alignmentInfo.name)} 
              size="lg" 
              alt={alignmentInfo.name}
            />
            <span className="text-xs font-medium text-center leading-tight">
              {alignmentInfo.name}
            </span>
          </button>
        );
      })}
    </div>
  );
}