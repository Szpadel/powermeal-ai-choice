# PowerMeal V2 API Specification

This document describes the API endpoints required for meal selection functionality based on analysis of the JavaScript codebase.

## Base Configuration

**Base URL:** `https://api.powerfoods.pl/api/v1`

Evidence: `powermealjs/main-S3CCR4PT.js:17922`

## Authentication

### Token Storage
- JWT token is stored in localStorage as `access_token`
- Evidence: `powermealjs/main-S3CCR4PT.js:57765`

### Token Passing
- Token is passed as Bearer token in Authorization header
- Format: `Authorization: Bearer {token}`
- Evidence:
  - `powermealjs/main-S3CCR4PT.js:42038` - Dynamic header setting
  - `powermealjs/main-S3CCR4PT.js:57768` - Bearer token format

### HTTP Headers Configuration
```javascript
httpOptions = {
    headers: new HttpHeaders({
        'Content-Type': 'application/json'
    })
}
```
Evidence: `powermealjs/main-S3CCR4PT.js:18260-18261`, `41950-41951`

## Core API Endpoints

### 1. Diet Management

#### List Client Diets
```
GET /clientDiets?brand_id={brand_id}&type={type}
```
- **Purpose:** List client diets by type (e.g., "active")
- **Evidence:** `powermealjs/main-S3CCR4PT.js:41969`
- **Parameters:**
  - `brand_id`: Brand identifier (required)
  - `type`: Diet type filter (e.g., "active")

#### Get Diet Details
```
GET /clientDiets/{id}
```
- **Purpose:** Get detailed information about a specific diet
- **Evidence:** `powermealjs/main-S3CCR4PT.js:41976`
- **Parameters:**
  - `id`: Client diet ID

#### Get Available Diets
```
GET /diets?brand_id={brand_id}
```
- **Purpose:** Get list of all available diets for selection
- **Evidence:** `powermealjs/main-S3CCR4PT.js:45272`

### 3. Menu Selection

#### Get Menu Options
```
GET /diets/menu?diet_id={diet_id}&var_id={var_id}&var_cal_id={var_cal_id}&dmenu={dmenu}&type={type}&brand_id={brand_id}&client_diet_id={client_diet_id}
```
- **Purpose:** Get menu dishes - either all available options or current selections
- **Evidence:** `powermealjs/main-S3CCR4PT.js:56596`
- **Parameters:**
  - `diet_id`: Diet identifier
  - `var_id`: Variant identifier
  - `var_cal_id`: Calorie variant identifier
  - `dmenu`: Menu date (ISO format, e.g., "2024-09-27")
  - `type`: **CRITICAL PARAMETER** - determines what is returned:
    - `"all"` - Returns all available dish options for selection
    - `"client"` - Returns currently selected dishes for the client
  - `brand_id`: Brand identifier
  - `client_diet_id`: Client diet ID (required when type="client")

#### How to Get Current Selections vs Available Options

**Evidence:** `powermealjs/main-S3CCR4PT.js:56657-56672`

To properly work with menu selection, you need to make TWO API calls:

1. **Get Currently Selected Dishes** (type="client"):
   ```http
   GET /diets/menu?diet_id=2&var_id=1&var_cal_id=2&dmenu=2024-09-27&type=client&brand_id=2&client_diet_id=123
   ```
   Returns only the dishes that are currently selected for each meal type.

2. **Get All Available Dishes** (type="all"):
   ```http
   GET /diets/menu?diet_id=2&var_id=1&var_cal_id=2&dmenu=2024-09-27&type=all&brand_id=2&client_diet_id=123
   ```
   Returns all available dish options that can be selected.

The frontend makes both calls concurrently:
- `picked` = result from type="client" (current selections)
- `full` = result from type="all" (all available options)

#### Response Structure

Both calls return the same data structure, but with different dishes based on the `type` parameter:

**Evidence:** `powermealjs/main-S3CCR4PT.js:56537-56576`

