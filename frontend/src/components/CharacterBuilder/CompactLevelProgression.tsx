import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card';

interface ClassLevel {
  level: number;
  classId: number;
  className?: string;
  featGained?: boolean;
  abilityIncrease?: boolean;
}

interface GameClass {
  id: number;
  name: string;
  label?: string;
}

interface CompactLevelProgressionProps {
  levels: ClassLevel[];
  gameClasses: GameClass[];
  onLevelClick?: (level: number) => void;
  maxLevel?: number;
}

const CompactLevelProgression: React.FC<CompactLevelProgressionProps> = ({
  levels,
  gameClasses,
  onLevelClick,
  maxLevel = 60
}) => {
  // Create array of all 40 levels
  const allLevels = Array.from({ length: maxLevel }, (_, i) => {
    const level = i + 1;
    const classData = levels.find(l => l.level === level);
    return {
      level,
      ...classData
    };
  });

  // Group levels into rows of 10
  const rows = [];
  for (let i = 0; i < maxLevel; i += 10) {
    rows.push(allLevels.slice(i, i + 10));
  }

  const getClassColor = (classId: number): string => {
    const colors = [
      'bg-amber-100 text-amber-800', // Barbarian
      'bg-yellow-100 text-yellow-800', // Bard
      'bg-gray-100 text-gray-800', // Cleric
      'bg-green-100 text-green-800', // Druid
      'bg-blue-100 text-blue-800', // Fighter
      'bg-orange-100 text-orange-800', // Monk
      'bg-sky-100 text-sky-800', // Paladin
      'bg-emerald-100 text-emerald-800', // Ranger
      'bg-slate-100 text-slate-800', // Rogue
      'bg-purple-100 text-purple-800', // Sorcerer
      'bg-indigo-100 text-indigo-800', // Wizard
      'bg-red-100 text-red-800', // Warlock
    ];
    return colors[classId % colors.length];
  };

  const getClassName = (classId: number): string => {
    const cls = gameClasses.find(c => c.id === classId);
    return cls?.label?.substring(0, 3).toUpperCase() || 'UNK';
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">Level Progression Overview</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          {rows.map((row, rowIndex) => (
            <div key={rowIndex} className="flex gap-1">
              {row.map((level) => (
                <div
                  key={level.level}
                  onClick={() => onLevelClick?.(level.level)}
                  className={`
                    relative w-14 h-12 border rounded-md flex flex-col items-center justify-center
                    ${onLevelClick ? 'cursor-pointer hover:border-blue-500' : ''}
                    ${level.classId !== undefined ? getClassColor(level.classId) : 'bg-gray-50'}
                    transition-colors
                  `}
                  title={`Level ${level.level}${level.classId !== undefined ? ` - ${gameClasses.find(c => c.id === level.classId)?.name || 'Unknown'}` : ''}`}
                >
                  <div className="text-xs font-bold">{level.level}</div>
                  {level.classId !== undefined && (
                    <div className="text-xs font-medium">
                      {getClassName(level.classId)}
                    </div>
                  )}
                  
                  {/* Feat indicator */}
                  {(level.level % 3 === 0 || level.level === 1) && (
                    <div className="absolute top-0.5 right-0.5 w-1.5 h-1.5 bg-green-500 rounded-full" />
                  )}
                  
                  {/* Ability increase indicator */}
                  {level.level % 4 === 0 && (
                    <div className="absolute top-0.5 left-0.5 w-1.5 h-1.5 bg-blue-500 rounded-full" />
                  )}
                </div>
              ))}
            </div>
          ))}
        </div>

        <div className="mt-4 flex gap-4 text-sm">
          <div className="flex items-center gap-1">
            <div className="w-2 h-2 bg-green-500 rounded-full" />
            <span className="text-gray-600">Feat levels</span>
          </div>
          <div className="flex items-center gap-1">
            <div className="w-2 h-2 bg-blue-500 rounded-full" />
            <span className="text-gray-600">Ability increase</span>
          </div>
        </div>
      </CardContent>
    </Card>
  );
};

export default CompactLevelProgression;