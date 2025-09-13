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
- [x] Create all new structures
- [x] Add proper serde annotations
- [x] Implement helper methods
- [x] Add Debug derives

### Phase 3: Implement New API Client Functions ✓ COMPLETED (2025-01-13)
**Goal**: Create new API client with proper error handling

**Completed Implementation in `src/api.rs`**:
- ✓ Added base64 dependency to Cargo.toml
- ✓ Implemented `API_BASE` constant
- ✓ Created `ApiError` enum for proper error handling (retryable vs non-retryable)
- ✓ Implemented `extract_brand_id` - JWT token parsing to extract brand_id
- ✓ Implemented `fetch_client_diets` - Get active client diets
- ✓ Implemented `fetch_diet_details` - Get diet details with days
- ✓ Implemented `fetch_menu` - Get menu (all available or current selections)
- ✓ Implemented `update_dish_selection` - Update single dish selection
- ✓ Implemented `fetch_delivery_config` - Get delivery configuration
- ✓ Enhanced `send_request_with_retry` - Exponential backoff for 5xx errors
- ✓ Proper error handling with typed ApiError enum (no string matching)

**Key Design Decisions**:
- Used enum-based error handling instead of string matching for reliability
- Separated rate limiting (429) from server errors (5xx) in retry logic
- Rate limiting handled with automatic retry inside send_request
- Server errors use exponential backoff up to 10 retries
- Client errors (4xx) fail immediately without retry

**Tasks**:
- [x] Implement JWT brand_id extraction
- [x] Create all API functions
- [x] Add retry logic for 500 errors with proper enum
- [x] Implement proper error context

### Phase 4: Implement Availability Validation ✓ COMPLETED (2025-01-13)
**Goal**: Client-side validation for menu selection availability

**Completed Implementation in `src/availability.rs`**:
- ✓ Created availability module with proper timezone handling
- ✓ Implemented `is_menu_selection_available()` - Main public API
- ✓ Added `calculate_time_remaining()` - Time until cutoff calculation
- ✓ Created `get_day_id()` - Weekday to API format conversion
- ✓ Added `parse_cutoff_time()` - Time string parsing
- ✓ Implemented `find_menu_selection_rule()` - Delivery rule lookup
- ✓ Created `calculate_cutoff_date()` - Cutoff date calculation
- ✓ Added comprehensive unit tests
- ✓ Integrated with main.rs module system

**Key Design Decisions**:
- Simplified to boolean availability check (no human-readable formatting)
- All times handled in Europe/Warsaw timezone
- Returns Result<bool> for clean error handling
- Proper edge case handling (no rules, invalid formats)

**Tasks**:
- [x] Create availability module
- [x] Implement cutoff calculation
- [x] Add timezone handling
- [x] Test with various scenarios

### Phase 5: Rewrite Main Workflow ✓ COMPLETED (2025-01-13)
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
- [x] Rewrite main selection flow
- [x] Implement day finding logic (find_available_days)
- [x] Update day processing logic (process_day_selection)
- [x] Handle sequential updates (submit_menu_updates)
- [x] Add progress indicators (status messages)
- [x] Create helper functions (group_dishes_by_meal, parse_ingredients_from_dish)

### Phase 6: Update AI Integration ✅ COMPLETED (2025-01-14)
**Status:** ✅ Completed

Updated the AI module to work with new API structures while maintaining the same intelligent meal selection capabilities.

**Completed Actions:**
- ✅ Removed ingredients cache module entirely (not needed with new API)
- ✅ Updated AI module imports to use MenuDish instead of old structures
- ✅ Reimplemented `select_dish` function with new signature:
  - Takes HashMap<i32, Vec<MenuDish>> for available/current meals
  - Processes meal history as Vec<MenuDish>
  - Returns AI recommendations with meal names as keys
