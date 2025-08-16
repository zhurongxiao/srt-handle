use clap::{Parser, Subcommand};
use regex::Regex;
use anyhow::{Result, Context};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::env;

// Embed config.txt contents at compile time
const EMBEDDED_CONFIG: &str = include_str!("../config.txt");

#[derive(Parser)]
#[command(name = "srt-handle")]
#[command(about = "A CLI tool to process SRT subtitle files")]
#[command(long_about = "
SRT Handle - A comprehensive SRT subtitle processing tool

COMMANDS:
  process       Process a single SRT file with configuration rules
  batch         Batch process SRT files in current directory with standardized naming
  merge         Merge bilingual SRT file with same timestamps into single entries

EXAMPLES:
  # Process single file
  srt-handle process input.srt

  # Batch process files in current directory
  srt-handle batch

  # Merge bilingual SRT file
  srt-handle merge bilingual.srt

  # Process with custom config and output
  srt-handle process input.srt -c custom.txt -o output.srt
")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Process a single SRT file with configuration rules
    Process {
        #[arg(help = "Input SRT file path")]
        input: PathBuf,
        
        #[arg(short, long, help = "Output SRT file path")]
        output: Option<PathBuf>,
        
        #[arg(short, long, help = "Configuration file path (uses embedded config if not specified)")]
        config: Option<PathBuf>,
    },
    /// Batch process SRT files in current directory with standardized naming
    Batch {
        #[arg(short, long, default_value = ".", help = "Directory to process")]
        dir: PathBuf,
        
        #[arg(short, long, help = "Configuration file path (uses embedded config if not specified)")]
        config: Option<PathBuf>,
    },
    /// Merge bilingual SRT file with same timestamps into single entries
    Merge {
        #[arg(help = "Input bilingual SRT file path")]
        input: PathBuf,
        
        #[arg(short, long, help = "Output merged SRT file path")]
        output: Option<PathBuf>,
    },
}

#[derive(Debug, Clone)]
struct SrtEntry {
    index: u32,
    timestamp: String,
    text: String,
}

#[derive(Debug, Default)]
struct Config {
    skip_words: Vec<String>,
    combine_phrases: Vec<(String, String)>,
    end_words: Vec<String>,
    insert_phrases: Vec<(String, String)>,
    split_words: Vec<String>,
}

impl Config {
    fn from_file(path: &PathBuf) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        Self::from_content(&content)
    }
    
    fn from_embedded() -> Result<Self> {
        Self::from_content(EMBEDDED_CONFIG)
    }
    
    fn from_content(content: &str) -> Result<Self> {
        let mut config = Config::default();
        
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            if let Some(skip_content) = line.strip_prefix("SKIP:") {
                config.skip_words = parse_quoted_list(skip_content);
            } else if let Some(combine_content) = line.strip_prefix("COMBINE:") {
                config.combine_phrases = parse_combine_phrases(combine_content);
            } else if let Some(end_content) = line.strip_prefix("END:") {
                config.end_words = parse_quoted_list(end_content);
            } else if let Some(insert_content) = line.strip_prefix("INSERT:") {
                config.insert_phrases = parse_combine_phrases(insert_content);
            } else if let Some(split_content) = line.strip_prefix("SPLIT:") {
                config.split_words = parse_quoted_list(split_content);
            }
        }
        
        Ok(config)
    }
}

