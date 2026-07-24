import React from 'react';

const TAG_REGEX = /<\/?(color(=[^>]*)?|b|i)>/gi;

export function stripNwn2Tags(text: string): string {
  return text.replace(TAG_REGEX, '');
}

export const NWN2_COLOR_NAMES: Record<string, string> = {
  red: '#FF0000',
  green: '#00FF00',
  blue: '#0000FF',
  white: '#FFFFFF',
  black: '#000000',
  yellow: '#FFFF00',
  cyan: '#00FFFF',
  magenta: '#FF00FF',
  orange: '#FFA500',
  pink: '#FFC0CB',
  purple: '#800080',
  gray: '#808080',
  grey: '#808080',
  silver: '#C0C0C0',
  gold: '#FFD700',
  brown: '#8B4513',
};

function resolveColor(name: string): string {
  if (name.startsWith('#')) return name;
  return NWN2_COLOR_NAMES[name.toLowerCase()] || name;
}

interface TagFrame {
  tag: 'b' | 'i' | 'color';
  color?: string;
}

export function renderNwn2Markup(text: string): React.ReactNode {
  if (!text.includes('<')) return text;

  const parts: React.ReactNode[] = [];
  const stack: TagFrame[] = [];
  let cursor = 0;
  let key = 0;

  const openTagRe = /^<(b|i|color(?:=([^>]*))?)>/i;
  const closeTagRe = /^<\/(b|i|color)>/i;

  while (cursor < text.length) {
    if (text[cursor] === '<') {
      const remaining = text.slice(cursor);

      const openMatch = openTagRe.exec(remaining);
      if (openMatch) {
        const tagName = openMatch[1].toLowerCase().startsWith('color') ? 'color' : openMatch[1].toLowerCase() as 'b' | 'i';
        const colorValue = openMatch[2];
        stack.push({ tag: tagName, color: colorValue });
        cursor += openMatch[0].length;
        continue;
      }

      const closeMatch = closeTagRe.exec(remaining);
      if (closeMatch) {
        const closingTag = closeMatch[1].toLowerCase() as 'b' | 'i' | 'color';
        const idx = findLastIndex(stack, f => f.tag === closingTag);
        if (idx !== -1) {
          stack.splice(idx, 1);
        }
        cursor += closeMatch[0].length;
        continue;
      }
    }

    let end = text.indexOf('<', cursor + 1);
    if (end === -1) end = text.length;
    const chunk = text.slice(cursor, end);

    if (chunk) {
      if (stack.length === 0) {
        parts.push(chunk);
      } else {
        let node: React.ReactNode = chunk;
        for (let i = 0; i < stack.length; i++) {
          const frame = stack[i];
          if (frame.tag === 'b') {
            node = <strong key={key++}>{node}</strong>;
          } else if (frame.tag === 'i') {
            node = <em key={key++}>{node}</em>;
          } else if (frame.tag === 'color' && frame.color) {
            node = <span key={key++} style={{ color: resolveColor(frame.color) }}>{node}</span>;
          }
        }
        parts.push(node);
      }
    }

    cursor = end;
  }

  if (parts.length === 1 && typeof parts[0] === 'string') return parts[0];
  return <>{parts}</>;
}

function findLastIndex<T>(arr: T[], predicate: (item: T) => boolean): number {
  for (let i = arr.length - 1; i >= 0; i--) {
    if (predicate(arr[i])) return i;
  }
  return -1;
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

export function nwn2ToHtml(text: string): string {
  if (!text) return '';
  if (!text.includes('<') && !text.includes('\n')) return escapeHtml(text);

  let result = '';
  let cursor = 0;
  const openTagRe = /^<(b|i|color(?:=([^>]*))?)>/i;
  const closeTagRe = /^<\/(b|i|color)>/i;

  while (cursor < text.length) {
    if (text[cursor] === '\n') {
      result += '<br>';
      cursor++;
      continue;
    }
    if (text[cursor] === '<') {
      const remaining = text.slice(cursor);
      const openMatch = openTagRe.exec(remaining);
      if (openMatch) {
        const tagPart = openMatch[1].toLowerCase();
        if (tagPart === 'b') {
          result += '<b>';
        } else if (tagPart === 'i') {
          result += '<i>';
        } else if (tagPart.startsWith('color')) {
          const colorValue = openMatch[2] || '';
          result += `<span style="color:${escapeHtml(resolveColor(colorValue))}" data-nwn2-color="${escapeHtml(colorValue)}">`;
        }
        cursor += openMatch[0].length;
        continue;
      }
      const closeMatch = closeTagRe.exec(remaining);
      if (closeMatch) {
        const closingTag = closeMatch[1].toLowerCase();
        result += closingTag === 'color' ? '</span>' : `</${closingTag}>`;
        cursor += closeMatch[0].length;
        continue;
      }
      result += '&lt;';
      cursor++;
      continue;
    }
    let end = cursor;
    while (end < text.length && text[end] !== '<' && text[end] !== '\n') end++;
    result += escapeHtml(text.slice(cursor, end));
    cursor = end;
  }
  return result;
}

function cssColorToNwn2Name(cssColor: string): string {
  let hex = cssColor.trim().toLowerCase();
  const rgbMatch = hex.match(/rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)/);
  if (rgbMatch) {
    hex = '#' + [rgbMatch[1], rgbMatch[2], rgbMatch[3]]
      .map(n => parseInt(n).toString(16).padStart(2, '0'))
      .join('');
  }
  if (/^#[0-9a-f]{3}$/.test(hex)) {
    hex = `#${hex[1]}${hex[1]}${hex[2]}${hex[2]}${hex[3]}${hex[3]}`;
  }
  for (const [name, value] of Object.entries(NWN2_COLOR_NAMES)) {
    if (value.toLowerCase() === hex) {
      return name.charAt(0).toUpperCase() + name.slice(1);
    }
  }
  return hex.toUpperCase();
}

export function htmlToNwn2(element: HTMLElement): string {
  let result = '';
  for (const node of Array.from(element.childNodes)) {
    if (node.nodeType === Node.TEXT_NODE) {
      result += node.textContent || '';
    } else if (node.nodeType === Node.ELEMENT_NODE) {
      const el = node as HTMLElement;
      const tag = el.tagName.toLowerCase();
      if (tag === 'br') {
        result += '\n';
      } else if (tag === 'div' || tag === 'p') {
        if (result.length > 0 && !result.endsWith('\n')) result += '\n';
        result += htmlToNwn2(el);
      } else if (tag === 'b' || tag === 'strong') {
        result += '<b>' + htmlToNwn2(el) + '</b>';
      } else if (tag === 'i' || tag === 'em') {
        result += '<i>' + htmlToNwn2(el) + '</i>';
      } else if (tag === 'span') {
        const nwn2Color = el.getAttribute('data-nwn2-color');
        if (nwn2Color) {
          result += `<color=${nwn2Color}>` + htmlToNwn2(el) + '</color>';
        } else if (el.style.color) {
          result += `<color=${cssColorToNwn2Name(el.style.color)}>` + htmlToNwn2(el) + '</color>';
        } else {
          result += htmlToNwn2(el);
        }
      } else if (tag === 'font') {
        const color = el.getAttribute('color');
        if (color) {
          result += `<color=${cssColorToNwn2Name(color)}>` + htmlToNwn2(el) + '</color>';
        } else {
          result += htmlToNwn2(el);
        }
      } else {
        result += htmlToNwn2(el);
      }
    }
  }
  return result;
}
