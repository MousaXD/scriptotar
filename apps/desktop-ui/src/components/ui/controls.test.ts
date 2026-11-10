import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import Select from './Select.svelte';
import Slider from './Slider.svelte';
import Checkbox from './Checkbox.svelte';
import Switch from './Switch.svelte';
import RadioGroup from './RadioGroup.svelte';
import TextField from './TextField.svelte';
import Popover from './Popover.svelte';

beforeEach(() => { document.documentElement.dir = 'ltr'; });

describe('shared workstation controls', () => {
  it('shows a styled listbox, skips disabled options and supports keyboard selection', async () => {
    const onChange = vi.fn();
    render(Select, { props: {
      label: 'Project',
      value: 'one',
      options: [{ value: 'one', label: 'One' }, { value: 'locked', label: 'Locked', disabled: true }, { value: 'two', label: 'Two' }],
      onChange
    } });
    const trigger = screen.getByRole('button', { name: 'Project One' });
    expect(trigger).toHaveAccessibleName('Project One');
    await fireEvent.keyDown(trigger, { key: 'ArrowDown' });
    const menu = await screen.findByRole('listbox', { name: 'Project' });
    expect(document.activeElement).toBe(menu);
    expect(within(menu).getAllByRole('option')).toHaveLength(3);
    await fireEvent.keyDown(menu, { key: 'ArrowDown' });
    expect(menu.getAttribute('aria-activedescendant')).toMatch(/option-2$/);
    await fireEvent.keyDown(menu, { key: 'Enter' });
    expect(onChange).toHaveBeenCalledWith('two');
    await waitFor(() => expect(screen.queryByRole('listbox')).not.toBeInTheDocument());
    expect(trigger).toHaveAttribute('aria-expanded', 'false');
    expect(document.activeElement).toBe(trigger);
  });

  it('closes with Escape and outside pointer presses without changing the value', async () => {
    const onChange = vi.fn();
    render(Select, { props: {
      label: 'Language', value: 'en',
      options: [{ value: 'en', label: 'English' }, { value: 'ar', label: 'Arabic' }],
      onChange
    } });
    const trigger = screen.getAllByRole('button')[0];
    await fireEvent.click(trigger);
    const menu = screen.getByRole('listbox');
    await fireEvent.keyDown(menu, { key: 'Escape' });
    await waitFor(() => expect(screen.queryByRole('listbox')).not.toBeInTheDocument());
    expect(onChange).not.toHaveBeenCalled();
    await fireEvent.click(trigger);
    expect(screen.getByRole('listbox')).toBeInTheDocument();
    await fireEvent.pointerDown(document.body);
    await waitFor(() => expect(screen.queryByRole('listbox')).not.toBeInTheDocument());
  });

  it('preserves accessible native slider keyboard semantics and exposes value', async () => {
    const onChange = vi.fn();
    render(Slider, { props: { label: 'Volume', min: 0, max: 10, step: 1, value: 4, onChange } });
    const range = screen.getByRole('slider', { name: 'Volume' });
    expect(range).toHaveAttribute('min', '0');
    expect(range).toHaveAttribute('max', '10');
    expect(range).toHaveAttribute('aria-valuetext', '4');
    await fireEvent.input(range, { target: { value: '7' } });
    expect(onChange).toHaveBeenCalledWith(7);
    expect(screen.getByText('7')).toBeInTheDocument();
  });

  it('uses native checkbox, switch and radio semantics', async () => {
    const checkboxChanged = vi.fn();
    const switchChanged = vi.fn();
    render(Checkbox, { props: { label: 'Include metadata', checked: false, onChange: checkboxChanged } });
    render(Switch, { props: { label: 'Watch folder', checked: true, onChange: switchChanged } });
    render(RadioGroup, { props: {
      label: 'Quality', value: 'fast',
      options: [{ value: 'fast', label: 'Fast' }, { value: 'accurate', label: 'Accurate' }]
    } });
    const checkbox = screen.getByRole('checkbox', { name: 'Include metadata' });
    const toggle = screen.getByRole('switch', { name: 'Watch folder' });
    const accurate = screen.getByRole('radio', { name: 'Accurate' });
    expect(checkbox).not.toBeChecked();
    expect(toggle).toBeChecked();
    expect(screen.getByRole('radio', { name: 'Fast' })).toBeChecked();
    await fireEvent.click(checkbox);
    await fireEvent.click(toggle);
    await fireEvent.click(accurate);
    expect(checkboxChanged).toHaveBeenCalledWith(true);
    expect(switchChanged).toHaveBeenCalledWith(false);
    expect(accurate).toBeChecked();
    const fast = screen.getByRole('radio', { name: 'Fast' }) as HTMLInputElement;
    expect(fast.name).toBeTruthy();
    expect(accurate).toHaveAttribute('name', fast.name);
    render(RadioGroup, { props: {
      label: 'Mode', value: 'manual',
      options: [{ value: 'manual', label: 'Manual' }, { value: 'auto', label: 'Automatic' }]
    } });
    expect(screen.getByRole('radio', { name: 'Manual' })).not.toHaveAttribute('name', fast.name);
  });

  it('offers a labeled input with validation and description, including RTL', async () => {
    document.documentElement.dir = 'rtl';
    const onInput = vi.fn();
    render(TextField, { props: { label: 'Source URL', value: '', required: true, invalid: true, description: 'Use a public URL', onInput } });
    const field = screen.getByRole('textbox', { name: 'Source URL' });
    expect(field).toHaveAttribute('aria-invalid', 'true');
    expect(field).toHaveAccessibleDescription('Use a public URL');
    await fireEvent.input(field, { target: { value: 'https://example.com' } });
    expect(onInput).toHaveBeenCalledWith('https://example.com');
  });

  it('provides a non-modal menu with Escape dismissal and focus restoration', async () => {
    render(Popover, { props: { label: 'Export format', triggerLabel: 'Export' } });
    const trigger = screen.getByRole('button', { name: 'Export' });
    await fireEvent.click(trigger);
    const panel = screen.getByRole('dialog', { name: 'Export format' });
    expect(panel).toBeInTheDocument();
    expect(trigger.getAttribute('aria-controls')).toBe(panel.id);
    expect(document.activeElement).toBe(panel);
    await fireEvent.keyDown(panel, { key: 'Escape' });
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(document.activeElement).toBe(trigger);
  });
});
