// ipadic_enum.rs
//
// Usage:
//   cargo run --release -- <PATH_TO_IPADIC_CSV_DIR>
//
// Example:
//   cargo run --release -- ~/src/mecab-ipadic-2.7.0-20070801
//
// This scans all *.csv under the given directory, collects distinct values for
// columns 5..=13 (POS..pronunciation), and prints them grouped by column index.

use std::collections::BTreeSet;
use std::env;
use std::path::Path;

use walkdir::WalkDir;

fn main() {
    let root = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("ERROR: expected one argument: path to directory containing IPADIC CSV files");
        std::process::exit(2);
    });

    let root_path = Path::new(&root);
    if !root_path.exists() {
        eprintln!("ERROR: path does not exist: {}", root);
        std::process::exit(2);
    }

    // Index 0..8 correspond to columns 5..13 in IPADIC CSV layout.
    let mut sets: Vec<BTreeSet<String>> = (0..9).map(|_| BTreeSet::new()).collect();

    let mut csv_file_count: u64 = 0;
    let mut row_count: u64 = 0;

    for entry in WalkDir::new(root_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("csv") {
            continue;
        }

        csv_file_count += 1;

        //use encoding_rs::SHIFT_JIS;

        let bytes = std::fs::read(path)
            .unwrap_or_else(|e| panic!("ERROR: could not read {}: {}", path.display(), e));

        fn decode_best_effort(path: &std::path::Path, bytes: &[u8]) -> String {
            // UTF-8 with BOM
            if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
                return std::str::from_utf8(&bytes[3..])
                    .unwrap_or_else(|e| panic!("ERROR: UTF-8 decode error in {}: {}", path.display(), e))
                    .to_string();
            }

            // Plain UTF-8
            if let Ok(s) = std::str::from_utf8(bytes) {
                return s.to_string();
            }

            // Try common Japanese encodings
            for enc in [
                encoding_rs::SHIFT_JIS,
                encoding_rs::EUC_JP,
                encoding_rs::ISO_2022_JP,
            ] {
                let (s, _, had_errors) = enc.decode(bytes);
                if !had_errors {
                    return s.to_string();
                }
            }

            panic!(
                "ERROR: could not decode {} as UTF-8, Shift-JIS, EUC-JP, or ISO-2022-JP",
                path.display()
            );
        }

        let text = decode_best_effort(path, &bytes);

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(text.as_bytes());

        for rec in rdr.records() {
            let rec = rec.unwrap_or_else(|e| panic!("ERROR: CSV parse error in {}: {}", path.display(), e));
            row_count += 1;

            if rec.len() < 13 {
                panic!(
                    "ERROR: expected >= 13 columns but got {} in {}",
                    rec.len(),
                    path.display()
                );
            }

            // Columns 5..13 are 0-based indices 4..12.
            for (i, col_idx) in (4usize..=12usize).enumerate() {
                let v = rec
                    .get(col_idx)
                    .unwrap_or_else(|| panic!("ERROR: missing column {} in {}", col_idx + 1, path.display()))
                    .trim();

                if v.is_empty() || v == "*" {
                    continue;
                }
                sets[i].insert(v.to_string());
            }
        }
    }

    if csv_file_count == 0 {
        eprintln!("ERROR: no .csv files found under: {}", root);
        std::process::exit(2);
    }

    println!("CSV files scanned: {}", csv_file_count);
    println!("Rows scanned: {}", row_count);
    println!();

    let labels = [
        "col5_pos",
        "col6_pos1",
        "col7_pos2",
        "col8_pos3",
        "col9_conj_type",
        "col10_conj_form",
        "col11_base_form",
        "col12_reading",
        "col13_pronunciation",
    ];

    for (i, (label, set)) in labels.iter().zip(sets.iter()).enumerate() {
        // Skip high-cardinality fields: base form, reading, pronunciation
        if i >= 6 {
            continue;
        }

        println!("{}: {}", label, set.len());
        for v in set {
            println!("  {}", v);
        }
        println!();
    }

}
