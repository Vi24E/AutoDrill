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
  /** Column-arithmetic answers use the same tabular sans digits as the printed algorithm. */
  numericSansFont?: boolean;
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
  numericSansFont = false,
}: MathLiveAnswerInputProps) {
  const mathfieldRef = useRef<MathfieldElement | null>(null);
  const initialLatexRef = useRef(initialLatex);
  const onRegisterRef = useRef(onRegister);
  const readOnlyRef = useRef(readOnly);
  const numericSansFontRef = useRef(numericSansFont);
  const numericGlyphObserverRef = useRef<MutationObserver | null>(null);
  onRegisterRef.current = onRegister;
  readOnlyRef.current = readOnly;
  numericSansFontRef.current = numericSansFont;

  const attach = useCallback((mathfield: MathfieldElement | null) => {
    numericGlyphObserverRef.current?.disconnect();
    numericGlyphObserverRef.current = null;
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
    if (numericSansFontRef.current) {
      const installNumericGrid = () => {
        const root = mathfield.shadowRoot;
        if (!root) return;
        if (!root.querySelector('style[data-autodrill-numeric-font]')) {
          const style = document.createElement('style');
          style.dataset.autodrillNumericFont = 'true';
          style.textContent = `
            .ML__base, .ML__latex, .ML__cmr, .ML__mathbf {
              font-family: 'Noto Sans JP', system-ui, sans-serif !important;
              font-variant-numeric: tabular-nums !important;
              font-feature-settings: 'tnum' 1 !important;
            }
            .ML__cmr {
              display: inline-flex !important;
              box-sizing: border-box !important;
              width: var(--column-digit-cell, 20px) !important;
              min-width: var(--column-digit-cell, 20px) !important;
              justify-content: center !important;
              letter-spacing: 0 !important;
              position: relative !important;
            }
            .ML__cmr[data-autodrill-decimal-marker="true"] {
              width: 0 !important;
              min-width: 0 !important;
              overflow: visible !important;
              color: transparent !important;
            }
            .ML__cmr[data-autodrill-decimal-marker="true"]::after {
              position: absolute;
              left: -2px;
              bottom: 0;
              width: 4px;
              height: 4px;
              border-radius: 50%;
              background: #111;
              content: '';
            }
          `;
          root.append(style);
        }
        const markDecimalGlyphs = () => {
          for (const glyph of root.querySelectorAll<HTMLElement>('.ML__cmr')) {
            if (glyph.textContent === '.') glyph.dataset.autodrillDecimalMarker = 'true';
            else delete glyph.dataset.autodrillDecimalMarker;
          }
        };
        markDecimalGlyphs();
        if (!numericGlyphObserverRef.current) {
          const observer = new MutationObserver(markDecimalGlyphs);
          observer.observe(root, { childList: true, subtree: true, characterData: true });
          numericGlyphObserverRef.current = observer;
        }
      };
      installNumericGrid();
      queueMicrotask(installNumericGrid);
    }
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
