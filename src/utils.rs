use colored::Colorize;
use std::{borrow::Cow, collections::HashMap};

pub enum Token {
    Text(String),
    Var { name: String, color: Option<String> },
}

pub fn parse_template(input: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '{' {
            if !buf.is_empty() {
                out.push(Token::Text(std::mem::take(&mut buf)));
            }

            let mut inner = String::new();
            while let Some(n) = chars.next() {
                if n == '}' {
                    break;
                }
                inner.push(n);
            }

            let mut parts = inner.splitn(2, ':');
            let name = parts.next().unwrap().to_string();
            let color = parts.next().map(|c| c.to_string());

            out.push(Token::Var { name, color });
        } else {
            buf.push(c);
        }
    }

    if !buf.is_empty() {
        out.push(Token::Text(buf));
    }

    out
}

pub fn render(tokens: &[Token], vars: &HashMap<&str, Cow<'_, str>>) -> String {
    let mut out = String::new();

    for token in tokens {
        match token {
            Token::Text(t) => out.push_str(t),
            Token::Var { name, color } => {
                if let Some(value) = vars.get(name.as_str()) {
                    let colored = apply_color(value, color.as_deref());
                    out.push_str(&colored);
                }
            }
        }
    }

    out
}

fn apply_color(mut value: &str, style: Option<&str>) -> String {
    if style.is_none() {
        return value.to_string();
    }

    let mut styled = value.normal();

    for s in style.unwrap().split(',') {
        styled = match s.trim() {
            "red" => styled.red(),
            "green" => styled.green(),
            "yellow" => styled.yellow(),
            "blue" => styled.blue(),
            "bold" => styled.bold(),
            "dim" => styled.dimmed(),
            "underline" => styled.underline(),
            _ => styled,
        };
    }

    styled.to_string()
}
