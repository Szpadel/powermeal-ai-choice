# PowerMeal AI Choice - System Architecture

## Overview

PowerMeal AI Choice is a Rust-based CLI application that automates meal selection for PowerMeal delivery service using AI-powered recommendations. The system integrates with PowerMeal's API, leverages LLM services for intelligent decision-making, and maintains user preferences with smart caching.

## High-Level Architecture

```
┌─────────────────┐
│   User (CLI)    │
└────────┬────────┘
         │
┌────────▼────────┐
│   Main Module   │◄──────────────┐
│  (Orchestrator) │               │
└────────┬────────┘               │
         │                        │
    ┌────┴─────┬──────┬──────┬───┴──┐
    │          │      │      │      │
┌───▼───┐ ┌───▼───┐ ┌▼──┐ ┌─▼──┐ ┌▼─────┐
│  API  │ │  AI   │ │Cache│ │Prefs│ │Prompts│
│Module │ │Module │ │Module│ │Module│ │Module │
└───┬───┘ └───┬───┘ └────┘ └────┘ └──────┘
    │         │
    │    ┌────▼─────┐
    │    │ OpenAI/  │
    │    │ LiteLLM  │
    │    └──────────┘
    │
┌───▼────────────┐
│ PowerMeal API  │
└────────────────┘
```

## Component Architecture

### 1. Main Module (Orchestrator)
**Responsibility**: Application lifecycle, user interaction, workflow orchestration

**Key Components**:
- CLI parser (clap-based)
- Authentication manager
- Day-by-day meal processor
- User interaction handler

### 2. API Module
**Responsibility**: PowerMeal API communication with resilient networking

**Features**:
- Centralized HTTP client
- Automatic retry with exponential backoff
- Rate limit handling
- Token management

### 3. AI Module
**Responsibility**: Intelligent meal selection using LLM services

**Features**:
- OpenAI/LiteLLM integration
- Structured JSON output with schema validation
- Preference interpretation
- Historical analysis

### 4. Cache Module
**Responsibility**: Reduce API calls through intelligent caching

**Features**:
- Thread-safe singleton LRU cache
- Persistent storage
- Automatic save on shutdown

### 5. Preferences Module
**Responsibility**: User configuration and preference management

**Features**:
- Free-text preference system
- Legacy format migration
- AI configuration storage
- Progress tracking

### 6. Prompts Module
**Responsibility**: AI prompt engineering and templates

**Features**:
- Structured prompt templates
- Dynamic prompt building
- Context injection

### 7. Serde Module
**Responsibility**: Data modeling and serialization

**Features**:
- API contract definitions
- Type-safe data structures
- JSON serialization/deserialization

## Core Workflows

### 1. Application Startup Flow

```
Start
  │
  ├─► Initialize Tracing
  ├─► Load Preferences
  ├─► Check Migration Need
  │     └─► Migrate if needed
  ├─► Parse CLI Arguments
  └─► Route to Command/Main Flow
```

### 2. Authentication Flow

```
Check Token
  │
  ├─► Token exists?
  │     ├─► Yes: Refresh Token
  │     │     ├─► Success: Continue
  │     │     └─► Failure: Request New Token
  │     └─► No: Request New Token
  │
  └─► Validate & Store Token
```

### 3. Meal Selection Workflow

```
For Each Available Day:
  │
  ├─► Fetch Diet Menu
  ├─► Get Historical Data (14 days)
  ├─► Enrich with Ingredients
  │     ├─► Check Cache
  │     └─► Fetch if not cached
  ├─► AI Analysis
  │     ├─► Build Context
  │     ├─► Generate Schema
  │     ├─► Call LLM
  │     └─► Parse Response
  ├─► User Interaction
  │     ├─► Show AI Reasoning
  │     ├─► Present Options
  │     └─► Confirm/Override
  └─► Submit Changes
        └─► Update Progress
```

### 4. AI Decision Process

```
Input: (Date, Options, History, Preferences)
  │
  ├─► Generate JSON Schema
  │     └─► Dynamic based on available options
  ├─► Build System Prompt
  │     └─► Include preferences and rules
  ├─► Prepare Context
  │     ├─► Current options with ingredients
  │     └─► Historical selections
  ├─► LLM Request
  │     ├─► Structured output mode
  │     └─► Retry on failure
  ├─► Validate Response
  │     └─► Schema compliance check
  └─► Return Structured Decision
        ├─► Global reasoning
        ├─► Per-meal analysis
        └─► Specific selections
```

### 5. Caching Strategy

```
Request Ingredients
  │
  ├─► Check LRU Cache
  │     ├─► Hit: Return cached
  │     └─► Miss: Continue
  ├─► Fetch from API
  ├─► Store in Cache
  └─► Persist on shutdown
```

## Data Flow

### 1. Configuration Data Flow
```
~/.config/powermeal-ai/preferences.json
         │
         ├─► Preferences struct
         │     ├─► user_preferences (text)
         │     ├─► ai_config
         │     ├─► token
         │     └─► last_day_selected
         │
         └─► Runtime usage
```

### 2. API Data Flow
```
PowerMeal API
    │
    ├─► JSON Response
    ├─► Deserialization (serde)
    ├─► Type-safe structures
    └─► Business logic processing
```

