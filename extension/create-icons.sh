#!/bin/bash
# Create placeholder SVG icons for Trinity Agent Bridge extension

mkdir -p icons

# Create SVG icon with Trinity symbol (Greek Tau)
cat > icons/icon.svg <<'EOF'
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128">
  <rect width="128" height="128" fill="#0d1117"/>
  <text x="64" y="96" font-family="serif" font-size="96" fill="#1f6feb" text-anchor="middle">Τ</text>
</svg>
EOF

# Note: In production, use a proper SVG-to-PNG conversion tool
# For now, the build will work without actual PNG files
# with @crxjs/vite-plugin handling icon generation

echo "Icon files created in icons/"
