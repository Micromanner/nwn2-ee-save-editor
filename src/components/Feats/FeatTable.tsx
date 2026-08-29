import { HTMLTable, Tag, Icon } from '@blueprintjs/core';
import { T } from '../theme';
import type { DummyFeat } from '../dummy-data';
import { FeatExpandedDetail } from './FeatExpandedDetail';

const TYPE_COLORS: Record<string, string> = {
  Combat: '#8b5e3c', General: '#5a7a5a', Class: '#6a7fa0', Proficiency: '#7a7a5a',
  Metamagic: '#7a5a8a', Divine: '#b8952f', Background: '#8a7a5a', Racial: '#6a8a7a',
  Epic: '#9c4040',
};

interface FeatTableProps {
  feats: (DummyFeat & { canTake?: boolean; hasFeat?: boolean })[];
  expandedId: number | null;
  onToggle: (id: number | null) => void;
  showOwned: boolean;
}

export function FeatTable({ feats, expandedId, onToggle, showOwned }: FeatTableProps) {
  const colCount = showOwned ? 4 : 3;

  return (
    <HTMLTable compact striped bordered interactive style={{ width: '100%', tableLayout: 'fixed' }}>
      <colgroup>
        <col />
        <col style={{ width: 100 }} />
        <col style={{ width: 56 }} />
        {showOwned && <col style={{ width: 80 }} />}
      </colgroup>
      <thead>
        <tr>
          <th style={{ textAlign: 'left' }}>Feat</th>
          <th style={{ textAlign: 'center' }}>Type</th>
          <th style={{ textAlign: 'center' }}>{' '}</th>
          {showOwned && <th style={{ textAlign: 'center' }}>Owned</th>}
        </tr>
      </thead>
      <tbody>
        {feats.map(feat => {
          const expanded = expandedId === feat.id;
          const tagColor = TYPE_COLORS[feat.type] || T.textMuted;

          return (
            <FeatRow
              key={feat.id}
              feat={feat}
              expanded={expanded}
              onToggle={() => onToggle(expanded ? null : feat.id)}
              tagColor={tagColor}
              colCount={colCount}
              showOwned={showOwned}
            />
          );
        })}
      </tbody>
    </HTMLTable>
  );
}

function FeatRow({ feat, expanded, onToggle, tagColor, colCount, showOwned }: {
  feat: DummyFeat & { canTake?: boolean; hasFeat?: boolean };
  expanded: boolean;
  onToggle: () => void;
  tagColor: string;
  colCount: number;
  showOwned: boolean;
}) {
  return (
    <>
      <tr onClick={onToggle} style={{ cursor: 'pointer' }}>
        <td>
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
            <span className="t-semibold" style={{ color: T.text }}>{feat.name}</span>
            {feat.isProtected && (
              <Tag minimal className="t-xs" style={{ padding: '0 4px', lineHeight: '14px', color: T.gold, border: `1px solid ${T.gold}40` }}>
                protected
              </Tag>
            )}
          </span>
        </td>
        <td style={{ textAlign: 'center' }}>
          <Tag minimal round className="t-xs" style={{
            background: `${tagColor}18`, color: tagColor,
            border: `1px solid ${tagColor}35`,
          }}>
            {feat.type}
          </Tag>
        </td>
        <td style={{ textAlign: 'center' }}>
          <Icon
            icon={expanded ? 'caret-down' : 'caret-right'}
            size={16}
            style={{ color: expanded ? T.accent : T.textMuted }}
          />
        </td>
        {showOwned && (
          <td style={{ textAlign: 'center' }}>
            {feat.hasFeat
              ? <Tag minimal className="t-xs" style={{ color: T.positive, background: `${T.positive}15` }}>Yes</Tag>
              : feat.canTake
                ? <Tag minimal className="t-xs" style={{ color: T.accent, background: `${T.accent}15` }}>Available</Tag>
                : <span className="t-sm" style={{ color: T.textMuted }}>-</span>
            }
          </td>
        )}
      </tr>
      {expanded && (
        <tr>
          <td colSpan={colCount} style={{ background: T.surfaceAlt, padding: '12px 16px' }}>
            <FeatExpandedDetail feat={feat} />
          </td>
        </tr>
      )}
    </>
  );
}