- ✅ Updated AiMenuDietOption to use raw ingredient strings
- ✅ Implemented relative date formatting ("yesterday", "2 days ago")
- ✅ Created `fetch_meal_history` function for retrieving past selections
- ✅ Restored original UI flow:
  - Shows current selections first
  - Fetches meal history for context
  - Calls AI for recommendations
  - Interactive meal-by-meal selection with AI pre-selected
- ✅ YOLO mode auto-accepts AI recommendations
- ✅ Fixed all compilation errors and warnings

**Key Changes:**
- AI receives meal names as identifiers (not meal_seq IDs)
- Relative dates in history for better AI understanding
- Raw ingredient strings passed without parsing
- Maintained original UX with [*] selection markers

**Tasks**:
- [x] Update ingredient parsing
- [x] Modify cache structure (removed entirely)
- [x] Remove old ingredient fetching
- [x] Test ingredient extraction

### Phase 7: Implement Caching Layer (Optional)
**Goal**: Simple in-memory cache for diet data

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

### Phase 8: Testing & Validation
**Goal**: Ensure everything works correctly

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

### Phase 9: Documentation Updates
**Goal**: Keep documentation current for multi-session work

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

### Phase 10: Final Polish & Release
**Goal**: Final cleanup and preparation for production use

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
- [✓] Phase 2: Implement new data models (Completed 2025-01-13)
- [✓] Phase 3: Create new API client (Completed 2025-01-13)
- [✓] Phase 4: Implement availability validation (Completed 2025-01-13)
- [✓] Phase 5: Rewrite main workflow (Completed 2025-01-13)
- [✓] Phase 6: Update AI integration (Completed 2025-01-14)
- [ ] Phase 7: Implement caching (Optional)
- [ ] Phase 8: Testing & validation
- [ ] Phase 9: Documentation updates
- [ ] Phase 10: Final polish & release

### Critical Milestones

1. **Milestone 1**: Old code removed, new models ready ✓ Phase 1 & 2 Complete
2. **Milestone 2**: API client functional with auth ✓ Phase 3 Complete
3. **Milestone 3**: Basic workflow operational ✓ Phase 5 Complete
4. **Milestone 4**: Full feature parity achieved ✓ Phase 6 Complete
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

### Phase 2 Completion Notes (2025-01-13)

**Completed Actions:**
- Implemented all new data structures in `src/serde.rs`:
  - ClientDietsResponse & ClientDietsData for diet listing
  - ClientDiet with all fields from API specification
  - ClientDietDetails & DietDetailsData for detailed diet information
  - ClientDietItem for individual diet days
  - DishInfo for basic dish information within diet items
  - MenuResponse & MenuDish for menu operations with full nutritional data
  - DishUpdateRequest for updating dish selections
  - DeliveryConfig, DeliveryData & DeliveryRule for delivery configuration
- Added proper serde derives (Debug, Deserialize, Serialize, Clone) to all structures
- Added serde rename attributes where needed (e.g., totalDays → total_days)
- Used Option<T> for nullable/optional fields
- Kept placeholder structures temporarily for backward compatibility
- All structures follow Rust naming conventions with proper serde mappings

**Current State:**
- All new data models implemented and ready for use
- Code passes `cargo clippy` (warnings are for unused code due to migration)
- Code compiles successfully with `cargo build`
- Ready to implement API client functions in Phase 3

**Next Steps (Completed):**
- ✓ Begin Phase 3: Implement new API client functions
- ✓ Create JWT brand_id extraction
- ✓ Implement all API endpoints with retry logic

### Phase 3 Completion Notes (2025-01-13)

**Completed Actions:**
- Added base64 dependency (v0.22) to Cargo.toml for JWT decoding
- Implemented all 6 new API functions in `src/api.rs`:
  - `extract_brand_id`: Decodes JWT token to extract brand_id from payload
  - `fetch_client_diets`: Gets active client diets from API
  - `fetch_diet_details`: Fetches detailed diet information with days
  - `fetch_menu`: Retrieves menu options (all available or current selections)
  - `update_dish_selection`: Updates a single dish selection
  - `fetch_delivery_config`: Gets delivery configuration for availability checks
