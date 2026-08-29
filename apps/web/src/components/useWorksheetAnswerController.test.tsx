import { act, renderHook } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { useWorksheetAnswerController } from './useWorksheetAnswerController';
import { fixtureWorksheet } from '@/test/fixtures';

describe('useWorksheetAnswerController', () => {
  it('resets answer/input session atomically for a worksheet', () => {
    const { result } = renderHook(() => useWorksheetAnswerController());
    const worksheet = fixtureWorksheet();

    act(() => {
      result.current.select(2, 'x');
      result.current.setAnswer(worksheet.problems[0]!.problem_id, { type: 'integer', value: '7' });
      result.current.setColumnDraft('draft', ['1', null]);
      result.current.setColumnDecimalBoundary('draft', 2);
      result.current.resetForWorksheet(worksheet);
    });

    expect(result.current.selectedIndex).toBeNull();
    expect(result.current.selectedSlot).toBe('single');
    expect(result.current.selectedColumnDigit).toBeNull();
    expect(result.current.inputEnabledRef.current).toBe(false);
    expect(result.current.columnDrafts).toEqual({});
    expect(result.current.columnDecimalBoundaries).toEqual({});
    expect(result.current.answersRef.current).toEqual(result.current.answers);
    expect(Object.values(result.current.answers)).toHaveLength(worksheet.problems.length);
    expect(Object.values(result.current.answers).every((answer) => answer.type === 'empty')).toBe(true);
  });

  it('keeps React selection state and same-tick refs synchronized', () => {
    const { result } = renderHook(() => useWorksheetAnswerController());

    act(() => result.current.selectColumnDigit({ problemIndex: 3, slot: 'quotient', digitIndex: 2 }));
    expect(result.current.selectedIndex).toBe(3);
    expect(result.current.selectedIndexRef.current).toBe(3);
    expect(result.current.selectedSlot).toBe('quotient');
    expect(result.current.selectedSlotRef.current).toBe('quotient');
    expect(result.current.selectedColumnDigitRef.current).toEqual({ problemIndex: 3, slot: 'quotient', digitIndex: 2 });
    expect(result.current.inputEnabledRef.current).toBe(true);

    act(() => result.current.clearSelection());
    expect(result.current.selectedIndex).toBeNull();
    expect(result.current.selectedIndexRef.current).toBeNull();
    expect(result.current.selectedSlot).toBe('single');
    expect(result.current.selectedColumnDigit).toBeNull();
    expect(result.current.inputEnabledRef.current).toBe(false);
  });
});
