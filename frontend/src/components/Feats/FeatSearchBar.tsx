'use client';

import { useState, useRef, useEffect } from 'react';
import { Search, X } from 'lucide-react';
import { Button } from '@/components/ui/Button';

interface FeatSearchBarProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  onClear?: () => void;
  autoFocus?: boolean;
  variant?: 'default' | 'compact';
  showIcon?: boolean;
  debounceMs?: number;
}

export default function FeatSearchBar({
  value,
  onChange,
  placeholder = 'Search feats...',
  onClear,
  autoFocus = false,
  variant = 'default',
  showIcon = true,
  debounceMs = 300,
}: FeatSearchBarProps) {
  const [localValue, setLocalValue] = useState(value || '');
  const inputRef = useRef<HTMLInputElement>(null);
  const debounceTimer = useRef<NodeJS.Timeout | null>(null);

  // Sync local value with prop value
  useEffect(() => {
    setLocalValue(value || '');
  }, [value]);

  // Auto-focus on mount if requested
  useEffect(() => {
    if (autoFocus && inputRef.current) {
      inputRef.current.focus();
    }
  }, [autoFocus]);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = e.target.value;
    setLocalValue(newValue);

    // Debounce the onChange callback
    if (debounceTimer.current) {
      clearTimeout(debounceTimer.current);
    }

    if (debounceMs > 0) {
      debounceTimer.current = setTimeout(() => {
        onChange(newValue);
      }, debounceMs);
    } else {
      onChange(newValue);
    }
  };

  const handleClear = () => {
    setLocalValue('');
    onChange('');
    onClear?.();
    inputRef.current?.focus();
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Escape') {
      handleClear();
    }
  };

  const isCompact = variant === 'compact';

  return (
    <div className="relative">
      {showIcon && (
        <Search 
          className={`absolute left-3 text-[rgb(var(--color-text-muted))] pointer-events-none ${
            isCompact ? 'top-1.5 w-3 h-3' : 'top-2.5 w-4 h-4'
          }`}
        />
      )}
      
      <input
        ref={inputRef}
        type="text"
        value={localValue}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        placeholder={placeholder}
        className={`
          w-full 
          ${showIcon ? (isCompact ? 'pl-8' : 'pl-10') : (isCompact ? 'pl-2' : 'pl-3')}
          ${localValue ? (isCompact ? 'pr-8' : 'pr-10') : (isCompact ? 'pr-2' : 'pr-3')}
          ${isCompact ? 'py-1 text-xs' : 'py-2 text-sm'}
          border border-[rgb(var(--color-border))] 
          rounded-md 
          bg-[rgb(var(--color-surface-1))]
          text-[rgb(var(--color-text-primary))]
          placeholder:text-[rgb(var(--color-text-muted))]
          focus:outline-none 
          focus:ring-2 
          focus:ring-[rgb(var(--color-primary))]/20 
          focus:border-[rgb(var(--color-primary))]
          transition-colors
        `}
      />
      
      {localValue && (
        <Button
          variant="ghost"
          size="sm"
          onClick={handleClear}
          className={`
            absolute 
            ${isCompact ? 'right-1 top-0.5 p-1' : 'right-2 top-1.5 p-1.5'}
            hover:bg-[rgb(var(--color-surface-3))]
          `}
          aria-label="Clear search"
        >
          <X className={isCompact ? 'w-3 h-3' : 'w-4 h-4'} />
        </Button>
      )}
    </div>
  );
}

// Compound component for search with results count
interface SearchWithCountProps extends FeatSearchBarProps {
  resultCount?: number;
  totalCount?: number;
}

export function FeatSearchWithCount({
  resultCount,
  totalCount,
  ...searchProps
}: SearchWithCountProps) {
  const showCount = searchProps.value && resultCount !== undefined && totalCount !== undefined;

  return (
    <div className="space-y-2">
      <FeatSearchBar {...searchProps} />
      {showCount && (
        <div className="text-xs text-[rgb(var(--color-text-muted))] px-1">
          Showing {resultCount} of {totalCount} feats
        </div>
      )}
    </div>
  );
}