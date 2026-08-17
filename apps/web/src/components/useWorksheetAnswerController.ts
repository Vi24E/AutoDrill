import { useCallback, useRef, useState } from 'react';

import type { AutoDrillMathfield } from '@/components/MathLiveMath';
import type { ColumnAnswerSlot } from '@/domain/column-arithmetic-input';
import { initialDigitGridAnswer } from '@/domain/digit-grid-input';
import type { AnswerNode, WorksheetDto } from '@/domain/drill-engine';

export type MathfieldSlot = 'single' | 'x' | 'y' | 'quotient' | 'remainder';
export type ColumnDigitSelection = { problemIndex: number; slot: ColumnAnswerSlot; digitIndex: number };
export type DigitGridSelection = { problemIndex: number; cellIndex: number };

function emptyAnswers(worksheet: WorksheetDto): Record<string, AnswerNode> {
  return Object.fromEntries(
    worksheet.problems.map((problem) => [
      problem.problem_id,
      problem.input_interface.type === 'digit_grid'
        ? initialDigitGridAnswer(problem)
        : ({ type: 'empty' } satisfies AnswerNode),
    ]),
  );
}

/**
 * Owns the mutable answer/input session for one worksheet. The top-level app
 * still orchestrates generation, grading, routing, and printing; this controller
 * centralizes the synchronized React state + imperative refs needed by MathLive
 * callbacks and same-tick keyboard/grading guards.
 */
