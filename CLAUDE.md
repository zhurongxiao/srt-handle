# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`srt-handle` is a CLI tool written in Rust for processing SRT subtitle files. It applies configurable text transformations to improve subtitle readability by skipping unwanted content, combining split phrases, and moving orphaned words to the next subtitle line.

## Development Commands

### Build and Run
```bash
cargo build                           # Build the project
cargo run -- input.srt              # Process SRT file with default config
cargo run -- input.srt -o output.srt # Specify output file
cargo run -- input.srt -c config.txt # Use custom config file
cargo check                          # Quick syntax/type check
```

### Testing
```bash
cargo test           # Run all tests
cargo test <name>    # Run specific test
```

### Code Quality
```bash
cargo fmt            # Format code
cargo clippy         # Run linter
```

### Documentation
```bash
cargo doc --open     # Generate and open documentation
```

## Project Structure

- `src/main.rs` - Main application logic with CLI parsing and SRT processing
- `config.txt` - Configuration file defining processing rules
- `Cargo.toml` - Project configuration with dependencies (clap, regex, anyhow)

## Architecture Overview

### Core Components

1. **CLI Interface** (`Args` struct) - Uses clap for argument parsing
2. **Configuration Parser** (`Config` struct) - Reads and parses config.txt
3. **SRT Parser** (`parse_srt` function) - Converts SRT content to structured entries
4. **Processing Engine** - Three main operations:
   - **SKIP**: Removes subtitle entries containing specified words
   - **COMBINE**: Merges adjacent subtitles when first ends with word A and second starts with word B
   - **END**: Moves specified trailing words from current subtitle to the beginning of next subtitle

### Configuration Format

The `config.txt` file uses this format:
```
SKIP: "word1", "word2", "word3"
COMBINE: "thank you", "entire life"
END: "I", "my", "she", "he", "as"
```

### Processing Flow

1. Parse command line arguments
2. Load configuration from file
3. Parse input SRT file into structured entries
4. Apply SKIP rules (filter out unwanted entries)
5. Apply COMBINE rules (merge split phrases)
6. Apply END rules (move orphaned words)
7. Write processed output to file

### Key Functions

- `Config::from_file()` - Parses configuration file using regex
- `parse_srt()` - Converts SRT text to `Vec<SrtEntry>`
- `should_skip_entry()` - Determines if entry should be removed
- `apply_combine_rules()` - Merges adjacent subtitle entries
- `apply_end_rules()` - Moves words between entries
- `format_srt_output()` - Converts processed entries back to SRT format