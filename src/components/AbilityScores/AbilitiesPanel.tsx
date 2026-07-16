import { useEffect } from 'react';
import { Button, Card, Elevation, HTMLTable, NonIdealState, Spinner } from '@blueprintjs/core';
import { GiVisoredHelm, GiBrokenShield, GiAnticlockwiseRotation } from 'react-icons/gi';
import { GameIcon } from '../shared/GameIcon';
import { T } from '../theme';
import { ModCell, mod, StepInput } from '../shared';
import { RespecDialog } from './RespecDialog';
import { useSubsystem, useCharacterContext } from '@/contexts/CharacterContext';
import { useAbilityScores } from '@/hooks/useAbilityScores';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { useTranslations } from '@/hooks/useTranslations';
import { CharacterStateAPI } from '@/lib/api/character-state';
import { useState } from 'react';
import type { AbilityScores } from '@/lib/bindings';

function SectionLabel({ children }: { children: string }) {
  return (
    <div className="t-section" style={{ color: T.accent, marginBottom: 8 }}>
      {children}
    </div>
  );
}

type AbilityShortName = 'STR' | 'DEX' | 'CON' | 'INT' | 'WIS' | 'CHA';

const ABILITY_ORDER: AbilityShortName[] = ['STR', 'DEX', 'CON', 'INT', 'WIS', 'CHA'];