```json
{
  "data": [
    {
      // Basic Identifiers
      "brand_id": 2,
      "dmenu": "2024-09-27",
      "diet_id": 2,
      "diet_name": "Pakiet Piastowski",
      "var_id": 1,
      "var_name": "Standard",
      "var_cal_id": 2,
      "var_cal_name": "2000 kcal",
      "var_cal_meal_id": 194,  // Unique meal variant identifier

      // Dish Information
      "dish_id": 590,
      "dish_name": "Kanapka jajeczna z jarzynami i dipem czosnkowym",
      "dish_photo": "https://admin.powerfoods.pl/backend/public/storage/brands/...",
      "dish_photo_client": "https://admin.powerfoods.pl/backend/public/storage/brands/...",
      "dish_suggest": "Najlepiej smakuje na zimno",
      "dish_tags": "vegetarian",

      // Meal Information
      "meal_id": 3,
      "meal_name": "Śniadanie",
      "meal_seq": 1,  // Sequence order (1=breakfast, 2=second breakfast, etc.)

      // Nutritional Information
      "macro": "P 18.84 T 23.89 W 48.85",  // Formatted macro string
      "protein": "18.84",        // Protein in grams
      "fat": "23.89",            // Fat in grams
      "fat_saturated": "5.72",   // Saturated fat in grams
      "carbohydrate": "48.85",   // Carbohydrates in grams
      "sugar": "9.37",           // Sugar in grams
      "salt": "2.73",            // Salt in grams
      "fiber": "5.54",           // Fiber in grams
      "calory": "370.47",        // Calories
      "weight": "269.51",        // Weight in grams

      // Ingredients and Allergens
      "dish_ing_names": "Chleb Viking(CHLEB Z KIEŁKAMI OWSA [MĄKA ŻYTNIA typ 720, woda, MĄKA PSZENNA typ 750...]),Dip czosnkowy (jogurt)...",
      "dish_allergens": "Gluten,Gorczyca,Jaja,Mleko",

      // Rating (if available)
      "rating": null,
      "rating_descr": null,
      "rating_id": null
    }
    // ... more dishes
  ]
}
```

#### Determining Selected Dishes - Complete Process

**Evidence:** `powermealjs/main-S3CCR4PT.js:55803-55804, 56687`

1. **Frontend Flow:**
   - Call API with `type="client"` to get current selections
   - Call API with `type="all"` to get all available options (only if menu can be changed)
   - Store current selections in `clientMenu` array
   - Track selections using `selectedIds` Set with key format: `${meal_id}_${dish_id}`

2. **Selection Identification:**
   - For each meal type (`meal_seq`), there's only one selected dish
   - The selected dish is the one returned by `type="client"` for that meal type
   - Match dishes between "client" and "all" responses using `meal_id`

3. **Example Implementation:**
   ```javascript
   // Get current selections
   const currentSelections = await fetch('/diets/menu?type=client&...');

   // Get all available options
   const allOptions = await fetch('/diets/menu?type=all&...');

   // Create selection map
   const selectedDishes = new Map();
   currentSelections.data.forEach(dish => {
     selectedDishes.set(dish.meal_id, dish);
   });

   // Group all options by meal_seq
   const mealGroups = {};
   allOptions.data.forEach(dish => {
     if (!mealGroups[dish.meal_seq]) {
       mealGroups[dish.meal_seq] = [];
     }
     mealGroups[dish.meal_seq].push(dish);
   });

   // For each meal group, identify which is selected
   Object.values(mealGroups).forEach(group => {
     const mealId = group[0].meal_id;
     const selectedDish = selectedDishes.get(mealId);
     // selectedDish is the currently selected option for this meal type
   });
   ```

## Data Models

### Dish Object
Based on analysis of the code, dish objects contain:
```json
{
  "dish_id": 590,
  "diet_id": 6,
  "diet_name": "Pakiet Premium",
  "meal_id": 1,
  "meal_type": "breakfast",
  "is_selected": true
}
```
Evidence: `powermealjs/main-S3CCR4PT.js:56081-56083`, `56188`, `50202`

### Client Diet Object
```json
{
  "id": 123,
  "diet_id": 6,
  "diet_name": "Pakiet Premium",
  "client_diet_name": "My Custom Diet",
  "var_id": 1,
  "var_cal_id": 2,
  "days": ["2024-01-15", "2024-01-16"],
  "order_id": 456
}
```
Evidence: `powermealjs/main-S3CCR4PT.js:41879-41900`, `57173`

