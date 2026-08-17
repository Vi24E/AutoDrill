import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { ColumnArithmeticAnswerInput } from '@/components/ColumnArithmeticAnswerInput';
import { DRILL_OPERATION_KIND_COUNT, DRILL_SCHEMA_VERSION, type ProblemDto } from '@/domain/drill-engine';

function additionProblem(): ProblemDto {
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    id: 1,
    problem_id: '1',
    numeric_theme_id: 27,
    prompt: {
      kind: 'column_arithmetic',
      operator: 'add',
      left: { kind: 'integer', value: 140 },
      right: { kind: 'integer', value: 5366 },
    },
    input_interface: { type: 'simple_numeric', allow_decimal: false, allow_negative: false },
    answer_schema: { kind: 'integer', min: '0', max: '99999' },
    canonical_answer: { type: 'integer', value: '5506' },
    solution_graph: { steps: [] },
    operation_vector: { values: Array.from({ length: DRILL_OPERATION_KIND_COUNT }, () => 0) },
    effort: 0,
  };
}

describe('ColumnArithmeticAnswerInput', () => {
  it('renders each place as an independent selectable grid slot', () => {
    const onSelectDigit = vi.fn();
    const problem = additionProblem();
    const { container } = render(
      <ColumnArithmeticAnswerInput
        problem={problem}
        problemNumber={1}
        slot="single"
        value={{ type: 'empty' }}
        selectedDigit={3}
        readOnly={false}
        onSelectDigit={onSelectDigit}
      />,
    );

    const editor = container.querySelector('.column-digit-answer')!;
    expect(editor).toHaveAttribute('data-column-direction', 'right-to-left');
    const slots = container.querySelectorAll('.column-digit-slot');
    expect(slots.length).toBeGreaterThanOrEqual(4);
    const ones = screen.getByRole('button', { name: /1番の答え 一の位/ });
    expect(ones).toHaveClass('column-digit-slot-selected');
    fireEvent.click(ones);
    expect(onSelectDigit).toHaveBeenCalledWith(3);
  });

  it('keeps each digit visible but removes edit controls after grading locks the answer', () => {
    const problem = additionProblem();
    render(
      <ColumnArithmeticAnswerInput
        problem={problem}
        problemNumber={1}
        slot="single"
        value={{ type: 'integer', value: '5506' }}
        selectedDigit={null}
        readOnly
        onSelectDigit={() => undefined}
      />,
    );
    expect(screen.queryByRole('button', { name: /1番の答え/ })).not.toBeInTheDocument();
    expect(screen.getByLabelText('1番の答え 一の位 6')).toHaveTextContent('6');
  });
  it('moves real DOM focus with the selected digit so no stale keyboard-focus box remains', () => {
    const problem = additionProblem();
    const onSelectDigit = vi.fn();
    const { rerender } = render(
      <ColumnArithmeticAnswerInput
        problem={problem}
        problemNumber={1}
        slot="single"
        value={{ type: 'empty' }}
        selectedDigit={3}
        readOnly={false}
        onSelectDigit={onSelectDigit}
      />,
    );
    expect(document.activeElement).toBe(screen.getByRole('button', { name: /1番の答え 一の位/ }));

    rerender(
      <ColumnArithmeticAnswerInput
        problem={problem}
        problemNumber={1}
        slot="single"
        value={{ type: 'empty' }}
        selectedDigit={2}
        readOnly={false}
        onSelectDigit={onSelectDigit}
      />,
    );
    expect(document.activeElement).toBe(screen.getByRole('button', { name: /1番の答え 十の位/ }));
  });

  it('marks canonical digits as an in-grid correction instead of rendering a separate answer string', () => {
    const problem = additionProblem();
    const { container } = render(
      <ColumnArithmeticAnswerInput
        problem={problem}
        problemNumber={1}
        slot="single"
        value={problem.canonical_answer}
        selectedDigit={null}
        readOnly
        correction
        onSelectDigit={() => undefined}
      />,
    );
    expect(container.querySelector('.column-digit-answer')).toHaveClass('column-digit-answer-correction');
    expect(screen.getByLabelText('1番の正しい答え 一の位 6')).toHaveTextContent('6');
  });

});
