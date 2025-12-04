'use client';

import { useState, useEffect, useMemo } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/Tabs';
import { Checkbox } from '@/components/ui/Checkbox';
import { useTranslations } from '@/hooks/useTranslations';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { useToast } from '@/contexts/ToastContext';
import { display, formatNumber } from '@/utils/dataHelpers';
import {
  gameStateAPI,
  EnrichedQuestData,
  UnmappedVariableData,
  QuestStats,
} from '@/services/gameStateApi';
import {
  Search,
  ChevronDown,
  ChevronUp,
  BookOpen,
  AlertCircle,
  CheckCircle2,
  Clock,
  HelpCircle,
  Filter,
} from 'lucide-react';

type QuestFilter = 'all' | 'active' | 'completed';
type TabMode = 'quests' | 'unmapped';

const isBooleanVariable = (variableName: string): boolean => {
  if (variableName.length < 2) return false;
  const prefixMatch = variableName.match(/^(\d+_)?(.+)$/);
  const nameWithoutPrefix = prefixMatch ? prefixMatch[2] : variableName;
  if (nameWithoutPrefix.length < 2) return false;
  if (nameWithoutPrefix[0] !== 'b') return false;
  // Match bUppercase (camelCase) or b_ (snake_case)
  const nextChar = nameWithoutPrefix[1];
  return nextChar === '_' || (nextChar === nextChar.toUpperCase() && nextChar !== nextChar.toLowerCase());
};

const cleanDisplayName = (name: string): string => {
  return name.replace(/_/g, ' ');
};

interface QuestCardProps {
  quest: EnrichedQuestData;
  isExpanded: boolean;
  onToggle: () => void;
  onUpdate: (variableName: string, value: number) => Promise<void>;
  t: (key: string) => string;
}

