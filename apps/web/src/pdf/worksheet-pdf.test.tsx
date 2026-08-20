import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { A4_PAGE, buildSharedWorksheetLayout, getCellTopPosition } from '@/domain/layout';
import { DRILL_SCHEMA_VERSION, type AnswerNode, type ProblemDto, type ProblemPrompt, type WorksheetDto } from '@/domain/drill-engine';
import { THEME_DEFINITIONS, type ThemeDefinition } from '@/domain/theme-registry';
import {
  buildPdfPageModel,
  openWorksheetPdf,
  WorksheetPrintDocument,
} from '@/pdf/worksheet-pdf';
import { fixtureWorksheet, linearFixtureWorksheet } from '@/test/fixtures';
import type { WorksheetMetadata } from '@/domain/worksheet-metadata';

function representativeAnswer(definition: ThemeDefinition): AnswerNode {
  if (definition.promptKind === 'liar_puzzle') {
    return { type: 'tuple', value: [{ type: 'integer', value: '1' }, { type: 'integer', value: '3' }] };
  }
  switch (definition.answerSchemaKind) {
    case 'integer': return { type: 'integer', value: '6' };
    case 'rational': return { type: 'fraction', value: { numerator: { type: 'integer', value: '3' }, denominator: { type: 'integer', value: '4' } } };
    case 'decimal': return { type: 'exact_decimal', value: { coefficient: '36', scale: 1 } };
    case 'ordered_pair': return { type: 'tuple', value: [{ type: 'integer', value: '2' }, { type: 'integer', value: '3' }] };
    case 'ordered_tuple': return { type: 'tuple', value: [1, 2, 3, 4, 3, 4, 1, 2, 2, 1, 4, 3, 4, 3, 2, 1].map((value) => ({ type: 'integer' as const, value: String(value) })) };
    case 'algebraic': return { type: 'plus_minus', value: { type: 'integer', value: '2' } };
  }
}

function representativeArithmeticExpression(definition: ThemeDefinition): Extract<ProblemPrompt, { kind: 'arithmetic' }>['expression'] {
  const operator = definition.tags.includes('division')
    ? 'divide'
    : definition.tags.includes('multiplication')
      ? 'multiply'
      : definition.tags.includes('subtraction')
        ? 'subtract'
        : 'add';
  if (definition.tags.includes('fractions')) {
    return {
      kind: 'binary', operator,
      left: { kind: 'rational', value: { numerator: 1, denominator: 3 } },
      right: { kind: 'rational', value: { numerator: 1, denominator: 4 } },
    };
  }
  if (definition.tags.includes('decimals')) {
    return {
      kind: 'binary', operator,
      left: { kind: 'exact_decimal', coefficient: 36, scale: 1 },
      right: { kind: 'exact_decimal', coefficient: 12, scale: 1 },
    };
  }
  const left = definition.tags.includes('negative_numbers') ? -6 : 12;
  return { kind: 'binary', operator, left: { kind: 'integer', value: left }, right: { kind: 'integer', value: 3 } };
}

function representativeColumnPrompt(definition: ThemeDefinition): Extract<ProblemPrompt, { kind: 'column_arithmetic' }> {
  const operator = definition.tags.includes('division')
    ? 'divide'
    : definition.tags.includes('multiplication')
      ? 'multiply'
      : definition.tags.includes('subtraction')
        ? 'subtract'
        : 'add';
  const decimal = definition.tags.includes('decimals');
  return {
    kind: 'column_arithmetic',
    operator,
    left: decimal ? { kind: 'exact_decimal', coefficient: 72, scale: 1 } : { kind: 'integer', value: 72 },
    right: decimal ? { kind: 'exact_decimal', coefficient: 6, scale: 1 } : { kind: 'integer', value: 6 },
  };
}

