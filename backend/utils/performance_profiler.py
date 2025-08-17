"""
Performance profiler for tracking hierarchical timing of operations.
Provides detailed breakdown of where time is spent during application startup.
"""
import time
import logging
from typing import Dict, List, Optional, Any
from dataclasses import dataclass, field
from contextlib import contextmanager
import json
from pathlib import Path

logger = logging.getLogger(__name__)


@dataclass
class TimingEntry:
    """Single timing entry in the profiling hierarchy."""
    name: str
    start_time: float
    end_time: Optional[float] = None
    children: List['TimingEntry'] = field(default_factory=list)
    metadata: Dict[str, Any] = field(default_factory=dict)
    
    @property
    def duration_ms(self) -> float:
        """Get duration in milliseconds."""
        if self.end_time is None:
            return 0.0
        return (self.end_time - self.start_time) * 1000
    
    @property
    def is_complete(self) -> bool:
        """Check if timing is complete."""
        return self.end_time is not None
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for serialization."""
        return {
            'name': self.name,
            'duration_ms': self.duration_ms,
            'metadata': self.metadata,
            'children': [child.to_dict() for child in self.children]
        }


class PerformanceProfiler:
    """
    Hierarchical performance profiler for tracking application startup times.
    
    Usage:
        profiler = PerformanceProfiler()
        
        with profiler.profile("Total Startup"):
            with profiler.profile("Load 2DA Files"):
                with profiler.profile("Parse Base 2DAs"):
                    # ... parse base files
                with profiler.profile("Parse Override 2DAs"):
                    # ... parse overrides
            
            with profiler.profile("Generate Classes"):
                # ... generate runtime classes
        
        profiler.print_report()
    """
    
    def __init__(self, name: str = "Application", log_to_file: bool = True):
        """
        Initialize the profiler.
        
        Args:
            name: Root name for the profiling session
            log_to_file: Whether to save detailed logs to file
        """
        self.root = TimingEntry(name, time.time())
        self.current_stack: List[TimingEntry] = [self.root]
        self.log_to_file = log_to_file
        self.log_file: Optional[Path] = None
        
        if log_to_file:
            log_dir = Path(__file__).parent.parent / "logs"
            log_dir.mkdir(exist_ok=True)
            timestamp = time.strftime("%Y%m%d_%H%M%S")
            self.log_file = log_dir / f"performance_{timestamp}.json"
    
    @contextmanager
    def profile(self, name: str, **metadata):
        """
        Context manager for profiling a code block.
        
        Args:
            name: Name of the operation being profiled
            **metadata: Additional metadata to store with the timing
        """
        entry = TimingEntry(name, time.time(), metadata=metadata)
        parent = self.current_stack[-1]
        parent.children.append(entry)
        self.current_stack.append(entry)
        
        try:
            logger.debug(f"{'  ' * (len(self.current_stack) - 2)}→ Starting: {name}")
            yield entry
        finally:
            entry.end_time = time.time()
            self.current_stack.pop()
            logger.debug(f"{'  ' * (len(self.current_stack) - 1)}← Completed: {name} ({entry.duration_ms:.2f}ms)")
    
    def mark(self, name: str, **metadata):
        """
        Mark a point in time without creating a context.
        Useful for tracking when specific events occur.
        
        Args:
            name: Name of the event
            **metadata: Additional metadata
        """
        entry = TimingEntry(name, time.time(), end_time=time.time(), metadata=metadata)
        parent = self.current_stack[-1]
        parent.children.append(entry)
        logger.debug(f"{'  ' * (len(self.current_stack) - 1)}• Marked: {name}")
    
    def add_metadata(self, key: str, value: Any):
        """Add metadata to the current timing context."""
        if self.current_stack:
            self.current_stack[-1].metadata[key] = value
    
    def finalize(self):
        """Finalize the root timing."""
        if not self.root.is_complete:
            self.root.end_time = time.time()
    
    def get_report(self, max_depth: Optional[int] = None) -> Dict[str, Any]:
        """
        Generate a timing report.
        
        Args:
            max_depth: Maximum depth to include in report (None for all)
        
        Returns:
            Dictionary containing timing breakdown
        """
        self.finalize()
        
        def build_report(entry: TimingEntry, depth: int = 0) -> Dict[str, Any]:
            if max_depth is not None and depth >= max_depth:
                return None
            
            report = {
                'name': entry.name,
                'duration_ms': entry.duration_ms,
                'percentage': 0.0,  # Will be calculated
            }
            
            if entry.metadata:
                report['metadata'] = entry.metadata
            
            if entry.children:
                children_reports = []
                for child in entry.children:
                    child_report = build_report(child, depth + 1)
                    if child_report:
                        children_reports.append(child_report)
                
                if children_reports:
                    report['children'] = children_reports
                    
                    # Calculate percentages
                    total_child_time = sum(c['duration_ms'] for c in children_reports)
                    for child_report in children_reports:
                        if entry.duration_ms > 0:
                            child_report['percentage'] = (child_report['duration_ms'] / entry.duration_ms) * 100
            
            return report
        
        return build_report(self.root)
    
    def print_report(self, max_depth: Optional[int] = None, min_duration_ms: float = 0.0):
        """
        Print a formatted timing report to console and optionally to file.
        
        Args:
            max_depth: Maximum depth to display
            min_duration_ms: Minimum duration to display (filters out fast operations)
        """
        self.finalize()
        
        print("\n" + "="*80)
        print(f"PERFORMANCE REPORT - {self.root.name}")
        print("="*80)
        
        def print_entry(entry: TimingEntry, indent: int = 0, parent_duration: Optional[float] = None):
            if max_depth is not None and indent >= max_depth:
                return
            
            if entry.duration_ms < min_duration_ms:
                return
            
            prefix = "  " * indent + ("|- " if indent > 0 else "")
            
            # Calculate percentage of parent time
            percentage = ""
            if parent_duration and parent_duration > 0:
                pct = (entry.duration_ms / parent_duration) * 100
                percentage = f" ({pct:.1f}%)"
            
            # Format duration
            if entry.duration_ms >= 1000:
                duration_str = f"{entry.duration_ms/1000:.2f}s"
            else:
                duration_str = f"{entry.duration_ms:.2f}ms"
            
            # Add metadata if present
            metadata_str = ""
            if entry.metadata:
                important_keys = ['count', 'size', 'files']
                meta_parts = []
                for key in important_keys:
                    if key in entry.metadata:
                        meta_parts.append(f"{key}={entry.metadata[key]}")
                if meta_parts:
                    metadata_str = f" [{', '.join(meta_parts)}]"
            
            try:
                print(f"{prefix}{entry.name}: {duration_str}{percentage}{metadata_str}")
            except UnicodeEncodeError:
                # Fallback for Windows cp1252 encoding issues
                safe_name = entry.name.encode('ascii', 'ignore').decode('ascii')
                print(f"{prefix}{safe_name}: {duration_str}{percentage}{metadata_str}")
            
            # Print children
            for child in sorted(entry.children, key=lambda x: x.duration_ms, reverse=True):
                print_entry(child, indent + 1, entry.duration_ms)
        
        print_entry(self.root)
        
        # Print summary statistics
        print("\n" + "-"*80)
        print("SUMMARY")
        print("-"*80)
        
        # Find slowest operations
        all_entries = []
        def collect_entries(entry: TimingEntry):
            all_entries.append(entry)
            for child in entry.children:
                collect_entries(child)
        
        collect_entries(self.root)
        slowest = sorted(all_entries, key=lambda x: x.duration_ms, reverse=True)[:10]
        
        print("\nTop 10 Slowest Operations:")
        for i, entry in enumerate(slowest, 1):
            if entry.duration_ms >= 1000:
                duration_str = f"{entry.duration_ms/1000:.2f}s"
            else:
                duration_str = f"{entry.duration_ms:.2f}ms"
            print(f"  {i:2d}. {entry.name}: {duration_str}")
        
        print("\n" + "="*80)
        print(f"Total Time: {self.root.duration_ms/1000:.2f}s")
        print("="*80)
        
        # Save to file if configured
        if self.log_to_file and self.log_file:
            report_data = {
                'timestamp': time.time(),
                'total_duration_ms': self.root.duration_ms,
                'report': self.get_report()
            }
            
            with open(self.log_file, 'w') as f:
                json.dump(report_data, f, indent=2)
            
            print(f"\nDetailed report saved to: {self.log_file}")
    
    def get_category_summary(self) -> Dict[str, float]:
        """
        Get a summary of time spent in major categories.
        
        Returns:
            Dictionary mapping category names to total time in ms
        """
        categories = {}
        
        def summarize(entry: TimingEntry, parent_name: str = ""):
            # Use top-level children as categories
            if parent_name == self.root.name:
                categories[entry.name] = entry.duration_ms
            
            for child in entry.children:
                summarize(child, entry.name)
        
        for child in self.root.children:
            summarize(child, self.root.name)
        
        return categories


# Global profiler instance for easy access
_global_profiler: Optional[PerformanceProfiler] = None


def get_profiler() -> PerformanceProfiler:
    """Get or create the global profiler instance."""
    global _global_profiler
    if _global_profiler is None:
        _global_profiler = PerformanceProfiler("FastAPI Startup")
    return _global_profiler


def reset_profiler():
    """Reset the global profiler."""
    global _global_profiler
    _global_profiler = None


@contextmanager
def profile(name: str, **metadata):
    """Convenience function to profile using the global profiler."""
    profiler = get_profiler()
    with profiler.profile(name, **metadata):
        yield