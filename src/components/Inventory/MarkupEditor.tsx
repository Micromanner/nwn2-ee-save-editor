import { useCallback, useEffect, useRef, useState } from 'react';
import { Button, ButtonGroup, InputGroup, Popover, Switch } from '@blueprintjs/core';
import { T } from '../theme';
import { useTranslations } from '@/hooks/useTranslations';
import { NWN2_COLOR_NAMES, findMarkupError, htmlToNwn2, isVisuallyEditable, nwn2ToHtml } from '@/utils/nwn2Markup';

const SHOW_TAGS_KEY = 'nwn2ee-markup-show-tags';

function readShowTagsPreference(): boolean {
  try {
    return localStorage.getItem(SHOW_TAGS_KEY) === '1';
  } catch {
    return false;
  }
}

const SWATCHES = Object.entries(NWN2_COLOR_NAMES).filter(([name]) => name !== 'grey');

interface MarkupEditorProps {
  value: string;
  onChange: (value: string) => void;
  multiline?: boolean;
  minHeight?: number;
}

// Edits NWN2 item markup (<b>, <i>, <color=...>) either visually or as raw
// tags. The raw string is the only state; the visual mode is offered only when
// a DOM round trip reproduces the text byte-for-byte, so nothing is ever lost.
export function MarkupEditor({ value, onChange, multiline = false, minHeight = 32 }: MarkupEditorProps) {
  const t = useTranslations();
  const [showTags, setShowTags] = useState(readShowTagsPreference);
  const [visualOk, setVisualOk] = useState(() => isVisuallyEditable(value));
  const [initToken, setInitToken] = useState(0);
  const [customHex, setCustomHex] = useState('#FF0000');

  const visualRef = useRef<HTMLDivElement>(null);
  const rawRef = useRef<HTMLTextAreaElement | HTMLInputElement>(null);
  const emittedRef = useRef<string | null>(null);
  const savedRangeRef = useRef<Range | null>(null);
  const savedRawSelectionRef = useRef<[number, number]>([0, 0]);

  const mode: 'visual' | 'raw' = showTags || !visualOk ? 'raw' : 'visual';

  // A value we did not emit ourselves means a different item was loaded.
  useEffect(() => {
    if (value === emittedRef.current) return;
    emittedRef.current = value;
    setVisualOk(isVisuallyEditable(value));
    setInitToken(n => n + 1);
  }, [value]);

  useEffect(() => {
    if (mode !== 'visual' || !visualRef.current) return;
    visualRef.current.innerHTML = nwn2ToHtml(value, true);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode, initToken]);

  const emit = useCallback((next: string) => {
    emittedRef.current = next;
    onChange(next);
  }, [onChange]);

  const syncVisual = useCallback(() => {
    if (visualRef.current) emit(htmlToNwn2(visualRef.current));
  }, [emit]);

  const toggleShowTags = (on: boolean) => {
    if (!on) setVisualOk(isVisuallyEditable(value));
    setShowTags(on);
    try { localStorage.setItem(SHOW_TAGS_KEY, on ? '1' : '0'); } catch { /* preference only */ }
  };

  const saveSelection = () => {
    if (mode === 'visual') {
      const sel = window.getSelection();
      const editor = visualRef.current;
      if (sel && sel.rangeCount > 0 && editor && editor.contains(sel.anchorNode)) {
        savedRangeRef.current = sel.getRangeAt(0).cloneRange();
      } else {
        savedRangeRef.current = null;
      }
    } else if (rawRef.current) {
      const el = rawRef.current;
      savedRawSelectionRef.current = [el.selectionStart ?? 0, el.selectionEnd ?? 0];
    }
  };

  const restoreVisualSelection = () => {
    const editor = visualRef.current;
    if (!editor) return;
    editor.focus();
    const range = savedRangeRef.current;
    if (!range) return;
    const sel = window.getSelection();
    if (!sel) return;
    sel.removeAllRanges();
    sel.addRange(range);
  };

  // Raw mode mirrors a word processor: a selection already wrapped in the tag
  // (either just outside or just inside the selection) has the tag removed, or
  // for color its value replaced; anything else gets wrapped.
  const applyRawTag = (openPattern: string, open: string, close: string, replace: boolean, useSaved: boolean) => {
    const el = rawRef.current;
    if (!el) return;
    const [start, end] = useSaved ? savedRawSelectionRef.current : [el.selectionStart ?? 0, el.selectionEnd ?? 0];
    const prefix = value.slice(0, start);
    const selected = value.slice(start, end);
    const suffix = value.slice(end);
    const outer = prefix.match(new RegExp(`${openPattern}$`, 'i'));
    const inner = selected.match(new RegExp(`^${openPattern}`, 'i'));

    let next: string;
    let selStart: number;
    let selEnd: number;
    if (outer && suffix.toLowerCase().startsWith(close)) {
      const keptPrefix = prefix.slice(0, prefix.length - outer[0].length);
      const newOpen = replace ? open : '';
      const newClose = replace ? close : '';
      next = keptPrefix + newOpen + selected + newClose + suffix.slice(close.length);
      selStart = keptPrefix.length + newOpen.length;
      selEnd = selStart + selected.length;
    } else if (inner && selected.toLowerCase().endsWith(close) && selected.length >= inner[0].length + close.length) {
      const core = selected.slice(inner[0].length, selected.length - close.length);
      const newSelected = replace ? open + core + close : core;
      next = prefix + newSelected + suffix;
      selStart = start;
      selEnd = start + newSelected.length;
    } else {
      next = prefix + open + selected + close + suffix;
      selStart = start + open.length;
      selEnd = end + open.length;
    }
    emit(next);
    requestAnimationFrame(() => {
      el.focus();
      el.setSelectionRange(selStart, selEnd);
    });
  };

  const applyBold = () => {
    if (mode === 'visual') {
      document.execCommand('bold');
      syncVisual();
    } else {
      applyRawTag('<b>', '<b>', '</b>', false, false);
    }
  };

  const applyItalic = () => {
    if (mode === 'visual') {
      document.execCommand('italic');
      syncVisual();
    } else {
      applyRawTag('<i>', '<i>', '</i>', false, false);
    }
  };

  // `literal` is what ends up inside <color=...>; `hex` is what the browser
  // needs. Named colors come back out of the DOM as names again.
  const applyColor = (literal: string, hex: string) => {
    if (mode === 'visual') {
      restoreVisualSelection();
      document.execCommand('styleWithCSS', false, 'true');
      document.execCommand('foreColor', false, hex);
      syncVisual();
    } else {
      applyRawTag('<color=[^>]*>', `<color=${literal}>`, '</color>', true, true);
    }
  };

  const customHexValid = /^#[0-9a-fA-F]{6}$/.test(customHex);

  const colorMenu = (
    <div style={{ padding: 8, width: 196 }}>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(5, 28px)', gap: 6, marginBottom: 8 }}>
        {SWATCHES.map(([name, hex]) => (
          <button
            key={name}
            type="button"
            title={name.charAt(0).toUpperCase() + name.slice(1)}
            onClick={() => applyColor(name.charAt(0).toUpperCase() + name.slice(1), hex)}
            style={{
              width: 28, height: 28, background: hex, border: `1px solid ${T.sectionBorder}`,
              borderRadius: 3, cursor: 'pointer', padding: 0,
            }}
          />
        ))}
      </div>
      <div style={{ display: 'flex', gap: 4 }}>
        <InputGroup
          small
          value={customHex}
          onChange={(e) => setCustomHex(e.target.value)}
          placeholder="#RRGGBB"
          style={{ fontFamily: 'monospace' }}
        />
        <Button
          small
          text={t('inventory.editor.apply')}
          disabled={!customHexValid}
          onClick={() => applyColor(customHex.toUpperCase(), customHex)}
        />
      </div>
    </div>
  );

  const keepFocus = (e: React.MouseEvent) => e.preventDefault();
  const markupError = mode === 'raw' ? findMarkupError(value) : null;

  const boxStyle: React.CSSProperties = {
    width: '100%', borderRadius: 3, border: `1px solid ${markupError ? T.negative : T.border}`,
    background: multiline ? T.surfaceAlt : T.surface, padding: '6px 10px', color: T.text,
    minHeight, outline: 'none', fontFamily: 'inherit', fontSize: 'inherit', lineHeight: 'inherit',
    overflowWrap: 'anywhere',
  };

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
        <ButtonGroup minimal>
          <Button icon="bold" title={t('inventory.editor.bold')} onMouseDown={keepFocus} onClick={applyBold} />
          <Button icon="italic" title={t('inventory.editor.italic')} onMouseDown={keepFocus} onClick={applyItalic} />
          <Popover content={colorMenu} placement="bottom-start" onOpening={saveSelection}>
            <Button icon="tint" title={t('inventory.editor.color')} onMouseDown={keepFocus} />
          </Popover>
        </ButtonGroup>
        <div style={{ flex: 1 }} />
        <Switch
          checked={mode === 'raw'}
          disabled={!visualOk}
          label={t('inventory.editor.showTags')}
          onChange={(e) => toggleShowTags(e.currentTarget.checked)}
          style={{ margin: 0 }}
        />
      </div>

      {mode === 'visual' ? (
        <div
          ref={visualRef}
          className="nwn2-markup-editor"
          contentEditable
          suppressContentEditableWarning
          onInput={syncVisual}
          onKeyDown={(e) => {
            if (e.key !== 'Enter') return;
            e.preventDefault();
            if (!multiline) return;
            document.execCommand('insertLineBreak');
            syncVisual();
          }}
          onPaste={(e) => {
            e.preventDefault();
            document.execCommand('insertText', false, e.clipboardData.getData('text/plain'));
          }}
          style={{
            ...boxStyle,
            whiteSpace: multiline ? 'pre-wrap' : 'nowrap',
            overflow: multiline ? undefined : 'hidden',
          }}
        />
      ) : multiline ? (
        <textarea
          ref={rawRef as React.RefObject<HTMLTextAreaElement>}
          value={value}
          onChange={(e) => emit(e.target.value)}
          spellCheck={false}
          style={{ ...boxStyle, resize: 'vertical' }}
        />
      ) : (
        <input
          ref={rawRef as React.RefObject<HTMLInputElement>}
          value={value}
          onChange={(e) => emit(e.target.value)}
          spellCheck={false}
          style={boxStyle}
        />
      )}

      {markupError && (
        <div className="t-sm" style={{ color: T.negative, marginTop: 3 }}>
          {t(markupError.kind === 'unclosed' ? 'inventory.editor.unclosedTag' : 'inventory.editor.unopenedTag', { tag: `<${markupError.tag}>` })}
        </div>
      )}

      {mode === 'raw' && !visualOk && (
        <div className="t-sm" style={{ color: T.textMuted, marginTop: 3 }}>
          {t('inventory.editor.rawOnly')}
        </div>
      )}

      {mode === 'raw' && (
        <div style={{ marginTop: 6 }}>
          <div className="t-sm t-semibold" style={{ color: T.textMuted, marginBottom: 2 }}>
            {t('inventory.editor.preview')}
          </div>
          <div
            style={{
              padding: '6px 10px', border: `1px dashed ${T.border}`, borderRadius: 3,
              background: T.surface, color: T.text, minHeight: 24,
              whiteSpace: multiline ? 'pre-wrap' : 'nowrap', overflow: multiline ? 'visible' : 'hidden',
              overflowWrap: 'anywhere',
            }}
            dangerouslySetInnerHTML={{ __html: nwn2ToHtml(value) }}
          />
        </div>
      )}
    </div>
  );
}
