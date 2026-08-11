'use client';

import 'mathlive';
import { useCallback, useEffect, useRef } from 'react';
import type { MathfieldElement } from 'mathlive';
import { deleteEmptyMathLiveStructureBackward } from '@/components/mathlive-structure';

export { deleteEmptyMathLiveStructureBackward } from '@/components/mathlive-structure';

export type AutoDrillMathfield = MathfieldElement;

export function MathLiveStatic({
  latex,
  className,
  ariaLabel,
  displayStyle = false,
}: {
  latex: string;
  className?: string;
  ariaLabel: string;
  displayStyle?: boolean;
}) {
  return (
    <math-span
      key={latex}
      class={className}
      mode={displayStyle ? 'displaystyle' : 'textstyle'}
      aria-label={ariaLabel}
    >
      {latex}
    </math-span>
  );
}

type MathLiveAnswerInputProps = {
  initialLatex: string;
  frameClassName: string;
  ariaLabel: string;
  selected: boolean;
  readOnly: boolean;
  onSelect: () => void;
  onInputLatex: (mathfield: MathfieldElement, latex: string) => void;
  onCommit: () => void;
  onRegister: (mathfield: MathfieldElement | null) => void;
};

export function MathLiveAnswerInput({
  initialLatex,
  frameClassName,
  ariaLabel,
  selected,
  readOnly,
  onSelect,
  onInputLatex,
  onCommit,
  onRegister,
}: MathLiveAnswerInputProps) {
  const mathfieldRef = useRef<MathfieldElement | null>(null);
  const initialLatexRef = useRef(initialLatex);
  const onRegisterRef = useRef(onRegister);
  const readOnlyRef = useRef(readOnly);
  onRegisterRef.current = onRegister;
  readOnlyRef.current = readOnly;

  const attach = useCallback((mathfield: MathfieldElement | null) => {
    mathfieldRef.current = mathfield;
    onRegisterRef.current(mathfield);
    if (!mathfield) return;

    mathfield.mathVirtualKeyboardPolicy = 'manual';
    mathfield.smartMode = false;
    mathfield.smartFence = false;
    mathfield.scriptDepth = 0;
    mathfield.minFontScale = 0.72;
    // MathLive's default placeholder glyph (U+25A2, ▢) loses its top edge
    // when reduced inside a fraction numerator on Chrome. U+2610 uses the
    // same public placeholder mechanism, stays unclipped, and its visible ink
    // is vertically centered much more closely to ordinary digits in fractions. The serialized value is still
    // \placeholder{}, so this is presentation-only.
    mathfield.placeholderSymbol = '☐';
    mathfield.removeExtraneousParentheses = false;
    mathfield.readOnly = readOnlyRef.current;
    mathfield.setValue(initialLatexRef.current, { silenceNotifications: true });
  }, []);

  useEffect(() => {
    if (mathfieldRef.current) mathfieldRef.current.readOnly = readOnly;
  }, [readOnly]);

  useEffect(() => {
    if (selected && !readOnly) mathfieldRef.current?.focus();
  }, [readOnly, selected]);

  return (
    <span
      className={`${frameClassName} ${readOnly ? 'answer-box-readonly' : ''}`}
      onClick={readOnly ? undefined : () => {
        onSelect();
        mathfieldRef.current?.focus();
      }}
    >
      <math-field
        ref={attach}
        class={`answer-mathfield ${selected ? 'answer-mathfield-selected' : ''}`}
        role="textbox"
        aria-label={ariaLabel}
        aria-readonly={readOnly ? 'true' : 'false'}
        read-only={readOnly ? '' : undefined}
        onFocus={readOnly ? undefined : onSelect}
        onKeyDown={(event) => {
          if (readOnly) {
            event.preventDefault();
            return;
          }
          if (event.key === 'Backspace' && deleteEmptyMathLiveStructureBackward(event.currentTarget)) {
            event.preventDefault();
            // MathLive's programmatic deleteBackward updates the field value but
            // does not emit an input event in this path. Keep the Rust AnswerNode
            // authority synchronized with the value that is now visible.
            onInputLatex(event.currentTarget, event.currentTarget.value);
            return;
          }
          if (event.key !== 'Enter') return;
          event.preventDefault();
          onCommit();
        }}
        onInput={(event) => {
          if (!readOnly) onInputLatex(event.currentTarget, event.currentTarget.value);
        }}
        onBeforeInput={(event) => {
          if (readOnly) {
            event.preventDefault();
            return;
          }
          const inputEvent = event.nativeEvent as InputEvent;
          if (inputEvent.inputType !== 'insertLineBreak' && inputEvent.data !== 'insertLineBreak') return;
          event.preventDefault();
          onCommit();
        }}
      />
    </span>
  );
}
