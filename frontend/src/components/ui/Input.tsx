import { InputHTMLAttributes, forwardRef } from 'react';
import { cn } from '@/lib/utils';

export type InputProps = InputHTMLAttributes<HTMLInputElement>

const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ className, type, ...props }, ref) => {
    return (
      <input
        type={type}
        className={cn(
          'flex h-10 w-full rounded-md border-2 border-[rgb(var(--color-surface-border)/0.6)] bg-[rgb(var(--color-surface-1))] px-3 py-2 text-sm text-[rgb(var(--color-text-primary))] placeholder:text-[rgb(var(--color-text-muted))] transition-all duration-200',
          'focus:outline-none focus:ring-2 focus:ring-[rgb(var(--color-primary))] focus:border-[rgb(var(--color-primary))] focus:ring-offset-2 focus:ring-offset-[rgb(var(--color-background))]',
          'hover:border-[rgb(var(--color-surface-border))]',
          'disabled:cursor-not-allowed disabled:opacity-50 disabled:bg-[rgb(var(--color-surface-2)/0.5)]',
          'file:border-0 file:bg-transparent file:text-sm file:font-medium',
          className
        )}
        ref={ref}
        {...props}
      />
    );
  }
);

Input.displayName = 'Input';

export { Input };