import type { Monaco } from '../../hooks/useCustomMonaco';

// Apply the generated DSL .d.ts library to Monaco and expose some
// Debug handles on window so we can inspect schemas and lib source
// From the browser console.
export function applyDslLibToMonaco(monaco: Monaco, libSource: string) {
    if (!monaco || !libSource) {
        return {};
    }

    const ts = monaco.typescript;
    const jsDefaults = ts.javascriptDefaults;
    const extraLib = jsDefaults.addExtraLib(
        libSource,
        'file:///modular/dsl-lib.d.ts',
    );
    const extraLibModel = monaco.editor.createModel(
        libSource,
        'typescript',
        monaco.Uri.parse('file:///modular/dsl-lib.d.ts'),
    );
    extraLibModel.onDidChangeContent((_e) => {
        // TODO: Make this model read-only
    });
    return { extraLib, extraLibModel };
}

export function formatPath(currentFile: string) {
    if (!currentFile.startsWith('/')) {
        currentFile = '/' + currentFile;
    }
    if (!currentFile.endsWith('.js') && !currentFile.endsWith('.mjs')) {
        currentFile += '.mjs';
    }
    return `file://${currentFile}`;
}

// Export const constrainedModel = function (model, ranges, monaco) {
//   Const rangeConstructor = monaco.Range;
//   Const sortRangesInAscendingOrder = function (rangeObject1, rangeObject2) {
//     Const rangeA = rangeObject1.range;
//     Const rangeB = rangeObject2.range;
//     If (
//       RangeA[0] < rangeB[0] ||
//       (rangeA[0] === rangeB[0] && rangeA[3] < rangeB[1])
//     ) {
//       Return -1;
//     }
//   }
//   Const normalizeRange = function (range, content) {
//     Const lines = content.split('\n');
//     Const noOfLines = lines.length;
//     Const normalizedRange = [];
//     Range.forEach(function (value, index) {
//       If (value === 0) {
//         Throw new Error('Range values cannot be zero');//No I18n
//       }
//       Switch (index) {
//         Case 0: {
//           If (value < 0) {
//             Throw new Error('Start Line of Range cannot be negative');//No I18n
//           } else if (value > noOfLines) {
//             Throw new Error('Provided Start Line(' + value + ') is out of bounds. Max Lines in content is ' + noOfLines);//No I18n
//           }
//           NormalizedRange[index] = value;
//         }
//           Break;
//         Case 1: {
//           Let actualStartCol = value;
//           Const startLineNo = normalizedRange[0];
//           Const maxCols = lines[startLineNo - 1].length
//           If (actualStartCol < 0) {
//             ActualStartCol = maxCols - Math.abs(actualStartCol);
//             If (actualStartCol < 0) {
//               Throw new Error('Provided Start Column(' + value + ') is out of bounds. Max Column in line ' + startLineNo + ' is ' + maxCols);//No I18n
//             }
//           } else if (actualStartCol > (maxCols + 1)) {
//             Throw new Error('Provided Start Column(' + value + ') is out of bounds. Max Column in line ' + startLineNo + ' is ' + maxCols);//No I18n
//           }
//           NormalizedRange[index] = actualStartCol;
//         }
//           Break;
//         Case 2: {
//           Let actualEndLine = value;
//           If (actualEndLine < 0) {
//             ActualEndLine = noOfLines - Math.abs(value);
//             If (actualEndLine < 0) {
//               Throw new Error('Provided End Line(' + value + ') is out of bounds. Max Lines in content is ' + noOfLines);//No I18n
//             }
//             If (actualEndLine < normalizedRange[0]) {
//               Console.warn('Provided End Line(' + value + ') is less than the start Line, the Restriction may not behave as expected');//No I18n
//             }
//           } else if (value > noOfLines) {
//             Throw new Error('Provided End Line(' + value + ') is out of bounds. Max Lines in content is ' + noOfLines);//No I18n
//           }
//           NormalizedRange[index] = actualEndLine;
//         }
//           Break;
//         Case 3: {
//           Let actualEndCol = value;
//           Const endLineNo = normalizedRange[2];
//           Const maxCols = lines[endLineNo - 1].length
//           If (actualEndCol < 0) {
//             ActualEndCol = maxCols - Math.abs(actualEndCol);
//             If (actualEndCol < 0) {
//               Throw new Error('Provided End Column(' + value + ') is out of bounds. Max Column in line ' + endLineNo + ' is ' + maxCols);//No I18n
//             }
//           } else if (actualEndCol > (maxCols + 1)) {
//             Throw new Error('Provided Start Column(' + value + ') is out of bounds. Max Column in line ' + endLineNo + ' is ' + maxCols);//No I18n
//           }
//           NormalizedRange[index] = actualEndCol;
//         }
//           Break;
//       }
//     })
//     Return normalizedRange;
//   }
//   Let restrictions = deepClone(ranges).sort(sortRangesInAscendingOrder);
//   Const prepareRestrictions = function (restrictions) {
//     Const content = model.getValue();
//     Restrictions.forEach(function (restriction, index) {
//       Const range = normalizeRange(restriction.range, content);
//       Const startLine = range[0];
//       Const startCol = range[1];
//       Const endLine = range[2];
//       Const endCol = range[3];
//       Restriction._originalRange = range.slice();
//       Restriction.range = new rangeConstructor(startLine, startCol, endLine, endCol);
//       Restriction.index = index;
//       If (!restriction.allowMultiline) {
//         Restriction.allowMultiline = rangeConstructor.spansMultipleLines(restriction.range)
//       }
//       If (!restriction.label) {
//         Restriction.label = `[${startLine},${startCol} -> ${endLine}${endCol}]`;
//       }
//     });
//   }
//   Const getCurrentEditableRanges = function () {
//     Return restrictions.reduce(function (acc, restriction) {
//       Acc[restriction.label] = {
//         AllowMultiline: restriction.allowMultiline || false,
//         Index: restriction.index,
//         Range: Object.assign({}, restriction.range),
//         OriginalRange: restriction._originalRange.slice()
//       };
//       Return acc;
//     }, {});
//   }
//   Const getValueInEditableRanges = function () {
//     Return restrictions.reduce(function (acc, restriction) {
//       Acc[restriction.label] = model.getValueInRange(restriction.range);
//       Return acc;
//     }, {});
//   }
//   Const updateValueInEditableRanges = function (object, forceMoveMarkers) {
//     If (typeof object === 'object' && !Array.isArray(object)) {
//       ForceMoveMarkers = typeof forceMoveMarkers === 'boolean' ? forceMoveMarkers : false;
//       Const restrictionsMap = restrictions.reduce(function (acc, restriction) {
//         If (restriction.label) {
//           Acc[restriction.label] = restriction;
//         }
//         Return acc;
//       }, {});
//       For (let label in object) {
//         Const restriction = restrictionsMap[label];
//         If (restriction) {
//           Const value = object[label];
//           If (doesChangeHasMultilineConflict(restriction, value)) {
//             Throw new Error('Multiline change is not allowed for ' + label);
//           }
//           Const newRange = deepClone(restriction.range);
//           NewRange.endLine = newRange.startLine + value.split('\n').length - 1;
//           NewRange.endColumn = value.split('\n').pop().length;
//           If (isChangeInvalidAsPerUser(restriction, value, newRange)) {
//             Throw new Error('Change is invalidated by validate function of ' + label);
//           }
//           Model.applyEdits([{
//             ForceMoveMarkers: !!forceMoveMarkers,
//             Range: restriction.range,
//             Text: value
//           }]);
//         } else {
//           Console.error('No restriction found for ' + label);
//         }
//       }
//     } else {
//       Throw new Error('Value must be an object');//No I18n
//     }
//   }
//   Const disposeRestrictions = function () {
//     Model._restrictionChangeListener.dispose();
//     Window.removeEventListener("error", handleUnhandledPromiseRejection);
//     Delete model.editInRestrictedArea;
//     Delete model.disposeRestrictions;
//     Delete model.getValueInEditableRanges;
//     Delete model.updateValueInEditableRanges;
//     Delete model.updateRestrictions;
//     Delete model.getCurrentEditableRanges;
//     Delete model.toggleHighlightOfEditableAreas;
//     Delete model._hasHighlight;
//     Delete model._isRestrictedModel;
//     Delete model._isCursorAtCheckPoint;
//     Delete model._currentCursorPositions;
//     Delete model._editableRangeChangeListener;
//     Delete model._restrictionChangeListener;
//     Delete model._oldDecorations;
//     Delete model._oldDecorationsSource;
//     Return model;
//   }
//   Const isCursorAtCheckPoint = function (positions) {
//     Positions.some(function (position) {
//       Const posLineNumber = position.lineNumber;
//       Const posCol = position.column;
//       Const length = restrictions.length;
//       For (let i = 0; i < length; i++) {
//         Const range = restrictions[i].range;
//         If (
//           (range.startLineNumber === posLineNumber && range.startColumn === posCol) ||
//           (range.endLineNumber === posLineNumber && range.endColumn === posCol)
//         ) {
//           Model.pushStackElement();
//           Return true;
//         }
//       }
//     });
//   };
//   Const addEditableRangeListener = function (callback) {
//     If (typeof callback === 'function') {
//       Model._editableRangeChangeListener.push(callback);
//     }
//   };
//   Const triggerChangeListenersWith = function (currentChanges, allChanges) {
//     Const currentRanges = getCurrentEditableRanges();
//     Model._editableRangeChangeListener.forEach(function (callback) {
//       Callback.call(model, currentChanges, allChanges, currentRanges);
//     });
//   };
//   Const doUndo = function () {
//     Return Promise.resolve().then(function () {
//       Model.editInRestrictedArea = true;
//       Model.undo();
//       Model.editInRestrictedArea = false;
//       If (model._hasHighlight && model._oldDecorationsSource) {
//         // id present in the decorations info will be omitted by monaco
//         // So we don't need to remove the old decorations id
//         Model.deltaDecorations(model._oldDecorations, model._oldDecorationsSource);
//         Model._oldDecorationsSource.forEach(function (object) {
//           Object.range = model.getDecorationRange(object.id);
//         });
//       }
//     });
//   };
//   Const updateRange = function (restriction, range, finalLine, finalColumn, changes, changeIndex) {
//     Let oldRangeEndLineNumber = range.endLineNumber;
//     Let oldRangeEndColumn = range.endColumn;
//     Restriction.prevRange = range;
//     Restriction.range = range.setEndPosition(finalLine, finalColumn);
//     Const length = restrictions.length;
//     Let changesLength = changes.length;
//     Const diffInCol = finalColumn - oldRangeEndColumn;
//     Const diffInRow = finalLine - oldRangeEndLineNumber;

