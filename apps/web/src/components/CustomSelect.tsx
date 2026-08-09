'use client';

import { useEffect, useId, useRef, useState, type KeyboardEvent, type ReactNode } from 'react';

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
};

/** Accessible custom combobox so option rows may contain ruby markup. */
export function CustomSelect({ id, ariaLabel, value, options, onChange, renderLabel }: CustomSelectProps) {
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);
  const rootRef = useRef<HTMLDivElement>(null);
  const listboxId = `${id}-${useId().replace(/:/g, '')}-listbox`;
  const selectedIndex = Math.max(0, options.findIndex((option) => option.value === value));
  const selected = options[selectedIndex] ?? options[0];

  useEffect(() => {
    if (!open) return undefined;
    const closeOnOutsidePointer = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener('pointerdown', closeOnOutsidePointer);
    return () => document.removeEventListener('pointerdown', closeOnOutsidePointer);
  }, [open]);

  useEffect(() => {
    if (!open) setActiveIndex(selectedIndex);
  }, [open, selectedIndex]);

  const optionId = (index: number) => `${listboxId}-option-${index}`;
  const openAt = (index: number) => {
    setActiveIndex(Math.min(Math.max(index, 0), Math.max(0, options.length - 1)));
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

  return (
    <div className="custom-select" ref={rootRef}>
      <button
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
        <span className="custom-select-value">{selected ? (renderLabel?.(selected) ?? selected.label) : null}</span>
        <svg className="custom-select-chevron" viewBox="0 0 12 8" aria-hidden="true"><path d="M1 1.5 6 6.5 11 1.5" /></svg>
      </button>
      {open ? (
        <div id={listboxId} className="custom-select-listbox" role="listbox" aria-label={`${ariaLabel}の選択肢`}>
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
              <span>{renderLabel?.(option) ?? option.label}</span>
              {option.value === value ? <span className="custom-select-check" aria-hidden="true">✓</span> : null}
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}
