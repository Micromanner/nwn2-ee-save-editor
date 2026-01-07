'use client';

import { useState, useRef, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { useTranslations } from '@/hooks/useTranslations';
import { Card, CardContent } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { display, formatModifier } from '@/utils/dataHelpers';
import { CharacterAPI } from '@/services/characterApi';

export default function SkillsEditor() {
  const t = useTranslations();
  const { character } = useCharacterContext();
  
  const skillsSubsystem = useSubsystem('skills');

  const [isUpdating, setIsUpdating] = useState(false);
  const [updatingSkills, setUpdatingSkills] = useState<Set<number>>(new Set());
  const [localSkillOverrides, setLocalSkillOverrides] = useState<Record<number, number>>({})
  

  const [hoveredSkillId, setHoveredSkillId] = useState<number | null>(null);
  const [clickedButton, setClickedButton] = useState<string | null>(null);
  const [showFixedHeader, setShowFixedHeader] = useState(false);
  const [columnWidths, setColumnWidths] = useState<number[]>([]);
  const [tableWidth, setTableWidth] = useState<number>(0);
  const [tableLeft, setTableLeft] = useState<number>(0);
  const tableRef = useRef<HTMLTableElement>(null);
  const headerRef = useRef<HTMLTableRowElement>(null);
  const cardRef = useRef<HTMLDivElement>(null);

  const [searchTerm, setSearchTerm] = useState('');
  const [sortColumn, setSortColumn] = useState<'name' | 'total' | 'ranks' | null>('name');
  const [sortDirection, setSortDirection] = useState<'asc' | 'desc'>('asc');

  useEffect(() => {
    const loadSkillsData = async () => {
      if (!character?.id) return;

      if (!skillsSubsystem.data) {
        try {
          await skillsSubsystem.load();
        } catch (err) {
          console.error('Failed to load skills:', err);
        }
      }
    };

    loadSkillsData();
  }, [character?.id, skillsSubsystem]);

  useEffect(() => {
    setLocalSkillOverrides({});
  }, [skillsSubsystem.data]);
  const applyOverrides = (skillList: Array<{
    id: number;
    name: string;
    key_ability: string;
    current_ranks: number;
    max_ranks: number;
    total_modifier: number;
    is_class_skill: boolean;
    armor_check: boolean;
  }>) => {
    return skillList.map(skill => {
      const overrideRanks = localSkillOverrides[skill.id];
      if (overrideRanks !== undefined) {
        const rankDiff = overrideRanks - skill.current_ranks;
        return {
          ...skill,
          current_ranks: overrideRanks,
          total_modifier: skill.total_modifier + rankDiff
        };
      }
      return skill;
    });
  };

  const isValidSkillName = (name: string) => !name.startsWith('DEL_') && !name.startsWith('***');
  const classSkills = applyOverrides(skillsSubsystem.data?.class_skills?.filter(skill => isValidSkillName(skill.name)) || []);
  const crossClassSkills = applyOverrides(skillsSubsystem.data?.cross_class_skills?.filter(skill => isValidSkillName(skill.name)) || []);
  const skills = [...classSkills, ...crossClassSkills];

  const totalAvailable = skillsSubsystem.data?.total_available ?? 0;
  const totalSpentPoints = skillsSubsystem.data?.spent_points || 0;
  const pointsBalance = totalAvailable - totalSpentPoints;
  const availableSkillPoints = Math.max(0, pointsBalance);
  const overdrawnSkillPoints = pointsBalance < 0 ? Math.abs(pointsBalance) : 0;
  const _totalSkillPoints = totalAvailable;
  const isLoading = skillsSubsystem.isLoading;
  const error = skillsSubsystem.error;

  useEffect(() => {
    const handleScroll = () => {
      if (headerRef.current) {
        const rect = headerRef.current.getBoundingClientRect();
        setShowFixedHeader(rect.bottom < 87);
      }
    };

    const measureColumnWidths = () => {
      if (headerRef.current && cardRef.current) {
        const ths = headerRef.current.querySelectorAll('th');
        const widths = Array.from(ths).map(th => th.getBoundingClientRect().width);
        setColumnWidths(widths);

        const cardRect = cardRef.current.getBoundingClientRect();
        setTableWidth(cardRect.width);
        setTableLeft(cardRect.left);
      }
    };

    const scrollContainer = document.querySelector('main');

    if (scrollContainer) {
      scrollContainer.addEventListener('scroll', handleScroll);
    }
    window.addEventListener('resize', measureColumnWidths);

    setTimeout(() => {
      handleScroll();
      measureColumnWidths();
    }, 100);

    return () => {
      if (scrollContainer) {
        scrollContainer.removeEventListener('scroll', handleScroll);
      }
      window.removeEventListener('resize', measureColumnWidths);
    };
  }, []);

  const handleUpdateSkillRank = async (skillId: number, newRank: number) => {
    if (!character?.id) return;

    const skill = skills.find(s => s.id === skillId);
    if (!skill) return;

    if (newRank < 0) return;

    setLocalSkillOverrides(prev => ({
      ...prev,
      [skillId]: newRank
    }));

    setUpdatingSkills(prev => new Set([...prev, skillId]));

    try {
      const updates = { [skillId]: newRank };
      const response = await CharacterAPI.updateSkills(character.id, updates);

      if (response && response.skill_summary) {
        skillsSubsystem.updateData({ ...response.skill_summary, error: null });
      } else {
        await skillsSubsystem.load();
      }

    } catch (err) {
      console.error('Error updating skill:', err);

      setLocalSkillOverrides(prev => {
        const updated = { ...prev };
        delete updated[skillId];
        return updated;
      });
    } finally {
      setUpdatingSkills(prev => {
        const newSet = new Set(prev);
        newSet.delete(skillId);
        return newSet;
      });
    }
  };

  const handleButtonClick = (buttonType: 'increase' | 'decrease', skillId: number) => {
    const buttonKey = `${buttonType}-${skillId}`;
    setClickedButton(buttonKey);
    setTimeout(() => setClickedButton(null), 150);

    const skill = skills.find(s => s.id === skillId);
    if (!skill) return;

    if (buttonType === 'increase') {
      handleUpdateSkillRank(skillId, skill.current_ranks + 1);
    } else {
      handleUpdateSkillRank(skillId, skill.current_ranks - 1);
    }
  };

  const resetAllSkills = async () => {
    if (!character?.id) return;

    setIsUpdating(true);

    try {
      await CharacterAPI.resetSkills(character.id);
      await skillsSubsystem.load();
    } catch (err) {
      console.error('Error resetting skills:', err);
    } finally {
      setIsUpdating(false);
    }
  };

  const handleSort = (column: 'name' | 'total' | 'ranks') => {
    if (sortColumn === column) {
      setSortDirection(sortDirection === 'asc' ? 'desc' : 'asc');
    } else {
      setSortColumn(column);
      setSortDirection('asc');
    }
  };

  const sortedAndFilteredSkills = [...skills]
    .filter(skill => 
      skill.name.toLowerCase().includes(searchTerm.toLowerCase())
    )
    .sort((a, b) => {
      if (!sortColumn) return 0;
      
      let compareValue = 0;
      switch (sortColumn) {
        case 'name':
          compareValue = a.name.localeCompare(b.name);
          break;
        case 'total':
          compareValue = a.total_modifier - b.total_modifier;
          break;
        case 'ranks':
          compareValue = a.current_ranks - b.current_ranks;
          break;
      }
      
      return sortDirection === 'asc' ? compareValue : -compareValue;
    });

  useEffect(() => {
    const measureColumnWidths = () => {
      if (headerRef.current && cardRef.current) {
        const ths = headerRef.current.querySelectorAll('th');
        const widths = Array.from(ths).map(th => th.getBoundingClientRect().width);
        setColumnWidths(widths);

        const cardRect = cardRef.current.getBoundingClientRect();
        setTableWidth(cardRect.width);
        setTableLeft(cardRect.left);
      }
    };

    setTimeout(measureColumnWidths, 0);
  }, [sortedAndFilteredSkills]);

  const FixedHeader = () => {
    if (!showFixedHeader || typeof document === 'undefined') return null;
    
    return createPortal(
      <div 
        className="fixed top-[87px] z-50"
        style={{ 
          left: `${tableLeft}px`, 
          width: `${tableWidth}px` 
        }}
      >
        <Card className="fixed-table-header rounded-t-none shadow-lg border-b-0">
          <CardContent className="p-0" style={{ paddingTop: '0', paddingBottom: '0' }}>
            <div className="overflow-x-auto">
              <table className="w-full" style={{ tableLayout: 'fixed' }}>
                <colgroup>
                  <col style={{ width: `${columnWidths[0]}px` }} />
                  <col style={{ width: `${columnWidths[1]}px` }} />
                  <col style={{ width: `${columnWidths[2]}px` }} />
                  <col style={{ width: `${columnWidths[3]}px` }} />
                  <col style={{ width: `${columnWidths[4]}px` }} />
                  <col style={{ width: `${columnWidths[5]}px` }} />
                </colgroup>
                <thead>
                  <tr className="border-b border-[rgb(var(--color-surface-border)/0.6)]">
                    <th 
                      className="text-left p-3 font-medium text-[rgb(var(--color-text-secondary))] cursor-pointer hover:text-[rgb(var(--color-text-primary))]"
                      onClick={() => handleSort('name')}
                    >
                      <div className="flex items-center space-x-1">
                        <span>{t('skills.skillName')}</span>
                        {sortColumn === 'name' && (
                          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d={sortDirection === 'asc' ? "M5 15l7-7 7 7" : "M19 9l-7 7-7-7"} />
                          </svg>
                        )}
                      </div>
                    </th>
                    <th 
                      className="text-center p-3 font-medium text-[rgb(var(--color-text-secondary))] cursor-pointer hover:text-[rgb(var(--color-text-primary))]"
                      onClick={() => handleSort('total')}
                    >
                      <div className="flex items-center justify-center space-x-1">
                        <span>{t('skills.total')}</span>
                        {sortColumn === 'total' && (
                          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d={sortDirection === 'asc' ? "M5 15l7-7 7 7" : "M19 9l-7 7-7-7"} />
                          </svg>
                        )}
                      </div>
                    </th>
                    <th 
                      className="text-center p-3 font-medium text-[rgb(var(--color-text-secondary))] cursor-pointer hover:text-[rgb(var(--color-text-primary))]"
                      onClick={() => handleSort('ranks')}
                    >
                      <div className="flex items-center justify-center space-x-1">
                        <span>{t('skills.ranks')}</span>
                        {sortColumn === 'ranks' && (
                          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d={sortDirection === 'asc' ? "M5 15l7-7 7 7" : "M19 9l-7 7-7-7"} />
                          </svg>
                        )}
                      </div>
                    </th>
                    <th className="text-center p-3 font-medium text-[rgb(var(--color-text-secondary))]">{t('skills.ability')}</th>
                    <th className="text-center p-3 font-medium text-[rgb(var(--color-text-secondary))]">{t('skills.misc')}</th>
                    <th className="text-center p-3 font-medium text-[rgb(var(--color-text-secondary))]">{t('skills.class')}</th>
                  </tr>
                </thead>
              </table>
            </div>
          </CardContent>
        </Card>
      </div>,
      document.body
    );
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-[rgb(var(--color-primary))]"></div>
      </div>
    );
  }

  if (error) {
    return (
      <Card variant="error">
        <p className="text-error">{error}</p>
      </Card>
    );
  }

  if (!character) {
    return (
      <Card variant="warning">
        <p className="text-muted">No character loaded. Please import a save file to begin.</p>
      </Card>
    );
  }

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-3 gap-3">
        <Card>
          <CardContent padding="p-3" className="text-center">
            <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('skills.pointsSpent')}</div>
            <div className="text-xl font-bold text-[rgb(var(--color-text-primary))]">
              {display(totalSpentPoints)}
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent padding="p-3" className="text-center">
            <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('skills.pointsAvailable')}</div>
            <div className="text-xl font-bold text-[rgb(var(--color-primary))]">
              {display(availableSkillPoints)}
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent padding="p-3" className="text-center">
            <div className="text-xs text-[rgb(var(--color-text-muted))]">Points Overdrawn</div>
            <div className={`text-xl font-bold ${overdrawnSkillPoints > 0 ? 'text-error' : 'text-[rgb(var(--color-text-muted))]'}`}>
              {display(overdrawnSkillPoints)}
            </div>
          </CardContent>
        </Card>
      </div>

      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-4">
          <Input
            type="text"
            placeholder={t('skills.searchSkills')}
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="w-64"
          />
        </div>
        <div className="flex items-center space-x-2">
          <Button
            variant="outline"
            size="sm"
            onClick={resetAllSkills}
            disabled={isUpdating}
          >
            {t('skills.reset')}
          </Button>
        </div>
      </div>

      <FixedHeader />

      <Card ref={cardRef} className="mt-5">
        <CardContent className="p-0">
          <div className="overflow-x-auto">
            <table ref={tableRef} className="w-full" style={{ tableLayout: 'fixed' }}>
              <colgroup>
                <col style={{ width: '40%' }} />
                <col style={{ width: '10%' }} />
                <col style={{ width: '15%' }} />
                <col style={{ width: '10%' }} />
                <col style={{ width: '10%' }} />
                <col style={{ width: '15%' }} />
              </colgroup>
              <thead>
                <tr ref={headerRef} className="border-b border-[rgb(var(--color-surface-border)/0.6)]">
                  <th 
                    className="text-left p-3 font-medium text-[rgb(var(--color-text-secondary))] cursor-pointer hover:text-[rgb(var(--color-text-primary))]"
                    onClick={() => handleSort('name')}
                  >
                    <div className="flex items-center space-x-1">
                      <span>Skill Name</span>
                      {sortColumn === 'name' && (
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d={sortDirection === 'asc' ? "M5 15l7-7 7 7" : "M19 9l-7 7-7-7"} />
                        </svg>
                      )}
                    </div>
                  </th>
                  <th 
                    className="text-center p-3 font-medium text-[rgb(var(--color-text-secondary))] cursor-pointer hover:text-[rgb(var(--color-text-primary))]"
                    onClick={() => handleSort('total')}
                  >
                    <div className="flex items-center justify-center space-x-1">
                      <span>Total</span>
                      {sortColumn === 'total' && (
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d={sortDirection === 'asc' ? "M5 15l7-7 7 7" : "M19 9l-7 7-7-7"} />
                        </svg>
                      )}
                    </div>
                  </th>
                  <th 
                    className="text-center p-3 font-medium text-[rgb(var(--color-text-secondary))] cursor-pointer hover:text-[rgb(var(--color-text-primary))]"
                    onClick={() => handleSort('ranks')}
                  >
                    <div className="flex items-center justify-center space-x-1">
                      <span>Ranks</span>
                      {sortColumn === 'ranks' && (
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d={sortDirection === 'asc' ? "M5 15l7-7 7 7" : "M19 9l-7 7-7-7"} />
                        </svg>
                      )}
                    </div>
                  </th>
                  <th className="text-center p-3 font-medium text-[rgb(var(--color-text-secondary))]">Ability</th>
                  <th className="text-center p-3 font-medium text-[rgb(var(--color-text-secondary))]">Misc</th>
                  <th className="text-center p-3 font-medium text-[rgb(var(--color-text-secondary))]">Class</th>
                </tr>
              </thead>
              <tbody>
                {sortedAndFilteredSkills.map((skill) => {
                  const isOverdrawn = skill.current_ranks > skill.max_ranks;
                  return (
                  <tr 
                    key={skill.id} 
                    className="border-b border-[rgb(var(--color-surface-border)/0.3)] hover:bg-[rgb(var(--color-surface-1))] transition-colors"
                    onMouseEnter={() => setHoveredSkillId(skill.id)}
                    onMouseLeave={() => setHoveredSkillId(null)}
                  >
                    <td className="p-3">
                      <div className="flex items-center space-x-2">
                        <span className="font-medium text-[rgb(var(--color-text-primary))]">{display(skill.name)}</span>
                        <span className="text-sm text-[rgb(var(--color-text-muted))]">({display(skill.key_ability)})</span>
                        {skill.armor_check && (
                          <span className="text-xs px-1.5 py-0.5 rounded bg-[rgb(var(--color-warning)/0.2)] text-[rgb(var(--color-warning))]">
                            {t('skills.armorCheck')}
                          </span>
                        )}
                        {!skill.is_class_skill && (
                          <span className="text-xs px-1.5 py-0.5 rounded bg-[rgb(var(--color-surface-3))] text-[rgb(var(--color-text-muted))]">
                            2pt
                          </span>
                        )}
                        {isOverdrawn && (
                          <span className="text-xs px-1.5 py-0.5 rounded bg-purple-500/20 text-purple-400 font-medium">
                            +{skill.current_ranks - skill.max_ranks} Over Cap
                          </span>
                        )}
                      </div>
                    </td>
                    <td className="p-3 text-center">
                      <span className="text-lg font-semibold text-[rgb(var(--color-primary))]">
                        {formatModifier(skill.total_modifier)}
                      </span>
                    </td>
                    <td className="p-3">
                      <div className={`flex items-center justify-center space-x-2 transition-opacity ${hoveredSkillId === skill.id ? 'opacity-100' : 'opacity-60'}`}>
                        <Button
                          onClick={() => handleButtonClick('decrease', skill.id)}
                          variant="outline"
                          size="sm"
                          disabled={(skill.current_ranks === 0) || updatingSkills.has(skill.id)}
                          clicked={clickedButton === `decrease-${skill.id}`}
                          aria-label={`Decrease ${skill.name}`}
                          title={`Decrease ${skill.name} (min: 0)`}
                          className="h-6 w-6 p-0 text-xs"
                        >
                          -
                        </Button>
                        <input
                          type="number"
                          value={skill.current_ranks}
                          onChange={(e) => handleUpdateSkillRank(skill.id, parseInt(e.target.value) || 0)}
                          className={`w-12 text-center h-6 text-sm border rounded font-medium ${isOverdrawn ? 'bg-purple-500/10 border-purple-500/40 text-purple-400' : 'bg-[rgb(var(--color-surface-2))] border-[rgb(var(--color-surface-border)/0.6)]'}`}
                          disabled={updatingSkills.has(skill.id)}
                        />
                        <Button
                          onClick={() => handleButtonClick('increase', skill.id)}
                          variant="outline"
                          size="sm"
                          disabled={updatingSkills.has(skill.id) || (!skill.is_class_skill && availableSkillPoints === 1)}
                          clicked={clickedButton === `increase-${skill.id}`}
                          aria-label={`Increase ${skill.name}`}
                          title={`Increase ${skill.name} (cost: ${skill.is_class_skill ? '1' : '2'} points, max: ${skill.max_ranks})`}
                          className="h-6 w-6 p-0 text-xs"
                        >
                          +
                        </Button>
                      </div>
                    </td>
                    <td className="p-3 text-center text-sm text-[rgb(var(--color-text-secondary))]">
                      {formatModifier(skill.total_modifier - skill.current_ranks)}
                    </td>
                    <td className="p-3 text-center text-sm text-[rgb(var(--color-text-secondary))]">
                      <span
                        className="cursor-help"
                        title={t('skills.miscModifiers')}
                      >
                        {display('-')}
                      </span>
                    </td>
                    <td className="p-3 text-center">
                      {skill.is_class_skill && (
                        <span className="text-[rgb(var(--color-primary))]">✓</span>
                      )}
                    </td>
                  </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}