## Authentication Flow

1. User logs in and receives JWT token
2. Token is stored in localStorage as "access_token"
3. For authenticated requests, retrieve token: `localStorage.getItem("access_token")`
4. Add to request headers: `Authorization: Bearer {token}`

## Common Parameters

- `brand_id`: Brand identifier (appears to be a constant in the app)
- Dates are typically in ISO format (YYYY-MM-DD)
- Response data is usually wrapped in a `data` field

## Error Handling

The API likely returns standard HTTP status codes:
- 200: Success
- 401: Unauthorized (token expired or invalid)
- 400: Bad Request
- 500: Server Error

## Listing Available Diet Days and Determining Diet IDs

### Step 1: List Active Client Diets
**Endpoint**: `GET /clientDiets?brand_id={brand_id}&type={type}`

**Evidence**: `powermealjs/main-S3CCR4PT.js:41969`

Returns a list of client diets. Common types:
- `"active"` - Currently active diets
- `"all"` - All client diets

**Response Structure** (Evidence: lines 41865-41902):
```json
{
  "status": "success",
  "data": {
    "diets": [
      {
        "id": 123,                        // client_diet_id (use this for details)
        "client_id": 456,
        "client_address_id": 789,
        "diet_id": 6,                     // The diet type ID
        "date_from": "2024-01-01",
        "date_to": "2024-01-31",
        "created_at": "2024-01-01T10:00:00",
        "updated_at": "2024-01-01T10:00:00",
        "var_cal_id": 2,                  // Calorie variant ID
        "quantity": 1,
        "is_active": 1,
        "diet_name": "Pakiet Premium",
        "var_cal_name": "2000 kcal",
        "var_id": 1,                      // Variant ID
        "variant_name": "Standard",
        "street": "Example Street",
        "building": "10",
        "flat": "5",
        "city_name": "Warsaw",
        "postcode": "00-001",
        "diet_price": 59.99,
        "total_days": 30,
        "has_menu_choice": 1,
        "order_id": 1001,
        "client_diet_name": "My January Diet",
        "invoice_id": 2001
      }
    ]
  }
}
```

### Step 2: Get Diet Days with Details
**Endpoint**: `GET /clientDiets/{client_diet_id}`

**Evidence**: `powermealjs/main-S3CCR4PT.js:41976`

Use the `id` from Step 1 to get detailed information about diet days.

**Response Structure** (Evidence: lines 41913-41917, 56646, 56694-56697):
```json
{
  "status": "success",
  "data": {
    "items": [
      {
        "id": 36024,                      // client_diet_item_id (unique day identifier)
        "date_dlv": "2025-01-15",         // Delivery date
        "has_menu_choice": 1,             // WARNING: Not reliable, requires client-side validation
        "diet_id": 6,                     // Diet type ID (matches parent diet)
        "var_id": 1,                      // Variant ID (matches parent diet)
        "var_cal_id": 2,                  // Calorie variant ID (matches parent diet)
        "dishes": [...]                   // Selected dishes for this day
      },
      {
        "id": 36025,
        "date_dlv": "2025-01-16",
        "has_menu_choice": 1,
        "diet_id": 6,
        "var_id": 1,
        "var_cal_id": 2,
        "dishes": [...]
      }
    ],
    "totalDays": 30,
    "pastDays": 5
  }
}
```

### JavaScript Implementation Example

**Evidence**: `powermealjs/main-S3CCR4PT.js:56693-56697`

```javascript
// Extract available days from clientDietDetails response
getAvailableDays() {
    return this.clientDietDetailsSubject$.getValue()?.items.map(n => ({
        day: n.date_dlv,      // The delivery date
        itemId: n.id          // The unique day identifier
    })) ?? []
}
```

### Complete Workflow to List Diet Days

1. **Get list of active diets**:
   ```http
   GET /clientDiets?brand_id=2&type=active
   ```
   Returns all active client diets with their IDs and diet configuration.

2. **For each client diet, get detailed days**:
   ```http
   GET /clientDiets/123
   ```
   Returns all days (items) for that diet with their dates and diet IDs.

