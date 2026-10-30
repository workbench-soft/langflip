# User flows

## First start

1. The person runs `langflip.exe` (Windows) or `Langflip` (macOS).
2. On macOS the system asks for Accessibility and Input Monitoring access. The person allows both and starts Langflip again.
3. The tray icon shows the current layout: `EN` on blue or `RU` on red. Langflip writes `settings.toml` with the defaults and turns on start at login, with no question.

```mermaid
flowchart LR
    A[Start Langflip] --> B{macOS?}
    B -- yes --> C[Allow Accessibility and Input Monitoring] --> D[Start again]
    B -- no --> E[Tray icon appears]
    D --> E
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

## Undo

1. Langflip just fixed a word the person meant as typed, such as `vtnf` meant as a code name.
2. The person presses Pause (Ctrl+F12 on macOS).
3. Langflip puts the word back and switches the layout back.
4. Langflip keeps that word as typed from now on.

```mermaid
flowchart LR
    A[Fixed word] --> B[Pause] --> C[Word as typed] --> D[Added to keep_words]
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
3. The next click turns fixing on, and the icon is blue or red again.

```mermaid
flowchart LR
    A[Left click on the icon] --> B[Grey icon, no fixes]
    B -->|left click| C[Blue or red icon, fixes on]
```

## Switch the layout from the tray

1. The person double-clicks the tray icon.
2. The window they typed in comes back to the front with the other layout.
3. The icon shows `EN` or `RU` for the new layout.

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
