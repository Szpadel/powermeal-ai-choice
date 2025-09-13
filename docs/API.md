# PowerMeal AI Choice - API Reference

Complete API documentation for all modules in the PowerMeal AI Choice application.

## Module Overview

| Module | Purpose |
|--------|---------|
| `main` | Application orchestration, CLI interface, and meal selection workflow |
| `ai` | AI service integration for intelligent meal selection |
| `api` | PowerMeal API client with retry logic |
| `cache` | LRU caching system for ingredient data |
| `preferences` | User preference and configuration management |
| `prompts` | AI prompt templates and builders |
| `serde` | Data structures and API models |

---

## main Module

The main application module that orchestrates the entire meal selection process.

### CLI Structure

```rust
struct Cli {
    command: Option<Commands>,
    yolo: bool,  // Auto-accept AI selections
}

enum Commands {
    ConfigureAi,      // Configure AI settings
    EditPreferences,  // Edit meal preferences
}
```

### Key Functions

#### `main() -> eyre::Result<()>`
Application entry point that handles initialization, authentication, and orchestrates the meal selection process.

#### `select_dishes_for_day(token: &str, date: DateTime<Local>, diets: &DietsList, yolo: bool, preferences: &Preferences) -> eyre::Result<()>`
**Location**: src/main.rs:302

Processes meal selection for a specific day:
- Fetches diet menu for the date
- Retrieves historical meal data
- Calls AI for recommendations
- Handles user interaction
- Submits changes to API

#### `select_dishes(calendar_day_items: &CalendarDayItems, date: &NaiveDate, ai_result: AiResponse, menu_changes: &mut ChangeMenuRequest, yolo: bool) -> eyre::Result<()>`
**Location**: src/main.rs:453

Interactive meal selection process:
- Presents AI analysis for each meal
- Allows user override of AI selections
- Records changes for API submission

#### `days_available_to_select(token: &str, diets: &DietsList) -> eyre::Result<Vec<DateTime<Local>>>`
**Location**: src/main.rs:165

Determines which days have modifiable menus based on calendar state.

#### `update_token() -> eyre::Result<RefreshTokenResponse>`
**Location**: src/main.rs:152

Interactive token update process when authentication fails.

#### `migrate_preferences(preferences: &mut Preferences) -> eyre::Result<()>`
**Location**: src/main.rs:670

Handles migration from legacy preference format to free-text format.

---

## ai Module

Handles all AI-related operations for intelligent meal selection.

### Data Structures

```rust
pub struct AiConfig {
    pub api_base: String,    // API endpoint URL
    pub api_key: String,     // Authentication key
    pub model: String,       // Model identifier
}

pub struct AiResponse {
    pub reasoning: Vec<String>,                        // Global reasoning
    pub selections: HashMap<String, ResponseItem>,     // Per meal selections
}

pub struct ResponseItem {
    pub dish_id: String,                              // Selected dish ID
    pub reason: String,                               // Selection justification
    pub analysis: HashMap<String, String>,            // Analysis per option
}
```

### Key Functions

#### `select_dish(date: NaiveDate, dish_items: &Vec<DishItem>, last_days_choices: &IndexMap<String, CalendarDayItems>, user_preferences: &str) -> eyre::Result<AiResponse>`
**Location**: src/ai.rs:206

Main AI meal selection function:
- Generates JSON schema for structured output
- Builds context with preferences and history
- Calls AI API with retry logic
- Returns structured recommendations

**Parameters**:
- `date`: Date for meal selection
- `dish_items`: Available meal options
- `last_days_choices`: Historical meal data (14 days)
- `user_preferences`: User's dietary preferences

#### `configure_ai() -> eyre::Result<AiConfig>`
**Location**: src/ai.rs:152

Interactive AI configuration:
- Prompts for API endpoint
- Requests API key
- Discovers available models
- Allows model selection

#### `ai_generate_preferences(adjustments: &[UserAdjustment], cfg: &AiConfig) -> eyre::Result<String>`
**Location**: src/ai.rs:404

Converts legacy adjustments to natural language preferences.

#### `get_openai_client(config: &AiConfig) -> Client<OpenAIConfig>`
**Location**: src/ai.rs:143

Creates configured OpenAI client instance.

---

## api Module

PowerMeal API client implementation with robust error handling.

### Key Functions

#### `refresh_token(refresh_token: &str) -> eyre::Result<RefreshTokenResponse>`
**Location**: src/api.rs:44

