# Test Files

This directory contains various test files and utilities for the RDumper project.

## Files

- `test_parse` - Test script for parsing functionality
- `setup_test_data.sql` - SQL script to set up test database data
- `setup_test_data.sh` - Shell script to set up test data
- `backup_process_test.rs` - Rust test for backup process functionality
- `tests.rs` - Additional Rust test utilities

## Usage

### Setting up test data
```bash
./setup_test_data.sh
```

This will create test database configurations and tasks for development and testing purposes.

### Running parse tests
```bash
./test_parse
```

## Note

These are development/testing utilities and should not be used in production environments.