fn parse_quoted_list(content: &str) -> Vec<String> {
    let re = Regex::new(r#""([^"]+)""#).unwrap();
    re.captures_iter(content)
        .map(|cap| cap[1].to_string())
        .collect()
}

fn parse_combine_phrases(content: &str) -> Vec<(String, String)> {
    let phrases = parse_quoted_list(content);
    let mut result = Vec::new();
    
    for phrase in phrases {
        if let Some((first, second)) = phrase.split_once(' ') {
            result.push((first.to_string(), second.to_string()));
        }
    }
    
    result
}

fn parse_srt(content: &str) -> Result<Vec<SrtEntry>> {
    let mut entries = Vec::new();
    let blocks: Vec<&str> = content.split("\n\n").collect();
    
    for block in blocks {
        let lines: Vec<&str> = block.lines().collect();
        if lines.len() >= 3 {
            if let Ok(index) = lines[0].parse::<u32>() {
                let timestamp = lines[1].to_string();
                let text = lines[2..].join(" ");
                
                entries.push(SrtEntry {
                    index,
                    timestamp,
                    text,
                });
            }
        }
    }
    
    Ok(entries)
}

fn should_skip_entry(text: &str, skip_words: &[String]) -> bool {
    let text_lower = text.to_lowercase();
    skip_words.iter().any(|word| text_lower.contains(&word.to_lowercase()))
}

fn apply_combine_rules(entries: &mut Vec<SrtEntry>, combine_phrases: &[(String, String)]) {
    let mut i = 0;
    while i < entries.len().saturating_sub(1) {
        let mut combined = false;
        
        for (first, second) in combine_phrases {
            let current_text = &entries[i].text.to_lowercase();
            let next_text = &entries[i + 1].text.to_lowercase();
            
            if current_text.ends_with(&first.to_lowercase()) && 
               next_text.starts_with(&second.to_lowercase()) {
                entries[i].text = format!("{} {}", entries[i].text, entries[i + 1].text);
                entries.remove(i + 1);
                combined = true;
                break;
            }
        }
        
        if !combined {
            i += 1;
        }
    }
}

fn apply_end_rules(entries: &mut Vec<SrtEntry>, end_words: &[String]) {
    let mut i = 0;
    while i < entries.len().saturating_sub(1) {
        let words: Vec<&str> = entries[i].text.split_whitespace().collect();
        
        for end_word in end_words {
            let end_word_parts: Vec<&str> = end_word.split_whitespace().collect();
            
            if words.len() >= end_word_parts.len() {
                let last_words = &words[words.len() - end_word_parts.len()..];
                
                if last_words.iter().zip(end_word_parts.iter())
                    .all(|(a, b)| a.to_lowercase() == b.to_lowercase()) {
                    
                    let remaining_words = &words[..words.len() - end_word_parts.len()];
                    entries[i].text = remaining_words.join(" ");
                    
                    entries[i + 1].text = format!("{} {}", end_word, entries[i + 1].text);
                    break;
                }
            }
        }
        i += 1;
    }
}

fn format_srt_output(entries: &[SrtEntry]) -> String {
    entries.iter()
        .enumerate()
        .map(|(i, entry)| {
            format!("{}\n{}\n{}\n", i + 1, entry.timestamp, entry.text)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn merge_bilingual_srt(input: &PathBuf, output: &Option<PathBuf>) -> Result<()> {
    println!("Merging bilingual SRT file: {}", input.display());
    
    let content = fs::read_to_string(input)
        .with_context(|| format!("Failed to read input file: {}", input.display()))?;
    
    let entries = parse_srt(&content)?;
    
    let mut merged_entries = Vec::new();
    let mut i = 0;
    
    while i < entries.len() {
        if i + 1 < entries.len() && entries[i].timestamp == entries[i + 1].timestamp {
            // Found a pair with same timestamp - merge them
            let english_text = &entries[i].text;
            let chinese_text = &entries[i + 1].text;
            
            let merged_entry = SrtEntry {
                index: entries[i].index,
                timestamp: entries[i].timestamp.clone(),
                text: format!("{}\n{}", english_text, chinese_text),
            };
            
            merged_entries.push(merged_entry);
            i += 2; // Skip both entries
        } else {
            // Single entry, keep as is
            merged_entries.push(entries[i].clone());
            i += 1;
        }
    }
    
    let output_content = format_srt_output(&merged_entries);
    
    let output_path = output.clone().unwrap_or_else(|| {
        let mut path = input.clone();
        if let Some(stem) = path.file_stem() {
            let new_name = format!("{}_merged.srt", stem.to_string_lossy());
            path.set_file_name(new_name);
        }
        path
    });
    
    fs::write(&output_path, output_content)
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;
    
    println!("Merged bilingual SRT saved to: {}", output_path.display());
    println!("Merged {} subtitle pairs into {} entries", entries.len() / 2, merged_entries.len());
    
    Ok(())
}

fn batch_process_srt_files(dir: &PathBuf, config_path: &Option<PathBuf>) -> Result<()> {
    println!("Scanning for SRT files in: {}", dir.display());
    
    let entries = fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?;
    
    let mut srt_files = Vec::new();
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("srt") {
            if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                srt_files.push((path.clone(), filename.to_string()));
            }
        }
    }
    
    if srt_files.is_empty() {
        println!("No SRT files found in directory.");
        return Ok(());
    }
    
    println!("Found {} SRT files", srt_files.len());
    
    let mut zh_files = Vec::new();
    let mut en_files = Vec::new();
    let mut bil_files = Vec::new();
    
    for (path, filename) in &srt_files {
        if filename.contains("[Chinese (Simplified)]") {
            zh_files.push(path.clone());
        } else if filename.contains("[English - English-Chinese (Simplified)]") {
            bil_files.push(path.clone());
        } else if filename.contains("[English - English]") {
            en_files.push(path.clone());
        }
    }
    
    if en_files.len() > 1 {
        println!("Warning: Found {} files with '[English - English]', only processing the first one", en_files.len());
    }
    
    let mut processed_files = Vec::new();
    
    if let Some(zh_file) = zh_files.first() {
        let target_path = dir.join("zh_srt.srt");
        fs::copy(zh_file, &target_path)
            .with_context(|| format!("Failed to copy {} to zh_srt.srt", zh_file.display()))?;
        println!("Renamed Chinese file to: zh_srt.srt");
        processed_files.push("zh_srt.srt");
    }
    
    if let Some(en_file) = en_files.first() {
        let target_path = dir.join("en_srt.srt");
        fs::copy(en_file, &target_path)
            .with_context(|| format!("Failed to copy {} to en_srt.srt", en_file.display()))?;
        println!("Renamed English file to: en_srt.srt");
        processed_files.push("en_srt.srt");
    }
    
    if let Some(bil_file) = bil_files.first() {
        let target_path = dir.join("bil_srt.srt");
        fs::copy(bil_file, &target_path)
            .with_context(|| format!("Failed to copy {} to bil_srt.srt", bil_file.display()))?;
        println!("Renamed bilingual file to: bil_srt.srt");
        processed_files.push("bil_srt.srt");
    }
    
    if processed_files.contains(&"en_srt.srt") {
        println!("Processing en_srt.srt for improved readability...");
        
        let en_srt_path = dir.join("en_srt.srt");
        
        let mut cmd = Command::new(env::current_exe().unwrap_or_else(|_| PathBuf::from("srt-handle")));
        cmd.arg("process").arg(&en_srt_path).current_dir(dir);
        
        // Only add config argument if one was specified
        if let Some(config_path) = config_path {
            cmd.arg("-c").arg(config_path);
        }
        
        let output = cmd.output()
            .with_context(|| "Failed to execute srt-handle process command")?;
        
        if output.status.success() {
            println!("Successfully processed en_srt.srt -> en_srt_ok.srt");
            
            println!("Cleaning up original files...");
            for (original_file, _) in &srt_files {
                if let Err(e) = fs::remove_file(original_file) {
                    eprintln!("Warning: Failed to delete {}: {}", original_file.display(), e);
                } else {
                    println!("Deleted: {}", original_file.display());
                }
            }
            
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("Failed to process en_srt.srt: {}", stderr);
        }
    }
    
    println!("Batch processing completed.");
    println!("Processed files: {}", processed_files.join(", "));
    
    Ok(())
}

fn process_single_file(input: &PathBuf, output: &Option<PathBuf>, config_path: &Option<PathBuf>) -> Result<()> {
    let config = if let Some(path) = config_path {
        Config::from_file(path)?
    } else {
        Config::from_embedded()?
    };
    
    let content = fs::read_to_string(input)
        .with_context(|| format!("Failed to read input file: {}", input.display()))?;
    
    let mut entries = parse_srt(&content)?;
    
    entries.retain(|entry| !should_skip_entry(&entry.text, &config.skip_words));
    
    apply_combine_rules(&mut entries, &config.combine_phrases);
    
    apply_end_rules(&mut entries, &config.end_words);
    
    let output_content = format_srt_output(&entries);
    
    let output_path = output.clone().unwrap_or_else(|| {
        let mut path = input.clone();
        if let Some(stem) = path.file_stem() {
            let new_name = format!("{}_ok.srt", stem.to_string_lossy());
            path.set_file_name(new_name);
        }
        path
    });
    
    fs::write(&output_path, output_content)
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;
    
    println!("Processed SRT file saved to: {}", output_path.display());
    
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    match args.command {
        Commands::Process { input, output, config } => {
            process_single_file(&input, &output, &config)?;
        }
        Commands::Batch { dir, config } => {
            batch_process_srt_files(&dir, &config)?;
        }
        Commands::Merge { input, output } => {
            merge_bilingual_srt(&input, &output)?;
        }
    }
    
    Ok(())
}
