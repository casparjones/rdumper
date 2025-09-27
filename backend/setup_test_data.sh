#!/bin/bash

# rDumper Test Data Setup Script
echo "🚀 Setting up test data for rDumper..."

# Check if database file exists
DB_FILE="data/db/rdumper.db"
if [ ! -f "$DB_FILE" ]; then
    echo "📁 Creating database directory and file..."
    mkdir -p data/db
    touch "$DB_FILE"
fi

# Check if sqlite3 is available
if ! command -v sqlite3 &> /dev/null; then
    echo "❌ sqlite3 is not installed. Please install it first."
    echo "   On Ubuntu/Debian: sudo apt-get install sqlite3"
    echo "   On macOS: brew install sqlite3"
    exit 1
fi

# Run the SQL script to insert test data
echo "📊 Inserting test data into database..."
sqlite3 "$DB_FILE" < setup_test_data.sql

if [ $? -eq 0 ]; then
    echo "✅ Test data setup completed successfully!"
    echo ""
    echo "📋 Created test configurations:"
    echo "   • Test Database (127.0.0.1:3306/test_db)"
    echo "   • Production Database (mysql.example.com:3306/production_db)"
    echo "   • Daily Test Backup task (enabled)"
    echo "   • Hourly Production Backup task (disabled)"
    echo ""
    echo "🎯 You can now start the backend server with:"
    echo "   cd backend && cargo run"
    echo ""
    echo "🌐 Then open the frontend at:"
    echo "   http://localhost:3000"
else
    echo "❌ Failed to insert test data. Please check the database file and try again."
    exit 1
fi
