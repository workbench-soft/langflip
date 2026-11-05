# User flows

## First start

1. The person runs `langflip.exe` (Windows) or `Langflip` (macOS).
2. On macOS the system asks for Accessibility and Input Monitoring access. The person allows both and starts Langflip again.
3. The tray icon shows the current layout in navy on a holo foil tile: `EN` or `RU`. Langflip writes `settings.toml` with the defaults and turns on start at login, with no question.

```mermaid
flowchart LR
    A[Start Langflip] --> B{macOS?}
    B -- yes --> C[Allow Accessibility and Input Monitoring] --> D[Start again]
    B -- no --> E[Tray icon appears]
    D --> E
```

## Install on Windows

1. The person double-clicks `Langflip.cmd` in the repo folder.
2. A console window builds Langflip and closes. On an error it stays open.
3. Langflip starts, and its icon appears on the desktop and in the Start menu.
4. To remove it, the person opens Settings, Apps and presses Uninstall next to Langflip.

```mermaid
flowchart LR
    A[Double-click Langflip.cmd] --> B[Build and install] --> C[Shortcuts and Apps entry] --> D[Langflip starts]
    C -. later .-> E[Settings, Apps, Uninstall] --> F[Folder and shortcuts removed]
```

## Update Langflip

1. A minute after the start, and about once a day, Langflip asks GitHub for its releases.
2. A newer release brings a dialog: «Langflip 2026.11.05.1 is out», the size in MB, Update and Later.
3. **Update** Langflip downloads the exe, swaps it in and restarts. The tray icon comes back in a few seconds.
4. **Later** the dialog stays away for that version until the next start.

```mermaid
flowchart LR
    A[Start or daily check] --> B{Newer release?}
    B -- no or offline --> A
    B -- yes --> C[Dialog: Update or Later]
    C -- Later --> A
    C -- Update --> D[Download and swap the exe] --> E[Langflip restarts]
```

## A word in the wrong layout

1. The English layout is on. The person types `ghb`.
2. The person presses `d`.
3. Langflip erases `ghb`, types «прив» and switches to Russian.
4. The person types the rest of the word, «ет», in Russian.
5. A word Langflip can't judge from its first letters gets the same fix on Space, Enter or Tab.

```mermaid
sequenceDiagram
    participant P as Person
    participant L as Langflip
    participant A as App
    P->>A: ghb
    P->>L: d
    L->>A: 3 Backspaces, «прив»
    L->>A: switch layout to Russian
    P->>A: ет and Space
```

## Go on with a word

1. «привет» sits in the text, and the English layout is on.
2. The person clicks right after «привет», or presses Backspace right after its Space.
3. The person types `c`.
4. Langflip erases `c`, types «с» and switches to Russian. «привет» stays.
5. The person types the rest, «твие», in Russian.

```mermaid
sequenceDiagram
    participant P as Person
    participant L as Langflip
    participant A as App
    P->>A: click after «привет»
    L->>A: read the text before the caret
    P->>L: c
    L->>A: «с»
    L->>A: switch layout to Russian
    P->>A: твие and Space
```

## Undo

1. Langflip just fixed a word the person meant as typed, such as `vtnf` meant as a code name.
2. The person presses Pause (Ctrl+F12 on macOS).
3. Langflip puts the word back and switches the layout back.
4. The next time Langflip fixes the word again. A second Pause puts it back and adds it to the exceptions.
5. The word shows in the tray menu under «Исключения». A click on it there removes it.

```mermaid
flowchart LR
    A[Fixed word] --> B[Pause] --> C{Second Undo of this word?}
    C -- no --> D[Word as typed, fixed next time]
    C -- yes --> E[Added to keep_words] --> F[Listed in the tray menu]
    F -- click --> G[Removed, fixed again]
```

## Convert by hand

1. The person typed `ytn` (3 letters, so Langflip left it).
2. The person presses Shift+Pause (Ctrl+Shift+F12 on macOS).
3. Langflip converts it to «нет» and switches the layout.

```mermaid
flowchart LR
    A[ytn and Space] --> B[Shift+Pause] --> C[«нет»]
```

## Pause it

1. The person clicks the tray icon with the left button.
2. The icon turns grey and keeps the layout letters. Typing passes through untouched.
3. The next click turns fixing on, and the icon is in colour again.

```mermaid
flowchart LR
    A[Left click on the icon] --> B[Grey icon, no fixes]
    B -->|left click| C[Colour icon, fixes on]
```

## Switch the layout from the tray

1. The person double-clicks the tray icon.
2. The window they typed in comes back to the front with the other layout.
3. The icon shows `EN` or `RU` for the new layout.
4. Other windows they go to keep the new layout.

```mermaid
flowchart LR
    A[Double click on the icon] --> B[Last window back in front] --> C[EN turns RU, RU turns EN]
```

## Stop the autostart

1. The person right-clicks the tray icon.
2. The person unchecks «Запускать при входе».
3. Langflip no longer starts at login, and later starts keep it off.

```mermaid
flowchart LR
    A[Right click on the icon] --> B[Uncheck Запускать при входе] --> C[No start at login]
```

## Pick the menu language

1. The person right-clicks the tray icon.
2. The person opens Language and clicks a language, such as Deutsch.
3. The menu and the tooltip switch to that language at once.
4. Langflip saves `language = "de"` in `settings.toml`, and later starts keep it.
5. A click on System language puts the system language back.

```mermaid
flowchart LR
    A[Right click on the icon] --> B[Language] --> C{Pick}
    C -- a language --> D[Menu in that language] --> F[Saved in settings.toml]
    C -- System language --> E[Menu in the system language] --> F
```

## Release build timing

Release builds MUST start on pushes alongside their matching checks. Publication MUST wait for successful checks of the same commit and branch. Open PRs MUST NOT delay the build. LocalCI MUST record the causing run, commit and PR before cancelling a replaced build or a release whose checks failed.
