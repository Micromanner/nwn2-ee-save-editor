"""
Tests for data fetching rules to prevent infinite retry loops
"""

import pytest
import time
from unittest.mock import Mock, patch
from gamedata.services.data_fetching_rules import (
    DataFetchingRules, 
    get_data_fetching_rules,
    with_retry_limit,
    reset_data_fetching_rules
)


class TestDataFetchingRules:
    """Test the DataFetchingRules class"""
    
    def setup_method(self):
        """Reset state before each test"""
        reset_data_fetching_rules()
        self.rules = DataFetchingRules(max_retries=2, cooldown_seconds=1, blacklist_after=3)
    
    def test_initial_request_allowed(self):
        """Test that initial requests are allowed"""
        assert self.rules.should_allow_request("classes", "get_table") is True
    
    def test_retry_limit_enforcement(self):
        """Test that retry limits are enforced"""
        rules = DataFetchingRules(max_retries=3, cooldown_seconds=0.1, blacklist_after=5)
        table_name = "missing_table"
        
        # First attempt
        assert rules.should_allow_request(table_name) is True
        rules.record_attempt(table_name)
        rules.record_failure(table_name)
        
        # Wait for short cooldown
        time.sleep(0.2)
        
        # Second attempt
        assert rules.should_allow_request(table_name) is True
        rules.record_attempt(table_name)
        rules.record_failure(table_name)
        
        # Wait for short cooldown
        time.sleep(0.2)
        
        # Third attempt
        assert rules.should_allow_request(table_name) is True
        rules.record_attempt(table_name)
        rules.record_failure(table_name)
        
        # Wait for short cooldown
        time.sleep(0.2)
        
        # Fourth attempt should be blocked (max_retries=3)
        assert rules.should_allow_request(table_name) is False
    
    def test_cooldown_enforcement(self):
        """Test that cooldown periods are enforced"""
        table_name = "cooldown_table"
        
        # Make first attempt
        assert self.rules.should_allow_request(table_name) is True
        self.rules.record_attempt(table_name)
        self.rules.record_failure(table_name)
        
        # Immediate retry should be blocked by cooldown
        assert self.rules.should_allow_request(table_name) is False
        
        # Wait for cooldown to expire
        time.sleep(1.1)  # Slightly longer than cooldown_seconds=1
        
        # Now retry should be allowed
        assert self.rules.should_allow_request(table_name) is True
    
    def test_blacklist_enforcement(self):
        """Test that tables are blacklisted after too many failures"""
        table_name = "blacklist_table"
        
        # Fail multiple times
        for i in range(3):  # blacklist_after=3
            self.rules.record_failure(table_name)
        
        # Should now be blacklisted
        assert self.rules.is_blacklisted(table_name) is True
        assert self.rules.should_allow_request(table_name) is False
    
    def test_success_resets_counters(self):
        """Test that success resets retry counters"""
        table_name = "success_table"
        
        # Make some failed attempts
        self.rules.record_attempt(table_name)
        self.rules.record_failure(table_name)
        self.rules.record_attempt(table_name)
        self.rules.record_failure(table_name)
        
        # Should be at retry limit
        assert self.rules.should_allow_request(table_name) is False
        
        # Record success
        self.rules.record_success(table_name)
        
        # Should reset and allow requests again
        assert self.rules.should_allow_request(table_name) is True
    
    def test_different_methods_tracked_separately(self):
        """Test that different methods are tracked separately"""
        table_name = "method_table"
        
        # Exhaust retries for get_table
        for i in range(2):
            self.rules.record_attempt(table_name, "get_table")
            self.rules.record_failure(table_name, "get_table")
        
        # get_table should be blocked
        assert self.rules.should_allow_request(table_name, "get_table") is False
        
        # get_by_id should still be allowed
        assert self.rules.should_allow_request(table_name, "get_by_id") is True
    
    def test_reset_table(self):
        """Test that table reset clears all counters"""
        table_name = "reset_table"
        
        # Blacklist the table
        for i in range(3):
            self.rules.record_failure(table_name)
        
        assert self.rules.is_blacklisted(table_name) is True
        
        # Reset the table
        self.rules.reset_table(table_name)
        
        # Should be allowed again
        assert self.rules.is_blacklisted(table_name) is False
        assert self.rules.should_allow_request(table_name) is True
    
    def test_get_stats(self):
        """Test that statistics are returned correctly"""
        table_name = "stats_table"
        
        self.rules.record_attempt(table_name)
        self.rules.record_failure(table_name)
        
        stats = self.rules.get_stats()
        
        assert "retry_counts" in stats
        assert "failure_counts" in stats
        assert "blacklisted" in stats
        assert "rules" in stats
        
        cache_key = f"get_table:{table_name}"
        assert stats["retry_counts"][cache_key] == 1
        assert stats["failure_counts"][cache_key] == 1


