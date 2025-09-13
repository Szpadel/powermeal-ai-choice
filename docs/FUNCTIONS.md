# PowerMeal AI Choice - Function Reference

Complete function reference for all public and significant functions in the PowerMeal AI Choice application, organized by module.

## Table of Contents
- [Main Module Functions](#main-module-functions)
- [AI Module Functions](#ai-module-functions)
- [API Module Functions](#api-module-functions)
- [Cache Module Functions](#cache-module-functions)
- [Preferences Module Functions](#preferences-module-functions)
- [Prompts Module Functions](#prompts-module-functions)
- [Serde Module Functions](#serde-module-functions)

---

## Main Module Functions

### Core Application Functions

#### `main()`
**Location**: src/main.rs:67  
**Signature**: `async fn main() -> eyre::Result<()>`  
**Purpose**: Application entry point  
**Flow**:
1. Initializes tracing
2. Loads preferences and checks for migration
3. Parses CLI arguments
4. Routes to appropriate command or main flow
5. Manages authentication and meal selection

#### `init_tracing()`
**Location**: src/main.rs:130  
**Signature**: `fn init_tracing()`  
**Purpose**: Initialize application logging and tracing

### Authentication Functions

#### `update_token()`
**Location**: src/main.rs:152  
**Signature**: `async fn update_token() -> eyre::Result<RefreshTokenResponse>`  
**Purpose**: Interactive token update when authentication fails  
**Returns**: New token and refresh token

### Date and Calendar Functions

#### `days_available_to_select()`
**Location**: src/main.rs:165  
**Signature**: `async fn days_available_to_select(token: &str, diets: &DietsList) -> eyre::Result<Vec<DateTime<Local>>>`  
**Purpose**: Determines which days have modifiable menus  
**Parameters**:
- `token`: Authentication token
- `diets`: User's active diets
**Returns**: List of dates available for selection

#### `should_select_day()`
**Location**: src/main.rs:206  
**Signature**: `fn should_select_day(diet_day_state: &DietDayState) -> bool`  
**Purpose**: Checks if a day's menu can be modified based on state

### Data Fetching Functions

#### `fetch_historical_orders()`
**Location**: src/main.rs:219  
**Signature**: `async fn fetch_historical_orders(token: &str, date: DateTime<Local>, diet_id: &str) -> IndexMap<String, CalendarDayItems>`  
**Purpose**: Fetches past 14 days of meal selections for AI context  
**Returns**: Date-indexed map of historical meals

#### `get_diet_with_ingredients()`
**Location**: src/main.rs:268  
**Signature**: `async fn get_diet_with_ingredients(date: &NaiveDate, diet_id: &str, token: &str) -> eyre::Result<CalendarDayItems>`  
**Purpose**: Fetches menu with enriched ingredient data

#### `fetch_ingredients_with_cache()`
**Location**: src/main.rs:319  
**Signature**: `async fn fetch_ingredients_with_cache(token: &str, dish_size_id: i64, dish_name: String) -> eyre::Result<DishSizeIngredients>`  
**Purpose**: Fetches ingredients using cache when available

### Meal Selection Functions

#### `select_dishes_for_day()`
**Location**: src/main.rs:302  
**Signature**: `async fn select_dishes_for_day(token: &str, date: DateTime<Local>, diets: &DietsList, yolo: bool, preferences: &Preferences) -> eyre::Result<()>`  
**Purpose**: Complete meal selection process for one day  
**Parameters**:
- `token`: Auth token
- `date`: Date to select meals for
- `diets`: Available diets
- `yolo`: Auto-accept mode
- `preferences`: User preferences

#### `select_dishes()`
**Location**: src/main.rs:453  
**Signature**: `async fn select_dishes(calendar_day_items: &CalendarDayItems, date: &NaiveDate, ai_result: AiResponse, menu_changes: &mut ChangeMenuRequest, yolo: bool) -> eyre::Result<()>`  
**Purpose**: Interactive meal selection with AI recommendations

#### `display_analysis()`
**Location**: src/main.rs:555  
**Signature**: `fn display_analysis(analysis: &HashMap<String, String>, candidates: &[String])`  
**Purpose**: Display AI analysis for meal options

#### `dish_name_to_id()`
**Location**: src/main.rs:565  
**Signature**: `fn dish_name_to_id(candidates: &HashMap<String, String>, selected_name: &str) -> eyre::Result<String>`  
**Purpose**: Convert human-readable dish name to ID

### Utility Functions

#### `status()`
**Location**: src/main.rs:47  
**Signature**: `fn status(txt: &str)`  
**Purpose**: Display status message with carriage return

#### `clear_status()`
**Location**: src/main.rs:53  
**Signature**: `fn clear_status()`  
**Purpose**: Clear current status line

#### `print_with_delay()`
**Location**: src/main.rs:58  
**Signature**: `async fn print_with_delay(message: &str, delay_ms: u64)`  
**Purpose**: Print message with character-by-character delay

### Preference Migration Functions

#### `migrate_preferences()`
**Location**: src/main.rs:670  
**Signature**: `async fn migrate_preferences(preferences: &mut Preferences) -> eyre::Result<()>`  
**Purpose**: Migrate from legacy to free-text preferences

#### `edit_preferences_cli()`
**Location**: src/main.rs:705  
**Signature**: `fn edit_preferences_cli(preferences: &Preferences) -> eyre::Result<()>`  
**Purpose**: Open editor for preference editing

---

## AI Module Functions

### Public API Functions

#### `select_dish()`
**Location**: src/ai.rs:206  
**Signature**: `pub async fn select_dish(date: NaiveDate, dish_items: &Vec<DishItem>, last_days_choices: &IndexMap<String, CalendarDayItems>, user_preferences: &str) -> eyre::Result<AiResponse>`  
**Purpose**: Main AI meal selection function  
**Parameters**:
- `date`: Selection date
- `dish_items`: Available options
- `last_days_choices`: Historical data
- `user_preferences`: User preferences text
**Returns**: Structured AI recommendations

#### `configure_ai()`
**Location**: src/ai.rs:152  
**Signature**: `pub async fn configure_ai() -> eyre::Result<AiConfig>`  
**Purpose**: Interactive AI service configuration  
**Returns**: Complete AI configuration

#### `ai_generate_preferences()`
**Location**: src/ai.rs:404  
**Signature**: `pub async fn ai_generate_preferences(adjustments: &[UserAdjustment], cfg: &AiConfig) -> eyre::Result<String>`  
**Purpose**: Convert legacy adjustments to natural language

### Internal Functions

#### `get_openai_client()`
**Location**: src/ai.rs:143  
**Signature**: `fn get_openai_client(config: &AiConfig) -> Client<OpenAIConfig>`  
**Purpose**: Create configured OpenAI client

#### `fetch_models()`
**Location**: src/ai.rs:188  
**Signature**: `async fn fetch_models(client: &Client<OpenAIConfig>) -> Result<Vec<String>>`  
**Purpose**: Discover available AI models

#### `log_ai_response()`
**Location**: src/ai.rs:346  
**Signature**: `fn log_ai_response(date: NaiveDate, ai_response: &AiResponse, candidates: &HashMap<String, Vec<(String, String)>>)`  
**Purpose**: Log AI decisions to file

---

## API Module Functions

### Public API Functions

#### `refresh_token()`
**Location**: src/api.rs:44  
**Signature**: `pub async fn refresh_token(refresh_token: &str) -> eyre::Result<RefreshTokenResponse>`  
**Purpose**: Refresh authentication token

#### `get_diet()`
**Location**: src/api.rs:52  
**Signature**: `pub async fn get_diet(date: &NaiveDate, diet_id: &str, token: &str) -> eyre::Result<CalendarDayItems>`  
**Purpose**: Fetch menu for specific date and diet

#### `fetch_diets()`
**Location**: src/api.rs:60  
**Signature**: `pub async fn fetch_diets(token: &str) -> eyre::Result<DietsList>`  
**Purpose**: Get user's active diets

#### `fetch_calendar()`
**Location**: src/api.rs:68  
**Signature**: `pub async fn fetch_calendar(token: &str, diet_id: &str, from: &str, to: &str) -> eyre::Result<Calendar>`  
**Purpose**: Get calendar state for date range

#### `change_menu()`
**Location**: src/api.rs:83  
**Signature**: `pub async fn change_menu(token: &str, date: &NaiveDate, diet_id: &str, change: &ChangeMenuRequest) -> eyre::Result<()>`  
**Purpose**: Submit menu changes

#### `fetch_ingredients()`
**Location**: src/api.rs:100  
**Signature**: `pub async fn fetch_ingredients(token: &str, dish_size_id: i64) -> eyre::Result<DishSizeIngredients>`  
**Purpose**: Get ingredients for dish

### Internal Functions

#### `send_request()`
**Location**: src/api.rs:11  
**Signature**: `async fn send_request(url: &str, token: &str, method: reqwest::Method, body: Option<String>) -> eyre::Result<String>`  
**Purpose**: Core HTTP request handler with retry logic

---

## Cache Module Functions

### Public Methods

#### `IngredientsCache::get_instance()`
**Location**: src/cache.rs:18  
**Signature**: `pub fn get_instance() -> &'static IngredientsCache`  
**Purpose**: Get singleton cache instance

#### `IngredientsCache::get()`
**Location**: src/cache.rs:48  
**Signature**: `pub fn get(&self, id: &i64) -> Option<DishSizeIngredients>`  
**Purpose**: Retrieve cached ingredients

#### `IngredientsCache::put()`
**Location**: src/cache.rs:53  
**Signature**: `pub fn put(&self, id: i64, ingredients: DishSizeIngredients)`  
**Purpose**: Store ingredients in cache

#### `IngredientsCache::save()`
**Location**: src/cache.rs:58  
**Signature**: `pub fn save(&self) -> eyre::Result<()>`  
**Purpose**: Persist cache to disk

#### `IngredientsCache::load()`
**Location**: src/cache.rs:73  
**Signature**: `pub fn load(&self) -> eyre::Result<()>`  
**Purpose**: Load cache from disk

---

## Preferences Module Functions

### Instance Methods

#### `Preferences::load_preferences()`
**Location**: src/preferences.rs:42  
**Signature**: `pub fn load_preferences() -> Self`  
**Purpose**: Load preferences from disk

#### `Preferences::save_preferences()`
**Location**: src/preferences.rs:53  
**Signature**: `pub fn save_preferences(&self)`  
**Purpose**: Save preferences to disk

#### `Preferences::needs_migration()`
**Location**: src/preferences.rs:101  
**Signature**: `pub fn needs_migration(&self) -> bool`  
**Purpose**: Check if migration needed

#### `Preferences::complete_migration()`
**Location**: src/preferences.rs:105  
**Signature**: `pub fn complete_migration(&mut self, edited_text: String)`  
**Purpose**: Complete preference migration

### Static Methods

#### `Preferences::token()`
**Location**: src/preferences.rs:63  
**Signature**: `pub fn token() -> Option<String>`  
**Purpose**: Get stored authentication token

#### `Preferences::save_token()`
**Location**: src/preferences.rs:67  
**Signature**: `pub fn save_token(token: &str)`  
**Purpose**: Store authentication token

#### `Preferences::ai_config()`
**Location**: src/preferences.rs:73  
**Signature**: `pub fn ai_config() -> Option<AiConfig>`  
**Purpose**: Get AI configuration

#### `Preferences::save_ai_config()`
**Location**: src/preferences.rs:77  
**Signature**: `pub fn save_ai_config(config: AiConfig)`  
**Purpose**: Store AI configuration

#### `Preferences::next_day_to_check()`
**Location**: src/preferences.rs:83  
**Signature**: `pub fn next_day_to_check() -> Option<DateTime<Local>>`  
**Purpose**: Get last processed date

#### `Preferences::set_next_day_to_check()`
**Location**: src/preferences.rs:92  
**Signature**: `pub fn set_next_day_to_check(date: NaiveDate)`  
**Purpose**: Update progress tracking

---

## Prompts Module Functions

#### `build_meal_selection_system_prompt()`
**Location**: src/prompts.rs:66  
**Signature**: `pub fn build_meal_selection_system_prompt(user_prefs: &str, has_history: bool) -> String`  
**Purpose**: Build complete system prompt for meal selection  
**Parameters**:
- `user_prefs`: User's dietary preferences
- `has_history`: Whether historical data is available
**Returns**: Complete system prompt

---

## Serde Module Functions

### DishItem Methods

#### `DishItem::options()`
**Location**: src/serde.rs:145  
**Signature**: `pub fn options(&self) -> Vec<&MenuDietOption>`  
**Purpose**: Get enabled meal options

#### `DishItem::get_selected_option()`
**Location**: src/serde.rs:149  
**Signature**: `pub fn get_selected_option(&self) -> Option<&MenuDietOption>`  
**Purpose**: Get currently selected option

#### `DishItem::debug_options()`
**Location**: src/serde.rs:157  
**Signature**: `pub fn debug_options(&self) -> String`  
**Purpose**: Generate human-readable options summary

### DietsList Methods

#### `DietsList::diet_for_date()`
**Location**: src/serde.rs:189  
**Signature**: `pub fn diet_for_date(&self, date: &DateTime<Local>) -> Option<&Diet>`  
**Purpose**: Find diet active on given date

#### `DietsList::diets_in_time_range()`
**Location**: src/serde.rs:200  
**Signature**: `pub fn diets_in_time_range(&self, from: DateTime<Local>, to: DateTime<Local>) -> Vec<&Diet>`  
**Purpose**: Find diets active in date range

### Helper Functions

#### `get_dish()`
**Location**: src/serde.rs:277  
**Signature**: `pub fn get_dish(dish_items: &[DishItem], dish_item_id: &str, dish_id: &str) -> Option<&MenuDietOption>`  
**Purpose**: Find specific dish option

#### `get_dish_item()`
**Location**: src/serde.rs:287  
**Signature**: `pub fn get_dish_item(dish_items: &[DishItem], dish_item_id: &str) -> Option<&DishItem>`  
**Purpose**: Find dish item by ID

---

## Cross-Module Function Relationships

### Authentication Flow
1. `main::main()` → `preferences::token()`
2. `main::main()` → `api::refresh_token()`
3. `main::update_token()` → `preferences::save_token()`

### Meal Selection Flow
1. `main::select_dishes_for_day()` → `api::get_diet()`
2. `main::select_dishes_for_day()` → `main::fetch_historical_orders()`
3. `main::select_dishes_for_day()` → `ai::select_dish()`
4. `main::select_dishes()` → User interaction
5. `main::select_dishes_for_day()` → `api::change_menu()`

### AI Processing Flow
1. `ai::select_dish()` → `prompts::build_meal_selection_system_prompt()`
2. `ai::select_dish()` → `ai::get_openai_client()`
3. `ai::select_dish()` → LLM API call
4. `ai::select_dish()` → `ai::log_ai_response()`

### Caching Flow
1. `main::fetch_ingredients_with_cache()` → `cache::get()`
2. `main::fetch_ingredients_with_cache()` → `api::fetch_ingredients()`
3. `main::fetch_ingredients_with_cache()` → `cache::put()`

### Configuration Flow
1. `main::main()` → `preferences::load_preferences()`
2. `ai::configure_ai()` → `preferences::save_ai_config()`
3. `main::migrate_preferences()` → `ai::ai_generate_preferences()`
4. `main::migrate_preferences()` → `preferences::complete_migration()`

---

## Function Categories

### User Interaction Functions
- `main::select_dishes()` - Interactive selection
- `main::display_analysis()` - Show AI analysis
- `main::edit_preferences_cli()` - Edit preferences
- `ai::configure_ai()` - Configure AI service

### Data Fetching Functions
- `api::fetch_diets()` - Get diets
- `api::get_diet()` - Get menu
- `api::fetch_calendar()` - Get calendar
- `api::fetch_ingredients()` - Get ingredients
- `main::fetch_historical_orders()` - Get history

### Processing Functions
- `ai::select_dish()` - AI selection
- `main::select_dishes_for_day()` - Day processing
- `main::days_available_to_select()` - Date filtering

### Storage Functions
- `preferences::save_preferences()` - Save config
- `cache::save()` - Save cache
- `ai::log_ai_response()` - Log decisions

### Utility Functions
- `main::status()` - Status display
- `prompts::build_meal_selection_system_prompt()` - Prompt building
- `serde::get_dish()` - Data navigation