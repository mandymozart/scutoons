// Expressions in texts (agents/EXPRESSIONS.md): ink's inline syntax – parser, evaluator, analyser, registry.
export * from './ast.ts';
export { parseExpression, parseText } from './parse.ts';
export { evaluateExpression, evaluateText, isTruthy, parsedText, showValue, type ExprResolver, type TextResult } from './evaluate.ts';
export { analyseExpression, analyseText, didYouMean, exprMessage, type AnalyseOptions, type ExprAnalysis, type ExprDiagnostic, type ExprReads, type ExprRegistry } from './analyse.ts';
export * from './registry.ts';