3. **Each day item contains**:
   - `id`: Unique identifier for that specific day (client_diet_item_id)
   - `date_dlv`: The delivery date
   - `diet_id`: The diet type ID
   - `var_id`: The variant ID
   - `var_cal_id`: The calorie variant ID

### Key Relationships

**Evidence**: `powermealjs/main-S3CCR4PT.js:56646`

The system uses a composite key to uniquely identify a diet day configuration:
```javascript
// Unique identifier for a diet day
let key = `${item.id}|${item.diet_id}|${item.var_id}|${item.var_cal_id}|${item.date_dlv}`;
```

This ensures that when fetching menu options, the correct diet configuration is used.

## Menu Selection Availability Determination

### Required API Endpoints

#### 1. Get Diet Days
**Endpoint**: `GET /clientDiets/{client_diet_id}`

Returns daily meal plans with `has_menu_choice` field (note: this field is not reliable for real-time availability):
```json
{
  "items": [
    {
      "id": 36024,           // client_diet_item_id
      "date_dlv": "2025-09-12",
      "has_menu_choice": 1,   // WARNING: Not reliable, requires client-side validation
      "diet_id": 41,
      "var_id": 32,
      "var_cal_id": 120,
      "dishes": [...]
    }
  ]
}
```

#### 2. Get Delivery Configuration
**Endpoint**: `GET /diets/delivery?brand_id={brand_id}`

Returns cutoff rules needed for availability calculation:
```json
{
  "data": {
    "delivery": [
      {
        "day_id": 1,        // Delivery day (1=Sunday...7=Saturday)
        "delv_day_id": 5,   // Cutoff day
        "delv_type_id": 5,  // 5 = Menu selection operation
        "delv_time": "05:00:00"  // Cutoff time
      }
    ]
  }
}
```

### JavaScript Validation Algorithm (from PowerMeal codebase)

#### 1. Operation Types (line 17931-17939)
```javascript
Tn = {
    ordering: 1,
    changeDeliveryTime: 3,
    changeAddress: 4,
    changeDietAndSelectMenu: 5,  // Menu selection operation
    addingAddons: 6,
    paymentForOrder: 7,
    deliveryInDay: 8,
    deliveryOnlyInPackage: 9
}
```

#### 2. Day ID Mapping (lines 45037-45048)
```javascript
getDayIdFromDate(t) {
    let n = {
            0: 1,  // Sunday → day_id 1
            1: 2,  // Monday → day_id 2
            2: 3,  // Tuesday → day_id 3
            3: 4,  // Wednesday → day_id 4
            4: 5,  // Thursday → day_id 5
            5: 6,  // Friday → day_id 6
            6: 7   // Saturday → day_id 7
        },
        r = t.getDay();  // JavaScript getDay() returns 0-6
    return n[r]
}
```

#### 3. Calculate Time Remaining (lines 45016-45024)
```javascript
calculateLeftTime(t, n) {
    // Filter delivery config for the operation type (n=5 for menu selection)
    let r = this.lastDeliveryConfig?.data.delivery?.filter(g => g.delv_type_id == n),
        // Get day_id for the delivery date
        a = this.getDayIdFromDate(new Date(t)),
        // Find rule matching the delivery day
        s = r?.find(g => g.day_id == a);

    // CRITICAL: If no rule found, return a past date (makes unavailable)
    if (!s) return this.formatTimeDifference(new Date(new Date("2025-01-01")));

    // Calculate actual cutoff datetime based on rule
    let l = addDays(t, this.daysBetween(s.delv_day_id, s.day_id)),
        m = parseInt(s?.delv_time?.split(":")?.[0] ?? "0"),
        h = parseInt(s?.delv_time?.split(":")?.[1] ?? "0");
    return this.formatTimeDifference(new Date(l.setHours(m, h)))
}
```

#### 4. Check if Editing Allowed (lines 52878-52880)
```javascript
canEdit(t) {
    return t.days > 0 || t.hours > 0 || t.minutes > 0 || t.seconds > 0
}
```