//     Const cursorPositions = model._currentCursorPositions || [];
//     Const noOfCursorPositions = cursorPositions.length;
//     // if (noOfCursorPositions > 0) {
//     If (changesLength !== noOfCursorPositions) {
//       Changes = changes.filter(function (change) {
//         Const range = change.range;
//         For (let i = 0; i < noOfCursorPositions; i++) {
//           Const cursorPosition = cursorPositions[i];
//           If (
//             (range.startLineNumber === cursorPosition.startLineNumber) &&
//             (range.endLineNumber === cursorPosition.endLineNumber) &&
//             (range.startColumn === cursorPosition.startColumn) &&
//             (range.endColumn === cursorPosition.endColumn)
//           ) {
//             Return true;
//           }
//         }
//         Return false;
//       });
//       ChangesLength = changes.length;
//     }
//     If (diffInRow !== 0) {
//       For (let i = restriction.index + 1; i < length; i++) {
//         Const nextRestriction = restrictions[i];
//         Const nextRange = nextRestriction.range;
//         If (oldRangeEndLineNumber === nextRange.startLineNumber) {
//           NextRange.startColumn += diffInCol;
//         }
//         If (oldRangeEndLineNumber === nextRange.endLineNumber) {
//           NextRange.endColumn += diffInCol;
//         }
//         NextRange.startLineNumber += diffInRow;
//         NextRange.endLineNumber += diffInRow;
//         NextRestriction.range = nextRange;
//       }
//       For (let i = changeIndex + 1; i < changesLength; i++) {
//         Const nextChange = changes[i];
//         Const rangeInChange = nextChange.range;
//         Const rangeAsString = rangeInChange.toString();
//         Const rangeMapValue = rangeMap[rangeAsString];
//         Delete rangeMap[rangeAsString];
//         If (oldRangeEndLineNumber === rangeInChange.startLineNumber) {
//           RangeInChange.startColumn += diffInCol;
//         }
//         If (oldRangeEndLineNumber === rangeInChange.endLineNumber) {
//           RangeInChange.endColumn += diffInCol;
//         }
//         RangeInChange.startLineNumber += diffInRow;
//         RangeInChange.endLineNumber += diffInRow;
//         NextChange.range = rangeInChange;
//         RangeMap[rangeInChange.toString()] = rangeMapValue;
//       }
//     } else {
//       // Only Column might have changed
//       For (let i = restriction.index + 1; i < length; i++) {
//         Const nextRestriction = restrictions[i];
//         Const nextRange = nextRestriction.range;
//         If (nextRange.startLineNumber > oldRangeEndLineNumber) {
//           Break;
//         } else {
//           NextRange.startColumn += diffInCol;
//           NextRange.endColumn += diffInCol;
//           NextRestriction.range = nextRange;
//         }
//       }
//       For (let i = changeIndex + 1; i < changesLength; i++) {
//         // rangeMap
//         Const nextChange = changes[i];
//         Const rangeInChange = nextChange.range;
//         Const rangeAsString = rangeInChange.toString();
//         Const rangeMapValue = rangeMap[rangeAsString];
//         Delete rangeMap[rangeAsString];
//         If (rangeInChange.startLineNumber > oldRangeEndLineNumber) {
//           RangeMap[rangeInChange.toString()] = rangeMapValue;
//           Break;
//         } else {
//           RangeInChange.startColumn += diffInCol;
//           RangeInChange.endColumn += diffInCol;
//           NextChange.range = rangeInChange;
//           RangeMap[rangeInChange.toString()] = rangeMapValue;
//         }
//       }
//     }
//     // }
//   };
//   Const getInfoFrom = function (change, editableRange) {
//     Const info = {};
//     Const range = change.range;
//     // Get State
//     If (change.text === '') {
//       Info.isDeletion = true;
//     } else if (
//       (range.startLineNumber === range.endLineNumber) &&
//       (range.startColumn === range.endColumn)
//     ) {
//       Info.isAddition = true;
//     } else {
//       Info.isReplacement = true;
//     }
//     // Get Position Of Range
//     Info.startLineOfRange = range.startLineNumber === editableRange.startLineNumber;
//     Info.startColumnOfRange = range.startColumn === editableRange.startColumn;

