# PowerMeal AI Choice

An intelligent meal selection automation tool for PowerMeal delivery service that uses AI to recommend meals based on your preferences and dietary requirements.

## Features

- **AI-Powered Meal Selection**: Leverages OpenAI/LiteLLM to intelligently select meals based on your preferences
- **Smart Preference System**: Free-text preference input with automatic migration from legacy formats
- **Historical Analysis**: Considers your past 14 days of meal choices to ensure variety
- **Multi-Diet Support**: Handles multiple diet subscriptions seamlessly
- **Ingredient Caching**: Reduces API calls with intelligent LRU caching
- **YOLO Mode**: Auto-accept AI selections without manual confirmation
- **Flexible AI Backend**: Support for OpenAI, LiteLLM, and other compatible APIs

## Prerequisites

- Rust 1.70+ (for building from source)
- PowerMeal account with active subscription
- OpenAI API key or compatible LLM service

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/powermeal
cd powermeal

# Build the project
cargo build --release

# The binary will be available at ./target/release/powermeal-ai-choice
```

### Running Directly

```bash
# Run with cargo
cargo run -- [OPTIONS] [COMMAND]

# Or use the compiled binary
./target/release/powermeal-ai-choice [OPTIONS] [COMMAND]
```

## Configuration

### First-Time Setup

1. **Configure AI Service** (required on first run):
```bash
powermeal-ai-choice configure-ai
```
This will prompt you to:
- Enter your API endpoint (e.g., `https://api.openai.com/v1` for OpenAI)
- Provide your API key
- Select or enter a model name

2. **Set Your Preferences**:
```bash
powermeal-ai-choice edit-preferences
```
This opens your default editor to set meal preferences. Example:
```
I'm vegetarian and allergic to nuts. I prefer high-protein meals and enjoy Mediterranean and Asian cuisines. 
I don't like overly spicy food. I try to avoid processed foods and prefer fresh ingredients.
Please ensure variety throughout the week and don't repeat the same protein source on consecutive days.
```

### Configuration Files

All configuration is stored in `~/.config/powermeal-ai/`:
- `preferences.json` - User preferences, AI config, and authentication
- `ingredients_cache.json` - Cached ingredient data

Logs are stored in `~/.local/state/powermeal-ai-choice/`:
- `ai_responses.log` - Human-readable AI decision logs

## Usage

### Basic Usage

Run the meal selection process:
```bash
powermeal-ai-choice
```

The tool will:
1. Authenticate with PowerMeal (prompting for token if needed)
2. Fetch available diets and days
3. For each available day:
   - Analyze meal options with AI
   - Present recommendations with reasoning
   - Allow you to confirm or override selections
   - Update your meal plan

### YOLO Mode (Automated)

Auto-accept all AI recommendations without confirmation:
```bash
powermeal-ai-choice --yolo
```

### Commands

| Command | Description |
|---------|-------------|
| `configure-ai` | Set up or modify AI service configuration |
| `edit-preferences` | Edit your meal preferences in your default editor |
| (no command) | Run the meal selection process |

### Options

| Option | Description |
|--------|-------------|
| `--yolo` | Auto-accept AI selections without confirmation |
| `--help` | Display help information |
| `--version` | Show version information |

## Meal Selection Process

1. **Authentication**: Validates and refreshes your PowerMeal session token
2. **Diet Discovery**: Fetches your active diet subscriptions
3. **Available Days**: Determines which days have modifiable menus
4. **For Each Day**:
   - Fetches menu options with ingredients
   - Retrieves your meal history for context
   - AI analyzes options based on:
     - Your dietary preferences
     - Allergies and exclusions
     - Recent meal history (avoid repetition)
     - Nutritional goals
   - Presents AI reasoning and recommendations
   - Allows manual override if desired
   - Confirms and submits changes

## AI Integration

The tool supports any OpenAI-compatible API:

### OpenAI
```
API Base: https://api.openai.com/v1
Model: gpt-4o-mini (or gpt-4o for better results)
```

### LiteLLM (Local or Proxy)
```
API Base: http://localhost:11434/v1
Model: [your-configured-model]
```

### Other Compatible Services
Any service implementing the OpenAI chat completions API with JSON schema support.

## Preference System

### Writing Effective Preferences

Your preferences should include:

1. **Hard Exclusions** (allergies, dietary restrictions):
   - "I'm allergic to nuts and shellfish"
   - "I'm vegetarian/vegan"
   - "I don't eat pork for religious reasons"

2. **Positive Preferences**:
   - "I love Mediterranean and Asian cuisines"
   - "I prefer high-protein meals"
   - "I enjoy meals with lots of vegetables"

3. **Soft Dislikes**:
   - "I prefer to avoid overly spicy food"
   - "I don't particularly enjoy cream-based sauces"

4. **Goals and Patterns**:
   - "Please ensure variety throughout the week"
   - "Avoid repeating proteins on consecutive days"
   - "I'm trying to reduce carb intake"

### Migration from Legacy Format

If you have existing preferences in the old format, the tool will automatically:
1. Detect the legacy format
2. Use AI to convert your adjustments to natural language
3. Open an editor for you to review and modify
4. Save the new format

## Troubleshooting

### Authentication Issues

If you encounter authentication errors:
1. The tool will automatically prompt for a new session token
2. Log into PowerMeal web interface
3. Open browser developer tools (F12)
4. Find any API request and copy the Authorization header token
5. Paste when prompted

### AI Service Issues

- **Rate Limiting**: The tool automatically handles rate limits with retries
- **Connection Errors**: Check your API endpoint and key configuration
- **Model Not Found**: Run `configure-ai` to select a different model

### Cache Issues

To clear the cache:
```bash
rm ~/.config/powermeal-ai/ingredients_cache.json
```

### Viewing AI Decisions

To understand AI decision-making:
```bash
tail -f ~/.local/state/powermeal-ai-choice/ai_responses.log
```

## Development

### Building from Source

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run
```

### Project Structure

```
src/
├── main.rs         # Application entry point and orchestration
├── ai.rs           # AI service integration
├── api.rs          # PowerMeal API client
├── cache.rs        # LRU caching system
├── preferences.rs  # Configuration management
├── prompts.rs      # AI prompt templates
└── serde.rs        # Data structures and models
```

## License

[Your License Here]

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Support

For issues or questions, please open an issue on GitHub.