#### 5. Complete Validation Flow
```javascript
// To check if menu selection is available for a date:
isMenuSelectionAvailable(deliveryDate) {
    // 1. Calculate time remaining until cutoff
    let timeRemaining = calculateLeftTime(deliveryDate, 5); // 5 = changeDietAndSelectMenu

    // 2. Check if time is positive (before cutoff)
    return canEdit(timeRemaining);
}
```

### Important Notes

1. **The `has_menu_choice` field from the API is not reliable** - client-side validation using delivery configuration is required
2. **All times are compared against the current time** to determine if the cutoff has passed

## Notes for Implementation

1. All endpoints require authentication via Bearer token
2. The `brand_id` parameter appears to be a constant value specific to the deployment
3. Diet selection involves multiple steps:
   - List available diets
   - Get diet details including available days
   - Get menu options for specific days
   - Submit dish selections
4. There's a pattern of "check" then "change" for validating operations before applying them
5. The `client_diet_id` parameter in menu endpoint suggests menu options may vary based on existing selections
6. **CRITICAL**: Menu selection availability must be determined using client-side validation with the JavaScript algorithm above

## Menu Selection Update

### Overview

Menu selection updates allow users to change their meal selections for specific days. The system uses a single POST endpoint that handles both adding new selections and updating existing ones.

### Update Endpoint

#### Update/Add Dish Selection
```
POST /clientDiets/dish
```
- **Purpose:** Update or add dish selection for a meal (backend determines the operation)
- **Evidence:** `powermealjs/main-S3CCR4PT.js:42017-42019`
- **Note:** While the frontend code contains logic for both POST and PATCH operations (lines 55842-55843), actual testing confirms only POST is used
- **Request Body:**
  ```json
  {
    "brand_id": 1,                    // Brand identifier (constant `de`)
    "client_diet_item_id": 36024,     // Specific day ID from currentDayInfo.id
    "dish_id": 590,                   // ID of the dish to select
    "diet_id": 6,                     // Diet type ID
    "var_cal_meal_id": 194            // Unique meal variant identifier
  }
  ```

### Component Flow

#### 1. UI Component: app-menu-diet-card
**Evidence:** `powermealjs/main-S3CCR4PT.js:55637`

Button trigger in template:
```javascript
// Lines 55568-55574
c(0, "button", 40), S("click", function() {
    F(o);
    let n = y();
    return R(n.onChoseMenu())  // Triggers selection
})
```

Component method:
```javascript
// Lines 55622-55624
onChoseMenu() {
    this.canChangeMenu && this.choseMenu.emit(this.data)
}
```

#### 2. Parent Component Handler
**Evidence:** `powermealjs/main-S3CCR4PT.js:55754-55758`

```javascript
c(0, "app-menu-diet-card", 23), S("choseMenu", function() {
    let n = F(o).$implicit,
        r = y(3);
    return R(r.onChoseMenu(n))  // Calls parent's handler with dish data
})
```

#### 3. Confirmation Dialog
**Evidence:** `powermealjs/main-S3CCR4PT.js:55821-55829`

```javascript
onChoseMenu(t) {
    !this.latestCanChange || !this.currentDayInfo || this.dialog.open(wu, {
        maxWidth: "100%",
        data: {
            title: "Jesteś pewny że chcesz zmienić menu?",
            onConfirm: () => this.saveMenuChange(t)
        }
    })
}
```

#### 4. Save Menu Change Implementation
**Evidence:** `powermealjs/main-S3CCR4PT.js:55833-55846`