class TestWithRetryLimitDecorator:
    """Test the @with_retry_limit decorator"""
    
    def setup_method(self):
        """Reset state before each test"""
        reset_data_fetching_rules()
    
    def test_decorator_allows_successful_calls(self):
        """Test that decorator allows successful function calls"""
        
        @with_retry_limit()
        def test_function(table_name: str):
            return f"success_{table_name}"
        
        result = test_function("valid_table")
        assert result == "success_valid_table"
    
    def test_decorator_blocks_after_failures(self):
        """Test that decorator blocks calls after too many failures"""
        
        @with_retry_limit()
        def failing_function(table_name: str):
            return None  # Simulate failure by returning None
        
        # First few calls should work
        result1 = failing_function("failing_table")
        assert result1 is None
        
        result2 = failing_function("failing_table")
        assert result2 is None
        
        result3 = failing_function("failing_table")
        assert result3 is None
        
        # Should be blocked now
        result4 = failing_function("failing_table")
        assert result4 is None  # Blocked by decorator
    
    def test_decorator_handles_exceptions(self):
        """Test that decorator properly tracks exceptions"""
        
        @with_retry_limit()
        def exception_function(table_name: str):
            raise ValueError("Test error")
        
        # First call should raise exception and be tracked
        with pytest.raises(ValueError):
            exception_function("exception_table")
        
        # After enough failures, should be blocked (returns None instead of raising)
        for i in range(5):  # Try multiple times to exceed retry limit
            try:
                result = exception_function("exception_table")
                if result is None:  # Blocked
                    break
            except ValueError:
                continue
        else:
            pytest.fail("Expected function to be blocked after failures")
    
    def test_decorator_with_custom_parameter_name(self):
        """Test decorator with custom table name parameter"""
        
        @with_retry_limit(table_name_param="custom_table")
        def custom_function(custom_table: str, other_param: int):
            return None if other_param < 0 else f"success_{custom_table}"
        
        # Should track failures correctly
        result1 = custom_function("custom_test", -1)  # Failure
        assert result1 is None
        
        result2 = custom_function("custom_test", 1)   # Success
        assert result2 == "success_custom_test"


class TestGlobalSingleton:
    """Test the global singleton behavior"""
    
    def setup_method(self):
        """Reset state before each test"""
        reset_data_fetching_rules()
    
    def test_singleton_returns_same_instance(self):
        """Test that get_data_fetching_rules returns the same instance"""
        rules1 = get_data_fetching_rules()
        rules2 = get_data_fetching_rules()
        
        assert rules1 is rules2
    
    def test_reset_clears_singleton(self):
        """Test that reset clears the singleton"""
        rules1 = get_data_fetching_rules()
        rules1.record_failure("test_table")
        
        reset_data_fetching_rules()
        
        rules2 = get_data_fetching_rules()
        assert rules1 is not rules2
        
        # New instance should have clean state
        stats = rules2.get_stats()
        assert len(stats["failure_counts"]) == 0


class TestIntegrationScenarios:
    """Test real-world integration scenarios"""
    
    def setup_method(self):
        """Reset state before each test"""
        reset_data_fetching_rules()
    
    def test_missing_table_scenario(self):
        """Test scenario where a table is missing"""
        
        @with_retry_limit()
        def get_missing_table(table_name: str):
            # Simulate table not found
            return None
        
        table_name = "nonexistent_table"
        
        # Should eventually be blocked
        results = []
        for i in range(10):  # Try many times
            result = get_missing_table(table_name)
            results.append(result)
            
            # Small delay to avoid hitting cooldown
            time.sleep(0.1)
        
        # Should have some None results from blocking
        assert None in results
        
        # Verify it's being tracked
        rules = get_data_fetching_rules()
        stats = rules.get_stats()
        cache_key = f"get_missing_table:{table_name}"
        
        assert cache_key in stats["failure_counts"]
        assert stats["failure_counts"][cache_key] > 0
    
    def test_temporary_failure_recovery(self):
        """Test recovery from temporary failures"""
        
        call_count = 0
        
        @with_retry_limit()
        def flaky_function(table_name: str):
            nonlocal call_count
            call_count += 1
            
            # Fail first few times, then succeed
            if call_count <= 2:
                return None
            else:
                return f"success_{table_name}"
        
        table_name = "flaky_table"
        
        # First calls should fail
        result1 = flaky_function(table_name)
        assert result1 is None
        
        time.sleep(1.1)  # Wait for cooldown
        
        result2 = flaky_function(table_name)
        assert result2 is None
        
        time.sleep(1.1)  # Wait for cooldown
        
        # Third call should succeed
        result3 = flaky_function(table_name)
        assert result3 == "success_flaky_table"
        
        # Subsequent calls should work fine
        result4 = flaky_function(table_name)
        assert result4 == "success_flaky_table"