function QuestCard({ quest, isExpanded, onToggle, onUpdate, t }: QuestCardProps) {
  const [editValue, setEditValue] = useState<string>(String(quest.current_value));
  const [isSaving, setIsSaving] = useState(false);
  const isBoolean = isBooleanVariable(quest.variable_name);

  const handleSave = async () => {
    const newValue = parseInt(editValue, 10);
    if (isNaN(newValue)) return;
    if (newValue === quest.current_value) return;

    setIsSaving(true);
    try {
      await onUpdate(quest.variable_name, newValue);
    } finally {
      setIsSaving(false);
    }
  };

  const handleBooleanToggle = async (checked: boolean) => {
    const newValue = checked ? 1 : 0;
    if (newValue === quest.current_value) return;

    setIsSaving(true);
    setEditValue(String(newValue));
    try {
      await onUpdate(quest.variable_name, newValue);
    } finally {
      setIsSaving(false);
    }
  };

  const getStatusBadge = () => {
    if (quest.is_completed) {
      return (
        <Badge variant="default" className="bg-green-600 text-white">
          <CheckCircle2 className="w-3 h-3 mr-1" />
          {t('gameState.quests.status.completed')}
        </Badge>
      );
    }
    if (quest.is_active) {
      return (
        <Badge variant="default" className="bg-blue-600 text-white">
          <Clock className="w-3 h-3 mr-1" />
          {t('gameState.quests.status.active')}
        </Badge>
      );
    }
    return (
      <Badge variant="outline">
        <HelpCircle className="w-3 h-3 mr-1" />
        {t('gameState.quests.status.unknown')}
      </Badge>
    );
  };

  const getConfidenceBadge = () => {
    // Only show confidence badge for medium (when there's a mapping with moderate confidence)
    // High confidence shows no badge (it's the expected state)
    // Low confidence shows no badge (it's the default when no mapping exists - not useful to show everywhere)
    if (quest.confidence === 'medium') {
      return (
        <Badge variant="outline" className="text-yellow-600 border-yellow-600 text-xs">
          {t('gameState.quests.confidence.medium')}
        </Badge>
      );
    }
    return null;
  };

  const rawQuestName = quest.quest_info?.quest_name || quest.variable_name;
  const categoryName = quest.quest_info?.category_name || '';
  // Strip category prefix from quest name if it starts with "Category: " to avoid redundancy
  const categoryPrefix = categoryName ? `${categoryName}: ` : '';
  const questName = rawQuestName.startsWith(categoryPrefix)
    ? rawQuestName.substring(categoryPrefix.length)
    : rawQuestName;
  const stageText = quest.quest_info?.current_stage_text || '';

  return (
    <Card className="mb-2">
      <div
        className="p-4 cursor-pointer hover:bg-[rgb(var(--color-bg-tertiary))] transition-colors"
        onClick={onToggle}
      >
        <div className="flex items-start justify-between">
          <div className="flex items-start gap-3 flex-1 min-w-0">
            <BookOpen className="w-5 h-5 text-[rgb(var(--color-primary))] flex-shrink-0 mt-0.5" />
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2 flex-wrap">
                <span className="font-medium text-[rgb(var(--color-text-primary))]">
                  {display(cleanDisplayName(questName))}
                </span>
                {getStatusBadge()}
              </div>
              {stageText && (
                <div className="text-sm text-[rgb(var(--color-text-secondary))] mt-1 whitespace-pre-wrap">
                  &ldquo;{stageText}&rdquo;
                </div>
              )}
            </div>
          </div>
          <div className="flex items-center gap-2 ml-2">
            {isExpanded ? (
              <ChevronUp className="w-5 h-5 text-[rgb(var(--color-text-muted))]" />
            ) : (
              <ChevronDown className="w-5 h-5 text-[rgb(var(--color-text-muted))]" />
            )}
          </div>
        </div>
      </div>

      {isExpanded && (
        <CardContent className="pt-0 border-t border-[rgb(var(--color-border))]">
          <div className="grid grid-cols-2 gap-4 mt-4 text-sm">
            <div>
              <span className="text-[rgb(var(--color-text-muted))]">
                {t('gameState.quests.card.variable')}:
              </span>
              <span className="ml-2 font-mono text-[rgb(var(--color-text-secondary))]">
                {quest.variable_name}
              </span>
            </div>
            <div>
              <span className="text-[rgb(var(--color-text-muted))]">
                {t('gameState.quests.card.source')}:
              </span>
              <span className="ml-2 text-[rgb(var(--color-text-secondary))]">
                {quest.source}
              </span>
            </div>
            {quest.quest_info?.xp ? (
              <div>
                <span className="text-[rgb(var(--color-text-muted))]">
                  {t('gameState.quests.card.xp')}:
                </span>
                <span className="ml-2 text-[rgb(var(--color-text-secondary))]">
                  {formatNumber(quest.quest_info.xp)}
                </span>
              </div>
            ) : null}
            <div className="flex items-center gap-2">
              {getConfidenceBadge()}
            </div>
          </div>

          <div className="mt-4">
            <label className="text-sm text-[rgb(var(--color-text-muted))] block mb-2">
              {t('gameState.quests.card.questStage')}:
            </label>

            {isBoolean ? (
              <div className="flex items-center gap-3">
                <Checkbox
                  checked={quest.current_value === 1}
                  onCheckedChange={handleBooleanToggle}
                  disabled={isSaving}
                  className="h-5 w-5"
                />
                <span className="text-sm text-[rgb(var(--color-text-secondary))]">
                  {quest.current_value === 1 ? t('common.yes') : t('common.no')}
                </span>
                {isSaving && (
                  <span className="text-sm text-[rgb(var(--color-text-muted))]">
                    {t('actions.saving')}
                  </span>
                )}
              </div>
            ) : quest.known_values.length > 0 ? (
              <select
                value={editValue}
                onChange={(e) => setEditValue(e.target.value)}
                className="w-full p-2 rounded border border-[rgb(var(--color-border))] bg-[rgb(var(--color-bg-secondary))] text-[rgb(var(--color-text-primary))]"
              >
                <option value={quest.current_value}>
                  {quest.current_value} - {t('gameState.quests.card.currentValue')}
                </option>
                {quest.known_values
                  .filter((kv) => kv.value !== quest.current_value)
                  .map((kv, idx) => (
                    <option key={`${kv.value}-${idx}`} value={kv.value}>
                      {kv.value} - {kv.description.substring(0, 50)}
                      {kv.is_completed ? ` [${t('gameState.quests.status.completed')}]` : ''}
                    </option>
                  ))}
              </select>
            ) : (
              <Input
                type="number"
                value={editValue}
                onChange={(e) => setEditValue(e.target.value)}
                className="max-w-[150px]"
              />
            )}

            {!isBoolean && (
              <div className="flex items-center gap-2 mt-3">
                <Button
                  onClick={handleSave}
                  disabled={isSaving || parseInt(editValue, 10) === quest.current_value}
                  size="sm"
                >
                  {isSaving ? t('actions.saving') : t('gameState.quests.card.apply')}
                </Button>
              </div>
            )}
          </div>
        </CardContent>
      )}
    </Card>
  );
}

