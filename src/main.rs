use clap::Parser;
use regex::Regex;
use anyhow::{Result, Context};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "srt-handle")]
#[command(about = "A CLI tool to process SRT subtitle files")]
struct Args {
    #[arg(help = "Input SRT file path")]
    input: PathBuf,
    
    #[arg(short, long, help = "Output SRT file path")]
    output: Option<PathBuf>,
    
    #[arg(short, long, default_value = "config.txt", help = "Configuration file path")]
    config: PathBuf,
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
}

impl Config {
    fn from_file(path: &PathBuf) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        
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

fn main() -> Result<()> {
    let args = Args::parse();
    
    let config = Config::from_file(&args.config)?;
    
    let content = fs::read_to_string(&args.input)
        .with_context(|| format!("Failed to read input file: {}", args.input.display()))?;
    
    let mut entries = parse_srt(&content)?;
    
    entries.retain(|entry| !should_skip_entry(&entry.text, &config.skip_words));
    
    apply_combine_rules(&mut entries, &config.combine_phrases);
    
    apply_end_rules(&mut entries, &config.end_words);
    
    let output_content = format_srt_output(&entries);
    
    let output_path = args.output.unwrap_or_else(|| {
        let mut path = args.input.clone();
        if let Some(stem) = path.file_stem() {
            let new_name = format!("{}_processed.srt", stem.to_string_lossy());
            path.set_file_name(new_name);
        }
        path
    });
    
    fs::write(&output_path, output_content)
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;
    
    println!("Processed SRT file saved to: {}", output_path.display());
    
    Ok(())
}
