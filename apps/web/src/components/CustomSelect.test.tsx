import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { CustomSelect } from '@/components/CustomSelect';

function rect(values: Partial<DOMRect>): DOMRect {
  return { x: 0, y: 0, top: 0, right: 0, bottom: 0, left: 0, width: 0, height: 0, toJSON: () => ({}), ...values } as DOMRect;
}

describe('CustomSelect popup geometry', () => {
  it('portals outside clipping panels, opens upward near the viewport bottom, and keeps the last option selectable', () => {
    Object.defineProperty(window, 'innerHeight', { configurable: true, value: 800 });
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1200 });
    const onChange = vi.fn();
    render(
      <div style={{ overflow: 'hidden', height: 100 }}>
        <CustomSelect
          id="grade"
          ariaLabel="学年"
          value="1"
          options={Array.from({ length: 9 }, (_, index) => ({ value: String(index + 1), label: `${index + 1}年` }))}
          onChange={onChange}
        />
      </div>,
    );
    const trigger = screen.getByRole('combobox', { name: '学年' });
    vi.spyOn(trigger, 'getBoundingClientRect').mockReturnValue(rect({
      left: 300, right: 600, top: 720, bottom: 772, width: 300, height: 52,
    }));

    fireEvent.click(trigger);
    const listbox = screen.getByRole('listbox', { name: '学年の選択肢' });
    expect(listbox.parentElement).toBe(document.body);
    expect(listbox.style.position).toBe('fixed');
    expect(listbox.style.bottom).not.toBe('');
    expect(Number.parseFloat(listbox.style.maxHeight)).toBeLessThanOrEqual(300);

    fireEvent.click(screen.getByRole('option', { name: '9年' }));
    expect(onChange).toHaveBeenCalledWith('9');
  });

  it('keyboard End reaches the final option in a scrollable menu', () => {
    const onChange = vi.fn();
    render(
      <CustomSelect
        id="difficulty"
        ariaLabel="難易度"
        value="3"
        options={Array.from({ length: 5 }, (_, index) => ({ value: String(index + 1), label: `${index + 1}` }))}
        onChange={onChange}
      />,
    );
    const trigger = screen.getByRole('combobox', { name: '難易度' });
    vi.spyOn(trigger, 'getBoundingClientRect').mockReturnValue(rect({
      left: 20, right: 320, top: 100, bottom: 152, width: 300, height: 52,
    }));
    fireEvent.click(trigger);
    fireEvent.keyDown(trigger, { key: 'End' });
    expect(trigger).toHaveAttribute('aria-activedescendant', expect.stringContaining('option-4'));
    fireEvent.keyDown(trigger, { key: 'Enter' });
    expect(onChange).toHaveBeenCalledWith('5');
  });
});