Refreshes authentication token using refresh token.

#### `get_diet(date: &NaiveDate, diet_id: &str, token: &str) -> eyre::Result<CalendarDayItems>`
**Location**: src/api.rs:52

Fetches menu items for specific date and diet.

#### `fetch_diets(token: &str) -> eyre::Result<DietsList>`
**Location**: src/api.rs:60

Retrieves user's active diet subscriptions.

#### `fetch_calendar(token: &str, diet_id: &str, from: &str, to: &str) -> eyre::Result<Calendar>`
**Location**: src/api.rs:68

Gets calendar state for date range.

#### `change_menu(token: &str, date: &NaiveDate, diet_id: &str, change: &ChangeMenuRequest) -> eyre::Result<()>`
**Location**: src/api.rs:83

Submits menu changes to PowerMeal API.

#### `fetch_ingredients(token: &str, dish_size_id: i64) -> eyre::Result<DishSizeIngredients>`
**Location**: src/api.rs:100

Retrieves ingredient information for a specific dish size.

### Internal Functions

#### `send_request(url: &str, token: &str, method: reqwest::Method, body: Option<String>) -> eyre::Result<String>`
**Location**: src/api.rs:11

Core HTTP request handler with:
- Automatic retry on 429 (rate limit) and 5xx errors
- Respects `Retry-After` header
- Consistent header configuration
- Error context wrapping

---

## cache Module

Thread-safe LRU caching system with persistence.

### Structure

```rust
pub struct IngredientsCache {
    cache: Mutex<LruCache<i64, DishSizeIngredients>>,
}
```

### Key Functions

#### `IngredientsCache::get_instance() -> &'static IngredientsCache`
**Location**: src/cache.rs:18

Returns singleton cache instance (lazy initialization).

#### `get(&self, id: &i64) -> Option<DishSizeIngredients>`
**Location**: src/cache.rs:48

Thread-safe cache lookup.

#### `put(&self, id: i64, ingredients: DishSizeIngredients)`
**Location**: src/cache.rs:53

Thread-safe cache insertion.

#### `save(&self) -> eyre::Result<()>`
**Location**: src/cache.rs:58

Persists cache to disk at `~/.config/powermeal-ai/ingredients_cache.json`.

#### `load(&self) -> eyre::Result<()>`
**Location**: src/cache.rs:73

Loads cache from disk on initialization.

### Configuration

- **Capacity**: 1000 entries
- **Eviction**: LRU (Least Recently Used)
- **Persistence**: JSON format
- **Thread Safety**: Mutex-protected

---

## preferences Module

User preference and configuration management.

### Data Structures

```rust
pub struct Preferences {
    pub user_preferences: String,              // Free-text preferences
    pub adjustments: Vec<UserAdjustment>,      // Legacy format
    last_day_selected: Option<NaiveDate>,      // Progress tracking
    token: Option<String>,                     // Auth token
    pub ai_config: Option<AiConfig>,           // AI configuration
}

pub struct UserAdjustment {
    pub dish_name: String,
    pub ingredient_name: String,
    pub adjustment_type: AdjustmentType,
}

pub enum AdjustmentType {
    AlwaysAdd,
    AlwaysRemove,
    NeverAdd,
    PreferNotToAdd,
}
```

### Key Functions

#### `Preferences::load_preferences() -> Self`
**Location**: src/preferences.rs:42

Loads preferences from `~/.config/powermeal-ai/preferences.json`.

#### `save_preferences(&self)`
**Location**: src/preferences.rs:53

Persists current preferences to disk.

#### `needs_migration(&self) -> bool`
**Location**: src/preferences.rs:101

Checks if legacy format needs migration.

#### `complete_migration(&mut self, edited_text: String)`
**Location**: src/preferences.rs:105

Completes migration to free-text format.

#### Static Accessors

- `token() -> Option<String>` - Get stored auth token
- `save_token(token: &str)` - Store auth token
- `ai_config() -> Option<AiConfig>` - Get AI configuration
- `save_ai_config(config: AiConfig)` - Store AI configuration
- `next_day_to_check() -> Option<DateTime<Local>>` - Get last processed date
- `set_next_day_to_check(date: NaiveDate)` - Update progress

---

## prompts Module

AI prompt templates and builders.

### Constants

#### `MEAL_SELECTION_BASE_PROMPT`
**Location**: src/prompts.rs:1