//     Info.endLineOfRange = range.endLineNumber === editableRange.endLineNumber;
//     Info.endColumnOfRange = range.endColumn === editableRange.endColumn;

//     Info.middleLineOfRange = !info.startLineOfRange && !info.endLineOfRange;

//     // Editable Range Span
//     If (editableRange.startLineNumber === editableRange.endLineNumber) {
//       Info.rangeIsSingleLine = true;
//     } else {
//       Info.rangeIsMultiLine = true;
//     }
//     Return info;
//   };
//   Const updateRestrictions = function (ranges) {
//     Restrictions = deepClone(ranges).sort(sortRangesInAscendingOrder);
//     PrepareRestrictions(restrictions);
//   };
//   Const toggleHighlightOfEditableAreas = function (cssClasses) {
//     If (!model._hasHighlight) {
//       Const cssClassForSingleLine = cssClasses.cssClassForSingleLine ||enums.SINGLE_LINE_HIGHLIGHT_CLASS
//       Const cssClassForMultiLine = cssClasses.cssClassForMultiLine ||enums.MULTI_LINE_HIGHLIGHT_CLASS
//       Const decorations = restrictions.map(function (restriction) {
//         Const decoration = {
//           Range: restriction.range,
//           Options: {
//             ClassName: restriction.allowMultiline ?
//               CssClassForMultiLine :
//               CssClassForSingleLine
//           }
//         }
//         If (restriction.label) {
//           Decoration.hoverMessage = restriction.label;
//         }
//         Return decoration;
//       });
//       Model._oldDecorations = model.deltaDecorations([], decorations);
//       Model._oldDecorationsSource = decorations.map(function (decoration, index) {
//         Return Object.assign({}, decoration, { id: model._oldDecorations[index] });
//       });
//       Model._hasHighlight = true;
//     } else {
//       Model.deltaDecorations(model._oldDecorations, []);
//       Delete model._oldDecorations;
//       Delete model._oldDecorationsSource;
//       Model._hasHighlight = false;
//     }
//   }
//   Const handleUnhandledPromiseRejection = function () {
//     Console.debug('handler for unhandled promise rejection');
//   };
//   Const setAllRangesToPrev = function (rangeMap) {
//     For (let key in rangeMap) {
//       Const restriction = rangeMap[key];
//       Restriction.range = restriction.prevRange;
//     }
//   };
//   Const doesChangeHasMultilineConflict = function (restriction, text) {
//     Return !restriction.allowMultiline && text.includes('\n');
//   };
//   Const isChangeInvalidAsPerUser = function (restriction, value, range) {
//     Return restriction.validate && !restriction.validate(value, range, restriction.lastInfo);
//   }

