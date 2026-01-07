#!/usr/bin/env python3
"""
Simple CPU stress test - maxes out one core
Press Ctrl+C to stop
"""

print("Starting CPU stress test on one core...")
print("Press Ctrl+C to stop")

try:
    while True:
        pass  # Infinite loop - will max out one core
except KeyboardInterrupt:
    print("\nStopped.")

