import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import ErrorState from './ErrorState.svelte';
import App from '../App.svelte';
import { createMockClient } from '../api/mockClient';
import { setLocale } from '../i18n';

beforeEach(() => {
  window.localStorage.clear();
  setLocale('en');
});

describe('shared error state and application failure recovery', () => {
  it('translates the retry action when the interface language changes, without rewriting error data', async () => {
    const retry = vi.fn();
    const backendDetail = 'Workspace path /home/user/محتوى is unavailable';
    render(ErrorState, { props: { message: backendDetail, onRetry: retry } });

    const alert = screen.getByRole('alert');
    expect(within(alert).getByRole('heading', { name: 'Something went wrong' })).toBeInTheDocument();
    expect(within(alert).getByRole('button', { name: 'Try again' })).toBeInTheDocument();
    expect(alert).toHaveAttribute('data-i18n-ignore');
    setLocale('ar');

    await waitFor(() => expect(within(alert).getByRole('heading', { name: 'حدث خطأ' })).toBeInTheDocument());
    expect(within(alert).getByRole('button', { name: 'إعادة المحاولة' })).toBeInTheDocument();
    expect(within(alert).getByText(backendDetail)).toBeInTheDocument();

    await fireEvent.click(within(alert).getByRole('button', { name: 'إعادة المحاولة' }));
    expect(retry).toHaveBeenCalledOnce();
  });

  it('keeps raw backend startup errors intact while localizing the recovery view', async () => {
    setLocale('ar');
    const api = createMockClient();
    const backendDetail = 'Denied for project /home/user/English Project';
    api.bootstrap = vi.fn().mockRejectedValue(new Error(backendDetail));
    render(App, { props: { api } });

    const alert = await screen.findByRole('alert');
    expect(within(alert).getByRole('heading', { name: 'تعذّر فتح Scriptotar' })).toBeInTheDocument();
    expect(within(alert).getByText(backendDetail)).toBeInTheDocument();
    expect(within(alert).getByRole('button', { name: 'إعادة المحاولة' })).toBeInTheDocument();
  });
});
