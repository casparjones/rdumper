# Changelog

All notable changes to rDumper will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.6] - 2025-10-02
### Added
- **Human-Readable Backup Directory Names**: Backup directories now use format `<database-config-name>-<database-name>-<uuid>` instead of just `<uuid>`
- **Improved Backup Organization**: Backup folders are now easily identifiable by database name for better file system navigation
- **Filesystem-Safe Naming**: Automatic sanitization of database names to ensure compatibility with all filesystems

### Changed
- **Backup Directory Structure**: New backups create directories with meaningful names (e.g., `fschen.de-canadmin-550e8400-e29b-41d4-a716-446655440000`)
- **Backward Compatibility**: Existing UUID-only backup directories continue to work seamlessly
- **Database Name Resolution**: Uses `used_database` field format for consistent naming across scheduled and manual backups

### Technical Details
- **FilesystemBackupService**: Enhanced `generate_backup_directory_name` function with task-aware naming
- **Character Sanitization**: Replaces filesystem-unsafe characters (/, \, :, *, ?, ", <, >, |) with underscores
- **Fallback Logic**: Uses database config name when task information is unavailable

## [0.1.5] - 2025-10-02
### Added
- **Runtime configuration for the frontend**
    - The API URL (`API_BASE_URL`) is now injected at runtime.
    - On container startup a `config.js` file is generated and consumed by the frontend.
    - No need to rebuild the Docker image to change the API URL.

### Changed
- Improved frontend fallback logic:
    1. `window.__RDUMPER_API_URL__` (from `config.js`)
    2. `import.meta.env` (used in local Vite development)
    3. automatic detection via `window.location`

## [0.1.4] - 2025-10-01

### Fixed
- **Job Error Handling**: Fixed critical issue where jobs would remain stuck in `pending` status due to database engine analysis failures
- **NULL Engine Handling**: Added graceful handling for NULL `ENGINE` values in MySQL table analysis
- **ColumnDecode Error**: Resolved `ColumnDecode { index: "ENGINE", source: UnexpectedNullError }` panic in mydumper service
- **Job Status Updates**: Improved job status management to ensure failed jobs are properly marked with error messages

### Changed
- **Database Engine Analysis**: Enhanced `analyze_table_engines` function to handle NULL engine values gracefully
- **Error Propagation**: Jobs now properly transition to `failed` status with detailed error messages when database analysis fails
- **Logging Improvements**: Added comprehensive error logging for database connection and analysis failures

### Technical Details
- **NULL Engine Fallback**: Tables with NULL engine values are now treated as InnoDB with appropriate warnings
- **Job Status Management**: Immediate job status updates to `running` at start, `failed` on errors
- **Error Message Storage**: Detailed error messages are now stored in job `error_message` field for debugging

## [0.1.3] - 2025-10-01

### Added
- **Database Independence for Jobs and Backups**: Added `used_database` field to both `jobs` and `backups` tables
- **Static Database Information**: Jobs and backups now store database information in format `<connection-name>/<database>` (e.g., `localhost/langify`)
- **Task-Specific Database Selection**: Tasks can now specify a specific database name independent of the connection's default database
- **Dynamic Database Discovery**: System automatically discovers available databases for connections without default database
- **Database Information Display**: Frontend now shows database information in consistent format across Jobs and Backups tables

### Changed
- **Job Creation**: Jobs now store `used_database` field derived from task configuration at creation time
- **Backup Creation**: Backups now store `used_database` field in metadata files and database records
- **Frontend Display Logic**: Updated `getDatabaseDisplayName` and `getDatabaseForJob` functions to prioritize `used_database` field
- **Database Migration**: Added migration to add `used_database` column to existing `jobs` and `backups` tables
- **Backup Metadata**: Enhanced `BackupMetadata` structure to include `used_database` field

### Fixed
- **Task Database Selection**: Fixed issue where task database changes would affect existing backup displays
- **Database Information Consistency**: Resolved problem where backup database information would change when task database was modified
- **Frontend Fallback Logic**: Improved fallback display logic for backups without `used_database` field

### Technical Details
- **Database Schema**: Added `used_database TEXT` column to both `jobs` and `backups` tables
- **Model Updates**: Extended `Job`, `Backup`, and `BackupMetadata` models with `used_database` field
- **API Compatibility**: Maintained backward compatibility while adding new database information fields
- **Migration Safety**: Database migrations safely add new columns without affecting existing data

## [0.1.2] - 2025-09-30

### Added
- **Background Task Worker**: Automated background service that runs every minute to check and execute scheduled tasks
- **Cron Schedule Support**: Full cron expression parsing with support for common patterns
- **Task Scheduling**: Tasks now have `last_run` and `next_run` fields for automatic execution tracking
- **Worker Status Monitoring**: Real-time worker status display in System tab with health indicators
- **Conflict Detection**: Automatic detection of running jobs to prevent overlapping executions
- **Worker API Endpoints**: New `/api/worker/status` and `/api/system/worker` endpoints for monitoring

### Changed
- **Cron Parser**: Replaced problematic `cron` crate with custom implementation supporting:
  - `* * * * *` - Every minute
  - `0 * * * *` - Every hour  
  - `0 1 * * *` - Daily at 1:00 AM (and any specific time)
  - `0 0 * * *` - Daily at midnight
  - `0 0 * * 1` - Weekly on Monday at midnight
  - `*/5 * * * *` - Every 5 minutes
  - `0 */2 * * *` - Every 2 hours
- **Frontend Schedule Display**: Improved cron expression formatting with accurate descriptions
- **Task Model**: Enhanced with cron parsing methods and next run calculation
- **Database Schema**: Added `last_run` and `next_run` columns to tasks table

### Fixed
- **Cron Expression Parsing**: Resolved issues with `* * * * *` and `0 1 * * *` expressions
- **Frontend Display**: Fixed `* * * * *` showing "Daily" instead of "Every minute"
- **Worker Integration**: Proper integration of background worker with main application
- **Job Conflict Handling**: Automatic cancellation of overlapping jobs with clear error messages

### Technical Improvements
- **Background Processing**: Tokio-based background worker with 60-second intervals
- **Thread Safety**: Arc<Mutex<WorkerStatus>> for safe status sharing across threads
- **Error Handling**: Comprehensive error handling for cron parsing and task execution
- **Status Tracking**: Real-time worker health monitoring with color-coded indicators
- **API Design**: RESTful endpoints for worker status and system information

## [0.1.1] - 2025-09-29

### Added
- **Knight Rider Loading Effect**: Replaced loading spinners with animated light bar that moves left-to-right like Knight Rider
- **Global Loading System**: Centralized loading management with smooth transitions and no page flashing
- **SPA Fallback Support**: Backend now serves index.html for all non-API routes to support Vue Router
- **Real-time System Information**: System tab now displays live OS, kernel, uptime, memory, and disk space data
- **Enhanced Dashboard**: Real-time statistics with recent backups and next scheduled tasks
- **Progress Tracking**: On-the-fly log parsing for accurate job progress without database flooding

### Changed
- **Loading UX**: Removed all local loading spinners in favor of global progress bar
- **Page Transitions**: Pages only display content when data is loaded, preventing default page flashing
- **Backend Architecture**: Improved static file serving with SPA fallback for Docker deployment
- **Job Status**: Added "compressing" status to show when MyDumper is done but archive is being created

### Fixed
- **Vue Template Errors**: Resolved v-else without v-if compilation errors
- **Loading States**: Consistent loading behavior across all views
- **File Cleanup**: Removed unused filesystem_backup_fixed.rs and filesystem_backup_old.rs

### Technical Improvements
- **Frontend**: Global loading store with multi-page support
- **Backend**: SPA fallback middleware for proper Vue Router support
- **UI/UX**: Smooth transitions and professional loading animations
- **Code Quality**: Removed unused imports and dead code

## [Unreleased]

### Added
- Authors section in README with project contributors
- CHANGELOG.md for tracking project changes

### Changed
- README authors section with bilingual credit attribution

## [2025-09-28] - 2025-09-28

### Added
- **Advanced Options Panel**: Collapsible "Advanced Options" section in Task creation/editing
- **MyISAM Support**: Option to backup non-transactional tables (MyISAM) with special parameters
- **InnoDB Table Analysis**: Automatic detection and filtering of table engines before backup
- **MyDumper Help Integration**: Automatic `--help` execution before each backup to show available options
- **Engine-based Filtering**: Automatic exclusion of MyISAM, MEMORY, CSV, ARCHIVE, FEDERATED, MERGE, BLACKHOLE tables
- **Duplicate Database Config**: "Duplicate" button for easy creation of similar database configurations
- **Password Visibility Toggle**: Show/hide password option in database configuration editing
- **Enhanced Logging**: Detailed warnings about excluded tables and engine types

### Changed
- **MyDumper Parameters**: 
  - Standard backup: Uses `--ignore-engines` to exclude non-InnoDB tables
  - Advanced backup: Uses `--trx-tables=0 --no-backup-locks` for MyISAM support
- **Database Configuration UI**: 
  - Password field shows hint "Leave empty to keep current password" when editing
  - Password is optional during editing (required only for new configurations)
  - Added "Show password" checkbox for visibility control
- **Task Creation Flow**: Advanced options are collapsed by default, auto-expand when MyISAM is enabled
- **Backup Strategy**: Intelligent engine detection with automatic filtering for data consistency

### Fixed
- **MyDumper Parameter Error**: Replaced invalid `--tables` option with correct `--ignore-engines` parameter
- **Multipart Upload**: Resolved "Failed to read file data" error by switching to `axum-extra::extract::Multipart`
- **Frontend API Configuration**: Automatic domain detection instead of hardcoded localhost:3000
- **Environment Variables**: Added support for `VITE_API_URL`, `VITE_API_PORT`, `VITE_DEV_PORT`

### Technical Improvements
- **Backend**: Enhanced MyDumper service with table engine analysis and intelligent parameter selection
- **Frontend**: Improved Advanced Options UI with collapsible design and smart state management
- **Database Schema**: Added `use_non_transactional` field to tasks table with migration support
- **API Integration**: Better error handling for multipart uploads and environment-based configuration
- **Logging**: Comprehensive logging of MyDumper options and table analysis results

### Security & Data Integrity
- **InnoDB-only Backups**: Default strategy ensures only transactional tables are backed up
- **MyISAM Warning**: Clear warnings about potential data inconsistency with non-transactional tables
- **Engine Validation**: Automatic detection and exclusion of problematic storage engines

## [2025-09-27] - 2025-09-27

### Added
- **Toast Notifications**: Replaced intrusive alert() dialogs with elegant toast notifications
- **Progress Parsing Enhancement**: Implemented detailed mydumper progress parsing from log output
- **Database Display Improvements**: Enhanced database name display in Tasks and Jobs views
- **Smooth Table Refresh**: Added CSS transitions for seamless table updates without flickering
- **Auto-refresh System**: Dynamic refresh intervals (1s for active jobs, 5s for inactive)
- **Vue Router Navigation**: Proper SPA navigation for task execution workflow
- **Log Cleanup Integration**: Automatic log file deletion when jobs are deleted
- **Icon-only UI Buttons**: Streamlined button design with ghost styling

### Changed
- **Jobs UI**: Integrated running jobs into main table instead of separate section
- **Task Execution Flow**: Removed confirmation dialog, added automatic Jobs tab navigation
- **Backup Deletion**: Fixed to delete only specific backup instead of all backups
- **Progress Tracking**: Enhanced from simple table counting to detailed percentage parsing
- **Database Information Display**: 
  - Tasks: Shows config name + database details (host:port)
  - Jobs: Shows config name + actual database name
  - Backups: Maintains existing database name display

### Fixed
- **Mydumper Logging**: Corrected log level parsing (INFO for ** Message:, ERROR for error/failed/fatal)
- **Backup Records**: Fixed missing backup record creation in database
- **Job Log Cleanup**: Ensured complete log directory removal including auxiliary files
- **Navigation Issues**: Fixed Vue Router integration for proper SPA navigation
- **Progress Parsing**: Resolved 0% → 100% jumps with continuous progress updates

### Technical Improvements
- **Rust Backend**: Enhanced mydumper service with intelligent progress parsing
- **Vue Frontend**: Improved component lifecycle and state management
- **API Integration**: Better error handling and user feedback
- **Database Schema**: Maintained compatibility while adding new features
- **File System Operations**: More robust cleanup and error handling

### Contributors
- **Frank** - Project Owner & Lead Developer
- **Claude (Anthropic)** - AI Assistant & Code Contributor
- **ChatGPT (OpenAI)** - AI Assistant & Code Contributor

---

*"Give credit where credit is due."*  
*"Honor should be given to whom honor is due."* 😊
