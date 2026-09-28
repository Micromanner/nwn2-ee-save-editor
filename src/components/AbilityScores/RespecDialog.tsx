import { useState, useMemo, useCallback, useEffect } from 'react';
import { Button, ProgressBar, Tooltip } from '@blueprintjs/core';
import { GiAnticlockwiseRotation } from 'react-icons/gi';
import { GameIcon } from '../shared/GameIcon';
import { T } from '../theme';
import { mod, ParchmentDialog } from '../shared';
import { useTranslations } from '@/hooks/useTranslations';
import type { AbilityScores, PointBuyState } from '@/lib/bindings';

type AbilityKey = 'Str' | 'Dex' | 'Con' | 'Int' | 'Wis' | 'Cha';

const ABILITY_NAME_KEYS: Record<AbilityKey, string> = {
  Str: 'abilityScores.names.strength', Dex: 'abilityScores.names.dexterity', Con: 'abilityScores.names.constitution',
  Int: 'abilityScores.names.intelligence', Wis: 'abilityScores.names.wisdom', Cha: 'abilityScores.names.charisma',
};

const ABILITY_SHORT: Record<AbilityKey, string> = {
  Str: 'STR', Dex: 'DEX', Con: 'CON', Int: 'INT', Wis: 'WIS', Cha: 'CHA',
};

const KEYS: AbilityKey[] = ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha'];
const POINT_COSTS = [0, 1, 2, 3, 4, 5, 6, 8, 10, 13, 16];
const BUDGET = 32;
const MIN = 8;
const MAX = 18;

function cost(score: number): number {
  if (score <= 8) return 0;
  if (score >= 18) return 16;
  return POINT_COSTS[score - 8];
}

interface RespecDialogProps {
  isOpen: boolean;
  onClose: () => void;
  pointBuyState: PointBuyState | null;
  onApply?: (scores: AbilityScores) => Promise<void>;
}

