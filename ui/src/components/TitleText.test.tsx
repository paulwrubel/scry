// @vitest-environment jsdom
import { cleanup, render } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { TitleText } from './TitleText';

afterEach(cleanup);

describe('TitleText', () => {
  it('renders a plain title as text', () => {
    const { container } = render(<TitleText title="Fix the parser" />);

    expect(container.textContent).toBe('Fix the parser');
    expect(container.querySelector('code')).toBeNull();
  });

  it('renders backtick spans as inline code', () => {
    const { container } = render(<TitleText title="Fix `parse()` crash" />);

    expect(container.querySelector('code')?.textContent).toBe('parse()');
    expect(container.textContent).toBe('Fix parse() crash');
  });

  it('leaves other markdown literal', () => {
    const { container } = render(<TitleText title="**bold** `code`" />);

    expect(container.querySelector('strong')).toBeNull();
    expect(container.textContent).toBe('**bold** code');
  });

  it('leaves unbalanced backticks literal', () => {
    const { container } = render(<TitleText title="Fix `parse()" />);

    expect(container.querySelector('code')).toBeNull();
    expect(container.textContent).toBe('Fix `parse()');
  });
});
