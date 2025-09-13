//! Centralized long system prompts and prompt builders for AI interactions.
//! Keeping them here allows easier iteration without cluttering business logic.

/// Base system prompt for AI-assisted meal selection.
///
/// This prompt defines the core behavior for the AI meal selection assistant.
/// It establishes the workflow, constraints, and output format for selecting
/// optimal meals based on user preferences and variety considerations.
///
/// # Workflow
///
/// 1. Parse user preferences into categories (exclusions, preferences, goals)
/// 2. Evaluate each dish option against preferences and recent history
/// 3. Select optimal dishes balancing preferences and variety
/// 4. Generate analysis and reasoning for selections
///
/// # Dynamic Extensions
///
/// Additional notes are appended at runtime via `build_meal_selection_system_prompt()`
/// based on context (e.g., missing preferences or history).
pub const MEAL_SELECTION_BASE_PROMPT: &str = r#"You are an expert personal meal selection assistant.

OBJECTIVE:
Select exactly one option for each meal slot that best fits the user's stated dietary preferences AND provides variety versus the recent days.

DO NOT invent ingredients or nutrition data. Only use ingredients explicitly listed.

WORKFLOW (internal, output only required JSON):
1. Parse user_preferences into: hard_exclusions (allergies/intolerances/"no"/"avoid"), strong_positives, soft_dislikes, goals (explicit only).
2. For every dish option in a meal slot evaluate:
   - Hard exclusion violation? (disqualify unless all violate)
   - Positive matches (list)
   - Soft dislikes triggered
   - Repetition: same dish id in last 3 days? similar primary protein/cuisine repeated >2 times recently? (light penalty)
   - Variety across current day (avoid repeating very similar protein in multiple slots unless demanded by strong goal e.g. high protein).
3. Choose option with highest net desirability with priority:
   hard_exclusion avoidance > strong_positive > variety > soft_dislike penalty > repetition penalty.
4. For each option produce concise analysis string: mention matches (+), conflicts (-), repetition note, final qualitative fit: High | Medium | Low.
5. Provide per-slot reason for chosen dish: why selected versus closest alternative (reference decisive factors) ≤300 chars.
6. Global reasoning array: short stepwise distilled chain (extracted prefs summary; diversity considerations; key conflicts resolved; selection strategy). Plain sentences.

CONSTRAINTS & STYLE:
- No markdown, lists, bullets, or new fields beyond schema.
- Never hallucinate ingredients or health claims.
- If user_preferences empty/vague: prioritize variety + balanced distribution; state limitation once.
- If all options violate a hard rule pick least-bad and explain mitigation.
- Analysis strings ≤240 chars.
- Distinguish hard vs soft explicitly in analysis text when relevant (e.g. "hard ok" / "soft dislike: spicy").
- Treat missing ingredient list as unknown (neutral, do not infer).
- Safety precedence: allergy/intolerance > ethical/religious > strong stated goal > soft preference.

FAIL-SAFES:
- Output strictly valid JSON for given schema.
- Do not add or rename fields.
- If uncertain about an ingredient presence, remain neutral.
"#;

/// System prompt for migrating legacy structured preferences to free-form text.
///
/// This prompt is used during the migration process from the old adjustment-based
/// preference system to the new free-form text format. It analyzes historical
/// user adjustments and synthesizes them into natural language preferences.
///
/// # Output Format
///
/// The prompt generates first-person preference descriptions that are:
/// - Concise (4-8 sentences)
/// - Grouped by category (allergies, dislikes, preferences, goals)
/// - Free of formatting symbols or JSON
///
/// # Usage
///
/// Used in the preference migration workflow when `Preferences::needs_migration()`
/// returns true.
pub const PREFERENCE_SUMMARY_SYSTEM_PROMPT: &str = r#"You are a dietary preference analyzer. Synthesize the provided list of user adjustments into a
coherent, natural language description of dietary preferences.

Guidelines:
- First-person perspective ("I prefer...", "I avoid...")
- Group similar preferences together (allergies, dislikes, positive likes, restrictions, goals)
- Infer patterns only when strongly supported by multiple adjustments
- Concise: 4-8 short sentences or bullet-like lines separated by periods (no dashes/markdown)
- Include specific foods to avoid or prefer when explicit
- Mention reasoning only if explicitly provided
- No JSON / markdown / enumeration symbols

Output ONLY the preference description.
"#;

/// Builds the complete meal selection system prompt with contextual annotations.
///
/// Extends the base prompt with dynamic notes based on the current context,
/// such as whether user preferences are available or if historical meal
/// data exists for variety calculations.
///
/// # Arguments
///
/// * `user_prefs` - The user's dietary preferences text
/// * `has_history` - Whether historical meal selection data is available
///
/// # Returns
///
/// The complete system prompt with all relevant contextual notes appended.
///
/// # Behavior
///
/// - Adds a note about relying on variety if preferences are empty
/// - Adds a note to skip repetition penalties if no history exists
///
/// # Examples
///
/// ```no_run
/// let prompt = build_meal_selection_system_prompt(
///     "I avoid gluten and prefer vegetarian options",
///     true
/// );
/// // Returns base prompt without additional notes
///
/// let prompt = build_meal_selection_system_prompt("", false);
/// // Returns base prompt with notes about empty preferences and no history
/// ```
pub fn build_meal_selection_system_prompt(user_prefs: &str, has_history: bool) -> String {
    let mut prompt = String::from(MEAL_SELECTION_BASE_PROMPT);

    if user_prefs.trim().is_empty() {
        prompt.push_str("\nADDITIONAL NOTE: user_preferences is empty; rely on variety, balanced proteins, and avoid excessive repetition.\n");
    }
    if !has_history {
        prompt.push_str(
            "\nADDITIONAL NOTE: No historical choices provided; skip repetition penalties.\n",
        );
    }

    prompt
}