export function RespecDialog({ isOpen, onClose, pointBuyState, onApply }: RespecDialogProps) {
  const t = useTranslations();
  const [scores, setScores] = useState<Record<AbilityKey, number>>({
    Str: 10, Dex: 10, Con: 10, Int: 10, Wis: 10, Cha: 10,
  });
  const [isApplying, setIsApplying] = useState(false);

  useEffect(() => {
    if (isOpen && pointBuyState?.starting_scores) {
      setScores({
        Str: pointBuyState.starting_scores.Str,
        Dex: pointBuyState.starting_scores.Dex,
        Con: pointBuyState.starting_scores.Con,
        Int: pointBuyState.starting_scores.Int,
        Wis: pointBuyState.starting_scores.Wis,
        Cha: pointBuyState.starting_scores.Cha,
      });
    }
  }, [isOpen, pointBuyState]);

  const initialScores = useMemo(() => {
    if (pointBuyState?.starting_scores) {
      const s = pointBuyState.starting_scores;
      return { Str: s.Str, Dex: s.Dex, Con: s.Con, Int: s.Int, Wis: s.Wis, Cha: s.Cha };
    }
    return { Str: 10, Dex: 10, Con: 10, Int: 10, Wis: 10, Cha: 10 };
  }, [pointBuyState]);

  const totalCost = useMemo(() => KEYS.reduce((s, k) => s + cost(scores[k]), 0), [scores]);
  const remaining = BUDGET - totalCost;
  const hasChanged = useMemo(() => KEYS.some(k => scores[k] !== initialScores[k]), [scores, initialScores]);

  const change = useCallback((key: AbilityKey, delta: number) => {
    const next = scores[key] + delta;
    if (next < MIN || next > MAX) return;
    const nextScores = { ...scores, [key]: next };
    if (delta > 0 && KEYS.reduce((s, k) => s + cost(nextScores[k]), 0) > BUDGET) return;
    setScores(nextScores);
  }, [scores]);

  const reset = () => setScores({ Str: 8, Dex: 8, Con: 8, Int: 8, Wis: 8, Cha: 8 });

  return (
    <ParchmentDialog
      isOpen={isOpen}
      onClose={onClose}
      title={t('abilityScores.pointBuy.title')}
      width={480}
      footerActions={
        <Button
          text={isApplying ? t('actions.saving') : t('actions.apply')}
          intent={remaining === 0 && hasChanged && !isApplying ? 'primary' : 'none'}
          disabled={remaining !== 0 || !hasChanged || isApplying}
          onClick={async () => {
            setIsApplying(true);
            try {
              await onApply?.(scores as AbilityScores);
              onClose();
            } finally {
              setIsApplying(false);
            }
          }}
          style={remaining === 0 && hasChanged && !isApplying ? { background: T.accent } : undefined}
        />
      }
      footerLeft={
        <Button text={t('abilityScores.pointBuy.reset')} icon={<GameIcon icon={GiAnticlockwiseRotation} size={14} />} minimal onClick={reset} style={{ color: T.textMuted }} />
      }
    >
      <div className="t-body" style={{
        padding: '8px 12px', marginBottom: 16, borderRadius: 4,
        background: '#fde8e8', border: `1px solid ${T.negative}30`,
        color: T.negative,
      }}>
        {t('abilityScores.pointBuy.warning')}
      </div>

      <div style={{ marginBottom: 16 }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 6 }}>
          <span className="t-semibold" style={{ color: T.textMuted }}>
            {t('abilityScores.pointBuy.pointsUsed')}: <strong style={{ color: T.text }}>{totalCost}</strong> / {BUDGET}
          </span>
          <span className="t-bold" style={{
            color: remaining === 0 ? T.positive : remaining < 0 ? T.negative : T.accent,
          }}>
            {remaining} {t('abilityScores.pointBuy.remaining')}
          </span>
        </div>
        <ProgressBar
          value={totalCost / BUDGET}
          intent={remaining === 0 ? 'success' : remaining < 0 ? 'danger' : 'none'}
          stripes={false} animate={false}
          style={{ height: 6 }}
        />
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
        {KEYS.map(key => {
          const score = scores[key];
          const modifier = Math.floor((score - 10) / 2);
          const pts = cost(score);
          const nextCost = cost(score + 1) - pts;
          return (
            <div key={key} style={{
              display: 'flex', alignItems: 'center', gap: 8,
              padding: '8px 12px', borderRadius: 4,
              background: T.surfaceAlt, border: `1px solid ${T.borderLight}`,
            }}>
              <span className="t-bold" style={{ width: 36, color: T.accent }}>{ABILITY_SHORT[key]}</span>
              <span className="t-medium" style={{ flex: 1, color: T.text }}>{t(ABILITY_NAME_KEYS[key])}</span>

              <Button
                icon="minus" small minimal
                disabled={score <= MIN}
                onClick={() => change(key, -1)}
              />
              <span className="t-xl t-bold" style={{
                width: 32, textAlign: 'center',
                color: T.text,
              }}>
                {score}
              </span>
              <Tooltip
                content={score >= MAX ? t('abilityScores.pointBuy.maxReached') : t('abilityScores.pointBuy.nextCost', { points: nextCost })}
                placement="top"
              >
                <Button
                  icon="plus" small minimal
                  disabled={score >= MAX || remaining < nextCost}
                  onClick={() => change(key, 1)}
                />
              </Tooltip>

              <span className="t-semibold" style={{
                width: 32, textAlign: 'center',
                color: modifier > 0 ? T.positive : modifier < 0 ? T.negative : T.textMuted,
              }}>
                {mod(modifier)}
              </span>
              <span className="t-sm" style={{
                minWidth: 64, textAlign: 'right',
                color: T.textMuted,
              }}>
                {t('abilityScores.pointBuy.spent', { points: pts })}
              </span>
            </div>
          );
        })}
      </div>
    </ParchmentDialog>
  );
}
