import { useCallback } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { useToast } from '@/contexts/ToastContext';
import {
  isCommandError,
  getTranslatedError,
  type CommandError,
  type TranslatedError,
} from '@/lib/api/errors';

type ErrorSeverity = 'warning' | 'error' | 'fatal';

function getErrorSeverity(error: CommandError): ErrorSeverity {
  switch (error.code) {
    case 'NoCharacterLoaded':
    case 'NoGameDataLoaded':
    case 'ValidationError':
    case 'InvalidValue':
    case 'InsufficientResources':
    case 'PrerequisitesNotMet':
      return 'warning';
    case 'CharacterNotFound':
    case 'FileError':
    case 'ParseError':
      return 'fatal';
    case 'NotFound':
    case 'AlreadyExists':
    case 'OperationFailed':
    case 'Internal':
    default:
      return 'error';
  }
}

interface UseErrorHandlerOptions {
  onFatal?: (translated: TranslatedError, error: CommandError) => void;
}

export function useErrorHandler(options?: UseErrorHandlerOptions) {
  const t = useTranslations();
  const { showToast } = useToast();

  const handleError = useCallback((error: unknown) => {
    if (isCommandError(error)) {
      const translated = getTranslatedError(error, t);
      const severity = getErrorSeverity(error);

      if (severity === 'fatal' && options?.onFatal) {
        options.onFatal(translated, error);
      } else {
        const toastType = severity === 'warning' ? 'warning' : 'error';
        showToast(
          translated.message,
          toastType,
          0,
          translated.recovery ?? undefined
        );
      }
    } else {
      const message = error instanceof Error
        ? error.message
        : t('errors.message.unknown');
      showToast(message, 'error');
    }
  }, [t, showToast, options]);

  return { handleError };
}
