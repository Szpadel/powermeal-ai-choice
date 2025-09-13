# PowerMeal API Migration Plan

## Overview
This document serves as the comprehensive guide for migrating the PowerMeal CLI application from the old PowerMeal API to the new PowerFoods API. The old API is no longer available, and we need to completely refactor the application to use the new API structure.

More details about api structures can be found in `new-api/API_INVESTIGATION.md`.

## Key Design Decisions (Confirmed by User)

### 1. Authentication & Brand ID
- **Brand ID**: Extracted from JWT token payload (example shows `brand_id: 1` embedded in token)
- **Token Migration**: Users must provide new tokens; old tokens are incompatible
- **Token Storage**: Continue storing in preferences.json but with new format, we need to make sure `user_preferences` and `ai_config` is preserved. `last_day_selected` should be reset to current date.

### 2. Diet Management
- **Single Diet Per Day**: Expect only one diet active per day
- **Multiple Diets**: If multiple diets cover same day, show warning (user error)
- **Diet Extensions**: Appear as new diets in the system

### 3. Menu Updates
- **API Constraint**: New API requires single-dish updates (not batch)
- **Implementation**: Send updates sequentially but hide this from user
- **User Experience**: Should appear as atomic batch update

### 4. Availability Validation
- **Field Reliability**: `has_menu_choice` is legacy artifact, ignore it
- **Validation**: Implement client-side validation using delivery configuration
- **Timezone**: Assume Europe/Warsaw for all calculations

### 5. Architecture Decisions
- **Backward Compatibility**: None required, complete replacement
- **Data Models**: Remove old structures entirely, create new ones
- **Caching**: Simple in-memory cache for diet list/details (non-persistent)

### 6. Error Handling
- **500 Errors**: Exponential backoff up to 10 retries
- **Other Errors**: Fail immediately with debug details
- **Logging**: Comprehensive logging for debugging

### 7. Testing & Documentation
- **Testing**: Use real account, record actual API responses
- **Test Coverage**: Create tests but avoid over-cementing
- **Documentation**: Update during implementation for session continuity

## API Endpoint Mapping

### Old API → New API

| Operation | Old API | New API |
|-----------|---------|---------|
| Get Diets | `GET /frontend/secure/my-diets` | `GET /clientDiets?brand_id={brand_id}&type=active` |
| Get Diet Details | N/A | `GET /clientDiets/{client_diet_id}` |
| Get Menu | `GET /v2/frontend/secure/calendar/{diet_id}/days/{date}/items` | `GET /diets/menu?diet_id={id}&var_id={v}&var_cal_id={vc}&dmenu={date}&type={type}&brand_id={b}&client_diet_id={cd}` |
| Update Menu | `PUT /v2/frontend/secure/calendar/{diet_id}/days/{date}/change-menu` | `POST /clientDiets/dish` |
| Get Calendar | `GET /frontend/secure/calendar/{diet_id}/{from}/{to}` | Replaced by client diet details |
| Get Delivery Config | N/A | `GET /diets/delivery?brand_id={brand_id}` |

## Implementation Phases

### Phase 1: Clean Removal of Old API Code
**Goal**: Remove all old API code to avoid confusion

**Tasks**:
1. Delete old API functions from `src/api.rs`:
   - [x] Remove `refresh_token()` function
   - [x] Remove `get_diet()` function
   - [x] Remove `fetch_diets()` function
   - [x] Remove `fetch_calendar()` function
   - [x] Remove `change_menu()` function
   - [x] Remove `fetch_ingredients()` function

2. Remove old data models from `src/serde.rs`:
   - [x] Remove `CalendarDayItems` structure
   - [x] Remove `DietElements` structure
   - [x] Remove `DishItem` structure
   - [x] Remove old `Diet` structure
   - [x] Remove `Calendar` structure
   - [x] Remove `ChangeMenuRequest` structure

3. Clean up main.rs:
   - [x] Comment out all workflow code temporarily
   - [x] Keep CLI structure intact

