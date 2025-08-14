import { HTMLAttributes, forwardRef } from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '@/lib/utils';

const cardVariants = cva(
  'transition-all duration-200', // Base transition for all cards
  {
    variants: {
      variant: {
        // Container cards (saves container, theme sections)
        container: 'bg-[rgb(var(--color-surface-1))] border border-[rgb(var(--color-surface-border))] rounded-lg p-4',
        
        // Interactive item cards (spell cards, attribute cards)
        interactive: 'bg-[rgb(var(--color-surface-1))] border border-[rgb(var(--color-surface-border)/0.5)] rounded-md p-3 hover:border-[rgb(var(--color-primary)/0.5)] hover:shadow-elevation-2 hover:bg-[rgb(var(--color-surface-2))] max-w-full box-border overflow-hidden',
        
        // List items (save items, spell list items)  
        listItem: 'p-3 hover:bg-[rgb(var(--color-surface-1))]',
        
        // Error/status containers
        error: 'bg-[rgb(var(--color-error)/0.1)] border border-[rgb(var(--color-error)/0.3)] rounded-lg p-4',
        warning: 'bg-[rgb(var(--color-warning)/0.1)] border border-[rgb(var(--color-warning)/0.3)] rounded-lg p-4',
        success: 'bg-[rgb(var(--color-success)/0.1)] border border-[rgb(var(--color-success)/0.3)] rounded-lg p-4',
        
        // Default fallback
        default: 'bg-[rgb(var(--color-surface-2))] border border-[rgb(var(--color-surface-border)/0.6)] rounded-lg shadow-elevation-1',
      },
      size: {
        sm: 'p-2',
        md: 'p-3', 
        lg: 'p-4',
        xl: 'p-6',
      },
      selected: {
        true: 'border-2 border-[rgb(var(--color-primary))] bg-[rgb(var(--color-primary)/0.05)] shadow-[0_0_0_1px_rgb(var(--color-primary)/0.3)]',
      },
      learned: {
        true: 'border-2 border-[rgb(var(--color-primary)/0.5)] shadow-[0_0_0_1px_rgb(var(--color-primary)/0.3)]',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'md',
    },
    compoundVariants: [
      // When interactive card is selected, override the border to be primary
      {
        variant: 'interactive',
        selected: true,
        className: '!border-[rgb(var(--color-primary))] !bg-[rgb(var(--color-primary)/0.05)]',
      },
    ],
  }
);

interface CardProps 
  extends HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof cardVariants> {
  // Legacy props for backward compatibility
  noBorder?: boolean;
  borderColor?: string;
  borderWidth?: string;
  borderRadius?: string;
  backgroundColor?: string;
  shadow?: string | boolean;
  padding?: string;
}

const Card = forwardRef<HTMLDivElement, CardProps>(
  ({ 
    className, 
    variant,
    size,
    selected,
    learned,
    // Legacy props for backward compatibility
    noBorder = false,
    borderColor,
    borderWidth,
    borderRadius,
    backgroundColor,
    shadow,
    padding,
    style,
    ...props 
  }, ref) => {
    // If using new variant system, prioritize that
    if (variant || size || selected || learned) {
      return (
        <div
          ref={ref}
          className={cn(
            cardVariants({ variant, size, selected, learned }),
            className
          )}
          style={style}
          {...props}
        />
      );
    }

    // Legacy implementation for backward compatibility
    const customStyle = {
      ...style,
      ...(borderColor && { borderColor }),
      ...(borderWidth && { borderWidth }),
      ...(borderRadius && { borderRadius }),
      ...(backgroundColor && { backgroundColor }),
    };

    const classes = cn(
      !borderRadius && 'rounded-lg',
      !noBorder && !borderColor && !borderWidth && 'border border-[rgb(var(--color-surface-border)/0.6)]',
      !backgroundColor && 'bg-[rgb(var(--color-surface-2))]',
      shadow && (typeof shadow === 'string' ? shadow : 'shadow-elevation-1'),
      padding,
      className
    );

    return (
      <div
        ref={ref}
        className={classes}
        style={customStyle}
        {...props}
      />
    );
  }
);
Card.displayName = 'Card';

interface CardHeaderProps extends HTMLAttributes<HTMLDivElement> {
  noBorder?: boolean;
  borderColor?: string;
  padding?: string;
  spacing?: string;
}

const CardHeader = forwardRef<HTMLDivElement, CardHeaderProps>(
  ({ 
    className, 
    noBorder = false,
    borderColor,
    padding,
    spacing,
    style,
    ...props 
  }, ref) => {
    const customStyle = {
      ...style,
      ...(borderColor && { borderBottomColor: borderColor }),
    };

    const classes = cn(
      'flex flex-col',
      spacing || 'space-y-1.5',
      padding || 'p-6',
      !noBorder && !borderColor && 'border-b border-[rgb(var(--color-surface-border)/0.4)]',
      className
    );

    return (
      <div
        ref={ref}
        className={classes}
        style={customStyle}
        {...props}
      />
    );
  }
);
CardHeader.displayName = 'CardHeader';

interface CardTitleProps extends HTMLAttributes<HTMLHeadingElement> {
  textSize?: string;
  textColor?: string;
  fontWeight?: string;
}

const CardTitle = forwardRef<HTMLParagraphElement, CardTitleProps>(
  ({ 
    className, 
    textSize,
    textColor,
    fontWeight,
    style,
    ...props 
  }, ref) => {
    const customStyle = {
      ...style,
      ...(textColor && { color: textColor }),
    };

    const classes = cn(
      textSize || 'text-lg',
      fontWeight || 'font-semibold',
      'leading-none tracking-tight',
      !textColor && 'text-[rgb(var(--color-text-primary))]',
      className
    );

    return (
      <h3
        ref={ref}
        className={classes}
        style={customStyle}
        {...props}
      />
    );
  }
);
CardTitle.displayName = 'CardTitle';

interface CardDescriptionProps extends HTMLAttributes<HTMLParagraphElement> {
  textSize?: string;
  textColor?: string;
}

const CardDescription = forwardRef<HTMLParagraphElement, CardDescriptionProps>(
  ({ 
    className, 
    textSize,
    textColor,
    style,
    ...props 
  }, ref) => {
    const customStyle = {
      ...style,
      ...(textColor && { color: textColor }),
    };

    const classes = cn(
      textSize || 'text-sm',
      !textColor && 'text-[rgb(var(--color-text-muted))]',
      className
    );

    return (
      <p
        ref={ref}
        className={classes}
        style={customStyle}
        {...props}
      />
    );
  }
);
CardDescription.displayName = 'CardDescription';

interface CardContentProps extends HTMLAttributes<HTMLDivElement> {
  padding?: string;
}

const CardContent = forwardRef<HTMLDivElement, CardContentProps>(
  ({ className, padding, ...props }, ref) => (
    <div 
      ref={ref} 
      className={cn(padding || 'p-6', className)} 
      {...props} 
    />
  )
);
CardContent.displayName = 'CardContent';

interface CardFooterProps extends HTMLAttributes<HTMLDivElement> {
  padding?: string;
  layout?: string;
}

const CardFooter = forwardRef<HTMLDivElement, CardFooterProps>(
  ({ className, padding, layout, ...props }, ref) => (
    <div
      ref={ref}
      className={cn(
        layout || 'flex items-center',
        padding || 'p-6 pt-0',
        className
      )}
      {...props}
    />
  )
);
CardFooter.displayName = 'CardFooter';

export { Card, CardHeader, CardFooter, CardTitle, CardDescription, CardContent, cardVariants };
export type { CardProps, CardHeaderProps, CardTitleProps, CardDescriptionProps, CardContentProps, CardFooterProps };