Base system prompt for meal selection AI (40+ lines).

#### `PREFERENCE_SUMMARY_SYSTEM_PROMPT`
**Location**: src/prompts.rs:44

System prompt for preference migration.

### Functions

#### `build_meal_selection_system_prompt(user_prefs: &str, has_history: bool) -> String`
**Location**: src/prompts.rs:66

Builds complete system prompt with:
- Base meal selection instructions
- User preferences integration
- Contextual notes for empty preferences
- Historical data availability flags

---

## serde Module

Data structures for API communication and internal representation.

### Core Data Structures

#### `CalendarDayItems`
```rust
pub struct CalendarDayItems {
    pub diet_elements: DietElements,
}
```
Main container for daily meal data.

#### `DishItem`
```rust
pub struct DishItem {
    pub id: String,
    pub options: Vec<MenuDietOption>,
    pub meal_type: MealType,
    pub dish_size: DishSize,
}
```
Represents a meal slot with available options.

**Methods**:
- `options(&self) -> Vec<&MenuDietOption>` - Get enabled options
- `get_selected_option(&self) -> Option<&MenuDietOption>` - Current selection
- `debug_options(&self) -> String` - Human-readable summary

#### `MenuDietOption`
```rust
pub struct MenuDietOption {
    pub name: String,
    pub enabled: bool,
    pub dish: Dish,
    pub dish_size_id: i64,
    pub ingredients: Option<DishSizeIngredients>,
}
```
Individual meal option with ingredients.

#### `DietsList`
```rust
pub struct DietsList {
    #[serde(rename = "hydra:member")]
    pub diets: Vec<Diet>,
}
```
User's active diet subscriptions.

**Methods**:
- `diet_for_date(&self, date: &DateTime<Local>) -> Option<&Diet>`
- `diets_in_time_range(from, to) -> Vec<&Diet>`

#### `Calendar`
```rust
pub struct Calendar {
    pub data: HashMap<String, DietDayDetails>,
}
```
Calendar state for date range.

#### `ChangeMenuRequest`
```rust
pub struct ChangeMenuRequest {
    pub changedDishes: Vec<ChangedDish>,
}
```
API request for menu modifications.

### Enumerations

#### `MealType`
```rust
pub enum MealType {
    #[serde(rename = "Śniadanie")]
    Breakfast,
    #[serde(rename = "II Śniadanie")]
    SecondBreakfast,
    #[serde(rename = "Lunch")]
    Lunch,
    #[serde(rename = "Obiad")]
    Dinner,
    #[serde(rename = "Kolacja")]
    Supper,
}
```

#### `DietDayState`
```rust
pub enum DietDayState {
    AvailableToSelect,
    CannotChange,
    Delivered,
    InDelivery,
    InPreparation,
    NoDelivery,
    NoMenu,
    OrderCancelled,
    OrderPaused,
}
```

### Helper Functions

#### `get_dish(dish_items: &[DishItem], dish_item_id: &str, dish_id: &str) -> Option<&MenuDietOption>`
**Location**: src/serde.rs:277

Finds specific dish option across all meal items.

#### `get_dish_item(dish_items: &[DishItem], dish_item_id: &str) -> Option<&DishItem>`
**Location**: src/serde.rs:287

Finds dish item by ID.

---

## Error Handling

All modules use consistent error handling:

- **Error Type**: `eyre::Result<T>` for all fallible operations
- **Context**: `.wrap_err()` and `.context()` for error context
- **Error Propagation**: `?` operator for clean error bubbling

## Thread Safety

- **Cache Module**: Mutex-protected for concurrent access
- **Preferences**: Static methods use internal locking
- **API Module**: Stateless, safe for concurrent use

## File System Locations

### Configuration
- `~/.config/powermeal-ai/preferences.json` - User preferences and config
- `~/.config/powermeal-ai/ingredients_cache.json` - Ingredient cache

### Logs
- `~/.local/state/powermeal-ai-choice/ai_responses.log` - AI decision logs

## Dependencies

Key external crates used:

- `tokio` - Async runtime
- `reqwest` - HTTP client
- `async-openai` - OpenAI API client
- `serde/serde_json` - Serialization
- `eyre` - Error handling
- `clap` - CLI parsing
- `dialoguer` - Interactive prompts
- `chrono` - Date/time handling
- `lru` - LRU cache implementation
- `tracing` - Structured logging