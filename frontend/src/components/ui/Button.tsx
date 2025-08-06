import { ButtonHTMLAttributes, forwardRef, ReactNode } from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '@/lib/utils';

const buttonVariants = cva(
  'inline-flex items-center justify-center rounded-md font-medium transition-all duration-200 focus:outline-none focus:ring-0 disabled:opacity-50 disabled:cursor-not-allowed',
  {
    variants: {
      variant: {
        // Enhanced primary variant - fixed border colors
        primary: 'bg-[rgb(var(--color-primary))] text-text-primary border border-[rgb(var(--color-primary))] hover:bg-[rgb(var(--color-primary-600))] hover:border-[rgb(var(--color-primary-600))]',
        secondary: 'bg-[rgb(var(--color-secondary))] text-white hover:bg-[rgb(var(--color-secondary-600))]', 
        // Enhanced outline variant - fixed border colors and hover effects
        outline: 'bg-transparent text-[rgb(var(--color-primary))] border border-[rgb(var(--color-primary))] hover:bg-[rgb(var(--color-primary))] hover:text-text-primary hover:border-[rgb(var(--color-primary))]',
        ghost: 'text-text-primary hover:bg-[rgb(var(--color-surface-1))]',
        danger: 'bg-[rgb(var(--color-error))] text-white hover:bg-[rgb(var(--color-error-dark))]',
        // Spell-specific variants
        'spell-ghost': 'bg-transparent text-text-muted border border-transparent hover:bg-[rgb(var(--color-surface-2))] hover:text-text-primary',
        'spell-learned': 'bg-[rgb(var(--color-primary))] text-text-primary border border-[rgb(var(--color-primary))] hover:bg-[rgb(var(--color-error))] hover:border-[rgb(var(--color-error))] relative overflow-hidden',
        
        // Interactive icon button (for attribute controls, etc.)
        'icon-interactive': 'bg-[rgb(var(--color-surface-2))] text-text-primary border border-[rgb(var(--color-surface-border)/0.7)] hover:bg-[rgb(var(--color-surface-3))] hover:border-[rgb(var(--color-primary)/0.5)] hover:scale-105 active:scale-95 active:bg-[rgb(var(--color-primary)/0.1)] focus:outline-2 focus:outline-[rgb(var(--color-primary))] focus:outline-offset-2 focus:z-10 disabled:opacity-40 disabled:cursor-not-allowed disabled:transform-none disabled:hover:bg-[rgb(var(--color-surface-2))] disabled:hover:border-[rgb(var(--color-surface-border)/0.7)]',
      },
      size: {
        xs: 'px-2 py-1 text-xs',      // Small buttons like spell actions
        sm: 'px-3 py-1.5 text-sm',    // Small regular buttons
        md: 'px-4 py-2 text-sm',      // Default button size
        lg: 'px-6 py-3 text-base',    // Large buttons
        icon: 'p-1.5',                // Small icon buttons (24x24px)
        'icon-md': 'p-2',             // Medium icon buttons (32x32px)  
        'icon-lg': 'w-8 h-8 p-0',     // Large icon buttons (32x32px with fixed dimensions)
      },
    },
    defaultVariants: {
      variant: 'primary',
      size: 'md',
    },
    compoundVariants: [
      // Enhanced disabled states for outline variant
      {
        variant: 'outline',
        className: 'disabled:border-surface-border disabled:text-text-muted disabled:hover:bg-transparent disabled:hover:text-text-muted disabled:hover:border-surface-border',
      },
    ],
  }
);

export interface ButtonProps
  extends ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  loading?: boolean;
  loadingText?: string;
  hoverText?: string;
  leftIcon?: ReactNode;
  rightIcon?: ReactNode;
  clicked?: boolean; // For touch button visual feedback
}

const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, loading, loadingText, hoverText, leftIcon, rightIcon, children, disabled, clicked, ...props }, ref) => {
    const isDisabled = disabled || loading;
    
    return (
      <button
        className={cn(
          buttonVariants({ variant, size }), 
          loading && 'cursor-wait',
          hoverText && 'relative overflow-hidden',
          clicked && variant === 'icon-interactive' && 'shadow-[0_0_0_3px_rgb(var(--color-primary)/0.4),_0_0_15px_rgb(var(--color-primary)/0.3)] bg-primary/20',
          className
        )}
        ref={ref}
        disabled={isDisabled}
        {...props}
      >
        {loading ? (
          <>
            <div className="w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin mr-2" />
            {loadingText || children}
          </>
        ) : (
          <>
            {leftIcon && <span className="mr-2">{leftIcon}</span>}
            
            {hoverText ? (
              <>
                <span className="btn-text-default transition-opacity duration-200">{children}</span>
                <span className="btn-text-hover absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 opacity-0 transition-opacity duration-200">
                  {hoverText}
                </span>
              </>
            ) : (
              children
            )}
            
            {rightIcon && <span className="ml-2">{rightIcon}</span>}
          </>
        )}
      </button>
    );
  }
);

Button.displayName = 'Button';

export { Button, buttonVariants };