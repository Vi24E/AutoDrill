import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach, vi } from 'vitest';

class TestMathfieldElement extends HTMLElement {
  private latex = '';
  private cursor = 0;
  readOnly = false;
  mathVirtualKeyboardPolicy = 'manual';
  smartMode = false;
  smartFence = false;
  scriptDepth = 0;
  minFontScale = 0.5;
  placeholderSymbol = '☐';
  removeExtraneousParentheses = false;

  get value() {
    return this.latex;
  }

  set value(value: string) {
    this.latex = String(value);
    this.cursor = this.latex.length;
  }

  setValue(value: string) {
    this.value = value;
  }

  insert(value: string) {
    if (this.readOnly) return;
    const latex = String(value).replaceAll('#?', '\\placeholder{}');
    const placeholder = this.placeholderAtOrAfterCursor();
    if (placeholder) {
      this.latex = `${this.latex.slice(0, placeholder.start)}${latex}${this.latex.slice(placeholder.end)}`;
      this.cursor = placeholder.start + latex.length;
    } else {
      this.latex = `${this.latex.slice(0, this.cursor)}${latex}${this.latex.slice(this.cursor)}`;
      this.cursor += latex.length;
    }
    const firstInsertedPlaceholder = this.latex.indexOf('\\placeholder{}', Math.max(0, this.cursor - latex.length));
    if (firstInsertedPlaceholder >= 0 && firstInsertedPlaceholder < this.cursor) this.cursor = firstInsertedPlaceholder;
    this.emitInput();
  }

  executeCommand(command: string) {
    if (this.readOnly) return false;
    if (command === 'moveToPreviousChar') {
      this.cursor = Math.max(0, this.cursor - 1);
      return true;
    }
    if (command === 'moveToNextChar') {
      const nextPlaceholder = this.latex.indexOf('\\placeholder{}', this.cursor + 1);
      this.cursor = nextPlaceholder >= 0 ? nextPlaceholder : Math.min(this.latex.length, this.cursor + 1);
      return true;
    }
    if (command === 'deleteAll') {
      this.latex = '';
      this.cursor = 0;
      this.emitInput();
      return true;
    }
    if (command === 'deleteBackward') {
      const placeholder = this.placeholderAtCursor();
      if (placeholder) {
        // MathLive owns structural deletion in production. The test double
        // models deleting an entirely empty template as one editor action.
        this.latex = '';
        this.cursor = 0;
      } else if (this.cursor > 0) {
        this.latex = `${this.latex.slice(0, this.cursor - 1)}${this.latex.slice(this.cursor)}`;
        this.cursor -= 1;
      }
      this.emitInput();
      return true;
    }
    if (command === 'deleteForward') {
      if (this.cursor < this.latex.length) {
        this.latex = `${this.latex.slice(0, this.cursor)}${this.latex.slice(this.cursor + 1)}`;
        this.emitInput();
      }
      return true;
    }
    return true;
  }

  connectedCallback() {
    this.tabIndex = 0;
    this.addEventListener('keydown', this.onKeyDown);
  }

  disconnectedCallback() {
    this.removeEventListener('keydown', this.onKeyDown);
  }

  private onKeyDown = (event: KeyboardEvent) => {
    if (this.readOnly) return;
    if (/^[0-9.]$/.test(event.key)) {
      event.preventDefault();
      this.insert(event.key);
    } else if (event.key === 'ArrowLeft') {
      event.preventDefault();
      this.executeCommand('moveToPreviousChar');
    } else if (event.key === 'ArrowRight') {
      event.preventDefault();
      this.executeCommand('moveToNextChar');
    } else if (event.key === 'Backspace') {
      event.preventDefault();
      this.executeCommand('deleteBackward');
    } else if (event.key === 'Delete') {
      event.preventDefault();
      this.executeCommand('deleteForward');
    }
  };

  private placeholderAtCursor() {
    const token = '\\placeholder{}';
    if (this.latex.startsWith(token, this.cursor)) return { start: this.cursor, end: this.cursor + token.length };
    return null;
  }

  private placeholderAtOrAfterCursor() {
    const current = this.placeholderAtCursor();
    if (current) return current;
    return null;
  }

  private emitInput() {
    this.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText' }));
  }
}

if (typeof window !== 'undefined' && window.customElements && !window.customElements.get('math-field')) {
  window.customElements.define('math-field', TestMathfieldElement);
}
if (typeof window !== 'undefined' && window.customElements && !window.customElements.get('math-span')) {
  window.customElements.define('math-span', class extends HTMLElement {});
}

if (typeof window !== 'undefined' && typeof window.localStorage?.clear !== 'function') {
  const values = new Map<string, string>();
  Object.defineProperty(window, 'localStorage', {
    configurable: true,
    value: {
      clear: () => values.clear(),
      getItem: (key: string) => values.get(key) ?? null,
      removeItem: (key: string) => values.delete(key),
      setItem: (key: string, value: string) => values.set(key, String(value)),
    } satisfies Pick<Storage, 'clear' | 'getItem' | 'removeItem' | 'setItem'>,
  });
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});
