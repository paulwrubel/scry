// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { LoginPage } from './index';

afterEach(cleanup);

describe('LoginPage', () => {
  it('renders the placeholder sign-in copy', () => {
    render(<LoginPage />);

    expect(screen.getByRole('heading', { name: 'Sign in' })).toBeTruthy();
    expect(screen.getByText('Browser sign-in is not available in this build.')).toBeTruthy();
  });
});
