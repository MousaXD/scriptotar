import { beforeEach, describe, expect, it } from 'vitest';
import { getLocale, initializeLocalization, setLocale, translateUiText } from './i18n';

describe('Arabic interface localization', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    window.localStorage.clear();
    setLocale('en');
  });

  it('translates fixed and dynamic interface copy', () => {
    expect(translateUiText('Settings')).toBe('الإعدادات');
    expect(translateUiText('3 active jobs')).toBe('3 مهام نشطة');
    expect(translateUiText('Queue selected (4)')).toBe('إضافة المحدد للطابور (4)');
    expect(translateUiText('Queued demo.mp4.')).toBe('تمت إضافة demo.mp4 إلى الطابور.');
  });

  it('switches the mounted UI to RTL without translating user transcript content', () => {
    document.body.innerHTML = `
      <main>
        <h1>Settings</h1>
        <input aria-label="Search workspace" placeholder="Search transcripts, projects, creators…" />
        <select id="language"><option>Arabic</option></select>
        <div class="transcript-content">Settings</div>
      </main>
    `;

    initializeLocalization();
    setLocale('ar');

    expect(document.documentElement.lang).toBe('ar');
    expect(document.documentElement.dir).toBe('rtl');
    expect(document.querySelector('h1')?.textContent).toBe('الإعدادات');
    expect(document.querySelector('input')?.getAttribute('aria-label')).toBe('البحث في مساحة العمل');
    expect(document.querySelector('input')?.getAttribute('placeholder')).toBe('ابحث في النصوص والمشاريع وصناع المحتوى…');

    const option = document.querySelector('option') as HTMLOptionElement;
    expect(option.textContent).toBe('العربية');
    expect(option.value).toBe('Arabic');
    expect(document.querySelector('.transcript-content')?.textContent).toBe('Settings');

    setLocale('en');
    expect(document.documentElement.lang).toBe('en');
    expect(document.documentElement.dir).toBe('ltr');
    expect(document.querySelector('h1')?.textContent).toBe('Settings');
    expect(option.textContent).toBe('Arabic');
    expect(option.value).toBe('Arabic');
  });

  it('persists the selected interface language locally', () => {
    setLocale('ar');
    expect(getLocale()).toBe('ar');
    expect(window.localStorage.getItem('scriptotar.uiLanguage')).toBe('ar');
  });
});
