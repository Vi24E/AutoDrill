'use client';

import { createPortal } from 'react-dom';
import { useCallback, useEffect, useId, useRef, useState, type CSSProperties, type KeyboardEvent, type ReactNode } from 'react';

export type CustomSelectOption = {
  value: string;
  label: string;
};

type CustomSelectProps = {
  id: string;
  ariaLabel: string;
  value: string;
  options: readonly CustomSelectOption[];
  onChange: (value: string) => void;
  renderLabel?: (option: CustomSelectOption) => ReactNode;
  renderValue?: (option: CustomSelectOption) => ReactNode;
  renderOptionEnd?: (option: CustomSelectOption) => ReactNode;
};

type PopupLayout = {
  left: number;
  width: number;
  maxHeight: number;
  top?: number;
  bottom?: number;
};

const POPUP_GAP = 5;
const VIEWPORT_MARGIN = 10;
const POPUP_MAX_HEIGHT = 300;
const POPUP_MIN_USEFUL_HEIGHT = 120;

/** Accessible custom combobox so option rows may contain ruby markup. */
export function CustomSelect({ id, ariaLabel, value, options, onChange, renderLabel, renderValue, renderOptionEnd }: CustomSelectProps) {
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);
  const [popupLayout, setPopupLayout] = useState<PopupLayout | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listboxRef = useRef<HTMLDivElement>(null);
  const listboxId = `${id}-${useId().replace(/:/g, '')}-listbox`;
  const selectedIndex = Math.max(0, options.findIndex((option) => option.value === value));
  const selected = options[selectedIndex] ?? options[0];

  const positionPopup = useCallback(() => {
    const trigger = triggerRef.current;
    if (!trigger || typeof window === 'undefined') return;
    const rect = trigger.getBoundingClientRect();
    const availableBelow = Math.max(0, window.innerHeight - rect.bottom - POPUP_GAP - VIEWPORT_MARGIN);
    const availableAbove = Math.max(0, rect.top - POPUP_GAP - VIEWPORT_MARGIN);
    const openBelow = availableBelow >= POPUP_MIN_USEFUL_HEIGHT || availableBelow >= availableAbove;
    const available = openBelow ? availableBelow : availableAbove;
    const maxHeight = Math.max(64, Math.min(POPUP_MAX_HEIGHT, available));
    const left = Math.min(
      Math.max(VIEWPORT_MARGIN, rect.left),
      Math.max(VIEWPORT_MARGIN, window.innerWidth - VIEWPORT_MARGIN - rect.width),
    );
    setPopupLayout(openBelow
      ? { left, width: rect.width, maxHeight, top: rect.bottom + POPUP_GAP }
      : { left, width: rect.width, maxHeight, bottom: window.innerHeight - rect.top + POPUP_GAP });
  }, []);

  useEffect(() => {
    if (!open) return undefined;
    const closeOnOutsidePointer = (event: PointerEvent) => {
      const target = event.target as Node;
      if (!rootRef.current?.contains(target) && !listboxRef.current?.contains(target)) setOpen(false);
    };
    const reposition = () => positionPopup();
    document.addEventListener('pointerdown', closeOnOutsidePointer);
    window.addEventListener('resize', reposition);
    window.addEventListener('scroll', reposition, true);
    return () => {
      document.removeEventListener('pointerdown', closeOnOutsidePointer);
      window.removeEventListener('resize', reposition);
      window.removeEventListener('scroll', reposition, true);
    };
  }, [open, positionPopup]);

  useEffect(() => {
    if (!open) setActiveIndex(selectedIndex);
  }, [open, selectedIndex]);

  useEffect(() => {
    if (!open) return;
    const option = document.getElementById(`${listboxId}-option-${activeIndex}`);
    if (option && typeof option.scrollIntoView === 'function') option.scrollIntoView({ block: 'nearest' });
  }, [activeIndex, listboxId, open]);

  const optionId = (index: number) => `${listboxId}-option-${index}`;
  const openAt = (index: number) => {
    setActiveIndex(Math.min(Math.max(index, 0), Math.max(0, options.length - 1)));
    positionPopup();
    setOpen(true);
  };
  const choose = (index: number) => {
    const option = options[index];
    if (!option) return;
    onChange(option.value);
    setActiveIndex(index);
    setOpen(false);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (options.length === 0) return;
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      if (!open) openAt(Math.min(selectedIndex + 1, options.length - 1));
      else setActiveIndex((index) => Math.min(index + 1, options.length - 1));
      return;
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      if (!open) openAt(Math.max(selectedIndex - 1, 0));
      else setActiveIndex((index) => Math.max(index - 1, 0));
      return;
    }
    if (event.key === 'Home' && open) { event.preventDefault(); setActiveIndex(0); return; }
    if (event.key === 'End' && open) { event.preventDefault(); setActiveIndex(options.length - 1); return; }
    if (event.key === 'Escape' && open) { event.preventDefault(); setOpen(false); return; }
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      if (open) choose(activeIndex);
      else openAt(selectedIndex);
    }
  };

  const listboxStyle: CSSProperties | undefined = popupLayout ? {
    position: 'fixed',
    left: popupLayout.left,
    width: popupLayout.width,
    maxHeight: popupLayout.maxHeight,
    top: popupLayout.top,
    bottom: popupLayout.bottom,
  } : undefined;

  return (
    <div className="custom-select" ref={rootRef}>
      <button
        ref={triggerRef}
        id={id}
        type="button"
        className="custom-select-trigger"
        role="combobox"
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listboxId}
        data-selected-label={selected?.label ?? ''}
        aria-activedescendant={open ? optionId(activeIndex) : undefined}
        data-value={selected?.value ?? ''}
        onClick={() => open ? setOpen(false) : openAt(selectedIndex)}
        onKeyDown={onKeyDown}
      >
        <span className="custom-select-value">{selected ? (renderValue?.(selected) ?? renderLabel?.(selected) ?? selected.label) : null}</span>
        <svg className="custom-select-chevron" viewBox="0 0 12 8" aria-hidden="true"><path d="M1 1.5 6 6.5 11 1.5" /></svg>
      </button>
      {open && popupLayout && typeof document !== 'undefined' ? createPortal(
        <div
          ref={listboxRef}
          id={listboxId}
          className="custom-select-listbox"
          role="listbox"
          aria-label={`${ariaLabel}の選択肢`}
          style={listboxStyle}
        >
          {options.map((option, index) => (
            <div
              id={optionId(index)}
              key={option.value}
              className={`custom-select-option ${index === activeIndex ? 'custom-select-option-active' : ''}`}
              role="option"
              aria-label={option.label}
              aria-selected={option.value === value}
              onPointerMove={() => setActiveIndex(index)}
              onPointerDown={(event) => event.preventDefault()}
              onClick={() => choose(index)}
            >
              <span className="custom-select-option-label">{renderLabel?.(option) ?? option.label}</span>
              <span className="custom-select-option-end">
                {renderOptionEnd?.(option)}
                {option.value === value ? <span className="custom-select-check" aria-hidden="true">✓</span> : <span className="custom-select-check-placeholder" aria-hidden="true" />}
              </span>
            </div>
          ))}
        </div>,
        document.body,
      ) : null}
    </div>
  );
}
