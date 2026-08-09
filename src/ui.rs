use std::io::{self, Write};

pub fn prompt(message: &str) -> io::Result<String> {
    print!("{message}");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

pub fn pause() {
    let _ = prompt("Press Enter to continue...");
}

pub fn narrate(message: &str) {
    println!("{message}");
    pause();
}

pub fn choose_from_list(title: &str, options: &[String], zero_label: Option<&str>) -> io::Result<Option<usize>> {
    println!("\n{title}");
    for (index, option) in options.iter().enumerate() {
        println!("  {}. {}", index + 1, option);
    }
    if let Some(label) = zero_label {
        println!("  0. {label}");
    }

    loop {
        let input = prompt("> ")?;
        match input.parse::<usize>() {
            Ok(0) if zero_label.is_some() => return Ok(None),
            Ok(choice) if choice >= 1 && choice <= options.len() => return Ok(Some(choice - 1)),
            _ => println!("Enter a valid number."),
        }
    }
}
