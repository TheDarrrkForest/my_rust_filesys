mod device;
mod fs;
mod layout;

use crate::fs::MyFileSystem;
use std::io::{self, Write};

// 在传入内层函数之前转绝对路径
fn to_absolute_path(cwd: &str, input: &str) -> String {
    let mut combined = if input.starts_with('/') {
        input.to_string()
    } else {
        let mut base = cwd.to_string();
        if !base.ends_with('/') {
            base.push('/');
        }
        base.push_str(input);
        base
    };

    let parts = combined.split('/');
    let mut stack = Vec::new();

    for part in parts {
        match part {
            "" | "." => continue,
            ".." => {
                stack.pop();
            }
            _ => {
                stack.push(part);
            }
        }
    }

    let mut result = "/".to_string() + &stack.join("/");
    if result.is_empty() {
        result = "/".to_string();
    }
    // print!("{}", result);
    result
}

fn main() {
    let mut fs = fs::MyFileSystem::new("disk.img");
    println!("MyFS CLI started.");

    loop {
        print!("myfs:{}> ", fs.cwd_path);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let cmd_parts: Vec<&str> = input.trim().split_whitespace().collect();
        if cmd_parts.is_empty() {
            continue;
        }

        match cmd_parts[0] {
            "format" => fs.format(),
            "cd" => {
                let raw_path = if cmd_parts.len() > 1 {
                    cmd_parts[1]
                } else {
                    "/"
                };
                let abs_path = to_absolute_path(&fs.cwd_path, raw_path);
                fs.cd(&abs_path);
            }
            "ls" => {
                let raw_path = if cmd_parts.len() > 1 {
                    cmd_parts[1]
                } else {
                    "."
                };
                let abs_path = to_absolute_path(&fs.cwd_path, raw_path);
                fs.list_dir(&abs_path);
            }
            "mkdir" => {
                if cmd_parts.len() > 1 {
                    let abs_path = to_absolute_path(&fs.cwd_path, cmd_parts[1]);
                    fs.mkdir(&abs_path);
                }
            }
            "touch" => {
                if cmd_parts.len() > 1 {
                    let abs_path = to_absolute_path(&fs.cwd_path, cmd_parts[1]);
                    fs.touch(&abs_path);
                }
            }
            "write" => {
                if cmd_parts.len() > 2 {
                    let abs_path = to_absolute_path(&fs.cwd_path, cmd_parts[1]);
                    let content = cmd_parts[2..].join(" ");
                    fs.write_file(&abs_path, &content);
                }
            }
            "cat" => {
                if cmd_parts.len() > 1 {
                    let abs_path = to_absolute_path(&fs.cwd_path, cmd_parts[1]);
                    fs.cat(&abs_path);
                }
            }
            "rm" => {
                if cmd_parts.len() > 1 {
                    let abs_path = to_absolute_path(&fs.cwd_path, cmd_parts[1]);
                    fs.rm(&abs_path);
                }
            }
            "cp" => {
                if cmd_parts.len() > 2 {
                    let src_abs = to_absolute_path(&fs.cwd_path, cmd_parts[1]);
                    let dst_abs = to_absolute_path(&fs.cwd_path, cmd_parts[2]);
                    fs.cp(&src_abs, &dst_abs);
                }
            }
            "mv" => {
                if cmd_parts.len() > 2 {
                    let src_abs = to_absolute_path(&fs.cwd_path, cmd_parts[1]);
                    let dst_abs = to_absolute_path(&fs.cwd_path, cmd_parts[2]);
                    fs.mv(&src_abs, &dst_abs);
                }
            }
            "exit" => break,
            _ => println!("Unknown command"),
        }
    }
}
