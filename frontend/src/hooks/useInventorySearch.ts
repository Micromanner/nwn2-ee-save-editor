
import { useMemo, useCallback } from 'react';
import Fuse from 'fuse.js';

interface SearchOptions {
  keys?: string[];
  threshold?: number;
  includeScore?: boolean;
  limit?: number;
}

const DEFAULT_SEARCH_OPTIONS: SearchOptions = {
  keys: ['name', 'description'],
  threshold: 0.3,
  includeScore: true,
  limit: 100,
};

export function useInventorySearch<T extends { name: string }>(
  items: T[],
  searchTerm: string,
  options: SearchOptions = {}
) {
  const searchOptions = useMemo(
    () => ({ ...DEFAULT_SEARCH_OPTIONS, ...options }),
    [options]
  );

  const fuseIndex = useMemo(() => {
    if (!items || items.length === 0) return null;

    const fuseOptions = {
      keys: searchOptions.keys,
      threshold: searchOptions.threshold,
      includeScore: searchOptions.includeScore,
      shouldSort: true,
      isCaseSensitive: false,
      includeMatches: false,
      findAllMatches: false,
      minMatchCharLength: 2,
      location: 0,
      distance: 100,
      useExtendedSearch: false,
      ignoreLocation: false,
      ignoreFieldNorm: false,
      fieldNormWeight: 1,
    };

    return new Fuse(items, fuseOptions);
  }, [items, searchOptions]);

  const searchResults = useMemo(() => {
    if (!fuseIndex || !searchTerm || searchTerm.trim().length < 2) {
      return items;
    }

    const results = fuseIndex.search(searchTerm.trim());

    const limitedResults = searchOptions.limit
      ? results.slice(0, searchOptions.limit)
      : results;

    return limitedResults.map(result => result.item);
  }, [fuseIndex, searchTerm, items, searchOptions.limit]);

  const highlightSearchTerm = useCallback((text: string): string => {
    if (!searchTerm || searchTerm.trim().length < 2) {
      return text;
    }

    const regex = new RegExp(`(${searchTerm.trim()})`, 'gi');
    return text.replace(regex, '<mark>$1</mark>');
  }, [searchTerm]);

  const isSearching = searchTerm && searchTerm.trim().length >= 2;

  const searchStats = useMemo(() => ({
    totalResults: searchResults.length,
    totalItems: items.length,
    isFiltered: isSearching,
    searchTerm: searchTerm.trim(),
  }), [searchResults.length, items.length, isSearching, searchTerm]);

  return {
    searchResults,
    isSearching,
    searchStats,
    highlightSearchTerm,
  };
}
