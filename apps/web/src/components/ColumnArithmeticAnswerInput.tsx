import { useEffect, useRef, type CSSProperties } from 'react';
import {
  columnDecimalBoundaryFromAnswer,
  columnDigitSpec,
  columnDigitsFromAnswer,
  type ColumnAnswerSlot,
} from '@/domain/column-arithmetic-input';
import type { AnswerNode, ProblemDto } from '@/domain/drill-engine';

type ColumnArithmeticAnswerInputProps = {
  problem: ProblemDto;
  problemNumber: number;
  slot: ColumnAnswerSlot;
  value: AnswerNode;
  draft?: readonly (string | null)[];
  decimalBoundary?: number | null;
  selectedDigit: number | null;
  readOnly: boolean;
  correction?: boolean;
  onSelectDigit: (digitIndex: number) => void;
};

function digitPlaceLabel(spec: ReturnType<typeof columnDigitSpec>, index: number, decimalBoundary: number | null): string {
  if (spec.decimalPoint.type === 'editable' && decimalBoundary === null) {
    return `解答欄${index - spec.activeStart + 1}桁目`;
  }
  const boundary = decimalBoundary ?? (spec.activeEnd + 1);
  const power = boundary - index - 1;
  if (power === 0) return '一の位';
  if (power === 1) return '十の位';
  if (power === 2) return '百の位';
  if (power === 3) return '千の位';
  if (power > 3) return `10の${power}乗の位`;
  return `小数第${-power}位`;
}

export function ColumnArithmeticAnswerInput({
  problem,
  problemNumber,
  slot,
  value,
  draft,
  decimalBoundary,
  selectedDigit,
  readOnly,
  correction = false,
  onSelectDigit,
}: ColumnArithmeticAnswerInputProps) {
  const spec = columnDigitSpec(problem, slot);
  const digits = draft ? [...draft] : columnDigitsFromAnswer(value, spec);
  const displayedDecimalBoundary = decimalBoundary !== undefined
    ? decimalBoundary
    : columnDecimalBoundaryFromAnswer(value, spec);
  const slotLabel = correction ? (slot === 'quotient' ? '正しい商' : '正しい答え') : (slot === 'quotient' ? '商' : '答え');
  const selectedButtonRef = useRef<HTMLButtonElement | null>(null);

  // selectedDigit is the interaction authority. Move the real DOM focus with it
  // so physical-keyboard input does not leave a second stale focus ring behind.
  useEffect(() => {
    if (!readOnly && selectedDigit !== null) selectedButtonRef.current?.focus({ preventScroll: true });
  }, [readOnly, selectedDigit]);

  return (
    <span
      className={`column-digit-answer column-digit-answer-${slot} ${correction ? 'column-digit-answer-correction' : ''}`}
      style={{ '--column-answer-digit-count': spec.cellCount } as CSSProperties}
      data-column-answer-slot={slot}
      data-column-direction={spec.direction}
      data-column-input-order={spec.order}
      data-column-decimal-mode={spec.decimalPoint.type}
    >
      {digits.map((digit, index) => {
        const active = index >= spec.activeStart && index <= spec.activeEnd;
        const className = [
          'column-digit-slot',
          active ? 'column-digit-slot-active' : 'column-digit-slot-inactive',
          selectedDigit === index ? 'column-digit-slot-selected' : '',
          readOnly ? 'column-digit-slot-readonly' : '',
        ].filter(Boolean).join(' ');
        const content = digit === null ? null : <span className="column-digit-glyph">{digit}</span>;
        if (!active || readOnly) {
          return (
            <span
              className={className}
              data-column-digit-index={index}
              key={`digit-${index}`}
              aria-label={active ? `${problemNumber}番の${slotLabel} ${digitPlaceLabel(spec, index, displayedDecimalBoundary)} ${digit ?? '未入力'}` : undefined}
            >{content}</span>
          );
        }
        return (
          <button
            type="button"
            className={className}
            data-column-digit-index={index}
            key={`digit-${index}`}
            ref={selectedDigit === index ? selectedButtonRef : undefined}
            aria-label={`${problemNumber}番の${slotLabel} ${digitPlaceLabel(spec, index, displayedDecimalBoundary)} ${digit ?? '未入力'}`}
            aria-pressed={selectedDigit === index}
            onFocus={() => onSelectDigit(index)}
            onClick={() => onSelectDigit(index)}
          >{content}</button>
        );
      })}
      {displayedDecimalBoundary !== null ? (
        <span
          className="column-digit-decimal-marker"
          style={{ left: `calc(${displayedDecimalBoundary} * var(--worksheet-grid-cell))` }}
          aria-hidden="true"
        />
      ) : null}
    </span>
  );
}