```javascript
saveMenuChange(t) {
    if (!this.currentDayInfo || !this.currentClientDiet) return;

    // Prepare payload
    let n = {
        brand_id: de,                              // Brand constant (value: 1)
        client_diet_item_id: this.currentDayInfo.id,  // Day-specific ID
        dish_id: t.dish_id,                        // New dish to select
        diet_id: t.diet_id,                        // Diet type
        var_cal_meal_id: t.var_cal_meal_id        // Meal variant
    },

    // Frontend checks for existing dish but always uses POST
    r = this.currentDayInfo.dishes.find(l =>
        Number(l.var_cal_meal_id) === t.var_cal_meal_id),

    // Note: Code shows PATCH logic but testing confirms only POST is used
    a = r ? this.cds.updateDishToClientDiet(n, Number(r.id))  // Would call PATCH (not used)
          : this.cds.addDishToClientDiet(n),                   // Calls POST (always used)

    // Find currently selected dish for this meal
    s = this.getSelectedDish(this.mealGroups.find(l =>
        l.seq === t.meal_seq).list);

    // Update local state immediately
    s && (
        this.selectedIds.delete(this.key(s.meal_id, s.dish_id)),
        this.clientMenu = this.clientMenu.filter(l =>
            l.meal_id !== s.meal_id)
    ),
    this.selectedIds.add(this.key(t.meal_id, t.dish_id)),
    this.clientMenu.push(t),
    this.clientMenuChange.emit([...this.clientMenu]),
    this.cdr.markForCheck(),

    // Execute API call and refresh on success
    a.pipe(_e(1)).subscribe(() =>
        this.cds.refreshClientDietsDetails(this.currentClientDiet.id))
}
```

### Service Implementation

**Service:** ClientDietService
**Location:** `powermealjs/main-S3CCR4PT.js:41947-42062`
**Base URL:** `https://api.powerfoods.pl/api/v1/clientDiets`

```javascript
// Line 41954
this.apiUrl = `${Fe}/clientDiets`  // Fe = "https://api.powerfoods.pl/api/v1"

// Lines 42017-42019 - This is the method actually used
addDishToClientDiet(t) {
    return this.http.post(`${this.apiUrl}/dish`, t, this.httpOptions)
}

// Lines 42020-42022 - This exists in code but is not used in practice
updateDishToClientDiet(t, n) {
    return this.http.patch(`${this.apiUrl}/dish/${n}`, t, this.httpOptions)
}
```

### Key Validations

1. **Component Level:** Checks `canChangeMenu` property
   - Evidence: Line 55623

2. **Parent Level:** Validates `latestCanChange` and `currentDayInfo`
   - Evidence: Line 55822

3. **Save Level:** Ensures required data exists
   - Evidence: Line 55834

### Post-Update Actions

#### Refresh Client Diet Details
**Evidence:** `powermealjs/main-S3CCR4PT.js:41993-41995`

```javascript
refreshClientDietsDetails(t) {
    this.detailsCache.delete(t),  // Clear cache
    this.getClientDietsDetails(t).subscribe(n =>
        this._clientDietsDetails.next(n)  // Update observable
    )
}
```

### Important Notes

1. **Single Endpoint for All Operations:** The backend handles both adding new selections and updating existing ones through the same POST endpoint
2. **Frontend Code vs Actual Behavior:** While the frontend code contains logic to choose between POST and PATCH based on existing dishes, testing confirms only POST is used in practice
3. **Backend Logic:** The API backend likely determines whether to update or add based on the combination of `client_diet_item_id` and `var_cal_meal_id`
4. **Day-Specific Updates:** Each update is tied to a specific day via `client_diet_item_id`
5. **Meal Variant Logic:** The system uses `var_cal_meal_id` to identify unique meal slots

### Update Process Summary

1. User clicks "Wybierz" (Select) button in dish card
2. Component validates `canChangeMenu` and emits event
3. Parent shows confirmation dialog: "Jesteś pewny że chcesz zmienić menu?"
4. On confirmation, `saveMenuChange` executes:
   - Prepares payload with day and dish identifiers
   - Always calls `addDishToClientDiet` (POST) regardless of existing dishes
   - Backend handles whether to update or add based on the payload
   - Updates local state optimistically
5. On API success, refreshes complete diet details

## Evidence Summary

All endpoint discoveries are based on analysis of minified JavaScript code in:
- `powermealjs/main-S3CCR4PT.js`
- `powermealjs/chunk-5LOSACQ5.js`
- `powermealjs/chunk-X7N74ZWE.js`

Key patterns identified:
- API URL construction at lines 41954, 45256, 56586
- HTTP options with headers at lines 18260, 41950, 45354
- Authorization header setting at lines 42038, 46079, 46086
- Endpoint usage throughout the service classes
- Menu update implementation at lines 55833-55846, 42017-42019
- Confirmed through testing: Only POST endpoint is used, not PATCH
