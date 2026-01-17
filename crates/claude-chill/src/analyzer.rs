use claude_chill::escape_sequences::{SYNC_END, SYNC_START};
use claude_chill::script_parser::strip_script_wrapper;
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: chill-analyzer <original.raw> <chill-output.raw>");
        eprintln!();
        eprintln!("Compares two script recordings, stripping headers and showing diffs.");
        return ExitCode::from(1);
    }

    let original_path = &args[1];
    let pty_path = &args[2];

    let original = match fs::read(original_path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to read {}: {}", original_path, e);
            return ExitCode::from(1);
        }
    };

    let pty_output = match fs::read(pty_path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to read {}: {}", pty_path, e);
            return ExitCode::from(1);
        }
    };

    let original_content = strip_script_wrapper(&original);
    let pty_content = strip_script_wrapper(&pty_output);

    println!(
        "Original: {} bytes -> {} bytes after stripping",
        original.len(),
        original_content.len()
    );
    println!(
        "PTY:      {} bytes -> {} bytes after stripping",
        pty_output.len(),
        pty_content.len()
    );
    println!();

    compare_and_report(original_content, pty_content);

    ExitCode::SUCCESS
}

fn compare_and_report(original: &[u8], pty: &[u8]) {
    let mut first_diff: Option<usize> = None;
    let common_len = original.len().min(pty.len());

    for i in 0..common_len {
        if original[i] != pty[i] {
            first_diff = Some(i);
            break;
        }
    }

    match first_diff {
        Some(pos) => {
            println!("FIRST DIFFERENCE at byte {}", pos);
            println!();

            let context_start = pos.saturating_sub(32);
            let context_end = (pos + 64).min(original.len()).min(pty.len());

            println!(
                "=== ORIGINAL @ {} (diff at +{}) ===",
                context_start,
                pos - context_start
            );
            print_hex_with_highlight(
                &original[context_start..context_end.min(original.len())],
                pos - context_start,
            );

            println!();
            println!(
                "=== PTY @ {} (diff at +{}) ===",
                context_start,
                pos - context_start
            );
            print_hex_with_highlight(
                &pty[context_start..context_end.min(pty.len())],
                pos - context_start,
            );

            println!();
            println!("Context around diff:");
            println!(
                "  Original byte: 0x{:02x} ({:?})",
                original[pos],
                char::from(original[pos])
            );
            println!(
                "  PTY byte:      0x{:02x} ({:?})",
                pty[pos],
                char::from(pty[pos])
            );

            let sync_before = count_sync_markers(&original[..pos]);
            println!();
            println!(
                "Sync blocks before diff: {} starts, {} ends",
                sync_before.0, sync_before.1
            );
        }
        None => {
            if original.len() == pty.len() {
                println!("FILES ARE IDENTICAL");
            } else {
                println!("Files match for {} bytes, then one ends early", common_len);
                println!("  Original: {} bytes", original.len());
                println!("  PTY:      {} bytes", pty.len());

                if original.len() > pty.len() {
                    println!();
                    println!("=== ORIGINAL CONTINUES WITH ===");
                    print_hex_with_highlight(
                        &original[common_len..(common_len + 64).min(original.len())],
                        0,
                    );
                } else {
                    println!();
                    println!("=== PTY CONTINUES WITH ===");
                    print_hex_with_highlight(&pty[common_len..(common_len + 64).min(pty.len())], 0);
                }
            }
        }
    }

    println!();
    println!("=== SYNC BLOCK SUMMARY ===");
    let orig_sync = count_sync_markers(original);
    let pty_sync = count_sync_markers(pty);
    println!(
        "Original: {} sync starts, {} sync ends",
        orig_sync.0, orig_sync.1
    );
    println!(
        "PTY:      {} sync starts, {} sync ends",
        pty_sync.0, pty_sync.1
    );
}

fn count_sync_markers(data: &[u8]) -> (usize, usize) {
    let mut starts = 0;
    let mut ends = 0;

    for i in 0..data.len().saturating_sub(SYNC_START.len()) {
        if &data[i..i + SYNC_START.len()] == SYNC_START {
            starts += 1;
        }
        if &data[i..i + SYNC_END.len()] == SYNC_END {
            ends += 1;
        }
    }

    (starts, ends)
}

fn print_hex_with_highlight(data: &[u8], highlight_pos: usize) {
    for (i, chunk) in data.chunks(16).enumerate() {
        let offset = i * 16;
        print!("{:08x}: ", offset);

        for (j, &byte) in chunk.iter().enumerate() {
            let pos = offset + j;
            if pos == highlight_pos {
                print!("\x1b[41m{:02x}\x1b[0m ", byte);
            } else {
                print!("{:02x} ", byte);
            }
            if j == 7 {
                print!(" ");
            }
        }

        for _ in chunk.len()..16 {
            print!("   ");
        }

        print!(" |");
        for (j, &byte) in chunk.iter().enumerate() {
            let pos = offset + j;
            let c = if byte.is_ascii_graphic() || byte == b' ' {
                byte as char
            } else {
                '.'
            };
            if pos == highlight_pos {
                print!("\x1b[41m{}\x1b[0m", c);
            } else {
                print!("{}", c);
            }
        }
        println!("|");
    }
}
