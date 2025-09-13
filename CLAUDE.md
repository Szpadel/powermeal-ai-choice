# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Building and Running
```bash
# Development build with debug symbols
cargo build

# Optimized release build
cargo build --release

# Run directly with cargo (development)
cargo run -- [OPTIONS] [COMMAND]

# Run with debug logging
RUST_LOG=debug cargo run

# Run with tracing enabled
RUST_LOG=powermeal_ai_choice=debug cargo run

# Check code without building
cargo check

# Format code
cargo fmt

# Lint code
cargo clippy
```

### Common Usage Patterns
```bash
# Configure AI service (first-time setup)
cargo run -- configure-ai

# Edit preferences
cargo run -- edit-preferences

# Run meal selection (interactive)
cargo run

# Run in YOLO mode (auto-accept AI suggestions)
cargo run -- --yolo
```

## High-Level Architecture

The application follows a modular architecture with `main.rs` as the orchestrator. Each module is self-contained with specific responsibilities:

### Module Interaction Pattern
```
main.rs (orchestrator) → coordinates all modules
    ├── api.rs → PowerMeal API communication (authentication, fetching, updating)
    ├── ai.rs → LLM integration for intelligent meal selection
    ├── cache.rs → Thread-safe LRU cache for ingredients (reduces API calls)
    ├── preferences.rs → User configuration and preference management
    ├── prompts.rs → AI prompt templates and dynamic prompt building
    └── serde.rs → Data structures and API contracts
```

### Key Architectural Patterns

1. **Singleton Cache Pattern**: The `IngredientsCache` uses a global singleton with `OnceLock` for thread-safe access across the application lifecycle. It automatically saves on drop.

2. **Authentication Flow**:
   - Token stored in preferences.json
   - Automatic refresh on 401 responses
   - Interactive token update when refresh fails
   - Token obtained from browser DevTools (Authorization header)

3. **AI Integration**:
   - Supports any OpenAI-compatible API (OpenAI, LiteLLM, local models)
   - Uses structured JSON output with schema validation
   - Logs all AI responses to `~/.local/state/powermeal-ai-choice/ai_responses.log`
   - Preference system uses free-text input with AI interpretation

4. **Error Handling**:
   - Uses `eyre` for error management throughout
   - Contextual error wrapping with `.wrap_err()`
   - Early returns with `?` operator
   - User-friendly error messages in main.rs

5. **Async Operations**:
   - Tokio runtime with full features
   - All API calls are async
   - Retry logic with exponential backoff in `send_request`

## Critical Implementation Details

### API Communication Pattern
All API calls go through `send_request` in `api.rs` which handles:
- Automatic token refresh on 401
- Retry with exponential backoff
- Rate limit handling
- Consistent error handling

### Preference Migration System
The app automatically migrates from legacy adjustment-based preferences to free-text format:
1. Detects legacy format in preferences.json
2. Uses AI to convert adjustments to natural language
3. Opens editor for user review
4. Saves new format

### Day Selection Workflow
1. Fetch all diets for user
2. Determine available days (not locked, not in past)
3. For each day:
   - Fetch menu with ingredient details (using cache)
   - Get 14-day meal history for context
   - AI analyzes and recommends
   - User confirms or overrides
   - Submit changes to API

### Configuration Storage
All configuration stored in:
- `~/.config/powermeal-ai/preferences.json` - User preferences, AI config, auth token
- `~/.config/powermeal-ai/ingredients_cache.json` - LRU cache for ingredients
- `~/.local/state/powermeal-ai-choice/ai_responses.log` - AI decision log

## Code Conventions

### Module Organization
- Each module exports only necessary public interfaces
- Internal functions remain private
- Data structures in `serde.rs` with derive macros
- Constants at module top (e.g., `FETCH_HISTORY_DAYS`, `PREFERENCES_FILE`)

### Async/Await Patterns
```rust
// All API calls use this pattern
async fn function_name() -> eyre::Result<T> {
    // Implementation with .await
}
```

### Error Context Pattern
```rust
operation()
    .await
    .wrap_err("Contextual error message")?
```

### Status Updates
The app uses a status line system with:
- `status()` - Updates current line
- `clear_status()` - Clears status line
- `print_with_delay()` - Prints with visual delay for readability

## AI Prompt Engineering

The system uses a two-tier prompt system:
1. **Base System Prompt** (`MEAL_SELECTION_BASE_PROMPT`) - Core instructions for meal selection
2. **Dynamic User Context** - Injected preferences, history, and current options

Prompts enforce structured JSON output with TypeScript-style schema definitions for reliable parsing.

## Testing Approach

Currently no automated tests. When adding tests:
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

## Performance Considerations

1. **Caching Strategy**: LRU cache for ingredients reduces API calls by ~90% for repeated ingredient fetches
2. **Parallel Fetching**: Uses futures for concurrent API calls where possible
3. **Lazy Loading**: Only fetches meal details when needed for selection

## Security Notes

- API tokens stored in plaintext in preferences.json (user configuration directory)
- No credentials in code or git repository
- Token refresh mechanism prevents token expiration issues