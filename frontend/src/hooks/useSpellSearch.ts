'use client';

import { useMemo, useCallback } from 'react';
import Fuse from 'fuse.js';

interface SearchOptions {
  keys?: string[];
  threshold?: number;
  includeScore?: boolean;
  limit?: number;
}

const DEFAULT_SEARCH_OPTIONS: SearchOptions = {
  keys: ['name', 'description', 'school_name'],
  threshold: 0.3,
  includeScore: true,
  limit: 100,
};

export function useSpellSearch<T extends { id: number; name: string }>(
  spells: T[],
  searchTerm: string,
  options: SearchOptions = {}
) {
  const searchOptions = useMemo(
    () => ({ ...DEFAULT_SEARCH_OPTIONS, ...options }),
    [options]
  );

  const fuseIndex = useMemo(() => {
    if (!spells || spells.length === 0) return null;

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

    return new Fuse(spells, fuseOptions);
  }, [spells, searchOptions]);

  const searchResults = useMemo(() => {
    if (!fuseIndex || !searchTerm || searchTerm.trim().length < 3) {
      return spells;
    }

    const results = fuseIndex.search(searchTerm.trim());

    const limitedResults = searchOptions.limit
      ? results.slice(0, searchOptions.limit)
      : results;

    return limitedResults.map(result => result.item);
  }, [fuseIndex, searchTerm, spells, searchOptions.limit]);

  const highlightSearchTerm = useCallback((text: string): string => {
    if (!searchTerm || searchTerm.trim().length < 3) {
      return text;
    }

    const regex = new RegExp(`(${searchTerm.trim()})`, 'gi');
    return text.replace(regex, '<mark>$1</mark>');
  }, [searchTerm]);

  const isSearching = searchTerm && searchTerm.trim().length >= 3;

  const searchStats = useMemo(() => ({
    totalResults: searchResults.length,
    totalSpells: spells.length,
    isFiltered: isSearching,
    searchTerm: searchTerm.trim(),
  }), [searchResults.length, spells.length, isSearching, searchTerm]);

  return {
    searchResults,
    isSearching,
    searchStats,
    highlightSearchTerm,
  };
}

export function useSpellSearchIndex<T extends { id: number; name: string }>(spells: T[], options: SearchOptions = {}) {
  const searchOptions = useMemo(
    () => ({ ...DEFAULT_SEARCH_OPTIONS, ...options }),
    [options]
  );

  const fuseIndex = useMemo(() => {
    if (!spells || spells.length === 0) return null;

    const fuseOptions = {
      keys: searchOptions.keys,
      threshold: searchOptions.threshold,
      includeScore: searchOptions.includeScore,
      shouldSort: true,
      isCaseSensitive: false,
      minMatchCharLength: 2,
    };

    return new Fuse(spells, fuseOptions);
  }, [spells, searchOptions]);

  const search = useCallback((searchTerm: string, limit?: number): T[] => {
    if (!fuseIndex || !searchTerm || searchTerm.trim().length < 3) {
      return spells;
    }

    const results = fuseIndex.search(searchTerm.trim());
    const limitedResults = limit ? results.slice(0, limit) : results;
    return limitedResults.map(result => result.item);
  }, [fuseIndex, spells]);

  const batchSearch = useCallback((searchTerms: string[]): Map<string, T[]> => {
    const results = new Map<string, T[]>();

    for (const term of searchTerms) {
      results.set(term, search(term));
    }

    return results;
  }, [search]);

  return {
    search,
    batchSearch,
    isIndexed: fuseIndex !== null,
    indexSize: spells.length,
  };
}
