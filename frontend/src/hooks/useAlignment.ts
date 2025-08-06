import { useState } from 'react';
import { useTranslations } from '@/hooks/useTranslations';

export interface Alignment {
  lawChaos: number;
  goodEvil: number;
}

export interface AlignmentInfo {
  text: string;
  color: string;
  description: string;
}

export function useAlignment(initialAlignment?: Alignment) {
  const t = useTranslations();
  
  const [alignment, setAlignment] = useState<Alignment>(
    initialAlignment || { lawChaos: 50, goodEvil: 50 }
  );

  const getAlignmentText = (lawChaos: number, goodEvil: number): string => {
    if (lawChaos >= 31 && lawChaos <= 69 && goodEvil >= 31 && goodEvil <= 69) {
      return t('alignment.trueNeutral');
    }
    
    const lawKey = lawChaos <= 30 ? 'chaotic' : lawChaos >= 70 ? 'lawful' : 'neutral';
    const goodKey = goodEvil <= 30 ? 'evil' : goodEvil >= 70 ? 'good' : 'neutral';
    
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

  const getAlignmentColor = (lawChaos: number, goodEvil: number): string => {
    // Good alignments (70-100)
    if (goodEvil >= 70) {
      if (lawChaos >= 70) return '#FFD700'; // Lawful Good - Gold
      if (lawChaos <= 30) return '#228B22'; // Chaotic Good - Forest Green
      return '#87CEEB'; // Neutral Good - Sky Blue
    }
    
    // Evil alignments (0-30)
    if (goodEvil <= 30) {
      if (lawChaos >= 70) return '#8B0000'; // Lawful Evil - Deep Red
      if (lawChaos <= 30) return '#483D8B'; // Chaotic Evil - Abyssal Purple
      return '#556B2F'; // Neutral Evil - Vile Green
    }
    
    // Neutral alignments (31-69)
    if (lawChaos >= 70) return '#71797E'; // Lawful Neutral - Steel Gray
    if (lawChaos <= 30) return '#FF4500'; // Chaotic Neutral - Fiery Orange
    return '#A0522D'; // True Neutral - Earthy Brown
  };

  const getAlignmentDescription = (lawChaos: number, goodEvil: number): string => {
    if (lawChaos >= 70 && goodEvil >= 70) return "Acts with compassion and honor, holding a strong sense of duty.";
    if (lawChaos >= 31 && lawChaos <= 69 && goodEvil >= 70) return "Does what is good and right without a strong bias for or against order.";
    if (lawChaos <= 30 && goodEvil >= 70) return "Follows their own conscience to do good, with little regard for societal laws.";
    if (lawChaos >= 70 && goodEvil >= 31 && goodEvil <= 69) return "Adheres to a personal code, tradition, or the law above all else.";
    if (lawChaos >= 31 && lawChaos <= 69 && goodEvil >= 31 && goodEvil <= 69) return "Maintains a balance, avoiding strong commitments to any single alignment extreme.";
    if (lawChaos <= 30 && goodEvil >= 31 && goodEvil <= 69) return "Values personal freedom and individuality above all other considerations.";
    if (lawChaos >= 70 && goodEvil <= 30) return "Methodically and intentionally uses order and structure to achieve malevolent goals.";
    if (lawChaos >= 31 && lawChaos <= 69 && goodEvil <= 30) return "Acts out of pure self-interest, harming others when it is convenient.";
    if (lawChaos <= 30 && goodEvil <= 30) return "Engages in destructive and unpredictable acts of evil and malice.";
    return "";
  };

  const getCurrentAlignmentInfo = (): AlignmentInfo => ({
    text: getAlignmentText(alignment.lawChaos, alignment.goodEvil),
    color: getAlignmentColor(alignment.lawChaos, alignment.goodEvil),
    description: getAlignmentDescription(alignment.lawChaos, alignment.goodEvil)
  });

  const updateAlignment = (updates: Partial<Alignment>) => {
    setAlignment(prev => ({ ...prev, ...updates }));
  };

  const setAlignmentFromGrid = (lawChaosValue: number, goodEvilValue: number) => {
    setAlignment({ lawChaos: lawChaosValue, goodEvil: goodEvilValue });
  };

  const isAlignmentActive = (lawChaosRange: [number, number], goodEvilRange: [number, number]): boolean => {
    const { lawChaos, goodEvil } = alignment;
    return lawChaos >= lawChaosRange[0] && lawChaos <= lawChaosRange[1] &&
           goodEvil >= goodEvilRange[0] && goodEvil <= goodEvilRange[1];
  };

  // Predefined alignment grid data
  const alignmentGridData = [
    { name: 'Lawful Good', lawChaos: 85, goodEvil: 85, ranges: [[70, 100], [70, 100]] },
    { name: 'Neutral Good', lawChaos: 50, goodEvil: 85, ranges: [[31, 69], [70, 100]] },
    { name: 'Chaotic Good', lawChaos: 15, goodEvil: 85, ranges: [[0, 30], [70, 100]] },
    { name: 'Lawful Neutral', lawChaos: 85, goodEvil: 50, ranges: [[70, 100], [31, 69]] },
    { name: 'True Neutral', lawChaos: 50, goodEvil: 50, ranges: [[31, 69], [31, 69]] },
    { name: 'Chaotic Neutral', lawChaos: 15, goodEvil: 50, ranges: [[0, 30], [31, 69]] },
    { name: 'Lawful Evil', lawChaos: 85, goodEvil: 15, ranges: [[70, 100], [0, 30]] },
    { name: 'Neutral Evil', lawChaos: 50, goodEvil: 15, ranges: [[31, 69], [0, 30]] },
    { name: 'Chaotic Evil', lawChaos: 15, goodEvil: 15, ranges: [[0, 30], [0, 30]] },
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