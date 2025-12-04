import React from 'react';
import { Input } from '@/components/ui/Input';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import { Copy } from 'lucide-react';
import { useTranslations } from '@/hooks/useTranslations';
import { display } from '@/utils/dataHelpers';
import { cn } from '@/lib/utils';

export interface VariableEdit {
  name: string;
  value: number | string;
  type: 'int' | 'string' | 'float';
}

interface VariableTableProps {
  variables: [string, number | string][];
  type: 'int' | 'string' | 'float';
  editedVars: Record<string, VariableEdit>;
  onVariableChange: (name: string, value: string, type: 'int' | 'string' | 'float') => void;
  searchQuery?: string;
  className?: string;
}

export function VariableTable({
  variables,
  type,
  editedVars,
  onVariableChange,
  searchQuery,
  className
}: VariableTableProps) {
  const t = useTranslations();

  if (variables.length === 0) {
    return (
      <div className="text-center text-[rgb(var(--color-text-muted))] py-12 border border-dashed border-[rgb(var(--color-border))] rounded-lg">
        {searchQuery ? 'No variables match your search' : 'No variables found'}
      </div>
    );
  }

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    // Could add a toast here if we had a toast system exposed
  };

  return (
    <div className={cn("w-full overflow-hidden rounded-lg border border-[rgb(var(--color-border))] bg-[rgb(var(--color-surface-primary))]", className)}>
      <div className="overflow-x-auto">
        <table className="w-full text-sm text-left">
          <thead className="text-xs uppercase bg-[rgb(var(--color-surface-secondary))] text-white border-b border-[rgb(var(--color-border))]">
            <tr>
              <th scope="col" className="px-6 py-3 font-medium w-1/2">Variable Name</th>
              <th scope="col" className="px-6 py-3 font-medium w-1/4">Current Value</th>
              <th scope="col" className="px-6 py-3 font-medium w-1/4">New Value</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-[rgb(var(--color-border))]">
            {variables.map(([name, value]) => {
              const editedValue = editedVars[name];
              const currentValue = editedValue ? editedValue.value : value;
              const isEdited = !!editedValue;

              return (
                <tr 
                  key={name} 
                  className={`
                    group transition-colors hover:bg-[rgb(var(--color-surface-hover))]
                    ${isEdited ? 'bg-yellow-500/5 hover:bg-yellow-500/10' : ''}
                  `}
                >
                  <td className="px-6 py-4 font-medium text-[rgb(var(--color-text-primary))] relative">
                    {isEdited && (
                      <div className="absolute left-0 top-0 bottom-0 w-1 bg-yellow-500" />
                    )}
                    <div className="flex items-center gap-2">
                      <span className="break-all">{name}</span>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity"
                        onClick={() => copyToClipboard(name)}
                        title="Copy variable name"
                      >
                        <Copy className="h-3 w-3" />
                      </Button>
                      {isEdited && (
                        <Badge variant="secondary" className="ml-2 bg-yellow-500/20 text-yellow-500 hover:bg-yellow-500/30 border-yellow-500/20">
                          Modified
                        </Badge>
                      )}
                    </div>
                  </td>
                  <td className="px-6 py-4 text-[rgb(var(--color-text-muted))] font-mono">
                    {display(value)}
                  </td>
                  <td className="px-6 py-4">
                    <Input
                      type={type === 'string' ? 'text' : 'number'}
                      step={type === 'float' ? '0.01' : '1'}
                      value={currentValue}
                      onChange={(e) => onVariableChange(name, e.target.value, type)}
                      className={`
                        w-full h-9 font-mono
                        ${isEdited ? 'border-yellow-500/50 focus-visible:ring-yellow-500' : ''}
                      `}
                      placeholder={String(value)}
                    />
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}