### 3. AI Data Flow
```
User Preferences + Historical Data + Available Options
                        │
                        ├─► Prompt Construction
                        ├─► LLM Processing
                        ├─► Structured JSON Response
                        └─► AiResponse struct
                              ├─► reasoning[]
                              └─► selections{}
```

## Security Architecture

### Authentication
- **Token Storage**: Secure file system storage with user-only permissions
- **Token Refresh**: Automatic refresh with fallback to manual entry
- **Bearer Token**: Standard HTTP Authorization header

### API Security
- **HTTPS Only**: All API communication encrypted
- **Origin Header**: Properly set to match expected web origin
- **No Credential Logging**: Sensitive data excluded from logs

### AI Service Security
- **API Key Storage**: Encrypted in preferences file
- **Configurable Endpoints**: Support for private/local LLM deployments
- **No PII in Prompts**: Only meal preferences and food data sent

## Error Handling Strategy

### Layered Error Handling
1. **Network Layer**: Automatic retry with backoff
2. **API Layer**: Rate limit respect, error context
3. **Business Layer**: Graceful degradation
4. **User Layer**: Clear error messages with recovery options

### Error Recovery Patterns
```rust
// Retry Pattern
loop {
    match operation() {
        Ok(result) => return Ok(result),
        Err(e) if retryable(e) => {
            sleep(backoff_time).await;
            continue;
        }
        Err(e) => return Err(e),
    }
}

// Fallback Pattern
let result = primary_operation()
    .or_else(|_| fallback_operation())
    .wrap_err("context")?;
```

## Performance Optimizations

### 1. Caching
- **LRU Cache**: 1000-entry capacity
- **Persistent Cache**: Survives restarts
- **Ingredient Deduplication**: One fetch per dish_size_id

### 2. Parallel Processing
- **Async/Await**: Non-blocking I/O operations
- **Tokio Runtime**: Efficient task scheduling
- **Concurrent Requests**: Where API allows

### 3. Memory Management
- **Lazy Loading**: Cache loaded on first use
- **Selective Loading**: Only fetch needed date ranges
- **Efficient Serialization**: JSON with minimal overhead

## File System Layout

```
~/
├── .config/powermeal-ai/
│   ├── preferences.json       # User config & preferences
│   └── ingredients_cache.json # LRU cache persistence
│
└── .local/state/powermeal-ai-choice/
    └── ai_responses.log       # AI decision audit log
```

## Dependency Architecture

### Core Dependencies
- **tokio**: Async runtime foundation
- **reqwest**: HTTP client for API calls
- **async-openai**: LLM service integration
- **serde**: Serialization framework
- **eyre**: Error handling and context

### UI Dependencies
- **clap**: Command-line argument parsing
- **dialoguer**: Interactive user prompts

### Utility Dependencies
- **chrono**: Date/time handling
- **lru**: Cache implementation
- **tracing**: Structured logging

## Design Patterns

### 1. Singleton Pattern (Cache)
```rust
static INSTANCE: LazyLock<IngredientsCache> = LazyLock::new(|| {
    IngredientsCache::new()
});
```

### 2. Builder Pattern (AI Requests)
```rust
CreateChatCompletionRequestArgs::default()
    .model(&model)
    .messages(messages)
    .response_format(format)
    .build()
```

### 3. Strategy Pattern (Interactive vs YOLO mode)
```rust
if yolo {
    auto_accept_selection()
} else {
    interactive_selection()
}
```

### 4. Repository Pattern (Preferences)
```rust
impl Preferences {
    pub fn load_preferences() -> Self
    pub fn save_preferences(&self)
}
```

## Scaling Considerations

### Current Limitations
- Single-user design
- Sequential day processing
- One diet at a time

### Potential Improvements
- Parallel day processing
- Batch API operations
- Multi-user support with profile switching
- Background preference learning

## Testing Strategy

### Unit Testing
- Module-level function testing
- Mock API responses
- Preference migration scenarios

### Integration Testing
- End-to-end workflow testing
- API retry mechanism validation
- Cache persistence verification

### Manual Testing
- CLI interaction flows
- AI response quality
- Error recovery paths

## Monitoring and Observability

### Logging
- **Structured Logging**: Via tracing crate
- **Log Levels**: Configurable via RUST_LOG
- **AI Audit Log**: Human-readable decision trail

### Metrics (Potential)
- API call success/failure rates
- Cache hit/miss ratios
- AI response times
- User interaction patterns

## Future Architecture Considerations

### Potential Enhancements
1. **Web Interface**: REST API wrapper for web access
2. **Mobile Support**: React Native or Flutter app
3. **Multi-Provider**: Support for multiple meal services
4. **Advanced AI**: Fine-tuned models for better recommendations
5. **Predictive Ordering**: Learn patterns and pre-select meals
6. **Nutritional Tracking**: Calorie and macro tracking
7. **Social Features**: Share preferences and recommendations

### Modularity for Extension
The current architecture supports extension through:
- Clean module boundaries
- Trait-based abstractions (potential)
- Configuration-driven behavior
- Plugin-style AI providers