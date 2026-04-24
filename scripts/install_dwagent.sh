#!/bin/bash

# DWService Agent Installation Script for Railway
# This script downloads and installs the DWService monitoring agent

set -e

echo "🚀 Installing DWService Agent..."
echo ""

# Download DWAgent
echo "📥 Downloading DWAgent..."
wget -q https://www.dwservice.net/download/dwagent_x86_64.sh -O /tmp/dwagent.sh

# Make executable
chmod +x /tmp/dwagent.sh

# Install
echo "🔧 Installing..."
sudo /tmp/dwagent.sh

echo ""
echo "✅ DWService Agent installation complete!"
echo "   Check status at: https://www.dwservice.net"
