# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`srt-handle` is a comprehensive CLI tool written in Rust for processing SRT subtitle files. It provides both single-file processing and batch processing capabilities with configurable text transformations to improve subtitle readability through intelligent merging, splitting, and cleanup operations.

## Development Commands

### Build and Run
```bash
cargo build --release                                    # Build optimized version
cargo run -- process input.srt                         # Process single SRT file
cargo run -- process input.srt -o output.srt           # Specify output file
cargo run -- process input.srt -c config.txt           # Use custom config file
cargo run -- batch                                     # Batch process current directory
cargo run -- batch -d /path/to/directory               # Batch process specific directory
cargo run -- merge bilingual.srt                      # Merge bilingual SRT file
cargo run -- merge bilingual.srt -o merged.srt        # Merge with custom output
cargo check                                            # Quick syntax/type check
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

- `src/main.rs` - Main application logic with CLI parsing, SRT processing, and batch operations
- `config.txt` - Configuration file (embedded at compile time)
- `Cargo.toml` - Project configuration with dependencies (clap, regex, anyhow)

## CLI Commands

### Process Command
Process a single SRT file with configuration rules.
```bash
srt-handle process input.srt [OPTIONS]
```

### Batch Command  
Batch process SRT files in a directory with standardized naming and automatic cleanup.
```bash
srt-handle batch [OPTIONS]
```

### Merge Command
Merge bilingual SRT file with same timestamps into single entries (English + Chinese).
```bash
srt-handle merge bilingual.srt [OPTIONS]
```

**Batch Processing Features:**
- Identifies SRT files by bracket naming patterns
- Renames files to standardized format:
  - `[Chinese (Simplified)]` → `zh_srt.srt`
  - `[English - English]` → `en_srt.srt` 
  - `[English - English-Chinese (Simplified)]` → `bil_srt.srt`
- Automatically processes English files for improved readability
- Cleans up original files with complex bracket names
- Preserves both original standardized and processed versions

**Merge Processing Features:**
- Identifies consecutive SRT entries with identical timestamps
- Merges pairs into single entries with English + Chinese text
- First line becomes English subtitle, second line becomes Chinese subtitle
- Maintains original timing and indexing
- Reduces total entry count while preserving all content

## Architecture Overview

### Core Components

1. **CLI Interface** (`Args`, `Commands` enums) - Uses clap for subcommand parsing
2. **Configuration Parser** (`Config` struct) - Reads and parses config.txt
3. **SRT Parser** (`parse_srt` function) - Converts SRT content to structured entries
4. **Processing Engine** - Six main operations:
   - **SKIP**: Removes subtitle entries containing specified words
   - **COMBINE**: Merges adjacent subtitles when first ends with word A and second starts with word B
   - **INSERT**: Moves words from next subtitle to current when patterns match
   - **END**: Moves specified trailing words from current subtitle to beginning of next subtitle
   - **SPLIT**: Splits long lines (>8 words) at specified words or middle
   - **Flexible Merging**: Intelligently merges short lines with adjacent entries
5. **Batch Processor** (`batch_process_srt_files`) - Handles directory scanning, file renaming, and cleanup
6. **Final Check Loop** - Continuously applies split and merge operations until stable

### Configuration Format

The `config.txt` file uses this format:
```
SKIP: "applause", "music", "laughter"
COMBINE: "thank you", "entire life", "drop out"
END: "I", "my", "she", "he", "as", "it was", "I could", "in", "in the", "on", "on the", "to", "be", "to be", "about", "what", "from", "I've", "it no","that's", "his", "and", "they", "by","I really","I was"
INSERT: "a"
SPLIT: "I", "my", "so"
```

### Processing Flow

#### Single File Processing:
1. Parse command line arguments
2. Load configuration from file  
3. Parse input SRT file into structured entries
4. Apply initial flexible merging (≤2 words with <5 word neighbors)
5. Apply SKIP rules (filter out unwanted entries)
6. Apply COMBINE rules (merge split phrases)
7. Apply INSERT rules (move words from next line to current)
8. Apply END rules (move orphaned words)
9. **Final Check Loop** (repeats until stable):
   - Apply SPLIT processing (lines >8 words)
   - Apply final check merging (lines <2 words)
10. Write processed output to file

#### Batch Processing:
1. Scan directory for SRT files
2. Identify files by bracket patterns in filenames
3. Rename to standardized format (zh_srt.srt, en_srt.srt, bil_srt.srt)
4. Process en_srt.srt using single file processing logic
5. Clean up original files with bracket names
6. Preserve both standardized and processed versions

### Key Functions

- `Config::from_file()` - Parses configuration file using regex
- `parse_srt()` - Converts SRT text to `Vec<SrtEntry>`
- `should_skip_entry()` - Determines if entry should be removed
- `apply_combine_rules()` - Merges adjacent subtitle entries
- `apply_insert_rules()` - Moves words from next subtitle to current
- `apply_end_rules()` - Moves words between entries
- `apply_split_processing()` - Splits long lines at specified points
- `apply_flexible_merging()` - Merges short lines with neighbors
- `apply_final_check_merging()` - Final cleanup of very short lines
- `batch_process_srt_files()` - Handles batch directory processing
- `merge_bilingual_srt()` - Merges bilingual SRT files with same timestamps
- `format_srt_output()` - Converts processed entries back to SRT format

## Configuration Management

The application uses embedded configuration for maximum portability and simplicity.

### Embedded Configuration

- **config.txt** is embedded into the binary at compile time using `include_str!`
- **No external files needed** - the binary is completely self-contained
- **Works from any directory** without requiring config file setup

### Configuration Behavior

- **Default**: Uses embedded config.txt content
- **Override**: Use `-c /path/to/custom.txt` to specify external config file
- **Embedded config includes**: All processing rules (SKIP, COMBINE, END, INSERT, SPLIT)

### Machine Portability

Deploying to a new machine is extremely simple:

1. **Copy just the binary** to the target machine
2. **Run from anywhere**: `srt-handle process file.srt`

That's it! No configuration files, environment variables, or setup required.

### Custom Configuration

If you need different processing rules:

```bash
# Use custom config file
srt-handle process input.srt -c /path/to/custom-config.txt

# Custom config file format (same as embedded)
SKIP: "applause", "music", "laughter"
COMBINE: "thank you", "entire life"
END: "I", "my", "she", "he"
INSERT: "very much"
SPLIT: "I", "my", "so"
```

### Default Configurations

- **Default behavior**: Uses embedded config.txt content
- **Default output naming**: `filename_ok.srt`
- **Self-contained**: No external dependencies