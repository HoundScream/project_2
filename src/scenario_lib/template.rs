use std::collections::BTreeMap;
use uuid::Uuid;

/// Holds scenario-level variables (from the scenario file's `vars` block,
/// values captured from earlier steps, and lazily-generated `random.*`
/// values) and resolves `{{token}}` placeholders against them.
#[derive(Debug, Default)]
pub struct TemplateContext {
    values: BTreeMap<String, String>,
}

impl TemplateContext {
    pub fn new(initial_vars: BTreeMap<String, String>) -> Self {
        Self {
            values: initial_vars,
        }
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    /// Replaces every `{{token}}` occurrence in `input`. Tokens starting
    /// with `random.` are generated once on first use and memoized so the
    /// same token yields the same value for the rest of the scenario run.
    /// Any other token must already exist (from `vars` or a prior `capture`)
    /// or rendering fails with a descriptive error.
    pub fn render(&mut self, input: &str) -> Result<String, String> {
        let mut output = String::with_capacity(input.len());
        let mut rest = input;

        while let Some(start) = rest.find("{{") {
            output.push_str(&rest[..start]);
            let after_open = &rest[start + 2..];
            let Some(end) = after_open.find("}}") else {
                return Err(format!(
                    "unterminated template placeholder in `{input}` (missing `}}}}`)"
                ));
            };
            let token = after_open[..end].trim();
            output.push_str(&self.resolve_token(token)?);
            rest = &after_open[end + 2..];
        }
        output.push_str(rest);
        Ok(output)
    }

    fn resolve_token(&mut self, token: &str) -> Result<String, String> {
        if let Some(existing) = self.values.get(token) {
            return Ok(existing.clone());
        }

        if let Some(kind) = token.strip_prefix("random.") {
            let value = generate_random(kind);
            self.values.insert(token.to_string(), value.clone());
            return Ok(value);
        }

        Err(format!(
            "unknown template variable `{{{{{token}}}}}` (not in scenario vars, not captured yet, and not a `random.*` generator)"
        ))
    }
}

fn generate_random(kind: &str) -> String {
    let unique = Uuid::new_v4().simple().to_string();
    let short = &unique[..10];
    match kind {
        "full_name" => format!("Сценарий Тест {short}"),
        "email" => format!("scenario-{short}@example.com"),
        "phone" => format!(
            "+7999{}",
            &unique[..7]
                .chars()
                .map(|c| c.to_ascii_lowercase())
                .map(|c| {
                    // fold hex digits into decimal-looking digits for a plausible phone number
                    (c as u8 % 10).to_string()
                })
                .collect::<String>()
        ),
        "telegram" => format!("@scenario_{short}"),
        "password" => format!("Scenario{short}!"),
        "uuid" => Uuid::new_v4().to_string(),
        other => panic!("no random generator registered for `random.{other}`"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_plain_vars() {
        let mut ctx = TemplateContext::new(BTreeMap::from([(
            "card_url".to_string(),
            "https://x".to_string(),
        )]));
        assert_eq!(ctx.render("go to {{card_url}}").unwrap(), "go to https://x");
    }

    #[test]
    fn random_values_are_memoized() {
        let mut ctx = TemplateContext::new(BTreeMap::new());
        let a = ctx.render("{{random.email}}").unwrap();
        let b = ctx.render("{{random.email}}").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn missing_var_is_an_error() {
        let mut ctx = TemplateContext::new(BTreeMap::new());
        assert!(ctx.render("{{nope}}").is_err());
    }
}