interface UnmappedVariableRowProps {
  variable: UnmappedVariableData;
  onUpdate: (variableName: string, value: number) => Promise<void>;
  t: (key: string) => string;
}

function UnmappedVariableRow({ variable, onUpdate, t }: UnmappedVariableRowProps) {
  const [editValue, setEditValue] = useState<string>(String(variable.current_value));
  const [isSaving, setIsSaving] = useState(false);
  const isBoolean = isBooleanVariable(variable.variable_name);

  const handleSave = async () => {
    const newValue = parseInt(editValue, 10);
    if (isNaN(newValue)) return;
    if (newValue === variable.current_value) return;

    setIsSaving(true);
    try {
      await onUpdate(variable.variable_name, newValue);
    } finally {
      setIsSaving(false);
    }
  };

  const handleBooleanToggle = async (checked: boolean) => {
    const newValue = checked ? 1 : 0;
    if (newValue === variable.current_value) return;

    setIsSaving(true);
    setEditValue(String(newValue));
    try {
      await onUpdate(variable.variable_name, newValue);
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <tr className="border-b border-[rgb(var(--color-border))]">
      <td className="p-3">
        <div className="font-mono text-sm">{variable.variable_name}</div>
        <div className="text-xs text-[rgb(var(--color-text-muted))]">
          {cleanDisplayName(variable.display_name)}
        </div>
      </td>
      <td className="p-3 text-sm text-[rgb(var(--color-text-muted))]">
        {cleanDisplayName(variable.category)}
      </td>
      <td className="p-3">
        {isBoolean ? (
          <div className="flex items-center gap-3">
            <Checkbox
              checked={variable.current_value === 1}
              onCheckedChange={handleBooleanToggle}
              disabled={isSaving}
              className="h-5 w-5"
            />
            <span className="text-sm text-[rgb(var(--color-text-secondary))]">
              {variable.current_value === 1 ? t('common.yes') : t('common.no')}
            </span>
            {isSaving && (
              <span className="text-sm text-[rgb(var(--color-text-muted))]">...</span>
            )}
          </div>
        ) : (
          <div className="flex items-center gap-2">
            <Input
              type="number"
              value={editValue}
              onChange={(e) => setEditValue(e.target.value)}
              className="w-24"
            />
            <Button
              onClick={handleSave}
              disabled={isSaving || parseInt(editValue, 10) === variable.current_value}
              size="sm"
              variant="outline"
            >
              {isSaving ? '...' : t('gameState.quests.card.apply')}
            </Button>
          </div>
        )}
      </td>
    </tr>
  );
}

export default function QuestsEditor() {
  const t = useTranslations();
  const { character } = useCharacterContext();
  const { showToast } = useToast();

  const [quests, setQuests] = useState<EnrichedQuestData[]>([]);
  const [unmappedVariables, setUnmappedVariables] = useState<UnmappedVariableData[]>([]);
  const [stats, setStats] = useState<QuestStats>({ total: 0, completed: 0, active: 0, unmapped: 0 });
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [tabMode, setTabMode] = useState<TabMode>('quests');
  const [filter, setFilter] = useState<QuestFilter>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [expandedQuests, setExpandedQuests] = useState<Set<string>>(new Set());
  const [expandedCategories, setExpandedCategories] = useState<Set<string>>(new Set());

  useEffect(() => {
    if (character?.id) {
      loadEnrichedQuests();
    }
  }, [character?.id]);

  const loadEnrichedQuests = async () => {
    if (!character?.id) return;

    setIsLoading(true);
    setError(null);

    try {
      const data = await gameStateAPI.getEnrichedQuests(character.id);
      setQuests(data.quests);
      setUnmappedVariables(data.unmapped_variables);
      setStats(data.stats);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load quests';
      setError(message);
    } finally {
      setIsLoading(false);
    }
  };

  const handleUpdateVariable = async (variableName: string, value: number) => {
    if (!character?.id) return;

    try {
      await gameStateAPI.updateQuestVariable(character.id, variableName, value, 'int');
      showToast(`${t('gameState.quests.toast.updateSuccess')}: ${variableName} = ${value}`, 'success');
      await loadEnrichedQuests();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to update quest';
      showToast(`${t('gameState.quests.toast.updateError')}: ${message}`, 'error');
    }
  };

  const toggleQuestExpanded = (variableName: string) => {
    setExpandedQuests((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(variableName)) {
        newSet.delete(variableName);
      } else {
        newSet.add(variableName);
      }
      return newSet;
    });
  };

  const toggleCategoryExpanded = (category: string) => {
    setExpandedCategories((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(category)) {
        newSet.delete(category);
      } else {
        newSet.add(category);
      }
      return newSet;
    });
  };

  const filteredQuests = useMemo(() => {
    let result = quests;

    if (filter === 'active') {
      result = result.filter((q) => q.is_active && !q.is_completed);
    } else if (filter === 'completed') {
      result = result.filter((q) => q.is_completed);
    }

    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      result = result.filter(
        (q) =>
          q.variable_name.toLowerCase().includes(query) ||
          q.quest_info?.quest_name?.toLowerCase().includes(query) ||
          q.quest_info?.category_name?.toLowerCase().includes(query) ||
          q.quest_info?.current_stage_text?.toLowerCase().includes(query)
      );
    }

    return result;
  }, [quests, filter, searchQuery]);

  // Group quests by category (Act 10, Act 11, Quest Progress, etc.)
  const groupedQuests = useMemo(() => {
    const groups: Record<string, EnrichedQuestData[]> = {};

    const isKnownCategory = (cat: string): boolean => {
      if (cat === 'Tutorial' || cat === 'Miscellaneous' || cat === 'Other') return true;
      if (cat.startsWith('0_') || cat === 'Act 0' || cat === 'Act 00') return true;
      if (/^Act \d+/.test(cat)) return true;
      if (cat === 'Quest Progress' || cat === 'Campaign Variables') return true;
      return false;
    };

    filteredQuests.forEach((quest) => {
      let category = quest.quest_info?.category_name || 'Other';

      // Group tutorial categories (0_*) into "Tutorial"
      if (category.startsWith('0_') || category === 'Act 0' || category === 'Act 00') {
        category = 'Tutorial';
      }
      // Group orphan categories into "Miscellaneous"
      else if (!isKnownCategory(category)) {
        category = 'Miscellaneous';
      }

      if (!groups[category]) {
        groups[category] = [];
      }
      groups[category].push(quest);
    });

    // Sort categories: Tutorial first, then Acts (numerically), then known, then Miscellaneous last
    const sortedCategories = Object.keys(groups).sort((a, b) => {
      // Tutorial always first
      if (a === 'Tutorial') return -1;
      if (b === 'Tutorial') return 1;

      // Miscellaneous always last
      if (a === 'Miscellaneous') return 1;
      if (b === 'Miscellaneous') return -1;

      const actMatchA = a.match(/^Act (\d+)/);
      const actMatchB = b.match(/^Act (\d+)/);

      if (actMatchA && actMatchB) {
        return parseInt(actMatchA[1], 10) - parseInt(actMatchB[1], 10);
      }
      if (actMatchA) return -1;
      if (actMatchB) return 1;
      return a.localeCompare(b);
    });

    return sortedCategories.map((category) => ({
      category,
      quests: groups[category],
    }));
  }, [filteredQuests]);

  const filteredUnmapped = useMemo(() => {
    if (!searchQuery.trim()) return unmappedVariables;

    const query = searchQuery.toLowerCase();
    return unmappedVariables.filter(
      (v) =>
        v.variable_name.toLowerCase().includes(query) ||
        v.display_name.toLowerCase().includes(query)
    );
  }, [unmappedVariables, searchQuery]);

  if (isLoading) {
    return (
      <Card className="p-8">
        <div className="flex items-center justify-center gap-3">
          <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-[rgb(var(--color-primary))]" />
          <span className="text-[rgb(var(--color-text-muted))]">
            {t('gameState.quests.loading')}
          </span>
        </div>
      </Card>
    );
  }

  if (error) {
    return (
      <Card className="p-8">
        <div className="flex items-center gap-3 text-red-500">
          <AlertCircle className="w-5 h-5" />
          <span>{error}</span>
        </div>
      </Card>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-4 flex-wrap">
        <div className="flex items-center gap-2 text-sm">
          <span className="text-[rgb(var(--color-text-muted))]">{t('gameState.quests.stats.total')}:</span>
          <span className="font-medium">{formatNumber(stats.total)}</span>
          <span className="mx-2 text-[rgb(var(--color-text-muted))]">|</span>
          <span className="text-green-600">{t('gameState.quests.stats.completed')}: {stats.completed}</span>
          <span className="mx-2 text-[rgb(var(--color-text-muted))]">|</span>
          <span className="text-blue-600">{t('gameState.quests.stats.active')}: {stats.active}</span>
          <span className="mx-2 text-[rgb(var(--color-text-muted))]">|</span>
          <span className="text-[rgb(var(--color-text-muted))]">{t('gameState.quests.stats.unmapped')}: {stats.unmapped}</span>
        </div>
      </div>

      <Tabs value={tabMode} onValueChange={(v) => setTabMode(v as TabMode)}>
        <div className="flex items-center justify-between gap-4 flex-wrap">
          <TabsList>
            <TabsTrigger value="quests">
              <BookOpen className="w-4 h-4 mr-2" />
              {t('gameState.quests.tabs.mapped')} ({quests.length})
            </TabsTrigger>
            <TabsTrigger value="unmapped">
              <HelpCircle className="w-4 h-4 mr-2" />
              {t('gameState.quests.tabs.technical') || 'Technical Variables'} ({unmappedVariables.length})
            </TabsTrigger>
          </TabsList>

          <div className="flex items-center gap-2">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[rgb(var(--color-text-muted))]" />
              <Input
                placeholder={t('gameState.quests.filters.searchPlaceholder')}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="pl-9 w-64"
              />
            </div>
          </div>
        </div>

        <TabsContent value="quests" className="mt-4">
          <div className="flex items-center gap-2 mb-4">
            <Filter className="w-4 h-4 text-[rgb(var(--color-text-muted))]" />
            <Button
              variant={filter === 'all' ? 'primary' : 'outline'}
              size="sm"
              onClick={() => setFilter('all')}
            >
              {t('gameState.quests.filters.all')}
            </Button>
            <Button
              variant={filter === 'active' ? 'primary' : 'outline'}
              size="sm"
              onClick={() => setFilter('active')}
            >
              {t('gameState.quests.filters.active')}
            </Button>
            <Button
              variant={filter === 'completed' ? 'primary' : 'outline'}
              size="sm"
              onClick={() => setFilter('completed')}
            >
              {t('gameState.quests.filters.completed')}
            </Button>
            {(filter !== 'all' || searchQuery) && (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  setFilter('all');
                  setSearchQuery('');
                }}
              >
                {t('gameState.quests.filters.clearFilters')}
              </Button>
            )}
          </div>

          {filteredQuests.length === 0 ? (
            <Card className="p-8">
              <div className="text-center text-[rgb(var(--color-text-muted))]">
                {t('gameState.quests.noQuests')}
              </div>
            </Card>
          ) : (
            <div className="space-y-3">
              {groupedQuests.map(({ category, quests: categoryQuests }) => {
                const isExpanded = expandedCategories.has(category);
                const completedCount = categoryQuests.filter((q) => q.is_completed).length;
                const activeCount = categoryQuests.filter((q) => q.is_active && !q.is_completed).length;

                return (
                  <Card key={category} className="overflow-hidden">
                    <button
                      onClick={() => toggleCategoryExpanded(category)}
                      className="w-full flex items-center justify-between p-4 hover:bg-[rgb(var(--color-bg-secondary))] transition-colors text-left"
                    >
                      <div className="flex items-center gap-3">
                        {isExpanded ? (
                          <ChevronDown className="w-5 h-5 text-[rgb(var(--color-text-muted))]" />
                        ) : (
                          <ChevronUp className="w-5 h-5 text-[rgb(var(--color-text-muted))] rotate-180" />
                        )}
                        <span className="font-medium text-lg">{cleanDisplayName(category)}</span>
                        <Badge variant="outline" className="text-xs">
                          {categoryQuests.length} {categoryQuests.length === 1 ? 'quest' : 'quests'}
                        </Badge>
                      </div>
                      <div className="flex items-center gap-2">
                        {completedCount > 0 && (
                          <Badge variant="default" className="bg-green-600 text-white text-xs">
                            <CheckCircle2 className="w-3 h-3 mr-1" />
                            {completedCount}
                          </Badge>
                        )}
                        {activeCount > 0 && (
                          <Badge variant="default" className="bg-blue-600 text-white text-xs">
                            <Clock className="w-3 h-3 mr-1" />
                            {activeCount}
                          </Badge>
                        )}
                      </div>
                    </button>
                    {isExpanded && (
                      <div className="border-t border-[rgb(var(--color-border))] p-3 space-y-2">
                        {categoryQuests.map((quest) => (
                          <QuestCard
                            key={quest.variable_name}
                            quest={quest}
                            isExpanded={expandedQuests.has(quest.variable_name)}
                            onToggle={() => toggleQuestExpanded(quest.variable_name)}
                            onUpdate={handleUpdateVariable}
                            t={t}
                          />
                        ))}
                      </div>
                    )}
                  </Card>
                );
              })}
            </div>
          )}
        </TabsContent>

        <TabsContent value="unmapped" className="mt-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-base flex items-center gap-2">
                <HelpCircle className="w-4 h-4" />
                {t('gameState.quests.technical.title') || 'Technical Variables'}
              </CardTitle>
              <p className="text-sm text-[rgb(var(--color-text-muted))]">
                {t('gameState.quests.technical.description') || 'These variables track internal game state and do not have associated journal entries.'}
              </p>
            </CardHeader>
            <CardContent>
              {filteredUnmapped.length === 0 ? (
                <div className="text-center text-[rgb(var(--color-text-muted))] py-4">
                  {t('gameState.quests.noVariables')}
                </div>
              ) : (
                <div className="overflow-x-auto">
                  <table className="w-full">
                    <thead>
                      <tr className="border-b border-[rgb(var(--color-border))]">
                        <th className="text-left p-3 text-sm font-medium text-[rgb(var(--color-text-muted))]">
                          {t('gameState.quests.card.variable')}
                        </th>
                        <th className="text-left p-3 text-sm font-medium text-[rgb(var(--color-text-muted))]">
                          {t('gameState.quests.filters.category')}
                        </th>
                        <th className="text-left p-3 text-sm font-medium text-[rgb(var(--color-text-muted))]">
                          {t('gameState.quests.card.value')}
                        </th>
                      </tr>
                    </thead>
                    <tbody>
                      {filteredUnmapped.map((variable) => (
                        <UnmappedVariableRow
                          key={variable.variable_name}
                          variable={variable}
                          onUpdate={handleUpdateVariable}
                          t={t}
                        />
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