function representativePrompt(definition: ThemeDefinition): { prompt: ProblemPrompt; answer: AnswerNode } {
  const answer = representativeAnswer(definition);
  switch (definition.promptKind) {
    case 'addition':
      return { prompt: { kind: 'addition', left: 7, right: 8 }, answer };
    case 'arithmetic':
      return { prompt: { kind: 'arithmetic', expression: representativeArithmeticExpression(definition) }, answer };
    case 'linear_equation':
      return {
        prompt: {
          kind: 'linear_equation',
          a: { numerator: 2, denominator: 1 }, b: { numerator: 3, denominator: 1 },
          c: { numerator: 1, denominator: 1 }, d: { numerator: 8, denominator: 1 },
          left_negative_constant_as_subtraction: false, right_negative_constant_as_subtraction: false,
        },
        answer,
      };
    case 'quadratic_equation':
      return {
        prompt: {
          kind: 'quadratic_equation', form: 'standard',
          a: { numerator: 1, denominator: 1 }, b: { numerator: 0, denominator: 1 }, c: { numerator: -4, denominator: 1 },
        },
        answer,
      };
    case 'simultaneous_equation':
      return { prompt: { kind: 'simultaneous_equation', a: 2, b: 1, c: 7, d: 1, e: -1, f: -1 }, answer };
    case 'liar_puzzle':
      return {
        prompt: { kind: 'liar_puzzle', people_count: 4, statements: [{ kind: 'says_liar', person: 2 }] },
        answer,
      };
    case 'column_arithmetic':
      return { prompt: representativeColumnPrompt(definition), answer };
    case 'mini_sudoku':
      return { prompt: { kind: 'mini_sudoku', givens: [1, null, null, 4, null, 4, 1, null, null, 1, 4, null, 4, null, null, 1] }, answer };
  }
}

function representativeWorkedSolution(
  prompt: ProblemPrompt,
): ProblemDto['worked_solution'] {
  if (prompt.kind !== 'column_arithmetic') return null;
  if (prompt.operator === 'multiply') {
    return {
      kind: 'column_multiplication',
      partial_products: [{ value: 432, place: 0 }, { value: 72, place: 1 }],
    };
  }
  if (prompt.operator === 'divide') {
    return {
      kind: 'long_division',
      divisor: 6,
      dividend_coefficient: 72,
      dividend_scale: 0,
      quotient_trailing_cells: 0,
      steps: [{ product: 72, after: 0, product_offset: 0, after_offset: 0 }],
    };
  }
  return null;
}

function representativeWorksheet(definition: ThemeDefinition): WorksheetDto {
  const representative = representativePrompt(definition);
  const answerSchema = definition.answerSchemaKind === 'integer'
    ? { kind: 'integer' as const, min: '-100', max: '100' }
    : definition.answerSchemaKind === 'rational'
      ? { kind: 'rational' as const, max_abs_numerator: 72, max_denominator: 72, require_reduced_fraction_form: true }
      : definition.answerSchemaKind === 'decimal'
        ? { kind: 'decimal' as const, max_scale: 6 }
        : definition.answerSchemaKind === 'ordered_pair'
          ? { kind: 'ordered_pair' as const }
          : definition.answerSchemaKind === 'ordered_tuple'
            ? { kind: 'ordered_tuple' as const, length: 16 }
            : { kind: 'algebraic' as const };
  const problems: ProblemDto[] = Array.from({ length: definition.problemCount }, (_, index) => ({
    schema_version: DRILL_SCHEMA_VERSION,
    id: index + 1,
    problem_id: String(index + 1),
    numeric_theme_id: definition.numeric_theme_id,
    prompt: representative.prompt,
    input_interface: definition.inputInterface,
    answer_schema: answerSchema,
    canonical_answer: representative.answer,
    worked_solution: representativeWorkedSolution(representative.prompt),
  }));
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    problem_set_id: `${DRILL_SCHEMA_VERSION}-${definition.numeric_theme_id}-${definition.generator_revision}-PdfTest1-3`,
    identity: { schema_version: DRILL_SCHEMA_VERSION, numeric_theme_id: definition.numeric_theme_id, generator_revision: definition.generator_revision, seed: 'PdfTest1', difficulty: 3 },
    skill_id: definition.themeKey,
    curriculum_path: definition.curriculumPath.map((segment) => segment.label),
    layout: definition.layout,
    problems,
    seed: 'PdfTest1',
  };
}