- Created `ApiError` enum for proper error categorization:
  - ServerError (5xx) - retryable with exponential backoff
  - ClientError (4xx) - immediate failure
  - RateLimited (429) - handled with Retry-After header
  - Other - network or parsing errors
- Enhanced retry logic with proper error type handling (no string matching)
- All functions use async/await pattern with eyre::Result return types
- Code passes `cargo clippy` (warnings are expected for unused code during migration)
- Code passes `cargo test`

**Current State:**
- API client layer fully implemented and ready for use
- All functions properly handle errors and retry logic
- Ready for Phase 4: Implement availability validation

**Next Steps (Completed):**
- ✓ Begin Phase 4: Implement availability validation module
- ✓ Create timezone-aware availability checking
- ✓ Implement delivery configuration parsing

### Phase 4 Completion Notes (2025-01-13)

**Completed Actions:**
- Created new `src/availability.rs` module for client-side menu selection validation
- Added chrono-tz dependency (v0.10) for proper timezone handling
- Implemented key functions:
  - `is_menu_selection_available()`: Main public API returning Result<bool>
  - `calculate_time_remaining()`: Calculates time until cutoff (private helper)
  - `get_day_id()`: Converts chrono::Weekday to API format (1=Sunday...7=Saturday)
  - `parse_cutoff_time()`: Parses "HH:MM" or "HH:MM:SS" time strings
  - `find_menu_selection_rule()`: Finds delivery rules for menu selection (type_id=5)
  - `calculate_cutoff_date()`: Calculates actual cutoff date based on delivery rules
- Added comprehensive unit tests for all helper functions
- Integrated module into main.rs with `mod availability;` declaration
- All time calculations properly handled in Europe/Warsaw timezone

**Current State:**
- Availability module fully implemented and tested
- Code passes `cargo clippy` (warnings are expected for unused code)
- All tests pass (4 total tests in the project)
- Ready for Phase 5: Rewrite main workflow

**Next Steps (Completed):**
- ✓ Begin Phase 5: Rewrite main workflow
- ✓ Integrate availability checks into the main selection flow
- ✓ Use new API functions with proper error handling

### Phase 5 Completion Notes (2025-01-13)

**Completed Actions:**
- Implemented `find_available_days()` function:
  - Fetches diet details for all client diets
  - Checks availability using `is_menu_selection_available()`
  - Returns sorted list of available days
- Created helper functions:
  - `group_dishes_by_meal()`: Groups dishes by meal_seq for organization
  - `parse_ingredients_from_dish()`: Extracts ingredients from dish_ing_names field
  - `submit_menu_updates()`: Sequentially submits dish updates to API
- Implemented `process_day_selection()` complete workflow:
  - Fetches all available and current menu options
  - Groups dishes by meal for display
  - Shows menu with checkbox format [X]/[ ]
  - Displays ingredient previews
  - Applies simple selection logic (keeps current or picks first)
  - Submits changes via API
- Updated main.rs with new workflow structure:
  - Extract brand_id from JWT token
  - Fetch active client diets
  - Fetch delivery configuration
  - Process each available day
- Created `AvailableDay` struct to track selectable days
- Fixed all compilation errors and API parameter ordering
- Used actual meal names from API response (not hardcoded)

**Current State:**
- Main workflow fully implemented and functional
- Simple selection logic in place (AI integration pending)
- Code passes `cargo clippy` (warnings for unused AI code expected)
- All 8 tests pass
- Application can fetch real menu data and submit selections
- Ready for Phase 6: AI integration

**Next Steps:**
- Begin Phase 6: Update AI integration
- Create new AI functions to work with MenuDish structure
- Replace simple selection logic with AI recommendations

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
