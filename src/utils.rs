use colored::Colorize;
use std::{borrow::Cow, collections::HashMap};

#[derive(Debug, Clone)]
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
            let name = parts.next().unwrap_or("").trim().to_string();
            let color = parts.next().map(|c| c.trim().to_string());

            if !name.is_empty() {
                out.push(Token::Var { name, color });
            }
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
                } else {
                    out.push_str(&format!("{{{}}}", name));
                }
            }
        }
    }

    out
}

fn apply_color(value: &str, style: Option<&str>) -> String {
    let style = match style {
        Some(s) => s,
        None => return value.to_string(),
    };

    let mut styled = value.normal();

    for s in style.split(',') {
        styled = match s.trim() {
            "red" => styled.red(),
            "green" => styled.green(),
            "yellow" => styled.yellow(),
            "blue" => styled.blue(),
            "magenta" => styled.magenta(),
            "cyan" => styled.cyan(),
            "white" => styled.white(),
            "bold" => styled.bold(),
            "dim" | "dimmed" => styled.dimmed(),
            "underline" => styled.underline(),
            "italic" => styled.italic(),
            "blink" => styled.blink(),
            _ => styled,
        };
    }

    styled.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let tokens = parse_template("Hello {name}!");
        assert_eq!(tokens.len(), 3);
    }

    #[test]
    fn test_parse_with_color() {
        let tokens = parse_template("{name:red,bold}");
        match &tokens[0] {
            Token::Var { name, color } => {
                assert_eq!(name, "name");
                assert_eq!(color.as_deref(), Some("red,bold"));
            }
            _ => panic!("Expected Var token"),
        }
    }

    #[test]
    fn test_render() {
        let tokens = parse_template("Hello {name}!");
        let mut vars = HashMap::new();
        vars.insert("name", Cow::Borrowed("World"));

        let result = render(&tokens, &vars);
        assert_eq!(result, "Hello World!");
    }
}