describe('shared worksheet layout and browser-native PDF printing', () => {
  const metadata: WorksheetMetadata = { generated_date: '2026-07-30', seed: 'repeatMe' };

  it('uses the same 2 x 10 model for web geometry and printable pages', () => {
    const worksheet = fixtureWorksheet();
    const layout = buildSharedWorksheetLayout(worksheet);
    const pages = buildPdfPageModel(worksheet, metadata);
    expect(layout.cells).toHaveLength(20);
    expect(layout.cells[0]).toMatchObject({ column: 0, row: 0 });
    expect(layout.cells[9]).toMatchObject({ column: 0, row: 9 });
    expect(layout.cells[10]).toMatchObject({ column: 1, row: 0 });
    expect(layout.cells[19]).toMatchObject({ column: 1, row: 9 });
    const firstPosition = getCellTopPosition(layout, layout.cells[0]!);
    const lastPosition = getCellTopPosition(layout, layout.cells[19]!);
    expect(firstPosition.y).toBeCloseTo(A4_PAGE.margin + A4_PAGE.headerHeight, 5);
    expect(lastPosition.y + lastPosition.height).toBeCloseTo(A4_PAGE.height - A4_PAGE.margin - A4_PAGE.footerHeight, 5);
    expect(pages).toHaveLength(2);
    expect(pages[0]).toMatchObject({ kind: 'problems', rotated: false, title: '1けたのたしざん(1)' });
    expect(pages[1]).toMatchObject({ kind: 'answers', rotated: true, title: '1けたのたしざん(1) 解答' });
    expect(pages[0]?.footer).toEqual({ text: 'date: 2026-07-30 / seed: repeatMe', physical_corner: 'bottom-right' });
  });

  it('derives a row-major 4 x 4 printable column-arithmetic model without visible vertical dividers', () => {
    const definition = THEME_DEFINITIONS.find((theme) => theme.numeric_theme_id === 25)!;
    const worksheet = representativeWorksheet(definition);
    const layout = buildSharedWorksheetLayout(worksheet);
    const { container } = render(<WorksheetPrintDocument worksheet={worksheet} />);
    expect(layout.cells).toHaveLength(16);
    expect(layout.cells[3]).toMatchObject({ column: 3, row: 0 });
    expect(layout.cells[4]).toMatchObject({ column: 0, row: 1 });
    expect(layout.cells[12]).toMatchObject({ column: 0, row: 3 });
    expect(layout.dividerXs).toHaveLength(3);
    expect(container.querySelectorAll('[data-print-page="problems"] .problem-divider')).toHaveLength(0);
    expect(container.querySelectorAll('[data-print-page="problems"] .problem-cell-column-arithmetic')).toHaveLength(16);
    expect(container.querySelectorAll('[data-print-page="problems"] .worksheet-print-empty-answer')).toHaveLength(16);
    expect(container.querySelectorAll('[data-print-page="answers"] .worksheet-print-empty-answer')).toHaveLength(0);
    expect(container.querySelectorAll('[data-print-page="answers"] [data-column-solution="true"]')).toHaveLength(16);
  });

  it('prints completed multiplication and long-division working on the answer page', () => {
    const multiplication = representativeWorksheet(THEME_DEFINITIONS.find((theme) => theme.numeric_theme_id === 30)!);
    const multiplicationRender = render(<WorksheetPrintDocument worksheet={multiplication} />);
    expect(multiplicationRender.container.querySelectorAll('[data-print-page="answers"] .column-multiply-partial')).toHaveLength(32);
    expect(multiplicationRender.container.querySelectorAll('[data-print-page="answers"] [data-column-solution="true"]')).toHaveLength(16);
    multiplicationRender.unmount();

    const division = representativeWorksheet(THEME_DEFINITIONS.find((theme) => theme.numeric_theme_id === 32)!);
    const divisionRender = render(<WorksheetPrintDocument worksheet={division} />);
    expect(divisionRender.container.querySelectorAll('[data-print-page="problems"] .column-division-answer-coordinate-remainder')).toHaveLength(0);
    expect(divisionRender.container.querySelectorAll('[data-print-page="problems"] .worksheet-print-empty-answer')).toHaveLength(12);
    expect(divisionRender.container.querySelectorAll('[data-print-page="answers"] .column-division-solution-step').length).toBeGreaterThanOrEqual(12);
    expect(divisionRender.container.querySelectorAll('[data-print-page="answers"] .problem-answer-area-column-division')).toHaveLength(0);
    const problemCell = divisionRender.container.querySelector<HTMLElement>('[data-print-page="problems"] .problem-cell-column-arithmetic-divide');
    const answerCell = divisionRender.container.querySelector<HTMLElement>('[data-print-page="answers"] .problem-cell-column-arithmetic-divide');
    expect(problemCell?.style.getPropertyValue('--column-digit-width')).toContain('var(--worksheet-grid-cell)');
    expect(problemCell?.style.getPropertyValue('--column-division-active-width')).not.toBe('');
    expect(answerCell?.style.getPropertyValue('--column-division-active-width')).toBe(problemCell?.style.getPropertyValue('--column-division-active-width'));
    divisionRender.unmount();
  });

  it('derives the 2 x 8 linear-equation print model from the same registry layout', () => {
    const worksheet = linearFixtureWorksheet(2);
    const layout = buildSharedWorksheetLayout(worksheet);
    const pages = buildPdfPageModel(worksheet);
    expect(layout.cells).toHaveLength(16);
    expect(layout.cells[7]).toMatchObject({ column: 0, row: 7 });
    expect(layout.cells[8]).toMatchObject({ column: 1, row: 0 });
    expect(pages[0]?.cells[0]?.expression).toBe('2x = x + (−5)');
    expect(pages[1]?.cells[0]?.answer).toBe('-5');
  });

  it('renders every registered theme through the same shared problem presentation used by Web', () => {
    for (const definition of THEME_DEFINITIONS) {
      const worksheet = representativeWorksheet(definition);
      const { container, unmount } = render(<WorksheetPrintDocument worksheet={worksheet} metadata={metadata} />);
      expect(container.querySelectorAll('[data-print-page]'), definition.label).toHaveLength(2);
      const expressions = container.querySelectorAll('math-span.problem-math-expression');
      if (definition.promptKind === 'liar_puzzle') {
        expect(expressions, definition.label).toHaveLength(0);
        expect(container.querySelectorAll('.liar-statements'), definition.label).toHaveLength(definition.problemCount * 2);
        expect(container.querySelectorAll('.liar-person-choice'), definition.label).toHaveLength(definition.problemCount * 4 * 2);
        expect(container.querySelectorAll('[data-print-page="answers"] .liar-person-choice-selected'), definition.label).toHaveLength(definition.problemCount * 2);
      } else if (definition.promptKind === 'mini_sudoku') {
        expect(expressions, definition.label).toHaveLength(0);
        expect(container.querySelectorAll('.mini-sudoku-grid'), definition.label).toHaveLength(definition.problemCount * 2);
        expect(container.querySelectorAll('[data-print-page=\"problems\"] button.digit-grid-cell'), definition.label).toHaveLength(0);
        expect(container.querySelectorAll('[data-print-page=\"answers\"] .digit-grid-cell-value'), definition.label).toHaveLength(definition.problemCount * 16);
        expect(container.querySelectorAll('.problem-grid-worksheet-grid'), definition.label).toHaveLength(2);
      } else {
        const isColumnArithmetic = definition.presentation.column_arithmetic;
        expect(expressions, definition.label).toHaveLength(isColumnArithmetic ? 0 : definition.problemCount * 2);
        if (isColumnArithmetic) {
          expect(container.querySelectorAll('[data-column-arithmetic]'), definition.label).toHaveLength(definition.problemCount * 2);
          const cells = [...container.querySelectorAll<HTMLElement>('.problem-cell-column-arithmetic')];
          expect(cells.every((cell) => cell.style.getPropertyValue('--column-digit-width').includes('var(--worksheet-grid-cell)')), definition.label).toBe(true);
          expect(container.querySelectorAll('.problem-grid-worksheet-grid'), definition.label).toHaveLength(2);
          expect(container.querySelectorAll('.column-arithmetic-digit-cell').length, definition.label).toBeGreaterThan(0);
        }
        if (isColumnArithmetic) {
          expect(container.querySelectorAll('[data-print-page="problems"] .worksheet-print-empty-answer'), definition.label).toHaveLength(definition.problemCount);
          expect(container.querySelectorAll('[data-print-page="answers"] [data-column-solution="true"]'), definition.label).toHaveLength(definition.problemCount);
          expect(container.querySelectorAll('[data-print-page="answers"] .worksheet-print-answer-value'), definition.label).toHaveLength(0);
          expect(container.querySelectorAll('[data-print-page="problems"] .column-division-answer-coordinate-remainder'), definition.label).toHaveLength(0);
        } else {
          const answerFieldCount = definition.numeric_theme_id === 19 || definition.answerSchemaKind === 'ordered_pair'
            ? definition.problemCount * 2
            : definition.problemCount;
          expect(container.querySelectorAll('.worksheet-print-empty-answer'), definition.label).toHaveLength(answerFieldCount);
          expect(container.querySelectorAll('math-span.worksheet-print-answer-value'), definition.label).toHaveLength(answerFieldCount);
        }
      }
      unmount();
    }
  });


  it('stacks signed-arithmetic answers below the expression without a trailing equals sign', () => {
    const signed = representativeWorksheet(THEME_DEFINITIONS.find((definition) => definition.numeric_theme_id === 7)!);
    const { container } = render(<WorksheetPrintDocument worksheet={signed} />);
    const firstProblem = container.querySelector('[data-print-page="problems"] [data-print-problem-index="0"]');
    expect(firstProblem).toHaveClass('problem-cell-answer-below');
    expect(firstProblem?.querySelector('math-span.problem-math-expression')?.textContent).not.toContain('=');
    expect(container.querySelector('[data-print-page="problems"] .worksheet-instruction')).toHaveTextContent('次の式を計算しなさい。');
  });

  it('keeps the standard unlike-denominator fraction as MathLive LaTeX instead of flattening it to slash text', () => {
    const fraction = representativeWorksheet(THEME_DEFINITIONS.find((definition) => definition.numeric_theme_id === 9)!);
    const { container } = render(<WorksheetPrintDocument worksheet={fraction} />);
    const expression = container.querySelector('math-span.problem-math-expression');
    expect(expression?.textContent).toContain('\\frac{1}{3}');
    expect(expression?.textContent).toContain('\\frac{1}{4}');
    expect(expression?.textContent).not.toContain('1/3');
  });

  it('opens an in-app preview and invokes native printing only after explicit confirmation', async () => {
    const print = vi.spyOn(window, 'print').mockImplementation(() => undefined);
    await openWorksheetPdf(fixtureWorksheet(), metadata);

    expect(print).not.toHaveBeenCalled();
    expect(await screen.findByRole('dialog', { name: '印刷プレビュー' })).toBeInTheDocument();
    expect(document.querySelectorAll('[data-print-page]')).toHaveLength(2);

    const printButton = screen.getByRole('button', { name: '印刷する' });
    expect(printButton).toBeEnabled();
    fireEvent.click(printButton);
    await waitFor(() => expect(print).toHaveBeenCalledTimes(1));
    expect(screen.getByRole('dialog', { name: '印刷プレビュー' })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '戻る' }));
    expect(document.querySelector('.worksheet-print-host-preview')).toBeNull();
    print.mockRestore();
  });
});
