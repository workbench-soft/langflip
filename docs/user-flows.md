# User flows

## First start

1. The person runs `langflip.exe` (Windows) or `Langflip` (macOS).
2. On macOS the system asks for Accessibility and Input Monitoring access. The person allows both and starts Langflip again.
3. The tray shows a blue and red square. Langflip writes `settings.toml` with the defaults.

```mermaid
flowchart LR
    A[Start Langflip] --> B{macOS?}
    B -- yes --> C[Allow Accessibility and Input Monitoring] --> D[Start again]
    B -- no --> E[Tray icon appears]
    D --> E
```

## A word in the wrong layout

1. The English layout is on. The person types `ghbdtn`.
2. The person presses Space.
3. Langflip erases `ghbdtn`, types «привет», types the Space and switches to Russian.
4. The person goes on typing in Russian.

```mermaid
sequenceDiagram
    participant P as Person
    participant L as Langflip
    participant A as App
    P->>A: ghbdtn
    P->>L: Space
    L->>A: 6 Backspaces, «привет», Space
    L->>A: switch layout to Russian
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

## Turn it off

1. The person clicks the tray icon.
2. The person unchecks «Исправлять раскладку». The icon turns grey.
3. Typing passes through untouched until the person checks it again.

```mermaid
flowchart LR
    A[Tray icon] --> B[Uncheck Исправлять раскладку] --> C[Grey icon, no fixes]
```
