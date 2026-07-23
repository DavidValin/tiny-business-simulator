// Product definition parser

use crate::lang::{self, Lang};

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Currency {
    USD,
    EUR,
    CAD,
}

impl Currency {
    fn from_str(s: &str) -> Option<Currency> {
        match s {
            "USD" => Some(Currency::USD),
            "EUR" => Some(Currency::EUR),
            "CAD" => Some(Currency::CAD),
            _ => None,
        }
    }
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Currency::USD => write!(f, "USD"),
            Currency::EUR => write!(f, "EUR"),
            Currency::CAD => write!(f, "CAD"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimeUnit {
    Mins,
    Hours,
}

impl TimeUnit {
    fn from_str(s: &str) -> Option<TimeUnit> {
        match s {
            "mins" => Some(TimeUnit::Mins),
            "hours" => Some(TimeUnit::Hours),
            _ => None,
        }
    }

    /// Localized human-readable label for this unit.
    pub fn label(&self, lang: &Lang) -> &'static str {
        match self {
            TimeUnit::Mins => lang.dict().time_mins,
            TimeUnit::Hours => lang.dict().time_hours,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cost {
    pub price: f64,
    pub currency: Currency,
    pub description: String,
}

impl Cost {
    /// Localized, human-readable single-line representation.
    pub fn display(&self, lang: &Lang) -> String {
        lang::fmt(
            lang.dict().cost_display,
            &[&format!("{}", self.price), &self.currency.to_string(), &self.description],
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProductDefinition {
    pub name: String,
    pub sale_price: f64,
    pub sale_currency: Currency,
    pub production_time: f64,
    pub production_time_unit: TimeUnit,
    pub costs: Vec<Cost>,
}

impl ProductDefinition {
    /// Localized, human-readable multi-line representation.
    pub fn display(&self, lang: &Lang) -> String {
        let d = lang.dict();
        let mut s = String::new();
        s.push_str(&lang::fmt(d.product_label, &[&self.name]));
        s.push('\n');
        s.push_str(&lang::fmt(
            d.sale_price_label,
            &[&format!("{}", self.sale_price), &self.sale_currency.to_string()],
        ));
        s.push('\n');
        s.push_str(&lang::fmt(
            d.production_time_label,
            &[&format!("{}", self.production_time), self.production_time_unit.label(lang)],
        ));
        s.push('\n');
        s.push_str(&lang::fmt(d.costs_header, &[&format!("{}", self.costs.len())]));
        s.push('\n');
        if self.costs.is_empty() {
            s.push_str(d.no_costs);
            s.push('\n');
        } else {
            for c in &self.costs {
                s.push_str(&c.display(lang));
                s.push('\n');
            }
        }
        s
    }
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Parse the full textual content of a product definition file.
///
/// Returns `Ok(ProductDefinition)` when the content is syntactically valid,
/// or `Err(Vec<String>)` containing one human-readable error message per
/// problem found (one error per line as required). All messages are rendered
/// in `lang`.
pub fn parse_content(content: &str, lang: &Lang) -> Result<ProductDefinition, Vec<String>> {
    let d = lang.dict();
    let mut errors: Vec<String> = Vec::new();
    let mut product: Option<ProductDefinition> = None;
    let mut costs: Vec<Cost> = Vec::new();
    let mut saw_product_line = false;

    for (idx, raw_line) in content.lines().enumerate() {
        let lineno = (idx + 1).to_string();
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('+') {
            saw_product_line = true;
            if product.is_some() {
                errors.push(lang::fmt(d.err_duplicate_product_line, &[&lineno]));
                continue;
            }
            match parse_product_line(line, lang) {
                Ok(p) => product = Some(p),
                Err(e) => errors.push(lang::fmt(d.err_line_prefix, &[&lineno, &e])),
            }
        } else if line.starts_with('-') {
            match parse_cost_line(line, lang) {
                Ok(c) => costs.push(c),
                Err(e) => errors.push(lang::fmt(d.err_line_prefix, &[&lineno, &e])),
            }
        } else {
            errors.push(lang::fmt(d.err_unexpected_line, &[&lineno]));
        }
    }

    if !saw_product_line {
        errors.push(d.err_missing_product_line.to_string());
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut p = product.expect("product must exist when there are no errors");
    p.costs = costs;
    Ok(p)
}

/// Parse a product header line of the form:
///   `+ <name> : <sale_price> <sale_currency> : <production_time> <time_unit>`
fn parse_product_line(line: &str, lang: &Lang) -> Result<ProductDefinition, String> {
    let d = lang.dict();
    let parts: Vec<&str> = line.split(':').collect();
    if parts.len() != 3 {
        return Err(lang::fmt(
            d.err_product_sections,
            &[&format!("{}", parts.len())],
        ));
    }

    // Section 0: "+ <name>"
    let s0 = parts[0].trim();
    if !s0.starts_with('+') {
        return Err(d.err_product_plus.to_string());
    }
    let name = s0[1..].trim();
    if name.is_empty() {
        return Err(d.err_empty_product_name.to_string());
    }

    // Section 1: "<sale_price> <sale_currency>"
    let s1 = parts[1].trim();
    let s1_tokens: Vec<&str> = s1.split_whitespace().collect();
    if s1_tokens.len() != 2 {
        return Err(lang::fmt(
            d.err_sale_section,
            &[&format!("{}", s1_tokens.len())],
        ));
    }
    let sale_price: f64 = s1_tokens[0]
        .parse()
        .map_err(|_| lang::fmt(d.err_invalid_sale_price, &[s1_tokens[0]]))?;
    let sale_currency = Currency::from_str(s1_tokens[1]).ok_or_else(|| {
        lang::fmt(d.err_invalid_sale_currency, &[s1_tokens[1]])
    })?;

    // Section 2: "<production_time> <time_unit>"
    let s2 = parts[2].trim();
    let s2_tokens: Vec<&str> = s2.split_whitespace().collect();
    if s2_tokens.len() != 2 {
        return Err(lang::fmt(
            d.err_time_section,
            &[&format!("{}", s2_tokens.len())],
        ));
    }
    let production_time: f64 = s2_tokens[0]
        .parse()
        .map_err(|_| lang::fmt(d.err_invalid_prod_time, &[s2_tokens[0]]))?;
    let production_time_unit = TimeUnit::from_str(s2_tokens[1]).ok_or_else(|| {
        lang::fmt(d.err_invalid_time_unit, &[s2_tokens[1]])
    })?;

    Ok(ProductDefinition {
        name: name.to_string(),
        sale_price,
        sale_currency,
        production_time,
        production_time_unit,
        costs: Vec::new(),
    })
}

/// Parse a cost line of the form:
///   `- <cost_price> <cost_currency> <cost_description>`
fn parse_cost_line(line: &str, lang: &Lang) -> Result<Cost, String> {
    let d = lang.dict();
    let trimmed = line.trim();
    if !trimmed.starts_with('-') {
        return Err(d.err_cost_minus.to_string());
    }
    let rest = trimmed[1..].trim();
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.len() < 3 {
        return Err(lang::fmt(
            d.err_cost_tokens,
            &[&format!("{}", tokens.len())],
        ));
    }
    let price: f64 = tokens[0]
        .parse()
        .map_err(|_| lang::fmt(d.err_invalid_cost_price, &[tokens[0]]))?;
    let currency = Currency::from_str(tokens[1])
        .ok_or_else(|| lang::fmt(d.err_invalid_cost_currency, &[tokens[1]]))?;
    let description = tokens[2..].join(" ");

    Ok(Cost {
        price,
        currency,
        description,
    })
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};

    const EN: Lang = Lang::En;

    fn test_data_path(name: &str) -> PathBuf {
        let candidates = [
            format!("test/test_data/{}", name),
            format!("../test/test_data/{}", name),
            format!("src/../test/test_data/{}", name),
        ];
        for c in &candidates {
            if Path::new(c).exists() {
                return PathBuf::from(c);
            }
        }
        PathBuf::from(&candidates[0])
    }

    /// Load the canonical valid product fixture. Every invalid-case test below
    /// starts from this content and overrides a single piece of it to inject
    /// the specific error scenario it exercises.
    fn valid_content() -> String {
        let path = test_data_path("valid_product.txt");
        fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {}", path.display(), e))
    }

    /// Parse `content` expecting failure and return the joined error messages.
    fn parse_errs(content: &str) -> String {
        let errors = parse_content(content, &EN).expect_err("should fail");
        assert!(!errors.is_empty(), "expected at least one error");
        errors.join("\n")
    }

    #[test]
    fn parses_a_valid_product_with_multiple_costs() {
        let content = "+ Cerveza : 2.7 USD : 0.2 mins\n\
                       - 0.27 USD labor humana\n\
                       - 0.32 USD cerveza\n\
                       - 0.10 USD gasto agua limpieza\n";
        let product = parse_content(content, &EN).expect("should parse");
        assert_eq!(product.name, "Cerveza");
        assert!((product.sale_price - 2.7).abs() < 1e-9);
        assert_eq!(product.sale_currency, Currency::USD);
        assert!((product.production_time - 0.2).abs() < 1e-9);
        assert_eq!(product.production_time_unit, TimeUnit::Mins);
        assert_eq!(product.costs.len(), 3);
        assert!((product.costs[0].price - 0.27).abs() < 1e-9);
        assert_eq!(product.costs[0].currency, Currency::USD);
        assert_eq!(product.costs[0].description, "labor humana");
        assert_eq!(product.costs[2].description, "gasto agua limpieza");
    }

    #[test]
    fn parses_product_with_usd_and_hours() {
        let content = "+ Widget : 10 USD : 1.5 hours\n  - 3 USD parts\n";
        let product = parse_content(content, &EN).expect("should parse");
        assert_eq!(product.sale_currency, Currency::USD);
        assert_eq!(product.production_time_unit, TimeUnit::Hours);
        assert_eq!(product.costs.len(), 1);
        assert_eq!(product.costs[0].currency, Currency::USD);
    }

    #[test]
    fn parses_product_with_cad_and_no_costs() {
        let content = "+ Maple : 5 CAD : 30 mins\n";
        let product = parse_content(content, &EN).expect("should parse");
        assert_eq!(product.sale_currency, Currency::CAD);
        assert!(product.costs.is_empty());
    }

    #[test]
    fn ignores_blank_lines() {
        let content = "\n+ Beer : 2 USD : 1 mins\n\n  - 1 USD hops\n\n";
        let product = parse_content(content, &EN).expect("should parse");
        assert_eq!(product.name, "Beer");
        assert_eq!(product.costs.len(), 1);
    }

    #[test]
    fn rejects_invalid_sale_currency() {
        let content = valid_content().replace("2.7 USD : 0.2 mins", "2.7 GBP : 0.2 mins");
        let errors = parse_errs(&content);
        assert!(errors.contains("invalid sale currency") && errors.contains("GBP"), "{}", errors);
    }

    #[test]
    fn rejects_invalid_cost_currency() {
        let content = valid_content().replace("0.27 USD labor", "0.27 GBP labor");
        let errors = parse_errs(&content);
        assert!(errors.contains("invalid cost currency") && errors.contains("GBP"), "{}", errors);
    }

    #[test]
    fn rejects_invalid_time_unit() {
        let content = valid_content().replace("0.2 mins", "0.2 seconds");
        let errors = parse_errs(&content);
        assert!(errors.contains("invalid production time unit") && errors.contains("seconds"), "{}", errors);
    }

    #[test]
    fn rejects_non_numeric_sale_price() {
        let content = valid_content().replace("2.7 USD", "cheap USD");
        let errors = parse_errs(&content);
        assert!(errors.contains("invalid sale price"), "{}", errors);
    }

    #[test]
    fn rejects_non_numeric_cost_price() {
        let content = valid_content().replace("0.27 USD labor", "cheap USD labor");
        let errors = parse_errs(&content);
        assert!(errors.contains("invalid cost price"), "{}", errors);
    }

    #[test]
    fn rejects_non_numeric_production_time() {
        let content = valid_content().replace("0.2 mins", "fast mins");
        let errors = parse_errs(&content);
        assert!(errors.contains("invalid production time"), "{}", errors);
    }

    #[test]
    fn rejects_line_with_bad_prefix() {
        let content = valid_content().replace("  - 0.27 USD labor", "  * 0.27 USD labor");
        let errors = parse_errs(&content);
        assert!(errors.contains("unexpected line"), "{}", errors);
    }

    #[test]
    fn rejects_product_line_without_plus_prefix() {
        let content = valid_content().replace("+ Beer : 2.7 USD : 0.2 mins", "Beer : 2.7 USD : 0.2 mins");
        let errors = parse_errs(&content);
        assert!(errors.contains("unexpected line"), "{}", errors);
        assert!(errors.contains("missing product definition"), "{}", errors);
    }

    #[test]
    fn rejects_cost_line_without_minus_prefix() {
        let content = valid_content().replace("  - 0.27 USD labor", "  0.27 USD labor");
        let errors = parse_errs(&content);
        assert!(errors.contains("unexpected line"), "{}", errors);
    }

    #[test]
    fn rejects_missing_product_line() {
        let content = valid_content().replace("+ Beer : 2.7 USD : 0.2 mins\n", "");
        let errors = parse_errs(&content);
        assert!(errors.contains("missing product definition"), "{}", errors);
    }

    #[test]
    fn rejects_empty_product_name() {
        let content = valid_content().replace("+ Beer : 2.7 USD : 0.2 mins", "+ : 2.7 USD : 0.2 mins");
        let errors = parse_errs(&content);
        assert!(errors.contains("product name is empty"), "{}", errors);
    }

    #[test]
    fn rejects_cost_line_with_too_few_tokens() {
        let content = valid_content().replace("  - 0.27 USD labor", "  - 0.27 USD");
        let errors = parse_errs(&content);
        assert!(errors.contains("cost line must be"), "{}", errors);
    }

    #[test]
    fn rejects_product_line_with_wrong_section_count() {
        let content = valid_content().replace(": 0.2 mins", " 0.2 mins");
        let errors = parse_errs(&content);
        assert!(errors.contains("colon-separated sections"), "{}", errors);
    }

    #[test]
    fn rejects_sale_section_with_wrong_token_count() {
        let content = valid_content().replace("2.7 USD", "2.7USD");
        let errors = parse_errs(&content);
        assert!(errors.contains("sale section"), "{}", errors);
    }

    #[test]
    fn rejects_time_section_with_wrong_token_count() {
        let content = valid_content().replace("0.2 mins", "0.2mins");
        let errors = parse_errs(&content);
        assert!(errors.contains("production-time section"), "{}", errors);
    }

    #[test]
    fn rejects_duplicate_product_line() {
        let content = format!("{}\n+ Ale : 3 USD : 1 mins", valid_content());
        let errors = parse_errs(&content);
        assert!(errors.contains("duplicate product definition"), "{}", errors);
    }

    #[test]
    fn parses_valid_test_data_file() {
        let path = test_data_path("valid_product.txt");
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {}", path.display(), e));
        let product = parse_content(&content, &EN).expect("valid_product.txt should parse");
        assert_eq!(product.name, "Beer");
        assert_eq!(product.sale_currency, Currency::USD);
        assert!(product.costs.len() >= 2);
    }

    #[test]
    fn display_uses_selected_language() {
        let content = "+ Beer : 2 USD : 1 mins\n  - 1 USD hops\n";
        let product = parse_content(content, &EN).expect("should parse");
        let en = product.display(&Lang::En);
        assert!(en.contains("Product: Beer"), "en was: {}", en);
        assert!(en.contains("Sale price:"), "en was: {}", en);
        assert!(en.contains("Costs (1):"), "en was: {}", en);

        let es = product.display(&Lang::Es);
        assert!(es.contains("Producto: Beer"), "es was: {}", es);
        assert!(es.contains("Precio de venta:"), "es was: {}", es);

        // The localized time-unit label follows the chosen language.
        assert_eq!(product.production_time_unit.label(&Lang::En), "mins");
        assert_eq!(product.production_time_unit.label(&Lang::Es), "minutos");
        assert_eq!(product.production_time_unit.label(&Lang::Zh), "分钟");
    }
}
