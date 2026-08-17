import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { MiniSudokuGrid } from './MiniSudokuGrid';
import { initialDigitGridAnswer, replaceDigitGridCell } from '@/domain/digit-grid-input';
import type { ProblemDto } from '@/domain/drill-engine';
import { fixtureWorksheet } from '@/test/fixtures';

function sudokuProblem(): ProblemDto {
  return {
    ...fixtureWorksheet().problems[0]!,
    numeric_theme_id: 38,
    prompt: {
      kind: 'mini_sudoku',
      givens: [1, null, null, 4, null, 4, 1, null, null, 1, 4, null, 4, null, null, 1],
    },
    input_interface: { type: 'digit_grid', min_digit: 1, max_digit: 4, cell_count: 16 },
    answer_schema: { kind: 'ordered_tuple', length: 16 },
    canonical_answer: {
      type: 'tuple',
      value: [1, 2, 3, 4, 3, 4, 1, 2, 2, 1, 4, 3, 4, 3, 2, 1].map((value) => ({ type: 'integer' as const, value: String(value) })),
    },
  };
}

describe('MiniSudokuGrid', () => {
  it('renders givens as fixed cells and empty cells as selectable buttons', () => {
    const problem = sudokuProblem();
    const answer = initialDigitGridAnswer(problem);
    const onSelect = vi.fn();
    const { container } = render(
      <MiniSudokuGrid problem={problem} answer={answer} onSelectCell={onSelect} />,
    );
    expect(container.querySelectorAll('[data-digit-grid-cell]')).toHaveLength(16);
    expect(container.querySelectorAll('button[data-digit-grid-cell]')).toHaveLength(8);
    fireEvent.click(screen.getByRole('button', { name: '2番目のマス 未入力' }));
    expect(onSelect).toHaveBeenCalledWith(1);
  });

  it('shows the canonical correction only on wrong editable cells', () => {
    const problem = sudokuProblem();
    let answer = initialDigitGridAnswer(problem);
    answer = replaceDigitGridCell(answer, problem.input_interface as Extract<ProblemDto['input_interface'], { type: 'digit_grid' }>, 1, 3);
    const { container } = render(
      <MiniSudokuGrid problem={problem} answer={answer} readOnly correctionAnswer={problem.canonical_answer} />,
    );
    expect(container.querySelector('[data-digit-grid-cell="1"] .digit-grid-cell-correction')?.textContent).toBe('2');
    expect(container.querySelector('[data-digit-grid-cell="0"] .digit-grid-cell-correction')).toBeNull();
  });
});
