import React from 'react';
import { Flame, Shield, Sparkles, BookOpen, Star, Zap, Eye, Skull } from 'lucide-react';

export const schoolIcons: Record<string, React.ReactNode> = {
  'Evocation': <Flame className="w-5 h-5" />,
  'Abjuration': <Shield className="w-5 h-5" />,
  'Conjuration': <Sparkles className="w-5 h-5" />,
  'Divination': <BookOpen className="w-5 h-5" />,
  'Enchantment': <Star className="w-5 h-5" />,
  'Transmutation': <Zap className="w-5 h-5" />,
  'Illusion': <Eye className="w-5 h-5" />,
  'Necromancy': <Skull className="w-5 h-5" />,
  'Universal': <Sparkles className="w-5 h-5" />
};

export function getSchoolIcon(school: string, size: 'sm' | 'md' | 'lg' = 'md'): React.ReactNode {
  const sizeClasses = {
    sm: 'w-4 h-4',
    md: 'w-5 h-5',
    lg: 'w-6 h-6'
  };

  const Icon = schoolIcons[school];
  if (!Icon) return <Sparkles className={sizeClasses[size]} />;

  if (React.isValidElement(Icon)) {
    return React.cloneElement(Icon as React.ReactElement<{className?: string}>, {
      className: sizeClasses[size]
    });
  }

  return Icon;
}