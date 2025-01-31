This code only tested on Windows 11.

Measure mouse usage time with this tool. Mouse clicks, scroll wheel and mouse movements are measured.


![alt text](https://iili.io/2QpOgrg.png "tray")
# Install Rust:
https://doc.rust-lang.org/cargo/getting-started/installation.html 

# Compile Code:
Download or Pull codebase.

At TrackMouseUsage folder open terminal.
`cargo run --release`

You can start program at .\TrackMouseUsage\target\release\TrackMouse.exe

# Reset usage time:
Program will create a SQLite Database at .\TrackMouseUsage\target\release\mouse_usage.db .
1. Stop the program.
   * Delete mouse_usage.db
   *  *OR*
   * Install https://sqlitebrowser.org/ SQLite Browser and set the last entry's usage_duration_seconds to 0.
2. Start the program.

# Autorun with  Task Scheduler
![alt text](https://iili.io/2ZFsUg9.png "task")

`Start-Process -FilePath "**enter your own file path**   like this C:\TrackMouse\target\release\TrackMouse.exe" -WindowStyle Hidden`
