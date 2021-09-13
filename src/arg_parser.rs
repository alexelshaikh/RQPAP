use std::collections::HashMap;
use std::num::{ParseIntError, ParseFloatError};
use std::str::FromStr;

pub struct ArgsParser {
    mappings: HashMap<String, String>
}

impl ArgsParser {
    pub fn new() -> Self {
        ArgsParser {
            mappings: Default::default()
        }
    }

    pub fn from(args: Vec<String>) -> Self {
        let mut mappings = HashMap::new();
        for arg in args.into_iter() {
            let split = arg.split("=").collect::<Vec<_>>();
            if split.len() == 2 {
                if mappings.contains_key(split[0]) {
                    panic!("duplicate argument: {} and {}={}", arg, split[0], mappings.get(split[0]).unwrap());
                }
                mappings.insert(split[0].to_owned(), split[1].to_owned());
            }
            else {
                panic!("failed parsing argument: {}", arg);
            }
        }
        ArgsParser {
            mappings
        }
    }

    pub fn print_params(&self) {
        for (k, v) in &self.mappings {
            println!("{}={}", k, v);
        }
    }

    pub fn get_as_usize(&self, name: &str, default: usize) -> usize {
        match self.mappings.get(name) {
            Some(v) => match v.parse() {
                Ok(v) => v,
                Err(_) => default
            },
            None => default
        }
    }

    pub fn get_as_f64(&self, name: &str, default: f64) -> f64 {
        match self.mappings.get(name) {
            Some(v) => match v.parse() {
                Ok(v) => v,
                Err(_) => default
            },
            None => default
        }
    }

    pub fn get_as<T>(&self, name: &str, default: T) -> T where T: FromStr {
        match self.mappings.get(name) {
            Some(v) => match v.parse() {
                Ok(v) => v,
                Err(_) => default
            },
            None => default
        }
    }

    pub fn get_as_f32(&self, name: &str, default: f32) -> f32 {
        match self.mappings.get(name) {
            Some(v) => match v.parse() {
                Ok(v) => v,
                Err(_) => default
            },
            None => default
        }
    }

    pub fn get_as_bool(&self, name: &str, default: bool) -> bool {
        match self.mappings.get(name) {
            Some(v) => {
                return if v.eq_ignore_ascii_case("1") || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes") || v.eq_ignore_ascii_case("y") {
                    true
                }
                else {
                    false
                }
            }
            None => default
        }
    }

    pub fn get(&self, name: &str) -> String {
        self.get_or_else(name, "")
    }

    pub fn get_or_else(&self, name: &str, or_else: &str) -> String {
        match self.mappings.get(name) {
            Some(v) => String::from(v),
            None => String::from(or_else)
        }
    }
}
