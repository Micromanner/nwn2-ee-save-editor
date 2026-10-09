/**
 * Type-safe error handling for Tauri commands.
 *
 * CommandError types match the Rust CommandError enum from:
 * src-tauri/src/commands/error.rs
 */

export type CommandErrorCode =
  | 'NoCharacterLoaded'
  | 'NoGameDataLoaded'
  | 'CharacterNotFound'
  | 'ValidationError'
  | 'InvalidValue'
  | 'FileError'
  | 'ParseError'
  | 'InsufficientResources'
  | 'PrerequisitesNotMet'
  | 'NotFound'
  | 'AlreadyExists'
  | 'OperationFailed'
  | 'Internal';

export interface CommandErrorBase {
  code: CommandErrorCode;
}

export interface NoCharacterLoadedError extends CommandErrorBase {
  code: 'NoCharacterLoaded';
}

export interface NoGameDataLoadedError extends CommandErrorBase {
  code: 'NoGameDataLoaded';
}

export interface CharacterNotFoundError extends CommandErrorBase {
  code: 'CharacterNotFound';
  details: { path: string };
}

export interface ValidationErrorError extends CommandErrorBase {
  code: 'ValidationError';
  details: { field: string; reason: string };
}

export interface InvalidValueError extends CommandErrorBase {
  code: 'InvalidValue';
  details: { field: string; expected: string; actual: string };
}

export interface FileErrorError extends CommandErrorBase {
  code: 'FileError';
  details: { message: string; path: string | null; diagnostics_path?: string | null };
}

export interface ParseErrorError extends CommandErrorBase {
  code: 'ParseError';
  details: { message: string; context: string | null; diagnostics_path?: string | null };
}

export interface InsufficientResourcesError extends CommandErrorBase {
  code: 'InsufficientResources';
  details: { resource: string; required: number; available: number };
}

export interface PrerequisitesNotMetError extends CommandErrorBase {
  code: 'PrerequisitesNotMet';
  details: { missing: string[] };
}

export interface NotFoundError extends CommandErrorBase {
  code: 'NotFound';
  details: { item: string };
}

export interface AlreadyExistsError extends CommandErrorBase {
  code: 'AlreadyExists';
  details: { item: string };
}

export interface OperationFailedError extends CommandErrorBase {
  code: 'OperationFailed';
  details: { operation: string; reason: string };
}

export interface InternalError extends CommandErrorBase {
  code: 'Internal';
  details: string;
}

export type CommandError =
  | NoCharacterLoadedError
  | NoGameDataLoadedError
  | CharacterNotFoundError
  | ValidationErrorError
  | InvalidValueError
  | FileErrorError
  | ParseErrorError
  | InsufficientResourcesError
  | PrerequisitesNotMetError
  | NotFoundError
  | AlreadyExistsError
  | OperationFailedError
  | InternalError;

/**
 * Type guard to check if an unknown error is a CommandError
 */
export function isCommandError(error: unknown): error is CommandError {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    typeof (error as { code: unknown }).code === 'string'
  );
}

export type TranslationFn = (key: string, values?: Record<string, unknown>) => string;

export interface TranslatedError {
  title: string;
  message: string;
  recovery: string | null;
}

function appendDiagnostics(message: string, path: string | null | undefined, t: TranslationFn): string {
  if (!path) return message;
  return `${message}\n\n${t('errors.message.diagnosticsPath', { path })}`;
}

export function getTranslatedError(error: CommandError, t: TranslationFn): TranslatedError {
  switch (error.code) {
    case 'NoCharacterLoaded':
      return {
        title: t('errors.title.stateError'),
        message: t('errors.message.noCharacterLoaded'),
        recovery: t('errors.recovery.noCharacterLoaded'),
      };
    case 'NoGameDataLoaded':
      return {
        title: t('errors.title.stateError'),
        message: t('errors.message.noGameDataLoaded'),
        recovery: t('errors.recovery.noGameDataLoaded'),
      };
    case 'CharacterNotFound':
      return {
        title: t('errors.title.fileError'),
        message: t('errors.message.characterNotFound', { path: error.details.path }),
        recovery: t('errors.recovery.characterNotFound'),
      };
    case 'ValidationError':
      return {
        title: t('errors.title.validationError'),
        message: t('errors.message.validationError', { field: error.details.field, reason: error.details.reason }),
        recovery: t('errors.recovery.validationError'),
      };
    case 'InvalidValue':
      return {
        title: t('errors.title.validationError'),
        message: t('errors.message.invalidValue', { field: error.details.field, expected: error.details.expected, actual: error.details.actual }),
        recovery: t('errors.recovery.invalidValue'),
      };
    case 'FileError': {
      const base = error.details.path
        ? t('errors.message.fileErrorWithPath', { path: error.details.path, message: error.details.message })
        : t('errors.message.fileError', { message: error.details.message });
      return {
        title: t('errors.title.fileError'),
        message: appendDiagnostics(base, error.details.diagnostics_path, t),
        recovery: t('errors.recovery.fileError'),
      };
    }
    case 'ParseError': {
      const base = error.details.context
        ? t('errors.message.parseErrorWithContext', { context: error.details.context, message: error.details.message })
        : t('errors.message.parseError', { message: error.details.message });
      return {
        title: t('errors.title.parseError'),
        message: appendDiagnostics(base, error.details.diagnostics_path, t),
        recovery: t('errors.recovery.parseError'),
      };
    }
    case 'InsufficientResources':
      return {
        title: t('errors.title.resourceError'),
        message: t('errors.message.insufficientResources', { resource: error.details.resource, required: error.details.required, available: error.details.available }),
        recovery: t('errors.recovery.insufficientResources'),
      };
    case 'PrerequisitesNotMet':
      return {
        title: t('errors.title.validationError'),
        message: t('errors.message.prerequisitesNotMet', { missing: error.details.missing.join(', ') }),
        recovery: t('errors.recovery.prerequisitesNotMet'),
      };
    case 'NotFound':
      return {
        title: t('errors.title.notFoundError'),
        message: t('errors.message.notFound', { item: error.details.item }),
        recovery: t('errors.recovery.notFound'),
      };
    case 'AlreadyExists':
      return {
        title: t('errors.title.conflictError'),
        message: t('errors.message.alreadyExists', { item: error.details.item }),
        recovery: t('errors.recovery.alreadyExists'),
      };
    case 'OperationFailed':
      return {
        title: t('errors.title.operationError'),
        message: t('errors.message.operationFailed', { operation: error.details.operation, reason: error.details.reason }),
        recovery: t('errors.recovery.operationFailed'),
      };
    case 'Internal':
      return {
        title: t('errors.title.internalError'),
        message: typeof error.details === 'string' ? error.details : t('errors.message.internal'),
        recovery: t('errors.recovery.internal'),
      };
    default:
      return {
        title: t('errors.title.internalError'),
        message: t('errors.message.unknown'),
        recovery: null,
      };
  }
}
