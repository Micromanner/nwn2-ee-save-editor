import { useState } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import type { Alignment } from '@/lib/bindings';

export interface AlignmentInfo {
  text: string;
  color: string;
  description: string;
}

export function useAlignment(initialAlignment?: Alignment) {
  const t = useTranslations();

  const [alignment, setAlignment] = useState<Alignment>(
    initialAlignment || { law_chaos: 50, good_evil: 50 }
  );

  const getAlignmentText = (law_chaos: number, good_evil: number): string => {
    if (law_chaos >= 31 && law_chaos <= 69 && good_evil >= 31 && good_evil <= 69) {
      return t('alignment.trueNeutral');
    }

    const lawKey = law_chaos <= 30 ? 'chaotic' : law_chaos >= 70 ? 'lawful' : 'neutral';
    const goodKey = good_evil <= 30 ? 'evil' : good_evil >= 70 ? 'good' : 'neutral';

    if (lawKey === 'lawful' && goodKey === 'good') return t('alignment.lawfulGood');
    if (lawKey === 'neutral' && goodKey === 'good') return t('alignment.neutralGood');
    if (lawKey === 'chaotic' && goodKey === 'good') return t('alignment.chaoticGood');
    if (lawKey === 'lawful' && goodKey === 'neutral') return t('alignment.lawfulNeutral');
    if (lawKey === 'chaotic' && goodKey === 'neutral') return t('alignment.chaoticNeutral');
    if (lawKey === 'lawful' && goodKey === 'evil') return t('alignment.lawfulEvil');
    if (lawKey === 'neutral' && goodKey === 'evil') return t('alignment.neutralEvil');
    if (lawKey === 'chaotic' && goodKey === 'evil') return t('alignment.chaoticEvil');

    return t('alignment.trueNeutral');
  };

  const getAlignmentColor = (law_chaos: number, good_evil: number): string => {
    if (good_evil >= 70) {
      if (law_chaos >= 70) return '#FFD700';
      if (law_chaos <= 30) return '#228B22';
      return '#87CEEB';
    }

    if (good_evil <= 30) {
      if (law_chaos >= 70) return '#8B0000';
      if (law_chaos <= 30) return '#483D8B';
      return '#556B2F';
    }

    if (law_chaos >= 70) return '#71797E';
    if (law_chaos <= 30) return '#FF4500';
    return '#A0522D';
  };

  const getAlignmentDescription = (law_chaos: number, good_evil: number): string => {
    if (law_chaos >= 70 && good_evil >= 70) return t('alignment.descriptions.lawfulGood');
    if (law_chaos >= 31 && law_chaos <= 69 && good_evil >= 70) return t('alignment.descriptions.neutralGood');
    if (law_chaos <= 30 && good_evil >= 70) return t('alignment.descriptions.chaoticGood');
    if (law_chaos >= 70 && good_evil >= 31 && good_evil <= 69) return t('alignment.descriptions.lawfulNeutral');
    if (law_chaos >= 31 && law_chaos <= 69 && good_evil >= 31 && good_evil <= 69) return t('alignment.descriptions.trueNeutral');
    if (law_chaos <= 30 && good_evil >= 31 && good_evil <= 69) return t('alignment.descriptions.chaoticNeutral');
    if (law_chaos >= 70 && good_evil <= 30) return t('alignment.descriptions.lawfulEvil');
    if (law_chaos >= 31 && law_chaos <= 69 && good_evil <= 30) return t('alignment.descriptions.neutralEvil');
    if (law_chaos <= 30 && good_evil <= 30) return t('alignment.descriptions.chaoticEvil');
    return "";
  };

  const getCurrentAlignmentInfo = (): AlignmentInfo => ({
    text: getAlignmentText(alignment.law_chaos, alignment.good_evil),
    color: getAlignmentColor(alignment.law_chaos, alignment.good_evil),
    description: getAlignmentDescription(alignment.law_chaos, alignment.good_evil)
  });

  const updateAlignment = (updates: Partial<Alignment>) => {
    setAlignment(prev => ({ ...prev, ...updates }));
  };

  const setAlignmentFromGrid = (law_chaos: number, good_evil: number) => {
    setAlignment({ law_chaos, good_evil });
  };

  const isAlignmentActive = (lawChaosRange: [number, number], goodEvilRange: [number, number]): boolean => {
    const { law_chaos, good_evil } = alignment;
    return law_chaos >= lawChaosRange[0] && law_chaos <= lawChaosRange[1] &&
           good_evil >= goodEvilRange[0] && good_evil <= goodEvilRange[1];
  };

  const alignmentGridData = [
    { name: t('alignment.names.lawfulGood'), law_chaos: 85, good_evil: 85, ranges: [[70, 100], [70, 100]] },
    { name: t('alignment.names.neutralGood'), law_chaos: 50, good_evil: 85, ranges: [[31, 69], [70, 100]] },
    { name: t('alignment.names.chaoticGood'), law_chaos: 15, good_evil: 85, ranges: [[0, 30], [70, 100]] },
    { name: t('alignment.names.lawfulNeutral'), law_chaos: 85, good_evil: 50, ranges: [[70, 100], [31, 69]] },
    { name: t('alignment.names.trueNeutral'), law_chaos: 50, good_evil: 50, ranges: [[31, 69], [31, 69]] },
    { name: t('alignment.names.chaoticNeutral'), law_chaos: 15, good_evil: 50, ranges: [[0, 30], [31, 69]] },
    { name: t('alignment.names.lawfulEvil'), law_chaos: 85, good_evil: 15, ranges: [[70, 100], [0, 30]] },
    { name: t('alignment.names.neutralEvil'), law_chaos: 50, good_evil: 15, ranges: [[31, 69], [0, 30]] },
    { name: t('alignment.names.chaoticEvil'), law_chaos: 15, good_evil: 15, ranges: [[0, 30], [0, 30]] },
  ];

  return {
    alignment,
    updateAlignment,
    setAlignmentFromGrid,
    getCurrentAlignmentInfo,
    isAlignmentActive,
    alignmentGridData,
    getAlignmentText,
    getAlignmentColor,
    getAlignmentDescription
  };
}