use std::{env, fs};

use inf::{Entry, Inf, Value};

fn main() {
    let path = env::args().nth(1).expect("expected path as first argument");
    let mut reader = fs::File::open(path).expect("failed to open file");
    let inf = Inf::from_reader(&mut reader).expect("failed to parse INF file");

    for section in inf.sections() {
        println!("[{}]", section.name());

        for entry in section.entries() {
            match entry {
                Entry::Item(key, Value::Raw(value)) => println!("{key} = \"{value}\""),
                Entry::Item(key, Value::List(values)) => {
                    println!(
                        "{key} = {}",
                        values
                            .iter()
                            .map(|v| format!("\"{v}\""))
                            .collect::<Vec<String>>()
                            .join(",")
                    )
                }
                Entry::ValueOnly(Value::Raw(value)) => println!("\"{value}\""),
                Entry::ValueOnly(Value::List(values)) => {
                    println!(
                        "{}",
                        values
                            .iter()
                            .map(|v| format!("\"{v}\""))
                            .collect::<Vec<String>>()
                            .join(",")
                    )
                }
            }
        }

        println!();
    }
}
