// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { Markdown } from './Markdown';

afterEach(cleanup);

describe('Markdown', () => {
  it('renders gfm tables, strikethrough, and task lists', () => {
    const { container } = render(
      <Markdown>{'| a | b |\n| - | - |\n| 1 | 2 |\n\n~~gone~~\n\n- [x] done'}</Markdown>,
    );

    expect(container.querySelector('table')).not.toBeNull();
    expect(container.querySelector('del')?.textContent).toBe('gone');
    expect(container.querySelector('input[type="checkbox"]')).not.toBeNull();
  });

  it('opens links in a new tab without leaking the referrer', () => {
    render(<Markdown>{'[docs](https://example.com)'}</Markdown>);

    const link = screen.getByRole('link', { name: 'docs' });
    expect(link.getAttribute('target')).toBe('_blank');
    expect(link.getAttribute('rel')).toBe('noreferrer');
  });

  it('ignores raw html', () => {
    const { container } = render(
      <Markdown>{'Before<script>alert(1)</script><b>after</b>'}</Markdown>,
    );

    expect(container.querySelector('script')).toBeNull();
    expect(container.querySelector('b')).toBeNull();
  });

  it('drops html comments', () => {
    const { container } = render(
      <Markdown>{'Before\n\n<!-- One or two sentences about the change -->\n\nAfter'}</Markdown>,
    );

    expect(container.textContent).not.toContain('One or two sentences');
    expect(container.textContent).toContain('Before');
    expect(container.textContent).toContain('After');
  });

  it('loads remote images lazily without a referrer', () => {
    const { container } = render(<Markdown>{'![shot](https://example.com/a.png)'}</Markdown>);

    const image = container.querySelector('img');
    expect(image?.getAttribute('loading')).toBe('lazy');
    expect(image?.getAttribute('referrerpolicy')).toBe('no-referrer');
  });

  it('drops relative image sources to their alt text', () => {
    const { container } = render(<Markdown>{'![diagram](./diagram.png)'}</Markdown>);

    expect(container.querySelector('img')).toBeNull();
    expect(screen.getByText('diagram')).toBeDefined();
  });

  it('demotes markdown headings below the page heading', () => {
    const { container } = render(<Markdown>{'# Title\n\n## Subtitle'}</Markdown>);

    expect(container.querySelector('h1')).toBeNull();
    expect(screen.getByRole('heading', { level: 2, name: 'Title' })).toBeDefined();
    expect(screen.getByRole('heading', { level: 3, name: 'Subtitle' })).toBeDefined();
  });

  it('renders inline markdown without block wrappers', () => {
    const { container } = render(<Markdown variant="inline">{'**bold** text'}</Markdown>);

    expect(container.querySelector('p')).toBeNull();
    expect(container.querySelector('strong')?.textContent).toBe('bold');
  });

  it('wraps tables so they can scroll on narrow screens', () => {
    const { container } = render(<Markdown>{'| a |\n| - |\n| 1 |'}</Markdown>);

    expect(container.querySelector('.overflow-x-auto > table')).not.toBeNull();
  });
});
