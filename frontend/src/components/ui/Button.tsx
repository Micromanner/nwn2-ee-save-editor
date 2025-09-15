import { ButtonHTMLAttributes, forwardRef, ReactNode } from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '@/lib/utils';

const buttonVariants = cva(
  'btn-base',
  {
    variants: {
      variant: {
        primary: 'btn-primary',
        secondary: 'btn-secondary',
        outline: 'btn-outline',
        ghost: 'btn-ghost',
        danger: 'btn-danger',
        'spell-ghost': 'btn-spell-ghost',
        'spell-learned': 'btn-spell-learned',
        'icon-interactive': 'btn-icon-interactive',
      },
      size: {
        xs: 'btn-xs',
        sm: 'btn-sm',
        md: 'btn-md',
        lg: 'btn-lg',
        icon: 'btn-icon',
        'icon-md': 'btn-icon-md',
        'icon-lg': 'btn-icon-lg',
      },
    },
    defaultVariants: {
      variant: 'primary',
      size: 'md',
    },
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