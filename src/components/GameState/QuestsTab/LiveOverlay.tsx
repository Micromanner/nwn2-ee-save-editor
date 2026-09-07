import { useEffect, useState } from 'react';
import { HTMLTable, Spinner } from '@blueprintjs/core';
import { invoke } from '@tauri-apps/api/core';
import { useTranslations } from '@/hooks/useTranslations';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { display } from '@/utils/dataHelpers';
import type { PendingQuestEdit, VariableKey } from './types';

interface QuestVariableValue {
  key: VariableKey;
  saved: string | number | null;
  effective: string | number | null;
  available: boolean;
}

export function LiveOverlay({ tag, pending }: { tag: string; pending: PendingQuestEdit[] }) {
  const t = useTranslations();
  const { handleError } = useErrorHandler();
  const [rows, setRows] = useState<QuestVariableValue[] | null>(null);
  useEffect(() => {
    let active = true;
    setRows(null);
    invoke<QuestVariableValue[]>('save_get_quest_variables', { tag })
      .then(result => { if (active) setRows(result); })
      .catch(error => { if (active) { handleError(error); setRows([]); } });
    return () => { active = false; };
  }, [tag, pending, handleError]);
  if (!rows) return <Spinner size={20} />;
  if (!rows.length) return <p>{t('gameState.quests.detail.none')}</p>;
  return <HTMLTable compact bordered style={{ width: '100%' }}>
    <thead><tr><th>{t('gameState.quests.edit.variable')}</th><th>{t('gameState.quests.edit.saved')}</th><th>{t('gameState.quests.edit.effective')}</th></tr></thead>
    <tbody>{rows.map(row => <tr key={JSON.stringify(row.key)}>
      <td>{row.key.name}<div>{row.key.module ?? t('gameState.quests.edit.campaign')} / {row.key.value_type}</div></td>
      <td>{row.available ? display(row.saved) : t('gameState.quests.edit.unavailable')}</td>
      <td>{row.available ? display(row.effective) : t('gameState.quests.edit.unavailable')}</td>
    </tr>)}</tbody>
  </HTMLTable>;
}