//   Const manipulatorApi = {
//     _isRestrictedModel: true,
//     _isRestrictedValueValid: true,
//     _editableRangeChangeListener: [],
//     _isCursorAtCheckPoint: isCursorAtCheckPoint,
//     _currentCursorPositions: []
//   }

//   PrepareRestrictions(restrictions);
//   Model._hasHighlight = false;
//   ManipulatorApi._restrictionChangeListener = model.onDidChangeContent(function (contentChangedEvent) {
//     Const isUndoing = contentChangedEvent.isUndoing;
//     Model._isRestrictedValueValid = true;
//     If (!(isUndoing && model.editInRestrictedArea)) {
//       Const changes = contentChangedEvent.changes.sort(sortRangesInAscendingOrder);
//       Const rangeMap = {};
//       Const length = restrictions.length;
//       Const isAllChangesValid = changes.every(function (change) {
//         Const editedRange = change.range;
//         Const rangeAsString = editedRange.toString();
//         RangeMap[rangeAsString] = null;
//         For (let i = 0; i < length; i++) {
//           Const restriction = restrictions[i];
//           Const range = restriction.range;
//           If (range.containsRange(editedRange)) {
//             If (doesChangeHasMultilineConflict(restriction, change.text)) {
//               Return false;
//             }
//             RangeMap[rangeAsString] = restriction;
//             Return true;
//           }
//         }
//         Return false;
//       })
//       If (isAllChangesValid) {
//         Changes.forEach(function (change, changeIndex) {
//           Const changedRange = change.range;
//           Const restriction = rangeMap[changedRange.toString()];
//           Const editableRange = restriction.range;
//           Const text = change.text || '';
//           /**
//            * Things to check before implementing the change
//            * - A | D | R => Addition | Deletion | Replacement
//            * - MC | SC => MultiLineChange | SingleLineChange
//            * - SOR | MOR | EOR => Change Occured in - Start Of Range | Middle Of Range | End Of Range
//            * - SSL | SML => Editable Range - Spans Single Line | Spans Multiple Line
//            */
//           Const noOfLinesAdded = (text.match(/\n/g) || []).length;
//           Const noOfColsAddedAtLastLine = text.split(/\n/g).pop().length;

