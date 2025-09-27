# Changelog

All notable changes to rDumper will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Authors section in README with project contributors
- CHANGELOG.md for tracking project changes

### Changed
- README authors section with bilingual credit attribution

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
