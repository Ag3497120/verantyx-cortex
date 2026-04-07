---
description: [VERANTYX CORE ARCHITECTURE & ZERO-DOM RULES]
---
# Verantyx Architecture & AI Modification Rules

**IMPORTANT**: If you are an AI modifying this codebase, you MUST read and strictly adhere to these architectural rules to prevent regressions. The user has specifically requested that these operational flows are **never** accidentally deleted or bypassed during bug fixes.

## 1. Zero-DOM Symbiotic Architecure
Verantyx operates by bypassing raw HTML injection clicks. 
- **Rule:** Never replace the biometric mouse tracking (`getSelection()` -> `drift_mouse_to_target`) with `HTMLElement.click()`.
- **Rule:** The mouse tracking always originates from the text caret (`window.getSelection().getRangeAt(0).getBoundingClientRect()`), NOT from the edge of the screen or `(0, 0)`.

## 2. Carbon Paper UI / Stealth Evasion Aesthetics
The CLI has a specific aesthetic tracking the "Ghost Terminal" flow.
- **Rule:** Even if workflows are automated to bypass blocking inputs (like removing `dialoguer::Select`), you MUST preserve the exact ASCII structures (`╭─ [ Verantyx Carbon Paper UI ] ────────────────────────────────────`, `✔ [SYS_AUTH] ペーストの承認待ち`, etc.). Do not strip out `println!()` statements that provide immersive CLI feedback.

## 3. Dual-Browser Visual Synchronization
Verantyx synchronizes a native Wry application (`vx-browser`) tiled on the left and Safari tiled on the right.
- **Rule:** Do not remove the `osascript` AppleScript hooks that arrange the windows.

## 4. Local SLM Pre-Analysis Sequence
The REPL requires a pre-analysis step by a local SLM.
- **Rule:** Before `StealthWebActor` dispatches the payload to Gemini, `OllamaProvider` (e.g. `gemma-2-2b-it`) must pre-analyze the prompt and combine the strategy with the user's input. Do not bypass this step when editing REPL loops (`interactive_chat.rs`).

Failure to follow these guidelines will destroy the "Verantyx Alpha" identity and core immersive flow.
