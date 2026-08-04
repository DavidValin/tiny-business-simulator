// Product definition parser

use crate::lang::{self, Lang};

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

/// A currency code (e.g. USD, EUR, GBP, JPY).  Stored as 3 ASCII bytes so the
/// type stays `Copy` and no heap allocation or unsafe transmute is needed.
/// Any 3-letter uppercase ISO 4217-style code is accepted, covering every
/// world currency without a hardcoded list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Currency([u8; 3]);

impl Currency {
    /// Create a `Currency` from a known-valid 3-letter code at compile time.
    /// Panics at runtime if the code is not 3 uppercase ASCII letters.
    #[allow(dead_code)]
    pub const fn new(code: &'static str) -> Currency {
        let bytes = code.as_bytes();
        assert!(
            bytes.len() == 3
                && bytes[0] >= b'A' && bytes[0] <= b'Z'
                && bytes[1] >= b'A' && bytes[1] <= b'Z'
                && bytes[2] >= b'A' && bytes[2] <= b'Z'
        );
        Currency([bytes[0], bytes[1], bytes[2]])
    }

    fn from_str(s: &str) -> Option<Currency> {
        // Accept any 3-letter uppercase ASCII code (ISO 4217 format).
        if s.len() == 3 && s.bytes().all(|b| b.is_ascii_uppercase()) {
            let bytes = s.as_bytes();
            Some(Currency([bytes[0], bytes[1], bytes[2]]))
        } else {
            None
        }
    }
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}",
            self.0[0] as char, self.0[1] as char, self.0[2] as char
        )
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


#[cfg(test)]
#[path = "../test/parser_tests.rs"]
mod tests;
