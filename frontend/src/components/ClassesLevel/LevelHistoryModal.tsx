'use client';

import { useEffect, useState, useCallback } from 'react';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { useTranslations } from '@/hooks/useTranslations';
import { Card, CardContent } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import DynamicAPI from '@/lib/utils/dynamicApi';
import { display } from '@/utils/dataHelpers';
import { ChevronDown, ChevronRight } from 'lucide-react';

const X = ({ className }: { className?: string }) => (
  <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
  </svg>
);

interface LevelHistoryEntry {
  level: number;
  class: string;
  class_level: number;
  hp_gained: number;
  skill_points_remaining: number;
  ability_increase?: string;
  skills_gained: { name: string; rank: number }[];
  feats_gained: { name: string }[];
  spells_learned: { name: string; level: number }[];
  spells_removed: { name: string; level: number }[];
}

interface LevelHistoryResponse {
  history: LevelHistoryEntry[];
}

interface LevelHistoryModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export default function LevelHistoryModal({ isOpen, onClose }: LevelHistoryModalProps) {
  const { characterId } = useCharacterContext();
  const t = useTranslations();
  const [history, setHistory] = useState<LevelHistoryEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expandedLevels, setExpandedLevels] = useState<Set<number>>(new Set());

  const fetchHistory = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await DynamicAPI.fetch(`/characters/${characterId}/classes/history`);
      if (!response.ok) {
        throw new Error(`API Error: ${response.status} ${response.statusText}`);
      }
      const data: LevelHistoryResponse = await response.json();
      setHistory(data.history || []);
      if (data.history?.length) {
        setExpandedLevels(new Set(data.history.map((h: LevelHistoryEntry) => h.level)));
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'An unknown error occurred');
    } finally {
      setLoading(false);
    }
  }, [characterId]);

  useEffect(() => {
    if (isOpen && characterId) {
      fetchHistory();
    }
  }, [isOpen, characterId, fetchHistory]);


  const toggleLevel = (level: number) => {
    setExpandedLevels(prev => {
      const next = new Set(prev);
      if (next.has(level)) {
        next.delete(level);
      } else {
        next.add(level);
      }
      return next;
    });
  };

  const collapseAll = () => setExpandedLevels(new Set());
  const expandAll = () => setExpandedLevels(new Set(history.map(h => h.level)));

  if (!isOpen) return null;

  return (
    <div className="level-history-modal-overlay">
      <Card className="level-history-modal-container">
        <CardContent padding="p-0" className="flex flex-col h-full">
          <div className="level-history-modal-header">
            <div className="level-history-modal-header-row">
              <h3 className="level-history-modal-title">
                {t('classes.levelHistory')}
              </h3>
              <Button
                onClick={onClose}
                variant="ghost"
                size="sm"
                className="level-history-modal-close-button"
              >
                <X className="w-4 h-4" />
              </Button>
            </div>
          </div>

          <div className="level-history-modal-content">
            {loading ? (
              <div className="level-history-modal-loading">
                <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-[rgb(var(--color-primary))]"></div>
                <span>{t('common.loading')}</span>
              </div>
            ) : error ? (
              <div className="level-history-modal-error">
                {t('common.error')}: {error}
              </div>
            ) : history.length === 0 ? (
              <div className="level-history-modal-empty">
                {t('classes.noLevelHistory')}
              </div>
            ) : (
              <div className="level-history-modal-list">
                <div className="level-history-modal-controls">
                  <Button variant="outline" size="sm" onClick={expandAll}>
                    {t('common.expandAll')}
                  </Button>
                  <Button variant="outline" size="sm" onClick={collapseAll}>
                    {t('common.collapseAll')}
                  </Button>
                </div>

                {[...history].reverse().map((entry) => (
                  <Card
                    key={entry.level}
                    className="level-history-modal-level-card"
                  >
                    <button
                      onClick={() => toggleLevel(entry.level)}
                      className="level-history-modal-level-header"
                    >
                      <div className="level-history-modal-level-title">
                        {expandedLevels.has(entry.level) ? (
                          <ChevronDown className="w-4 h-4 text-[rgb(var(--color-text-muted))]" />
                        ) : (
                          <ChevronRight className="w-4 h-4 text-[rgb(var(--color-text-muted))]" />
                        )}
                        <span className="level-history-modal-level-number">
                          {t('classes.level')} {entry.level}
                        </span>
                      </div>
                    </button>

                    {expandedLevels.has(entry.level) && (
                      <div className="level-history-modal-level-details">
                        <div className="level-history-modal-section">
                          <div className="level-history-modal-section-title">
                            {t('classes.class')}
                          </div>
                          <ul className="level-history-modal-list">
                            <li className="level-history-modal-list-item">
                              {display(entry.class)} {t('classes.level')} {entry.class_level}
                            </li>
                          </ul>
                        </div>

                        <div className="level-history-modal-section">
                          <div className="level-history-modal-section-title">
                            {t('classes.stats')}
                          </div>
                          <ul className="level-history-modal-list">
                            <li className="level-history-modal-list-item">
                              {t('classes.hpGained')}: <span className="level-history-modal-hp">+{entry.hp_gained}</span>
                            </li>
                            <li className="level-history-modal-list-item">
                              {t('classes.skillPointsRemaining')}: <span>{entry.skill_points_remaining}</span>
                            </li>
                            {entry.ability_increase && (
                              <li className="level-history-modal-list-item">
                                {t('classes.abilityIncrease')}: <span className="level-history-modal-ability">{entry.ability_increase}</span>
                              </li>
                            )}
                          </ul>
                        </div>

                        {entry.skills_gained.length > 0 && (
                          <div className="level-history-modal-section">
                            <div className="level-history-modal-section-title">
                              {t('classes.skills')}
                            </div>
                            <ul className="level-history-modal-list">
                              {entry.skills_gained.map((skill, idx) => (
                                <li key={idx} className="level-history-modal-list-item">
                                  {display(skill.name)} <span className="level-history-modal-skill-rank">+{skill.rank}</span>
                                </li>
                              ))}
                            </ul>
                          </div>
                        )}

                        {entry.feats_gained.length > 0 && (
                          <div className="level-history-modal-section">
                            <div className="level-history-modal-section-title">
                              {t('classes.feats')}
                            </div>
                            <ul className="level-history-modal-list">
                              {entry.feats_gained.map((feat, idx) => (
                                <li key={idx} className="level-history-modal-list-item">
                                  {display(feat.name)}
                                </li>
                              ))}
                            </ul>
                          </div>
                        )}

                        {entry.spells_learned.length > 0 && (
                          <div className="level-history-modal-section">
                            <div className="level-history-modal-section-title">
                              {t('classes.spellsLearned')}
                            </div>
                            <ul className="level-history-modal-list">
                              {entry.spells_learned.map((spell, idx) => (
                                <li key={idx} className="level-history-modal-list-item">
                                  {display(spell.name)} <span className="level-history-modal-spell-level">({t('classes.lvl')} {spell.level})</span>
                                </li>
                              ))}
                            </ul>
                          </div>
                        )}

                        {entry.spells_removed.length > 0 && (
                          <div className="level-history-modal-section">
                            <div className="level-history-modal-section-title">
                              {t('classes.spellsRemoved')}
                            </div>
                            <ul className="level-history-modal-list level-history-modal-list-removed">
                              {entry.spells_removed.map((spell, idx) => (
                                <li key={idx} className="level-history-modal-list-item">
                                  {display(spell.name)} <span className="level-history-modal-spell-level">({t('classes.lvl')} {spell.level})</span>
                                </li>
                              ))}
                            </ul>
                          </div>
                        )}

                        {entry.skills_gained.length === 0 &&
                         entry.feats_gained.length === 0 &&
                         entry.spells_learned.length === 0 &&
                         entry.spells_removed.length === 0 && (
                          <div className="level-history-modal-empty-level">
                            {t('classes.noGainsRecorded')}
                          </div>
                        )}
                      </div>
                    )}
                  </Card>
                ))}
              </div>
            )}
          </div>

          <div className="level-history-modal-footer">
            <span>{history.length} {t('classes.levelsRecorded')}</span>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