//           Const lineDiffInRange = changedRange.endLineNumber - changedRange.startLineNumber;
//           Const colDiffInRange = changedRange.endColumn - changedRange.startColumn;

//           Let finalLine = editableRange.endLineNumber;
//           Let finalColumn = editableRange.endColumn;

//           Let columnsCarriedToEnd = 0;
//           If (
//             (editableRange.endLineNumber === changedRange.startLineNumber) ||
//             (editableRange.endLineNumber === changedRange.endLineNumber)
//           ) {
//             ColumnsCarriedToEnd += (editableRange.endColumn - changedRange.startColumn) + 1;
//           }

//           Const info = getInfoFrom(change, editableRange);
//           Restriction.lastInfo = info;
//           If (info.isAddition || info.isReplacement) {
//             If (info.rangeIsSingleLine) {
//               /**
//                * Only Column Change has occurred , so regardless of the position of the change
//                * Addition of noOfCols is enough
//                */
//               If (noOfLinesAdded === 0) {
//                 FinalColumn += noOfColsAddedAtLastLine;
//               } else {
//                 FinalLine += noOfLinesAdded;
//                 If (info.startColumnOfRange) {
//                   FinalColumn += noOfColsAddedAtLastLine
//                 } else if (info.endColumnOfRange) {
//                   FinalColumn = (noOfColsAddedAtLastLine + 1)
//                 } else {
//                   FinalColumn = (noOfColsAddedAtLastLine + columnsCarriedToEnd)
//                 }
//               }
//             }
//             If (info.rangeIsMultiLine) {
//               // Handling for Start Of Range is not required
//               FinalLine += noOfLinesAdded;
//               If (info.endLineOfRange) {
//                 If (noOfLinesAdded === 0) {
//                   FinalColumn += noOfColsAddedAtLastLine;
//                 } else {
//                   FinalColumn = (columnsCarriedToEnd + noOfColsAddedAtLastLine);
//                 }
//               }
//             }
//           }
//           If (info.isDeletion || info.isReplacement) {
//             If (info.rangeIsSingleLine) {
//               FinalColumn -= colDiffInRange;
//             }
//             If (info.rangeIsMultiLine) {
//               If (info.endLineOfRange) {
//                 FinalLine -= lineDiffInRange;
//                 FinalColumn -= colDiffInRange;
//               } else {
//                 FinalLine -= lineDiffInRange;
//               }
//             }
//           }
//           UpdateRange(restriction, editableRange, finalLine, finalColumn, changes, changeIndex);
//         });
//         Const values = model.getValueInEditableRanges();
//         Const currentlyEditedRanges = {};
//         For (let key in rangeMap) {
//           Const restriction = rangeMap[key];
//           Const range = restriction.range;
//           Const rangeString = restriction.label || range.toString();
//           Const value = values[rangeString];
//           If (isChangeInvalidAsPerUser(restriction, value, range)) {
//             SetAllRangesToPrev(rangeMap);
//             DoUndo();
//             Return; // Breaks the loop and prevents the triggerChangeListener
//           }
//           CurrentlyEditedRanges[rangeString] = value;
//         }
//         If (model._hasHighlight) {
//           Model._oldDecorationsSource.forEach(function (object) {
//             Object.range = model.getDecorationRange(object.id);
//           });
//         }
//         TriggerChangeListenersWith(currentlyEditedRanges, values);
//       } else {
//         DoUndo();
//       }
//     } else if (model.editInRestrictedArea) {
//       Model._isRestrictedValueValid = false;
//     }
//   });
//   Window.onerror = handleUnhandledPromiseRejection;
//   Const exposedApi = {
//     EditInRestrictedArea: false,
//     GetCurrentEditableRanges: getCurrentEditableRanges,
//     GetValueInEditableRanges: getValueInEditableRanges,
//     DisposeRestrictions: disposeRestrictions,
//     OnDidChangeContentInEditableRange: addEditableRangeListener,
//     UpdateRestrictions: updateRestrictions,
//     UpdateValueInEditableRanges: updateValueInEditableRanges,
//     ToggleHighlightOfEditableAreas: toggleHighlightOfEditableAreas
//   }
//   For (let funcName in manipulatorApi) {
//     Object.defineProperty(model, funcName, {
//       Enumerable: false,
//       Configurable: true,
//       Writable: true,
//       Value: manipulatorApi[funcName]
//     })
//   }
//   For (let apiName in exposedApi) {
//     Object.defineProperty(model, apiName, {
//       Enumerable: false,
//       Configurable: true,
//       Writable: true,
//       Value: exposedApi[apiName]
//     })
//   }
//   Return model;
// }
