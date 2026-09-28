import { Component, type ReactNode } from 'react';
import { Button, NonIdealState } from '@blueprintjs/core';
import { GiBrokenShield } from 'react-icons/gi';
import { T } from '../theme';
import { useTranslations } from '@/hooks/useTranslations';
import { GameIcon } from './GameIcon';

interface ErrorBoundaryProps {
  children: ReactNode;
  fallbackTitle?: string;
  fallbackMessage?: string;
  fallbackRetry?: string;
}

interface ErrorBoundaryState {
  hasError: boolean;
}

function ErrorFallback({ title, message, retry, onRetry }: { title?: string; message?: string; retry?: string; onRetry: () => void }) {
  const t = useTranslations();
  return (
    <NonIdealState
      icon={<GameIcon icon={GiBrokenShield} size={40} color={T.negative} />}
      title={title ?? t('errors.boundary.title')}
      description={message ?? t('errors.boundary.message')}
      action={
        <Button icon="refresh" text={retry ?? t('errors.boundary.retry')} onClick={onRetry} />
      }
    />
  );
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { hasError: false };

  static getDerivedStateFromError(): ErrorBoundaryState {
    return { hasError: true };
  }

  handleRetry = () => {
    this.setState({ hasError: false });
  };

  render() {
    if (this.state.hasError) {
      return (
        <ErrorFallback
          title={this.props.fallbackTitle}
          message={this.props.fallbackMessage}
          retry={this.props.fallbackRetry}
          onRetry={this.handleRetry}
        />
      );
    }
    return this.props.children;
  }
}
