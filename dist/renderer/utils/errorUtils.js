"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.getErrorMessage = void 0;
const getErrorMessage = (error, fallback) => error instanceof Error ? error.message : fallback;
exports.getErrorMessage = getErrorMessage;
//# sourceMappingURL=errorUtils.js.map