export function AbilitiesPanel() {
  const t = useTranslations();
  const { character } = useCharacterContext();
  const { handleError } = useErrorHandler();
  const [isRespecOpen, setIsRespecOpen] = useState(false);

  const abilitiesSubsystem = useSubsystem('abilityScores');
  const savesSubsystem = useSubsystem('saves');
  const combatSubsystem = useSubsystem('combat');

  const { abilityScores, stats, pointSummary, updateAbilityScore, updateStats } = useAbilityScores(
    abilitiesSubsystem.data,
    { combat: combatSubsystem.data, saves: savesSubsystem.data }
  );

  useEffect(() => {
    if (character?.id) {
      if (!abilitiesSubsystem.data && !abilitiesSubsystem.isLoading) {
        abilitiesSubsystem.load();
      }
      if (!savesSubsystem.data && !savesSubsystem.isLoading) {
        savesSubsystem.load();
      }
      if (!combatSubsystem.data && !combatSubsystem.isLoading) {
        combatSubsystem.load();
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [character?.id]);

  const isLoading = abilitiesSubsystem.isLoading || savesSubsystem.isLoading || combatSubsystem.isLoading;
  const hasError = abilitiesSubsystem.error || savesSubsystem.error || combatSubsystem.error;

  if (!character) {
    return (
      <div style={{ padding: 32 }}>
        <NonIdealState
          icon={<GameIcon icon={GiVisoredHelm} size={40} />}
          title={t('common.noCharacterLoaded')}
          description={t('common.loadSaveToView', { section: t('abilityScores.abilityScores').toLowerCase() })}
        />
      </div>
    );
  }

  if (isLoading && !abilitiesSubsystem.data) {
    return (
      <div style={{ padding: 32 }}>
        <NonIdealState icon={<Spinner />} title={t('common.loading')} />
      </div>
    );
  }

  if (hasError && !abilitiesSubsystem.data) {
    return (
      <div style={{ padding: 32 }}>
        <NonIdealState
          icon={<GameIcon icon={GiBrokenShield} size={40} />}
          title={t('common.failedToLoad', { section: t('abilityScores.abilityScores').toLowerCase() })}
          description={abilitiesSubsystem.error ?? savesSubsystem.error ?? combatSubsystem.error ?? undefined}
        />
      </div>
    );
  }

  const abilitiesData = abilitiesSubsystem.data;
  const savesData = savesSubsystem.data;
  const combatData = combatSubsystem.data;

  const spent = pointSummary?.total_spent ?? 0;
  const available = pointSummary?.available ?? 0;

  const saveRows = [
    {
      key: 'fortitude',
      label: t('abilityScores.fortitude'),
      data: savesData?.saves?.fortitude,
    },
    {
      key: 'reflex',
      label: t('abilityScores.reflex'),
      data: savesData?.saves?.reflex,
    },
    {
      key: 'will',
      label: t('abilityScores.will'),
      data: savesData?.saves?.will,
    },
  ];

  const acBreakdown = combatData?.armor_class?.breakdown;
  const acTotal = combatData?.armor_class?.total ?? 10;
  const touchTotal = combatData?.armor_class?.touch ?? 10;
  const flatFootedTotal = combatData?.armor_class?.flat_footed ?? 10;

  const acRows = [
    { name: 'AC',          base: acBreakdown?.base ?? 10, dex: acBreakdown?.dex ?? 0, armor: acBreakdown?.armor ?? 0, shield: acBreakdown?.shield ?? 0, natural: acBreakdown?.natural ?? 0, deflection: acBreakdown?.deflection ?? 0, size: acBreakdown?.size ?? 0, misc: acBreakdown?.misc ?? 0, total: acTotal },
    { name: 'Touch',       base: acBreakdown?.base ?? 10, dex: acBreakdown?.dex ?? 0, armor: 0,                       shield: 0,                        natural: 0,                         deflection: acBreakdown?.deflection ?? 0, size: acBreakdown?.size ?? 0, misc: acBreakdown?.misc ?? 0, total: touchTotal },
    { name: 'Flat-Footed', base: acBreakdown?.base ?? 10, dex: 0,                     armor: acBreakdown?.armor ?? 0, shield: acBreakdown?.shield ?? 0, natural: acBreakdown?.natural ?? 0, deflection: acBreakdown?.deflection ?? 0, size: acBreakdown?.size ?? 0, misc: acBreakdown?.misc ?? 0, total: flatFootedTotal },
  ];

  const initDex = combatData?.initiative?.dex ?? stats.initiative.dexMod ?? 0;
  const initFeat = combatData?.initiative?.feat ?? stats.initiative.feats ?? 0;
  const initTotal = combatData?.initiative?.total ?? stats.initiative.total ?? 0;

  const handleAbilityChange = async (index: number, value: number) => {
    try {
      await updateAbilityScore(index, value);
    } catch (err) {
      handleError(err);
    }
  };

  const handleSaveMiscChange = async (saveKey: 'fortitude' | 'reflex' | 'will', value: number) => {
    try {
      await updateStats({ [saveKey]: { ...stats[saveKey], base: value } });
    } catch (err) {
      handleError(err);
    }
  };

  const handleNaturalAcChange = async (value: number) => {
    try {
      await CharacterStateAPI.updateNaturalArmor(value);
      await combatSubsystem.load();
    } catch (err) {
      handleError(err);
    }
  };

  const handleRespecApply = async (scores: AbilityScores) => {
    try {
      await CharacterStateAPI.applyPointBuy(scores);
      await abilitiesSubsystem.load();
      await combatSubsystem.load();
      await savesSubsystem.load();
    } catch (err) {
      handleError(err);
    }
  };

  return (
    <div style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 12 }}>

      <Card elevation={Elevation.ONE} style={{ padding: 0, background: T.surface, overflow: 'hidden' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 16, padding: '10px 16px', borderBottom: `1px solid ${T.borderLight}` }}>
          <span className="t-lg" style={{ color: T.textMuted }}>
            {t('abilityScores.pointsSpent')}: <strong style={{ color: T.text }}>{spent}</strong>
          </span>
          <span className="t-lg" style={{ color: T.textMuted }}>
            {t('abilityScores.availablePoints')}: <strong style={{ color: T.accent }}>{available}</strong>
          </span>
          <Button icon={<GameIcon icon={GiAnticlockwiseRotation} size={14} />} text={t('abilityScores.pointBuy.button')} small minimal style={{ color: T.textMuted }} onClick={() => setIsRespecOpen(true)} />
        </div>
        <div style={{ padding: '12px 16px 16px' }}>
          <SectionLabel>{t('abilityScores.abilityScores')}</SectionLabel>
          <HTMLTable compact striped bordered interactive style={{ width: '100%', tableLayout: 'fixed' }}>
            <colgroup>
              <col />
              <col style={{ width: 144 }} />
              <col style={{ width: 72 }} />
              <col style={{ width: 72 }} />
              <col style={{ width: 72 }} />
              <col style={{ width: 80 }} />
              <col style={{ width: 72 }} />
            </colgroup>
            <thead>
              <tr>
                <th>{t('abilityScores.ability')}</th>
                <th style={{ textAlign: 'center' }}>{t('abilityScores.base')}</th>
                <th style={{ textAlign: 'center' }}>{t('abilityScores.level')}</th>
                <th style={{ textAlign: 'center' }}>{t('abilityScores.racial')}</th>
                <th style={{ textAlign: 'center' }}>{t('abilityScores.equip')}</th>
                <th style={{ textAlign: 'center' }}>{t('abilityScores.effective')}</th>
                <th style={{ textAlign: 'center' }}>Mod</th>
              </tr>
            </thead>
            <tbody>
              {abilityScores.length > 0 ? (
                abilityScores.map((a, i) => {
                  const shortName = ABILITY_ORDER[i];
                  const levelBonus = a.breakdown?.levelUp ?? 0;
                  const racialBonus = a.breakdown?.racial ?? 0;
                  const equipBonus = a.breakdown?.equipment ?? 0;

                  return (
                    <tr key={a.shortName}>
                      <td>
                        <strong style={{ color: T.text }}>{shortName}</strong>
                        <span className="t-base" style={{ marginLeft: 6, color: T.textMuted }}>{a.name}</span>
                      </td>
                      <td style={{ textAlign: 'center' }}>
                        <StepInput
                          value={a.baseValue ?? 10}
                          onValueChange={(v) => handleAbilityChange(i, v)}
                          min={3} max={50} width={88}
                        />
                      </td>
                      <td style={{ textAlign: 'center' }}><ModCell value={levelBonus} /></td>
                      <td style={{ textAlign: 'center' }}><ModCell value={racialBonus} /></td>
                      <td style={{ textAlign: 'center' }}><ModCell value={equipBonus} /></td>
                      <td className="t-xl t-bold" style={{ textAlign: 'center', color: T.text }}>{a.value}</td>
                      <td className="t-semibold" style={{
                        textAlign: 'center',
                        color: a.modifier > 0 ? T.positive : a.modifier < 0 ? T.negative : T.textMuted,
                      }}>
                        {mod(a.modifier)}
                      </td>
                    </tr>
                  );
                })
              ) : abilitiesData ? (
                (['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha'] as const).map((key, i) => {
                  const shortNames: Record<string, string> = { Str: 'STR', Dex: 'DEX', Con: 'CON', Int: 'INT', Wis: 'WIS', Cha: 'CHA' };
                  const fullNames: Record<string, string> = {
                    Str: t('abilityScores.strength'), Dex: t('abilityScores.dexterity'),
                    Con: t('abilityScores.constitution'), Int: t('abilityScores.intelligence'),
                    Wis: t('abilityScores.wisdom'), Cha: t('abilityScores.charisma'),
                  };
                  const base = abilitiesData.base_scores?.[key] ?? 10;
                  const effective = abilitiesData.effective_scores?.[key] ?? base;
                  const modifier = Math.floor((effective - 10) / 2);
                  const racial = abilitiesData.racial_modifiers?.[key] ?? 0;
                  const equip = abilitiesData.equipment_modifiers?.[key] ?? 0;
                  const levelIncs = abilitiesData.point_summary?.level_increases ?? [];
                  const abilityIndexMap: Record<string, number> = { Str: 0, Dex: 1, Con: 2, Int: 3, Wis: 4, Cha: 5 };
                  const levelBonus = levelIncs.filter(inc => inc.ability === abilityIndexMap[key]).length;

                  return (
                    <tr key={key}>
                      <td>
                        <strong style={{ color: T.text }}>{shortNames[key]}</strong>
                        <span className="t-base" style={{ marginLeft: 6, color: T.textMuted }}>{fullNames[key]}</span>
                      </td>
                      <td style={{ textAlign: 'center' }}>
                        <StepInput
                          value={base}
                          onValueChange={(v) => handleAbilityChange(i, v)}
                          min={3} max={50} width={88}
                        />
                      </td>
                      <td style={{ textAlign: 'center' }}><ModCell value={levelBonus} /></td>
                      <td style={{ textAlign: 'center' }}><ModCell value={racial} /></td>
                      <td style={{ textAlign: 'center' }}><ModCell value={equip} /></td>
                      <td className="t-xl t-bold" style={{ textAlign: 'center', color: T.text }}>{effective}</td>
                      <td className="t-semibold" style={{
                        textAlign: 'center',
                        color: modifier > 0 ? T.positive : modifier < 0 ? T.negative : T.textMuted,
                      }}>
                        {mod(modifier)}
                      </td>
                    </tr>
                  );
                })
              ) : null}
            </tbody>
          </HTMLTable>
        </div>
      </Card>

      <Card elevation={Elevation.ONE} style={{ padding: '12px 16px 16px', background: T.surface }}>
        <SectionLabel>{t('abilityScores.savingThrowsAndInitiative')}</SectionLabel>
        <HTMLTable compact striped bordered style={{ width: '100%', tableLayout: 'fixed' }}>
          <colgroup>
            <col />
            <col style={{ width: 72 }} />
            <col style={{ width: 72 }} />
            <col style={{ width: 72 }} />
            <col style={{ width: 72 }} />
            <col style={{ width: 72 }} />
            <col style={{ width: 120 }} />
            <col style={{ width: 80 }} />
          </colgroup>
          <thead>
            <tr>
              <th></th>
              <th style={{ textAlign: 'center' }}>Base</th>
              <th style={{ textAlign: 'center' }}>Ability</th>
              <th style={{ textAlign: 'center' }}>Equip</th>
              <th style={{ textAlign: 'center' }}>Feat</th>
              <th style={{ textAlign: 'center' }}>Racial</th>
              <th style={{ textAlign: 'center' }}>Misc</th>
              <th style={{ textAlign: 'center' }}>Total</th>
            </tr>
          </thead>
          <tbody>
            {saveRows.map(s => (
              <tr key={s.key}>
                <td><strong style={{ color: T.text }}>{s.label}</strong></td>
                <td style={{ textAlign: 'center' }}>{mod(s.data?.base ?? 0)}</td>
                <td style={{ textAlign: 'center' }}><ModCell value={s.data?.ability ?? 0} /></td>
                <td style={{ textAlign: 'center' }}><ModCell value={s.data?.equipment ?? 0} /></td>
                <td style={{ textAlign: 'center' }}><ModCell value={(s.data?.feat ?? 0) + (s.data?.class_bonus ?? 0)} /></td>
                <td style={{ textAlign: 'center' }}><ModCell value={s.data?.racial ?? 0} /></td>
                <td style={{ textAlign: 'center' }}>
                  <StepInput
                    value={stats[s.key as 'fortitude' | 'reflex' | 'will'].base}
                    onValueChange={(v) => handleSaveMiscChange(s.key as 'fortitude' | 'reflex' | 'will', v)}
                    min={-35} max={255} width={88}
                  />
                </td>
                <td style={{ textAlign: 'center' }}>
                  <strong className="t-xl" style={{ color: T.text }}>{mod(s.data?.total ?? 0)}</strong>
                </td>
              </tr>
            ))}
            <tr>
              <td><strong style={{ color: T.text }}>{t('overview.initiative')}</strong></td>
              <td />
              <td style={{ textAlign: 'center' }}><ModCell value={initDex} /></td>
              <td />
              <td style={{ textAlign: 'center' }}><ModCell value={initFeat} /></td>
              <td />
              <td />
              <td style={{ textAlign: 'center' }}>
                <strong className="t-xl" style={{ color: T.text }}>{mod(initTotal)}</strong>
              </td>
            </tr>
          </tbody>
        </HTMLTable>
      </Card>

      <Card elevation={Elevation.ONE} style={{ padding: '12px 16px 16px', background: T.surface }}>
        <SectionLabel>{t('overview.armorClass')}</SectionLabel>
        <HTMLTable compact striped bordered style={{ width: '100%', tableLayout: 'fixed' }}>
          <colgroup>
            <col />
            <col style={{ width: 72 }} />
            <col style={{ width: 72 }} />
            <col style={{ width: 72 }} />
            <col style={{ width: 72 }} />
            <col style={{ width: 120 }} />
            <col style={{ width: 72 }} />
            <col style={{ width: 72 }} />
            <col style={{ width: 72 }} />
            <col style={{ width: 80 }} />
          </colgroup>
          <thead>
            <tr>
              <th>{t('abilityScores.type')}</th>
              <th style={{ textAlign: 'center' }}>Base</th>
              <th style={{ textAlign: 'center' }}>DEX</th>
              <th style={{ textAlign: 'center' }}>Armor</th>
              <th style={{ textAlign: 'center' }}>Shield</th>
              <th style={{ textAlign: 'center' }}>Natural</th>
              <th style={{ textAlign: 'center' }}>Deflect</th>
              <th style={{ textAlign: 'center' }}>Size</th>
              <th style={{ textAlign: 'center' }}>Misc</th>
              <th style={{ textAlign: 'center' }}>Total</th>
            </tr>
          </thead>
          <tbody>
            {acRows.map(ac => (
              <tr key={ac.name}>
                <td><strong style={{ color: T.text }}>{ac.name}</strong></td>
                <td style={{ textAlign: 'center' }}>{ac.base}</td>
                <td style={{ textAlign: 'center' }}><ModCell value={ac.dex} /></td>
                <td style={{ textAlign: 'center' }}><ModCell value={ac.armor} /></td>
                <td style={{ textAlign: 'center' }}><ModCell value={ac.shield} /></td>
                <td style={{ textAlign: 'center' }}>
                  {ac.name === 'AC' ? (
                    <StepInput
                      value={ac.natural}
                      onValueChange={handleNaturalAcChange}
                      min={0} max={255} width={72}
                    />
                  ) : (
                    <ModCell value={ac.natural} />
                  )}
                </td>
                <td style={{ textAlign: 'center' }}><ModCell value={ac.deflection} /></td>
                <td style={{ textAlign: 'center' }}><ModCell value={ac.size} /></td>
                <td style={{ textAlign: 'center' }}><ModCell value={ac.misc} /></td>
                <td style={{ textAlign: 'center' }}>
                  <strong className="t-xl" style={{ color: T.text }}>{ac.total}</strong>
                </td>
              </tr>
            ))}
          </tbody>
        </HTMLTable>
      </Card>

      <RespecDialog isOpen={isRespecOpen} onClose={() => setIsRespecOpen(false)} pointBuyState={abilitiesData?.point_buy ?? null} onApply={handleRespecApply} />
    </div>
  );
}
