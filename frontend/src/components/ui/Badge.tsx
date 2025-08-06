import * as React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';

const badgeVariants = cva(
  'inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2',
  {
    variants: {
      variant: {
        default:
          'bg-[rgb(var(--color-primary))] text-[rgb(var(--color-text-on-primary))] hover:bg-[rgb(var(--color-primary)/0.8)]',
        secondary:
          'bg-[rgb(var(--color-surface-3))] text-[rgb(var(--color-text-secondary))] hover:bg-[rgb(var(--color-surface-4))]',
        destructive:
          'bg-[rgb(var(--color-error))] text-white hover:bg-[rgb(var(--color-error)/0.8)]',
        outline:
          'text-[rgb(var(--color-text-primary))] border border-[rgb(var(--color-border))]',
      },
    },
    defaultVariants: {
      variant: 'default',
    },
  }
);

export interface BadgeProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof badgeVariants> {}

function Badge({ className, variant, ...props }: BadgeProps) {
  return (
    <div className={badgeVariants({ variant, className })} {...props} />
  );
}

export { Badge, badgeVariants };