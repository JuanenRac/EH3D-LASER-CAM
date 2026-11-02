// EH3D-LASER-CAM - command line arguments.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//
// The arguments of the command line, the same way in every small utility of the suite: this file is a template that is copied, and only its first line (the name of the utility) differs
// between the copies. The mode is strict: an option that is written wrong or a word too many is an error, never something to ignore.
#![allow(dead_code)]

/// The value of the option `name` (`--name VALUE`), taken out of `args`; `None` when the option is not there. An option without a value is an error.
pub fn flag(args: &mut Vec<String>, name: &str) -> Result<Option<String>, String> {
    if let Some(i) = args.iter().position(|a| a == name) {
        let v = args
            .get(i + 1)
            .ok_or_else(|| format!("{name} needs a value"))?
            .clone();
        args.drain(i..=i + 1);
        return Ok(Some(v));
    }
    Ok(None)
}

/// Whether the switch `name` (an option with no value) is there; it is taken out of `args`.
pub fn switch(args: &mut Vec<String>, name: &str) -> bool {
    if let Some(i) = args.iter().position(|a| a == name) {
        args.remove(i);
        true
    } else {
        false
    }
}

/// A finite number, or an error that names what it is for.
pub fn num(text: &str, what: &str) -> Result<f64, String> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{what} '{text}' is not a number"))
}

/// What is left of the arguments once the options and the wanted words are taken must be nothing more than `max` words: an option that is written wrong or an extra word is an
/// error, not something to ignore (the program would then make its output with the default values).
pub fn finish(args: &[String], max: usize) -> Result<(), String> {
    if let Some(o) = args.iter().find(|a| a.starts_with("--")) {
        return Err(format!("{o}: unknown option (see help)"));
    }
    if args.len() > max {
        return Err(format!("{}: unexpected argument (see help)", args[max]));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn flag_takes_the_option_and_its_value() {
        let mut a = v(&["x", "--feed", "300", "y"]);
        assert_eq!(flag(&mut a, "--feed").unwrap(), Some("300".to_string()));
        assert_eq!(a, v(&["x", "y"]));
        assert_eq!(flag(&mut a, "--feed").unwrap(), None);
    }

    #[test]
    fn flag_without_a_value_is_an_error() {
        let mut a = v(&["x", "--feed"]);
        assert!(flag(&mut a, "--feed")
            .unwrap_err()
            .contains("needs a value"));
    }

    #[test]
    fn switch_is_taken_out_once() {
        let mut a = v(&["--force", "x"]);
        assert!(switch(&mut a, "--force"));
        assert!(!switch(&mut a, "--force"));
        assert_eq!(a, v(&["x"]));
    }

    #[test]
    fn num_wants_a_finite_number() {
        assert_eq!(num(" 2.5 ", "n").unwrap(), 2.5);
        assert!(num("abc", "n").is_err());
        assert!(num("NaN", "n").is_err());
        assert!(num("inf", "n").is_err());
    }

    #[test]
    fn finish_refuses_an_unknown_option_and_an_extra_word() {
        assert!(finish(&v(&["a", "b"]), 2).is_ok());
        assert!(finish(&v(&["a", "--oops"]), 2)
            .unwrap_err()
            .contains("unknown option"));
        assert!(finish(&v(&["a", "b", "c"]), 2)
            .unwrap_err()
            .contains("unexpected argument"));
    }
}
