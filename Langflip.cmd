@echo off
rem Double-click to build Langflip from this checkout, install it for this user and start it.
cd /d "%~dp0"
mise run install
if errorlevel 1 pause
