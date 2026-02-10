"use strict";
/**
 * Shared types and state for source span analysis.
 *
 * This file is deliberately free of ts-morph (or any Node.js-only) imports
 * so it can be consumed from the renderer process without pulling in
 * Node.js built-ins via webpack.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.setActiveInterpolationResolutions = setActiveInterpolationResolutions;
exports.getActiveInterpolationResolutions = getActiveInterpolationResolutions;
/**
 * Active interpolation resolution map, set during DSL analysis.
 * Read by moduleStateTracking to redirect highlights into const declarations.
 */
let _activeInterpolationResolutions = null;
/**
 * Set the active interpolation resolution map.
 * Called by executor.ts after analysis and before/after execution.
 */
function setActiveInterpolationResolutions(map) {
    _activeInterpolationResolutions = map;
}
/**
 * Get the active interpolation resolution map.
 * Read by moduleStateTracking.ts during decoration polling.
 */
function getActiveInterpolationResolutions() {
    return _activeInterpolationResolutions;
}
//# sourceMappingURL=spanTypes.js.map