'use client';

import { useMemo, useCallback } from 'react';
import Fuse from 'fuse.js';

// Use a generic type parameter instead of defining our own FeatInfo
// This allows the hook to work with any feat-like data structure

interface SearchOptions {
  keys?: string[];
  threshold?: number;
  includeScore?: boolean;
  limit?: number;
}

const DEFAULT_SEARCH_OPTIONS: SearchOptions = {
  keys: ['label', 'name', 'description'],
  threshold: 0.3,
  includeScore: true,
  limit: 100,
};

export function useFeatSearch<T extends { id: number; label: string; name: string }>(
  feats: T[],
  searchTerm: string,
  options: SearchOptions = {}
) {
  // Merge options with defaults
  const searchOptions = useMemo(
    () => ({ ...DEFAULT_SEARCH_OPTIONS, ...options }),
    [options]
  );

  // Create Fuse index for the feats
  const fuseIndex = useMemo(() => {
    if (!feats || feats.length === 0) return null;
    
    const fuseOptions = {
      keys: searchOptions.keys,
      threshold: searchOptions.threshold,
      includeScore: searchOptions.includeScore,
      // Additional performance optimizations
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
    
    return new Fuse(feats, fuseOptions);
  }, [feats, searchOptions]);

  // Perform search
  const searchResults = useMemo(() => {
    if (!fuseIndex || !searchTerm || searchTerm.trim().length < 3) {
      return feats;
    }

    const results = fuseIndex.search(searchTerm.trim());
    
    // Apply limit if specified
    const limitedResults = searchOptions.limit 
      ? results.slice(0, searchOptions.limit)
      : results;
    
    // Extract items from Fuse results
    return limitedResults.map(result => result.item);
  }, [fuseIndex, searchTerm, feats, searchOptions.limit]);

  // Highlight search terms in text
  const highlightSearchTerm = useCallback((text: string): string => {
    if (!searchTerm || searchTerm.trim().length < 3) {
      return text;
    }

    const regex = new RegExp(`(${searchTerm.trim()})`, 'gi');
    return text.replace(regex, '<mark>$1</mark>');
  }, [searchTerm]);

  // Check if search is active
  const isSearching = searchTerm && searchTerm.trim().length >= 3;

  // Get search statistics
  const searchStats = useMemo(() => ({
    totalResults: searchResults.length,
    totalFeats: feats.length,
    isFiltered: isSearching,
    searchTerm: searchTerm.trim(),
  }), [searchResults.length, feats.length, isSearching, searchTerm]);

  return {
    searchResults,
    isSearching,
    searchStats,
    highlightSearchTerm,
  };
}

// Hook for pre-indexing feats for multiple searches
export function useFeatSearchIndex<T extends { id: number; label: string; name: string }>(feats: T[], options: SearchOptions = {}) {
  const searchOptions = useMemo(
    () => ({ ...DEFAULT_SEARCH_OPTIONS, ...options }),
    [options]
  );

  // Create persistent Fuse index
  const fuseIndex = useMemo(() => {
    if (!feats || feats.length === 0) return null;
    
    const fuseOptions = {
      keys: searchOptions.keys,
      threshold: searchOptions.threshold,
      includeScore: searchOptions.includeScore,
      shouldSort: true,
      isCaseSensitive: false,
      minMatchCharLength: 2,
    };
    
    return new Fuse(feats, fuseOptions);
  }, [feats, searchOptions]);

  // Search function
  const search = useCallback((searchTerm: string, limit?: number): T[] => {
    if (!fuseIndex || !searchTerm || searchTerm.trim().length < 3) {
      return feats;
    }

    const results = fuseIndex.search(searchTerm.trim());
    const limitedResults = limit ? results.slice(0, limit) : results;
    return limitedResults.map(result => result.item);
  }, [fuseIndex, feats]);

  // Batch search for multiple terms
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
    indexSize: feats.length,
  };
}