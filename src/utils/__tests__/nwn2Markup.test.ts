import { describe, it, expect } from 'vitest';
import { nwn2ToHtml, stripNwn2Tags } from '../nwn2Markup';

describe('nwn2ToHtml', () => {
  it('converts supported NWN2 tags', () => {
    expect(nwn2ToHtml('<b>bold</b> and <i>italic</i>')).toBe('<b>bold</b> and <i>italic</i>');
    expect(nwn2ToHtml('<color=Gold>text</color>')).toBe(
      '<span style="color:#FFD700" data-nwn2-color="Gold">text</span>',
    );
  });

  it('passes hex color values through', () => {
    expect(nwn2ToHtml('<color=#a0522d>text</color>')).toBe(
      '<span style="color:#a0522d" data-nwn2-color="#a0522d">text</span>',
    );
  });

  it('converts newlines to <br>', () => {
    expect(nwn2ToHtml('line1\nline2')).toBe('line1<br>line2');
  });

  it('escapes HTML that is not a supported NWN2 tag', () => {
    expect(nwn2ToHtml('<img src=x onerror=alert(1)>')).toBe('&lt;img src=x onerror=alert(1)&gt;');
    expect(nwn2ToHtml('<script>alert(1)</script>')).toBe('&lt;script&gt;alert(1)&lt;/script&gt;');
    expect(nwn2ToHtml('a < b & c > d')).toBe('a &lt; b &amp; c &gt; d');
  });

  it('escapes injection attempts inside color values', () => {
    const html = nwn2ToHtml('<color="><img onerror=alert(1) src=x>">text</color>');
    expect(html).not.toContain('<img');
  });

  it('cannot break out of the style attribute via quotes in color values', () => {
    const html = nwn2ToHtml('<color=" onmouseover=alert(1) x=">text</color>');
    expect(html).not.toContain('" onmouseover');
    expect(html).toContain('style="color:&quot;');
  });
});

describe('stripNwn2Tags', () => {
  it('removes color, b, and i tags', () => {
    expect(stripNwn2Tags('<color=Gold><b>Bard</b></color>')).toBe('Bard');
  });
});