**Documentation Updates**:
- [x] Update API.md to reflect removal (removed old API.md content)
- [x] Update ARCHITECTURE.md (not needed - file doesn't exist)

### Phase 2: Implement New Data Models
**Goal**: Create data structures matching new API exactly

**New Structures in `src/serde.rs`**:

```rust
// Client Diet List Response
pub struct ClientDietsResponse {
    pub status: String,
    pub data: ClientDietsData,
}

pub struct ClientDietsData {
    pub diets: Vec<ClientDiet>,
}

pub struct ClientDiet {
    pub id: i64,                    // client_diet_id
    pub diet_id: i64,                // diet type
    pub var_id: i64,                 // variant
    pub var_cal_id: i64,             // calorie variant
    pub date_from: String,
    pub date_to: String,
    pub diet_name: String,
    pub has_menu_choice: i32,
    // ... other fields
}

// Diet Details Response
pub struct ClientDietDetails {
    pub status: String,
    pub data: DietDetailsData,
}

pub struct DietDetailsData {
    pub items: Vec<ClientDietItem>,
    pub totalDays: i32,
    pub pastDays: i32,
}

pub struct ClientDietItem {
    pub id: i64,                     // client_diet_item_id (day identifier)
    pub date_dlv: String,             // delivery date
    pub diet_id: i64,
    pub var_id: i64,
    pub var_cal_id: i64,
    pub has_menu_choice: i32,        // ignore this, unreliable
    pub dishes: Vec<DishInfo>,
}

// Menu Response (flat structure)
pub struct MenuResponse {
    pub data: Vec<MenuDish>,
}

pub struct MenuDish {
    pub brand_id: i32,
    pub dmenu: String,                // date
    pub diet_id: i64,
    pub var_id: i64,
    pub var_cal_id: i64,
    pub var_cal_meal_id: i64,         // unique meal variant
    pub dish_id: i64,
    pub dish_name: String,
    pub meal_id: i64,
    pub meal_name: String,
    pub meal_seq: i32,                // sequence (1=breakfast, etc.)
    pub dish_ing_names: String,       // ingredients text
    pub dish_allergens: String,
    // nutritional fields...
}

// Dish Update Request
pub struct DishUpdateRequest {
    pub brand_id: i32,
    pub client_diet_item_id: i64,
    pub dish_id: i64,
    pub diet_id: i64,
    pub var_cal_meal_id: i64,
}

// Delivery Configuration
pub struct DeliveryConfig {
    pub data: DeliveryData,
}

pub struct DeliveryData {
    pub delivery: Vec<DeliveryRule>,
}

pub struct DeliveryRule {
    pub day_id: i32,                  // cutoff day (1=Sunday...7=Saturday)
    pub delv_day_id: i32,             // delivery day
    pub delv_type_id: i32,            // 5 = menu selection
    pub delv_time: String,             // cutoff time
}
```

**Tasks**:
- [ ] Create all new structures
- [ ] Add proper serde annotations
- [ ] Implement helper methods
- [ ] Add Debug derives

### Phase 3: Implement New API Client Functions
**Goal**: Create new API client with proper error handling

**New Functions in `src/api.rs`**:

```rust
// Base URL constant
const API_BASE: &str = "https://api.powerfoods.pl/api/v1";

// Extract brand_id from JWT token
pub fn extract_brand_id(token: &str) -> eyre::Result<i32> {
    // Decode JWT payload (base64)
    // Parse JSON
    // Extract brand_id field
}

// Fetch active client diets
pub async fn fetch_client_diets(token: &str, brand_id: i32) -> eyre::Result<ClientDietsResponse> {
    let url = format!("{}/clientDiets?brand_id={}&type=active", API_BASE, brand_id);
    // Implementation with retry logic
}

// Fetch diet details with days
pub async fn fetch_diet_details(token: &str, client_diet_id: i64) -> eyre::Result<ClientDietDetails> {
    let url = format!("{}/clientDiets/{}", API_BASE, client_diet_id);
    // Implementation
}

// Fetch menu (all available or current selections)
pub async fn fetch_menu(
    token: &str,
    diet_id: i64,
    var_id: i64,
    var_cal_id: i64,
    date: &str,
    menu_type: &str, // "all" or "client"
    brand_id: i32,
    client_diet_id: i64,
) -> eyre::Result<MenuResponse> {
    // Build URL with all parameters
    // Implementation
}

// Update single dish selection
pub async fn update_dish_selection(
    token: &str,
    update: &DishUpdateRequest,
) -> eyre::Result<()> {
    let url = format!("{}/clientDiets/dish", API_BASE);
    // POST request
}

// Fetch delivery configuration
pub async fn fetch_delivery_config(token: &str, brand_id: i32) -> eyre::Result<DeliveryConfig> {
    let url = format!("{}/diets/delivery?brand_id={}", API_BASE, brand_id);
    // Implementation
}

// Core request function with retry logic
async fn send_request_with_retry(
    url: &str,
    token: &str,
    method: Method,
    body: Option<String>,
) -> eyre::Result<String> {
    let mut retries = 0;
    let max_retries = 10;

    loop {
        match send_request(url, token, method.clone(), body.clone()).await {
            Ok(response) => return Ok(response),
            Err(e) if is_server_error(&e) && retries < max_retries => {
                let delay = 2_u64.pow(retries);
                tokio::time::sleep(Duration::from_secs(delay)).await;
                retries += 1;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}
```

**Tasks**:
- [ ] Implement JWT brand_id extraction
- [ ] Create all API functions
- [ ] Add retry logic for 500 errors
- [ ] Implement proper error context

### Phase 4: Implement Availability Validation
**Goal**: Client-side validation for menu selection availability

**Implementation in new module `src/availability.rs`**:

```rust
use chrono::{DateTime, Local, NaiveDate, Timelike, Datelike};
use chrono_tz::Europe::Warsaw;

// Check if menu selection is available for a date
pub fn is_menu_selection_available(
    delivery_date: &NaiveDate,
    delivery_config: &DeliveryConfig,
) -> bool {
    let time_remaining = calculate_time_remaining(delivery_date, delivery_config);
    time_remaining > Duration::zero()
}

// Calculate time until cutoff
fn calculate_time_remaining(
    delivery_date: &NaiveDate,
    config: &DeliveryConfig,
) -> Duration {
    // Get day of week for delivery date (1=Sunday...7=Saturday)
    let day_id = get_day_id(delivery_date);

    // Find rule for menu selection (delv_type_id = 5)
    let rule = config.data.delivery
        .iter()
        .find(|r| r.delv_type_id == 5 && r.day_id == day_id);

    if let Some(rule) = rule {
        // Calculate cutoff datetime
        let cutoff = calculate_cutoff(delivery_date, rule);

        // Get current time in Warsaw timezone
        let now = Local::now().with_timezone(&Warsaw);

        // Return time difference
        cutoff - now
    } else {
        // No rule found, assume unavailable
        Duration::seconds(-1)
    }
}

fn get_day_id(date: &NaiveDate) -> i32 {
    match date.weekday() {
        Weekday::Sun => 1,
        Weekday::Mon => 2,
        Weekday::Tue => 3,
        Weekday::Wed => 4,
        Weekday::Thu => 5,
        Weekday::Fri => 6,
        Weekday::Sat => 7,
    }
}
```

**Tasks**:
- [ ] Create availability module
- [ ] Implement cutoff calculation
- [ ] Add timezone handling
- [ ] Test with various scenarios

### Phase 5: Rewrite Main Workflow
**Goal**: Update main.rs to use new API and data structures

**Key Changes**:

```rust
// New workflow structure
async fn run_meal_selection(token: &str) -> eyre::Result<()> {
    // 1. Extract brand_id from token
    let brand_id = extract_brand_id(token)?;

    // 2. Fetch all client diets
    let diets = fetch_client_diets(token, brand_id).await?;

    // 3. Build cache and mappings
    let mut diet_cache = DietCache::new();

    // 4. Process each diet
    for diet in diets.data.diets {
        // Fetch diet details
        let details = fetch_diet_details(token, diet.id).await?;
        diet_cache.store(diet.id, details);

        // Process available days
        for item in &details.data.items {
            if is_available_for_selection(&item, &delivery_config) {
                process_day_selection(token, &item, &diet).await?;
            }
        }
    }
}

// Process single day selection
async fn process_day_selection(
    token: &str,
    day_item: &ClientDietItem,
    diet: &ClientDiet,
) -> eyre::Result<()> {
    // 1. Fetch all available dishes
    let all_dishes = fetch_menu(
        token, diet.diet_id, diet.var_id, diet.var_cal_id,
        &day_item.date_dlv, "all", brand_id, diet.id
    ).await?;

    // 2. Fetch current selections
    let current = fetch_menu(
        token, diet.diet_id, diet.var_id, diet.var_cal_id,
        &day_item.date_dlv, "client", brand_id, diet.id
    ).await?;

    // 3. Group by meal
    let meal_groups = group_dishes_by_meal(&all_dishes);

    // 4. Get AI recommendations
    let recommendations = get_ai_recommendations(&meal_groups, &history).await?;

    // 5. User interaction
    let selections = present_and_confirm(recommendations)?;

    // 6. Submit updates sequentially
    submit_menu_updates(token, &selections, day_item.id).await?;
}

// Submit updates appearing atomic
async fn submit_menu_updates(
    token: &str,
    selections: &[DishSelection],
    day_item_id: i64,
) -> eyre::Result<()> {
    status("Saving menu changes...");

    for selection in selections {
        let update = DishUpdateRequest {
            brand_id: extract_brand_id(token)?,
            client_diet_item_id: day_item_id,
            dish_id: selection.dish_id,
            diet_id: selection.diet_id,
            var_cal_meal_id: selection.var_cal_meal_id,
        };

        update_dish_selection(token, &update).await?;
    }

    clear_status();
    println!("✓ Menu updated successfully");
    Ok(())
}
```

**Tasks**:
- [ ] Rewrite main selection flow
- [ ] Implement diet cache
- [ ] Update day processing logic
- [ ] Handle sequential updates
- [ ] Add progress indicators

### Phase 6: Adapt Ingredients System
**Goal**: Parse ingredients from new API structure

**Changes to ingredient handling**:

```rust
// Parse ingredients from dish_ing_names field
fn parse_ingredients(dish: &MenuDish) -> Vec<String> {
    // dish_ing_names format: "Ingredient1(details...),Ingredient2(...),..."
    dish.dish_ing_names
        .split(',')
        .map(|s| {
            // Extract ingredient name before parenthesis
            s.split('(').next().unwrap_or(s).trim().to_string()
        })
        .collect()
}

// Update cache to handle new structure
impl IngredientsCache {
    pub fn get_from_dish(&self, dish: &MenuDish) -> Vec<String> {
        // No more dish_size_id lookup needed
        parse_ingredients(dish)
    }
}
```

**Tasks**:
- [ ] Update ingredient parsing
- [ ] Modify cache structure
- [ ] Remove old ingredient fetching
- [ ] Test ingredient extraction

### Phase 7: Update AI Integration
**Goal**: Adapt AI system to new data structures

**Changes needed**:

```rust
// Update data passed to AI
pub async fn get_ai_recommendations(
    meal_groups: &HashMap<i32, Vec<MenuDish>>,
    history: &[MenuDish],
    preferences: &str,
) -> eyre::Result<AiResponse> {
    // Build context with new structure
    let context = build_ai_context(meal_groups, history);

    // Generate prompt
    let prompt = format_ai_prompt(&context, preferences);

    // Get AI response
    let response = call_ai_service(prompt).await?;

    // Parse and return
    parse_ai_response(response)
}
```

**Tasks**:
- [ ] Update AI context building
- [ ] Modify prompt generation
- [ ] Adapt response parsing
- [ ] Test AI integration

### Phase 8: Implement Caching Layer
**Goal**: Simple in-memory cache for diet data

**Implementation**:

```rust
// Simple in-memory cache
pub struct DietCache {
    diets: HashMap<i64, ClientDietDetails>,
    delivery_config: Option<DeliveryConfig>,
}

impl DietCache {
    pub fn new() -> Self {
        Self {
            diets: HashMap::new(),
            delivery_config: None,
        }
    }

    pub fn store(&mut self, id: i64, details: ClientDietDetails) {
        self.diets.insert(id, details);
    }

    pub fn get(&self, id: i64) -> Option<&ClientDietDetails> {
        self.diets.get(&id)
    }
}
```

**Tasks**:
- [ ] Create cache structure
- [ ] Implement storage methods
- [ ] Add cache to main flow
- [ ] Test cache effectiveness

### Phase 9: Testing & Validation
**Goal**: Ensure everything works correctly

**Test Plan**:

1. **Unit Tests**:
   - [ ] JWT parsing tests
   - [ ] Availability calculation tests
   - [ ] Ingredient parsing tests
   - [ ] Data transformation tests

2. **Integration Tests**:
   - [ ] Record real API responses
   - [ ] Create test fixtures
   - [ ] Test full workflow
   - [ ] Error scenario testing

3. **End-to-End Testing**:
   - [ ] Test with real account
   - [ ] Verify meal selections
   - [ ] Check AI recommendations
   - [ ] Validate updates work

**Tasks**:
- [ ] Create test infrastructure
- [ ] Record API responses
- [ ] Write unit tests
- [ ] Write integration tests
- [ ] Perform E2E testing

### Phase 10: Documentation Updates
**Goal**: Keep documentation current for multi-session work

**Documentation to Update**:

1. **API.md**:
   - [ ] Update all function signatures
   - [ ] Document new endpoints
   - [ ] Update examples

2. **ARCHITECTURE.md**:
   - [ ] Update architecture diagrams
   - [ ] Document new flow
   - [ ] Update component descriptions

3. **FUNCTIONS.md**:
   - [ ] Document all new functions
   - [ ] Update call graphs
   - [ ] Add usage examples

4. **README.md**:
   - [ ] Update setup instructions
   - [ ] Document token requirements
   - [ ] Update usage examples

**Tasks**:
- [ ] Update during implementation
- [ ] Review for completeness
- [ ] Add migration notes

## Progress Tracking

### Session Checkpoints
Mark completed items with ✓ and in-progress with ⚡

- [✓] Phase 1: Remove old API code (Completed 2025-01-13)
- [ ] Phase 2: Implement new data models
- [ ] Phase 3: Create new API client
- [ ] Phase 4: Implement availability validation
- [ ] Phase 5: Rewrite main workflow
- [ ] Phase 6: Adapt ingredients system
- [ ] Phase 7: Update AI integration
- [ ] Phase 8: Implement caching
- [ ] Phase 9: Testing & validation
- [ ] Phase 10: Documentation complete

### Critical Milestones

1. **Milestone 1**: Old code removed, new models ready ✓ Phase 1 Complete
2. **Milestone 2**: API client functional with auth
3. **Milestone 3**: Basic workflow operational
4. **Milestone 4**: Full feature parity achieved
5. **Milestone 5**: Testing complete, ready for use

## Risk Log

| Risk | Mitigation | Status |
|------|------------|---------|
| API changes during development | Document API responses, version lock | Active |
| Token expiration during testing | Implement token refresh reminder | Pending |
| Ingredient parsing errors | Fallback to raw text display | Pending |
| Rate limiting | Implement backoff and caching | Pending |

## Notes for Next Session

When resuming work:
1. Check this plan for current phase
2. Review completed checkboxes
3. Check git log for recent changes
4. Run tests to verify state
5. Continue from last checkpoint

### Phase 1 Completion Notes (2025-01-13)

**Completed Actions:**
- Removed all old API functions from `src/api.rs`, keeping only `send_request()` for reuse
- Cleared `src/serde.rs` and added temporary placeholder structures (CalendarDayItems, DishItem, DishSizeIngredients)
- Commented out main workflow in `src/main.rs` with TODO markers for Phase 5
- Commented out dependent functions (diet_for_date, days_available_to_select, etc.)
- Updated imports to comment out unused dependencies
- Fixed AI module compilation by commenting out select_dish and ai_generate_preferences functions
- Added informative message when running the application about migration status

**Current State:**
- Application compiles successfully with `cargo build`
- Main functionality is disabled with clear migration messages
- Structure preserved for Phase 2 implementation
- All old API code completely removed
- Ready to implement new PowerFoods API structures

**Next Steps:**
- Begin Phase 2: Implement new data models
- Create structures for new PowerFoods API
- Start with ClientDietsResponse, ClientDiet, MenuResponse, etc.

## Implementation Order (Recommended)

1. Start with Phase 1 (clean removal)
2. Implement Phase 2-3 (models and API)
3. Test API connectivity
4. Implement Phase 4-5 (validation and workflow)
5. Test basic flow
6. Add Phase 6-7 (ingredients and AI)
7. Complete Phase 8-10 (polish and docs)

## Success Criteria

- [ ] All meal selections work correctly
- [ ] Ingredients properly extracted and displayed
- [ ] AI recommendations maintain quality
- [ ] Error handling robust with retries
- [ ] Performance acceptable (under 2s per operation)
- [ ] Documentation complete and accurate
- [ ] Tests passing and comprehensive

---

*This plan is a living document. Update it as implementation progresses and new requirements emerge.*