export function useWorksheetAnswerController() {
  const [answers, setAnswersState] = useState<Record<string, AnswerNode>>({});
  const [selectedIndex, setSelectedIndexState] = useState<number | null>(null);
  const [selectedSlot, setSelectedSlotState] = useState<MathfieldSlot>('single');
  const [selectedColumnDigit, setSelectedColumnDigitState] = useState<ColumnDigitSelection | null>(null);
  const [selectedDigitGridCell, setSelectedDigitGridCellState] = useState<DigitGridSelection | null>(null);
  const [columnDrafts, setColumnDraftsState] = useState<Record<string, Array<string | null>>>({});

  const answersRef = useRef<Record<string, AnswerNode>>({});
  const selectedIndexRef = useRef<number | null>(null);
  const selectedSlotRef = useRef<MathfieldSlot>('single');
  const selectedColumnDigitRef = useRef<ColumnDigitSelection | null>(null);
  const selectedDigitGridCellRef = useRef<DigitGridSelection | null>(null);
  const columnDraftsRef = useRef<Record<string, Array<string | null>>>({});
  const inputEnabledRef = useRef(false);
  const actionQueueRef = useRef(Promise.resolve());
  const mathfieldRefs = useRef(new Map<string, AutoDrillMathfield>());
  const acceptedLatexRef = useRef<Record<string, string>>({});

  const replaceAnswers = useCallback((next: Record<string, AnswerNode>) => {
    answersRef.current = next;
    setAnswersState(next);
  }, []);

  const setAnswer = useCallback((problemId: string, answer: AnswerNode) => {
    const next = { ...answersRef.current, [problemId]: answer };
    answersRef.current = next;
    setAnswersState(next);
    return next;
  }, []);

  const replaceColumnDrafts = useCallback((next: Record<string, Array<string | null>>) => {
    columnDraftsRef.current = next;
    setColumnDraftsState(next);
  }, []);

  const setColumnDraft = useCallback((key: string, draft: Array<string | null>) => {
    const next = { ...columnDraftsRef.current, [key]: draft };
    columnDraftsRef.current = next;
    setColumnDraftsState(next);
    return next;
  }, []);


  const setSelectedIndex = useCallback((index: number | null) => {
    selectedIndexRef.current = index;
    setSelectedIndexState(index);
  }, []);

  const setSelectedSlot = useCallback((slot: MathfieldSlot) => {
    selectedSlotRef.current = slot;
    setSelectedSlotState(slot);
  }, []);

  const setSelectedColumnDigit = useCallback((selection: ColumnDigitSelection | null) => {
    selectedColumnDigitRef.current = selection;
    setSelectedColumnDigitState(selection);
    setSelectedDigitGridCellState(null);
  }, []);

  const setSelectedDigitGridCell = useCallback((selection: DigitGridSelection | null) => {
    selectedDigitGridCellRef.current = selection;
    setSelectedDigitGridCellState(selection);
  }, []);

  const select = useCallback((index: number, slot: MathfieldSlot = 'single') => {
    inputEnabledRef.current = true;
    selectedIndexRef.current = index;
    selectedSlotRef.current = slot;
    selectedColumnDigitRef.current = null;
    selectedDigitGridCellRef.current = null;
    setSelectedIndexState(index);
    setSelectedSlotState(slot);
    setSelectedColumnDigitState(null);
    setSelectedDigitGridCellState(null);
  }, []);

  const selectColumnDigit = useCallback((selection: ColumnDigitSelection) => {
    inputEnabledRef.current = true;
    selectedIndexRef.current = selection.problemIndex;
    selectedSlotRef.current = selection.slot;
    selectedColumnDigitRef.current = selection;
    selectedDigitGridCellRef.current = null;
    setSelectedIndexState(selection.problemIndex);
    setSelectedSlotState(selection.slot);
    setSelectedColumnDigitState(selection);
  }, []);

  const selectDigitGridCell = useCallback((selection: DigitGridSelection) => {
    inputEnabledRef.current = true;
    selectedIndexRef.current = selection.problemIndex;
    selectedSlotRef.current = 'single';
    selectedColumnDigitRef.current = null;
    selectedDigitGridCellRef.current = selection;
    setSelectedIndexState(selection.problemIndex);
    setSelectedSlotState('single');
    setSelectedColumnDigitState(null);
    setSelectedDigitGridCellState(selection);
  }, []);

  const clearSelection = useCallback(() => {
    selectedIndexRef.current = null;
    selectedSlotRef.current = 'single';
    selectedColumnDigitRef.current = null;
    selectedDigitGridCellRef.current = null;
    inputEnabledRef.current = false;
    setSelectedIndexState(null);
    setSelectedSlotState('single');
    setSelectedColumnDigitState(null);
    setSelectedDigitGridCellState(null);
  }, []);

  const selectWithoutEnabling = useCallback((index: number | null, slot: MathfieldSlot = 'single') => {
    selectedIndexRef.current = index;
    selectedSlotRef.current = slot;
    selectedColumnDigitRef.current = null;
    selectedDigitGridCellRef.current = null;
    setSelectedIndexState(index);
    setSelectedSlotState(slot);
    setSelectedColumnDigitState(null);
    setSelectedDigitGridCellState(null);
  }, []);

  const registerMathfield = useCallback((key: string, mathfield: AutoDrillMathfield | null) => {
    if (mathfield) mathfieldRefs.current.set(key, mathfield);
    else mathfieldRefs.current.delete(key);
  }, []);

  const getMathfield = useCallback((key: string) => mathfieldRefs.current.get(key), []);

  const blurMathfields = useCallback(() => {
    for (const mathfield of mathfieldRefs.current.values()) mathfield.blur();
  }, []);

  const setMathfieldsReadOnly = useCallback((readOnly: boolean) => {
    for (const mathfield of mathfieldRefs.current.values()) {
      mathfield.readOnly = readOnly;
      if (readOnly) mathfield.blur();
    }
  }, []);

  const resetForWorksheet = useCallback((worksheet: WorksheetDto) => {
    const nextAnswers = emptyAnswers(worksheet);
    replaceAnswers(nextAnswers);
    acceptedLatexRef.current = Object.fromEntries(worksheet.problems.map((problem) => [problem.problem_id, '']));
    replaceColumnDrafts({});
    clearSelection();
    return nextAnswers;
  }, [clearSelection, replaceAnswers, replaceColumnDrafts]);

  const disableInputAndClearSelection = useCallback(() => {
    clearSelection();
  }, [clearSelection]);

  return {
    answers,
    selectedIndex,
    selectedSlot,
    selectedColumnDigit,
    selectedDigitGridCell,
    columnDrafts,
    answersRef,
    selectedIndexRef,
    selectedSlotRef,
    selectedColumnDigitRef,
    selectedDigitGridCellRef,
    columnDraftsRef,
    inputEnabledRef,
    actionQueueRef,
    acceptedLatexRef,
    replaceAnswers,
    setAnswer,
    replaceColumnDrafts,
    setColumnDraft,
    setSelectedIndex,
    setSelectedSlot,
    setSelectedColumnDigit,
    setSelectedDigitGridCell,
    select,
    selectColumnDigit,
    selectDigitGridCell,
    clearSelection,
    selectWithoutEnabling,
    registerMathfield,
    getMathfield,
    blurMathfields,
    setMathfieldsReadOnly,
    resetForWorksheet,
    disableInputAndClearSelection,
  };
}
