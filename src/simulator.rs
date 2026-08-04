// Interactive product simulator.
//
// Ports the three functions from `simulator.js`:
//   - `show_interactive_menu`  (was showInteractiveMenu)
//   - `ask_goal_and_calculate` (was askGoalAndCalculate)
//   - `get_result_summary`     (was getResultSummary)
//
// Unlike the JS version, parsing is delegated to `parser::parse_content`
// (which already validates currencies USD/USD/CAD and time units mins/hours),
// and every amount is shown in the product's own sale currency instead of a
// hard-coded USD.
//
// All user-facing strings are pulled from [`crate::lang`] and rendered in the
// language selected at startup via `--lang`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use dialoguer::{Input, MultiSelect};
use rand::Rng;

use crate::lang::{self, Lang};
use crate::parser::{parse_content, Currency, ProductDefinition, TimeUnit};

const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";
const YELLOW: &str = "\x1b[33m";

pub(crate) fn print_error(msg: &str) {
    eprintln!("{}{}{}", RED, msg, RESET);
}

fn print_warn(msg: &str) {
    eprintln!("{}{}{}", YELLOW, msg, RESET);
}

// ---------------------------------------------------------------------------
// File helpers
// ---------------------------------------------------------------------------

/// List `.txt` product-definition files in `path`, excluding generated
/// `*.simulation_results.txt` files, sorted for stable ordering.
pub(crate) fn collect_txt_files(path: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.ends_with(".txt") && !name.ends_with(".simulation_results.txt") {
                files.push(p);
            }
        }
    }
    files.sort();
    files
}

/// Given `foo.txt`, return `foo.simulation_results.txt` (alongside the original).
fn result_file_path(file_path: &Path) -> PathBuf {
    let mut p = file_path.to_path_buf();
    p.set_extension("simulation_results.txt");
    p
}

// ---------------------------------------------------------------------------
// Computed product result
// ---------------------------------------------------------------------------

/// Derived figures for a parsed product, equivalent to the JS `result` object.
#[derive(Clone)]
pub struct ProductResult {
    pub name: String,
    pub price: f64,
    pub currency: Currency,
    pub total_cost: f64,
    pub net_profit: f64,
    pub profit_percent: f64,
    pub duration_minutes: f64,
}

/// Compute price, total cost, net profit, profit margin (%) and production
/// duration in minutes (converting `hours` -> minutes) from a parsed product.
pub fn compute_result(product: &ProductDefinition) -> ProductResult {
    let price = product.sale_price;
    let total_cost: f64 = product.costs.iter().map(|c| c.price).sum();
    let net_profit = price - total_cost;
    let profit_percent = if price.abs() > f64::EPSILON {
        (net_profit / price) * 100.0
    } else {
        0.0
    };
    let duration_minutes = match product.production_time_unit {
        TimeUnit::Mins => product.production_time,
        TimeUnit::Hours => product.production_time * 60.0,
    };
    ProductResult {
        name: product.name.clone(),
        price,
        currency: product.sale_currency,
        total_cost,
        net_profit,
        profit_percent,
        duration_minutes,
    }
}

// ---------------------------------------------------------------------------
// Result file (write + read back)
// ---------------------------------------------------------------------------

/// Slice of a template up to its first `{` placeholder; used as a stable
/// per-language anchor when parsing a result file back. Trailing whitespace
/// (spaces/tabs) is trimmed so the anchor matches regardless of how the
/// label was padded when the file was written. Multibyte-safe: `{` is ASCII
/// and never part of a UTF-8 continuation byte.
fn anchor(template: &str) -> &str {
    match template.find('{') {
        Some(i) => template[..i].trim_end(),
        None => template.trim_end(),
    }
}

/// One formatted "time line":
///   `<minutes> minutes (<hours> hours) (<parallel> parallel products in
///    <workday_hours> workday hours) (<workdays> workdays)`
fn time_line(template: &str, minutes: f64, parallel: i64, workday_hours: i64, label_width: usize) -> String {
    let hours = minutes / 60.0;
    let workdays = hours / (workday_hours as f64 * parallel.max(1) as f64);
    lang::fmt_aligned(
        template,
        &[
            &format!("{:.2}", minutes),
            &format!("{:.2}", hours),
            &parallel.to_string(),
            &workday_hours.to_string(),
            &format!("{:.2}", workdays),
        ],
        label_width,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn write_result_file(
    file_path: &Path,
    result: &ProductResult,
    monthly_goal: f64,
    annual_goal: f64,
    monthly_sales: i64,
    annual_sales: i64,
    workday_hours: i64,
    parallel_products: i64,
    lang: &Lang,
) -> io::Result<()> {
    let out_path = result_file_path(file_path);
    let d = lang.dict();
    let cur = result.currency;
    let total_monthly_minutes = result.duration_minutes * monthly_sales as f64;
    let total_annual_minutes = result.duration_minutes * annual_sales as f64;

    let cur_s = cur.to_string();
    let stats_templates = [
        d.result_product,
        d.result_sale_price,
        d.result_total_cost,
        d.result_net_profit_unit,
        d.result_profit_margin,
        d.result_prod_time,
    ];
    let stats_rows: Vec<Vec<String>> = vec![
        vec![result.name.clone()],
        vec![format!("{:.2}", result.price), cur_s.clone()],
        vec![format!("{:.2}", result.total_cost), cur_s.clone()],
        vec![format!("{:.2}", result.net_profit), cur_s.clone()],
        vec![format!("{:.2}", result.profit_percent)],
        vec![format!("{:.2}", result.duration_minutes)],
    ];
    let all_templates = [
        d.result_product,
        d.result_sale_price,
        d.result_total_cost,
        d.result_net_profit_unit,
        d.result_profit_margin,
        d.result_prod_time,
        d.result_monthly_goal,
        d.result_monthly_time,
        d.result_annual_goal,
        d.result_annual_time,
        d.result_workday,
        d.result_parallel,
    ];
    let label_w = all_templates
        .iter()
        .map(|t| lang::str_width(lang::prefix_before_value(t)))
        .max()
        .unwrap_or(0)
        + 2;

    let mut lines: Vec<String> = Vec::with_capacity(stats_templates.len() + 8);
    for (t, r) in stats_templates.iter().zip(stats_rows.iter()) {
        let refs: Vec<&str> = r.iter().map(String::as_str).collect();
        lines.push(lang::fmt_aligned(t, &refs, label_w));
    }
    lines.push(String::new());
    lines.push(lang::fmt_aligned(
        d.result_monthly_goal,
        &[&format!("{:.2}", monthly_goal), &cur_s, &monthly_sales.to_string()],
        label_w,
    ));
    lines.push(time_line(d.result_monthly_time, total_monthly_minutes, parallel_products, workday_hours, label_w));
    lines.push(String::new());
    lines.push(lang::fmt_aligned(
        d.result_annual_goal,
        &[&format!("{:.2}", annual_goal), &cur_s, &annual_sales.to_string()],
        label_w,
    ));
    lines.push(time_line(d.result_annual_time, total_annual_minutes, parallel_products, workday_hours, label_w));
    lines.push(String::new());
    lines.push(lang::fmt_aligned(d.result_workday, &[&workday_hours.to_string()], label_w));
    lines.push(lang::fmt_aligned(d.result_parallel, &[&parallel_products.to_string()], label_w));

    let mut output = lines.join("\n");
    output.push('\n');
    fs::write(out_path, output)
}

/// Path of the aggregate totals file written in `folder`:
/// `totals.simulation_results.txt`. Excluded from [`collect_txt_files`] by the
/// `.simulation_results.txt` suffix rule, so it is never treated as a product
/// definition on the next run.
fn totals_file_path(folder: &Path) -> PathBuf {
    folder.join("totals.simulation_results.txt")
}

/// Write the aggregate totals file `totals.simulation_results.txt` in `folder`.
///
/// Mirrors the on-screen totals block from the multi-product run: the totals
/// header, the total monthly and annual sales rows (each carrying the shared
/// parallel/workday/workdays breakdown), plus the workday and parallel settings
/// shared by every product in the run. Sales counts and minutes are
/// currency-free.
#[allow(clippy::too_many_arguments)]
pub(crate) fn write_totals_file(
    folder: &Path,
    num_products: usize,
    total_monthly_sales: i64,
    total_annual_sales: i64,
    total_monthly_minutes: f64,
    total_annual_minutes: f64,
    workday_hours: i64,
    parallel_products: i64,
    lang: &Lang,
) -> io::Result<()> {
    let d = lang.dict();
    let out_path = totals_file_path(folder);

    // Label width across every line we write, so the value columns of the
    // workday/parallel lines align with the totals rows.
    let label_w = [d.total_monthly_sales, d.total_annual_sales, d.result_workday, d.result_parallel]
        .iter()
        .map(|t| lang::str_width(lang::prefix_before_value(t)))
        .max()
        .unwrap_or(0)
        + 2;

    let rows: Vec<(&'static str, Vec<String>)> = vec![
        (
            d.total_monthly_sales,
            vec![
                total_monthly_sales.to_string(),
                format!("{:.2}", total_monthly_minutes),
                format!("{:.2}", total_monthly_minutes / 60.0),
                parallel_products.to_string(),
                workday_hours.to_string(),
                format!(
                    "{:.2}",
                    (total_monthly_minutes / 60.0) / (workday_hours as f64 * parallel_products.max(1) as f64)
                ),
            ],
        ),
        (
            d.total_annual_sales,
            vec![
                total_annual_sales.to_string(),
                format!("{:.2}", total_annual_minutes),
                format!("{:.2}", total_annual_minutes / 60.0),
                parallel_products.to_string(),
                workday_hours.to_string(),
                format!(
                    "{:.2}",
                    (total_annual_minutes / 60.0) / (workday_hours as f64 * parallel_products.max(1) as f64)
                ),
            ],
        ),
    ];
    let totals_lines = lang::fmt_block(&rows, label_w);

    let mut output = String::new();
    output.push_str(&lang::fmt(d.totals_header, &[&num_products.to_string()]));
    output.push('\n');
    for l in &totals_lines {
        output.push_str(l);
        output.push('\n');
    }
    output.push('\n');
    output.push_str(&lang::fmt_aligned(
        d.result_workday,
        &[&workday_hours.to_string()],
        label_w,
    ));
    output.push('\n');
    output.push_str(&lang::fmt_aligned(
        d.result_parallel,
        &[&parallel_products.to_string()],
        label_w,
    ));
    output.push('\n');
    fs::write(out_path, output)
}

/// Month abbreviations used in the per-month export rows. Delegates to the
/// language's localized month names.
pub(crate) fn months_abbr(lang: &Lang) -> [&'static str; 12] {
    lang.months_abbr()
}

/// Write a per-product `*.simulation_results.txt` with a **12-month breakdown**.
///
/// Like [`write_result_file`] but, instead of a single "monthly" row, writes one
/// row per month (Jan..Dec) using [`Dict::result_month_row`], followed by the
/// annual goal / time (annual = sum of the 12 months).  Each month's goal,
/// sales and minutes are provided as 12-element arrays.
#[allow(clippy::too_many_arguments)]
pub(crate) fn write_result_file_monthly(
    file_path: &Path,
    result: &ProductResult,
    monthly_goals: &[f64; 12],
    monthly_sales: &[i64; 12],
    monthly_minutes: &[f64; 12],
    annual_goal: f64,
    annual_sales: i64,
    annual_minutes: f64,
    workday_hours: i64,
    parallel_products: i64,
    lang: &Lang,
) -> io::Result<()> {
    let out_path = result_file_path(file_path);
    let d = lang.dict();
    let cur = result.currency;
    let cur_s = cur.to_string();

    // Stats block (same six rows as the single-month variant).
    let stats_templates = [
        d.result_product,
        d.result_sale_price,
        d.result_total_cost,
        d.result_net_profit_unit,
        d.result_profit_margin,
        d.result_prod_time,
    ];
    let stats_rows: Vec<Vec<String>> = vec![
        vec![result.name.clone()],
        vec![format!("{:.2}", result.price), cur_s.clone()],
        vec![format!("{:.2}", result.total_cost), cur_s.clone()],
        vec![format!("{:.2}", result.net_profit), cur_s.clone()],
        vec![format!("{:.2}", result.profit_percent)],
        vec![format!("{:.2}", result.duration_minutes)],
    ];
    let all_templates = [
        d.result_product,
        d.result_sale_price,
        d.result_total_cost,
        d.result_net_profit_unit,
        d.result_profit_margin,
        d.result_prod_time,
        d.result_annual_goal,
        d.result_annual_time,
        d.result_workday,
        d.result_parallel,
    ];
    let label_w = all_templates
        .iter()
        .map(|t| lang::str_width(lang::prefix_before_value(t)))
        .max()
        .unwrap_or(0)
        + 2;

    let mut lines: Vec<String> = Vec::new();
    for (t, r) in stats_templates.iter().zip(stats_rows.iter()) {
        let refs: Vec<&str> = r.iter().map(String::as_str).collect();
        lines.push(lang::fmt_aligned(t, &refs, label_w));
    }
    lines.push(String::new());

    // 12 monthly rows.
    for m in 0..12 {
        let hours = monthly_minutes[m] / 60.0;
        let prefix = format!("  📆 {}:", months_abbr(lang)[m]);
        lines.push(lang::fmt_prefixed(
            d.result_month_row,
            &prefix,
            &[
                &format!("{:.2}", monthly_goals[m]),
                &cur_s,
                &monthly_sales[m].to_string(),
                &format!("{:.2}", monthly_minutes[m]),
                &format!("{:.2}", hours),
            ],
            label_w,
        ));
    }
    lines.push(String::new());

    // Annual goal + time (annual = sum of the 12 months).
    lines.push(lang::fmt_aligned(
        d.result_annual_goal,
        &[&format!("{:.2}", annual_goal), &cur_s, &annual_sales.to_string()],
        label_w,
    ));
    lines.push(time_line(
        d.result_annual_time,
        annual_minutes,
        parallel_products,
        workday_hours,
        label_w,
    ));
    lines.push(String::new());
    lines.push(lang::fmt_aligned(d.result_workday, &[&workday_hours.to_string()], label_w));
    lines.push(lang::fmt_aligned(
        d.result_parallel,
        &[&parallel_products.to_string()],
        label_w,
    ));

    let mut output = lines.join("\n");
    output.push('\n');
    fs::write(out_path, output)
}

/// Write the aggregate `totals.simulation_results.txt` with a **12-month
/// breakdown**: one row per month (Jan..Dec) using [`Dict::total_month_row`],
/// followed by the annual total row.  Mirrors [`write_totals_file`] but for
/// the per-month model.
#[allow(clippy::too_many_arguments)]
pub(crate) fn write_totals_file_monthly(
    folder: &Path,
    num_products: usize,
    monthly_sales: &[i64; 12],
    monthly_minutes: &[f64; 12],
    total_annual_sales: i64,
    total_annual_minutes: f64,
    workday_hours: i64,
    parallel_products: i64,
    lang: &Lang,
) -> io::Result<()> {
    let d = lang.dict();
    let out_path = totals_file_path(folder);

    let label_w = [d.total_annual_sales, d.result_workday, d.result_parallel]
        .iter()
        .map(|t| lang::str_width(lang::prefix_before_value(t)))
        .max()
        .unwrap_or(0)
        + 2;

    // Compute column widths across the 12 month rows for right-alignment.
    let month_vals: Vec<[String; 3]> = (0..12)
        .map(|m| {
            [
                monthly_sales[m].to_string(),
                format!("{:.2}", monthly_minutes[m]),
                format!("{:.2}", monthly_minutes[m] / 60.0),
            ]
        })
        .collect();
    let cw = [
        month_vals.iter().map(|r| lang::str_width(&r[0])).max().unwrap_or(0),
        month_vals.iter().map(|r| lang::str_width(&r[1])).max().unwrap_or(0),
        month_vals.iter().map(|r| lang::str_width(&r[2])).max().unwrap_or(0),
    ];

    let mut output = String::new();
    output.push_str(&lang::fmt(d.totals_header, &[&num_products.to_string()]));
    output.push('\n');

    // 12 monthly rows with the month abbreviation right next to the 📆 emoji.
    for m in 0..12 {
        let prefix = format!("  📆 {}:", months_abbr(lang)[m]);
        let padded: Vec<String> = (0..3)
            .map(|i| lang::pad_left(&month_vals[m][i], cw[i]))
            .collect();
        let refs: Vec<&str> = padded.iter().map(String::as_str).collect();
        output.push_str(&lang::fmt_prefixed(
            d.total_month_row,
            &prefix,
            &refs,
            label_w,
        ));
        output.push('\n');
    }

    // Annual total row.
    let annual_row: Vec<String> = vec![
        total_annual_sales.to_string(),
        format!("{:.2}", total_annual_minutes),
        format!("{:.2}", total_annual_minutes / 60.0),
        parallel_products.to_string(),
        workday_hours.to_string(),
        format!(
            "{:.2}",
            (total_annual_minutes / 60.0) / (workday_hours as f64 * parallel_products.max(1) as f64)
        ),
    ];
    let annual_refs: Vec<&str> = annual_row.iter().map(String::as_str).collect();
    output.push_str(&lang::fmt_aligned(
        d.total_annual_sales,
        &annual_refs,
        label_w,
    ));
    output.push('\n');
    output.push_str(&lang::fmt_aligned(
        d.result_workday,
        &[&workday_hours.to_string()],
        label_w,
    ));
    output.push('\n');
    output.push_str(&lang::fmt_aligned(
        d.result_parallel,
        &[&parallel_products.to_string()],
        label_w,
    ));
    output.push('\n');
    fs::write(out_path, output)
}

/// Read the `*.simulation_results.txt` file for `file_path` and return a one-line
/// summary string, or `None` if no result file exists. The summary is rendered
/// in `lang` and the file is parsed using the same language's anchors.
#[allow(dead_code)]
pub fn get_result_summary(file_path: &Path, lang: &Lang) -> Option<String> {
    let d = lang.dict();
    let result_file = result_file_path(file_path);
    let content = fs::read_to_string(&result_file).ok()?;

    let sale_price_anchor = anchor(d.result_sale_price);
    let net_profit_anchor = anchor(d.result_net_profit_unit);
    let margin_anchor = anchor(d.result_profit_margin);
    let monthly_goal_anchor = anchor(d.result_monthly_goal);
    let monthly_time_anchor = anchor(d.result_monthly_time);
    let annual_goal_anchor = anchor(d.result_annual_goal);
    let annual_time_anchor = anchor(d.result_annual_time);
    let sales_needle = d.required_sales_needle;

    let mut beneficio: Option<f64> = None;
    let mut margen: Option<f64> = None;
    let mut mensual_sales: Option<i64> = None;
    let mut tiempo_mensual: Option<f64> = None;
    let mut anual_sales: Option<i64> = None;
    let mut tiempo_anual: Option<f64> = None;
    let mut currency: String = String::new();

    for line in content.lines() {
        if line.contains(sale_price_anchor) && currency.is_empty() {
            for tok in line.split_whitespace() {
                // Any 3-letter uppercase token is a currency code.
                if tok.len() == 3 && tok.bytes().all(|b| b.is_ascii_uppercase()) {
                    currency = tok.to_string();
                    break;
                }
            }
        }
        if line.contains(net_profit_anchor) {
            beneficio = first_number(line);
        } else if line.contains(margin_anchor) {
            margen = first_number(line);
        } else if line.contains(monthly_goal_anchor) {
            mensual_sales = number_after(line, sales_needle).map(|x| x as i64);
        } else if line.contains(monthly_time_anchor) {
            tiempo_mensual = first_number(line);
        } else if line.contains(annual_goal_anchor) {
            anual_sales = number_after(line, sales_needle).map(|x| x as i64);
        } else if line.contains(annual_time_anchor) {
            tiempo_anual = first_number(line);
        }
    }

    let cur = if currency.is_empty() { "USD" } else { &currency };
    let mut resumen = String::new();
    if let Some(b) = beneficio {
        resumen.push_str(&lang::fmt(d.summary_net_profit, &[&format!("{:.2}", b), cur]));
    }
    if let Some(m) = margen {
        resumen.push_str(&lang::fmt(d.summary_margin, &[&format!("{:.2}", m)]));
    }
    if let (Some(ms), Some(tm)) = (mensual_sales, tiempo_mensual) {
        resumen.push_str(&lang::fmt(
            d.summary_monthly,
            &[&ms.to_string(), &(tm.round() as i64).to_string()],
        ));
    }
    if let (Some(an), Some(ta)) = (anual_sales, tiempo_anual) {
        resumen.push_str(&lang::fmt(
            d.summary_annual,
            &[&an.to_string(), &((ta / 60.0).round() as i64).to_string()],
        ));
    }

    if resumen.is_empty() {
        None
    } else {
        Some(resumen)
    }
}

// ---------------------------------------------------------------------------
// Workday / parallel bounds (30-day monthly and 365-day annual caps)
// ---------------------------------------------------------------------------

/// Upper cap on monthly workdays.
const MAX_MONTHLY_WORKDAYS: f64 = 30.0;
/// Upper cap on annual workdays.
const MAX_ANNUAL_WORKDAYS: f64 = 365.0;
/// Upper cap on hours per workday.
const MAX_WORKDAY_HOURS: i64 = 24;

/// Bounds on `parallel_products` given the chosen `workday_hours`, so the
/// production fits within 30 workdays/month and 365/year (workdays = hours /
/// (parallel × workday_hours)). `min` is the throughput that hits the binding
/// cap (30 or 365 workdays); `max` brings that same period down to 1 workday.
/// Always `min ≤ max` and `min ≥ 1`.
pub(crate) fn parallel_range(monthly_minutes: f64, annual_minutes: f64, workday_hours: i64) -> (i64, i64) {
    let wh = workday_hours.max(1) as f64;
    let monthly_hours = monthly_minutes / 60.0;
    let annual_hours = annual_minutes / 60.0;
    let monthly_p_cap = monthly_hours / (MAX_MONTHLY_WORKDAYS * wh);
    let annual_p_cap = annual_hours / (MAX_ANNUAL_WORKDAYS * wh);
    let (min_p, max_p) = if monthly_p_cap >= annual_p_cap {
        (monthly_p_cap, monthly_hours / wh)
    } else {
        (annual_p_cap, annual_hours / wh)
    };
    let min_parallel = (min_p.ceil() as i64).max(1);
    let max_parallel = (max_p.floor() as i64).max(min_parallel);
    (min_parallel, max_parallel)
}

// ---------------------------------------------------------------------------
// Interactive goal prompt
// ---------------------------------------------------------------------------

/// Ask the user for monthly and annual net-profit goals, compute the required
/// sales and production time, print them, and persist a `*.simulation_results.txt`.
pub fn ask_goal_and_calculate(result: &ProductResult, file_path: &Path, lang: &Lang) {
    let d = lang.dict();
    if result.net_profit <= 0.0 {
        print_warn(&lang::fmt(
            d.warn_net_profit_nonpositive,
            &[&format!("{:.2}", result.net_profit), &result.currency.to_string()],
        ));
        return;
    }
    let cur = result.currency.to_string();

    let monthly_goal: f64 = match Input::new()
        .with_prompt(lang::fmt(d.prompt_monthly_goal, &[&cur]))
        .interact()
    {
        Ok(v) => v,
        Err(e) => {
            print_error(&lang::fmt(d.err_read_monthly_goal, &[&e.to_string()]));
            return;
        }
    };
    let monthly_sales = ((monthly_goal / result.net_profit).ceil() as i64).max(0);
    let total_monthly_minutes = result.duration_minutes * monthly_sales as f64;

    let annual_goal: f64 = match Input::new()
        .with_prompt(lang::fmt(d.prompt_annual_goal, &[&cur]))
        .interact()
    {
        Ok(v) => v,
        Err(e) => {
            print_error(&lang::fmt(d.err_read_annual_goal, &[&e.to_string()]));
            return;
        }
    };
    let annual_sales = ((annual_goal / result.net_profit).ceil() as i64).max(0);
    let total_annual_minutes = result.duration_minutes * annual_sales as f64;

    let wh_lo = 1i64.to_string();
    let wh_hi = MAX_WORKDAY_HOURS.to_string();
    let workday_hours: i64 = match Input::new()
        .with_prompt(lang::fmt(d.prompt_workday_hours, &[&wh_lo, &wh_hi]))
        .validate_with(|v: &i64| {
            if (1..=MAX_WORKDAY_HOURS).contains(v) {
                Ok::<(), String>(())
            } else {
                Err(lang::fmt(d.validate_workday_range, &[&wh_lo, &wh_hi]))
            }
        })
        .interact()
    {
        Ok(v) => v,
        Err(e) => {
            print_error(&lang::fmt(d.err_read_workday, &[&e.to_string()]));
            return;
        }
    };

    let (p_min, p_max) = parallel_range(total_monthly_minutes, total_annual_minutes, workday_hours);
    let parallel_products: i64 = match Input::new()
        .with_prompt(lang::fmt(d.prompt_parallel_products, &[&p_min.to_string(), &p_max.to_string()]))
        .validate_with(|v: &i64| {
            if (p_min..=p_max).contains(v) {
                Ok::<(), String>(())
            } else {
                Err(lang::fmt(d.validate_parallel_range, &[&p_min.to_string(), &p_max.to_string()]))
            }
        })
        .interact()
    {
        Ok(v) => v,
        Err(e) => {
            print_error(&lang::fmt(d.err_read_parallel, &[&e.to_string()]));
            return;
        }
    };

    println!();
    println!("{}", d.sales_needed_header);
    let stdout_templates = [d.monthly_label, d.result_monthly_time, d.annual_label, d.result_annual_time];
    let stdout_w = stdout_templates
        .iter()
        .map(|t| lang::str_width(lang::prefix_before_value(t)))
        .max()
        .unwrap_or(0)
        + 2;
    println!(
        "{}",
        lang::fmt_aligned(
            d.monthly_label,
            &[&format!("{:.2}", monthly_goal), &cur, &monthly_sales.to_string()],
            stdout_w,
        )
    );
    println!(
        "{}",
        time_line(d.result_monthly_time, total_monthly_minutes, parallel_products, workday_hours, stdout_w)
    );
    println!(
        "{}",
        lang::fmt_aligned(
            d.annual_label,
            &[&format!("{:.2}", annual_goal), &cur, &annual_sales.to_string()],
            stdout_w,
        )
    );
    println!(
        "{}",
        time_line(d.result_annual_time, total_annual_minutes, parallel_products, workday_hours, stdout_w)
    );

    if let Err(e) = write_result_file(
        file_path,
        result,
        monthly_goal,
        annual_goal,
        monthly_sales,
        annual_sales,
        workday_hours,
        parallel_products,
        lang,
    ) {
        print_error(&lang::fmt(d.err_write_result_file, &[&e.to_string()]));
    }
}

// ---------------------------------------------------------------------------
// Multi-product goal split
// ---------------------------------------------------------------------------

/// Per-product figures computed from a share of a net-profit goal.
pub(crate) struct ProductShare {
    /// Raw random roll in the 1..=70 range (shown to the user).
    pub(crate) raw_percent: i64,
    /// Normalized fraction of the goal this product is responsible for
    /// (the raw rolls are scaled so all shares sum to 1.0).
    pub(crate) share: f64,
    pub(crate) monthly_goal: f64,
    pub(crate) annual_goal: f64,
    pub(crate) monthly_sales: i64,
    pub(crate) annual_sales: i64,
    pub(crate) monthly_minutes: f64,
    pub(crate) annual_minutes: f64,
}

/// Pure computation behind the multi-product split: given each product's raw
/// random roll (1..=70) and the user's net-profit goals, return the normalized
/// share and derived sales / production time for each product. Exposed for
/// unit testing.
///
/// `raw_pcts` must be non-empty and the same length as `results`. Callers are
/// expected to have already filtered out products with non-positive
/// `net_profit`; a defensive `0` is produced for any such product anyway.
pub(crate) fn compute_product_shares(
    results: &[&ProductResult],
    raw_pcts: &[i64],
    monthly_goal: f64,
    annual_goal: f64,
) -> Vec<ProductShare> {
    assert_eq!(
        results.len(),
        raw_pcts.len(),
        "results and raw_pcts must have the same length"
    );
    let total_raw: i64 = raw_pcts.iter().sum();
    let total_raw_f = total_raw as f64;
    results
        .iter()
        .zip(raw_pcts.iter())
        .map(|(r, raw)| {
            let share = *raw as f64 / total_raw_f;
            let monthly_goal_i = share * monthly_goal;
            let annual_goal_i = share * annual_goal;
            let monthly_sales = if r.net_profit > 0.0 {
                ((monthly_goal_i / r.net_profit).ceil() as i64).max(0)
            } else {
                0
            };
            let annual_sales = if r.net_profit > 0.0 {
                ((annual_goal_i / r.net_profit).ceil() as i64).max(0)
            } else {
                0
            };
            let monthly_minutes = r.duration_minutes * monthly_sales as f64;
            let annual_minutes = r.duration_minutes * annual_sales as f64;
            ProductShare {
                raw_percent: *raw,
                share,
                monthly_goal: monthly_goal_i,
                annual_goal: annual_goal_i,
                monthly_sales,
                annual_sales,
                monthly_minutes,
                annual_minutes,
            }
        })
        .collect()
}

/// Multi-product variant of [`ask_goal_and_calculate`].
///
/// Each product is assigned a random percentage between 1% and 70%. The rolls
/// are normalized so they sum to 100%, and the user's single net-profit goal is
/// split across products according to those normalized shares. Currencies are
/// ignored: the goal is one raw number applied to every product regardless of
/// its sale currency. One `*.simulation_results.txt` is written next to each selected
/// product's source file, containing that product's share of the goal, and a
/// single `totals.simulation_results.txt` is written in `folder` recording the
/// aggregate totals across every selected product.
pub fn ask_goal_and_calculate_multi(folder: &Path, items: Vec<(PathBuf, ProductResult)>, lang: &Lang) {
    let d = lang.dict();
    // Drop products with non-positive net profit: they cannot service a
    // net-profit goal (the division would be undefined / negative).
    let valid: Vec<(PathBuf, ProductResult)> = items
        .into_iter()
        .filter(|(_, r)| {
            if r.net_profit <= 0.0 {
                print_warn(&lang::fmt(
                    d.warn_product_excluded,
                    &[&r.name, &format!("{:.2}", r.net_profit), &r.currency.to_string()],
                ));
                false
            } else {
                true
            }
        })
        .collect();
    if valid.is_empty() {
        print_warn(d.warn_no_positive_products);
        return;
    }

    // Random 1..=70 per product, then normalize to fractions summing to 1.0.
    let mut rng = rand::thread_rng();
    let raw_pcts: Vec<i64> = (0..valid.len()).map(|_| rng.gen_range(1..=70)).collect();
    let total_raw: i64 = raw_pcts.iter().sum();
    let total_raw_f = total_raw as f64;

    // Pad product names to a common column so the 🎲 dice column lines up
    // regardless of how long each product's name is.
    let name_w = valid
        .iter()
        .map(|(_, r)| lang::str_width(&r.name))
        .max()
        .unwrap_or(0);

    println!();
    println!("{}", d.random_split_header);
    for ((_, r), raw) in valid.iter().zip(&raw_pcts) {
        let pname = lang::pad_to(&r.name, name_w);
        println!(
            "{}",
            lang::fmt(
                d.split_item,
                &[
                    &pname,
                    &raw.to_string(),
                    &format!("{:.2}", (*raw as f64 / total_raw_f) * 100.0),
                ],
            )
        );
    }
    println!();

    // A single goal, applied to every product as a raw number (currencies
    // ignored per the multi-product mode).
    let monthly_goal: f64 = match Input::new()
        .with_prompt(d.prompt_monthly_goal_plain.to_string())
        .interact()
    {
        Ok(v) => v,
        Err(e) => {
            print_error(&lang::fmt(d.err_read_monthly_goal, &[&e.to_string()]));
            return;
        }
    };
    let annual_goal: f64 = match Input::new()
        .with_prompt(d.prompt_annual_goal_plain.to_string())
        .interact()
    {
        Ok(v) => v,
        Err(e) => {
            print_error(&lang::fmt(d.err_read_annual_goal, &[&e.to_string()]));
            return;
        }
    };

    // Compute each product's share (pure helper, unit-tested) up front so the
    // workday/parallel bounds can be derived from the required totals.
    let results_refs: Vec<&ProductResult> = valid.iter().map(|(_, r)| r).collect();
    let shares = compute_product_shares(&results_refs, &raw_pcts, monthly_goal, annual_goal);
    let total_monthly_sales: i64 = shares.iter().map(|s| s.monthly_sales).sum();
    let total_annual_sales: i64 = shares.iter().map(|s| s.annual_sales).sum();
    let total_monthly_minutes: f64 = shares.iter().map(|s| s.monthly_minutes).sum();
    let total_annual_minutes: f64 = shares.iter().map(|s| s.annual_minutes).sum();

    let wh_lo = 1i64.to_string();
    let wh_hi = MAX_WORKDAY_HOURS.to_string();
    let workday_hours: i64 = match Input::new()
        .with_prompt(lang::fmt(d.prompt_workday_hours, &[&wh_lo, &wh_hi]))
        .validate_with(|v: &i64| {
            if (1..=MAX_WORKDAY_HOURS).contains(v) {
                Ok::<(), String>(())
            } else {
                Err(lang::fmt(d.validate_workday_range, &[&wh_lo, &wh_hi]))
            }
        })
        .interact()
    {
        Ok(v) => v,
        Err(e) => {
            print_error(&lang::fmt(d.err_read_workday, &[&e.to_string()]));
            return;
        }
    };

    let (p_min, p_max) = parallel_range(total_monthly_minutes, total_annual_minutes, workday_hours);
    let parallel_products: i64 = match Input::new()
        .with_prompt(lang::fmt(d.prompt_parallel_products, &[&p_min.to_string(), &p_max.to_string()]))
        .validate_with(|v: &i64| {
            if (p_min..=p_max).contains(v) {
                Ok::<(), String>(())
            } else {
                Err(lang::fmt(d.validate_parallel_range, &[&p_min.to_string(), &p_max.to_string()]))
            }
        })
        .interact()
    {
        Ok(v) => v,
        Err(e) => {
            print_error(&lang::fmt(d.err_read_parallel, &[&e.to_string()]));
            return;
        }
    };

    // Per-product breakdown.
    // Collect all Monthly/Annual rows first so `fmt_block` can right-align
    // each numeric column to a common width across every row in the block.
    let per_prod_w = [d.per_product_monthly, d.per_product_annual]
        .iter()
        .map(|t| lang::str_width(lang::prefix_before_value(t)))
        .max()
        .unwrap_or(0)
        + 2;
    let mut per_rows: Vec<(&'static str, Vec<String>)> = Vec::with_capacity(valid.len() * 2);
    for ((_, r), s) in valid.iter().zip(&shares) {
        per_rows.push((
            d.per_product_monthly,
            vec![
                format!("{:.2}", s.monthly_goal),
                r.currency.to_string(),
                s.monthly_sales.to_string(),
                format!("{:.2}", s.monthly_minutes),
                format!("{:.2}", s.monthly_minutes / 60.0),
            ],
        ));
        per_rows.push((
            d.per_product_annual,
            vec![
                format!("{:.2}", s.annual_goal),
                r.currency.to_string(),
                s.annual_sales.to_string(),
                format!("{:.2}", s.annual_minutes),
                format!("{:.2}", s.annual_minutes / 60.0),
            ],
        ));
    }
    let per_lines = lang::fmt_block(&per_rows, per_prod_w);

    println!();
    println!("{}", d.per_product_header);
    let mut li = 0;
    for ((_, r), s) in valid.iter().zip(&shares) {
        let pname = lang::pad_to(&r.name, name_w);
        println!(
            "{}",
            lang::fmt(
                d.split_item,
                &[&pname, &s.raw_percent.to_string(), &format!("{:.2}", s.share * 100.0)],
            )
        );
        println!("{}", per_lines[li]);
        li += 1;
        println!("{}", per_lines[li]);
        li += 1;
    }

    // Totals across all products (sales counts and minutes are currency-free).
    let totals_w = [d.total_monthly_sales, d.total_annual_sales]
        .iter()
        .map(|t| lang::str_width(lang::prefix_before_value(t)))
        .max()
        .unwrap_or(0)
        + 2;
    let totals_rows: Vec<(&'static str, Vec<String>)> = vec![
        (
            d.total_monthly_sales,
            vec![
                total_monthly_sales.to_string(),
                format!("{:.2}", total_monthly_minutes),
                format!("{:.2}", total_monthly_minutes / 60.0),
                parallel_products.to_string(),
                workday_hours.to_string(),
                format!("{:.2}", (total_monthly_minutes / 60.0) / (workday_hours as f64 * parallel_products.max(1) as f64)),
            ],
        ),
        (
            d.total_annual_sales,
            vec![
                total_annual_sales.to_string(),
                format!("{:.2}", total_annual_minutes),
                format!("{:.2}", total_annual_minutes / 60.0),
                parallel_products.to_string(),
                workday_hours.to_string(),
                format!("{:.2}", (total_annual_minutes / 60.0) / (workday_hours as f64 * parallel_products.max(1) as f64)),
            ],
        ),
    ];
    println!();
    println!("{}", lang::fmt(d.totals_header, &[&valid.len().to_string()]));
    for line in lang::fmt_block(&totals_rows, totals_w) {
        println!("{}", line);
    }

    // One resultado file per product, containing that product's share.
    for ((file, r), s) in valid.iter().zip(&shares) {
        if let Err(e) = write_result_file(
            file,
            r,
            s.monthly_goal,
            s.annual_goal,
            s.monthly_sales,
            s.annual_sales,
            workday_hours,
            parallel_products,
            lang,
        ) {
            print_error(&lang::fmt(d.err_write_result_file_for, &[&r.name, &e.to_string()]));
        }
    }

    // Aggregate totals across every selected product.
    if let Err(e) = write_totals_file(
        folder,
        valid.len(),
        total_monthly_sales,
        total_annual_sales,
        total_monthly_minutes,
        total_annual_minutes,
        workday_hours,
        parallel_products,
        lang,
    ) {
        print_error(&lang::fmt(d.err_write_totals_file, &[&e.to_string()]));
    }
}

// ---------------------------------------------------------------------------
// Interactive menu
// ---------------------------------------------------------------------------

/// Print the computed stats for a single product (price, cost, net profit,
/// margin, production time). Shared by the single- and multi-product flows.
/// All six lines are space-padded to a common label column so the values
/// align regardless of label length or terminal tab width.
fn print_product_stats(result: &ProductResult, lang: &Lang) {
    let d = lang.dict();
    let cur = result.currency.to_string();
    let templates = [
        d.result_product,
        d.result_sale_price,
        d.result_total_cost,
        d.result_net_profit_unit,
        d.result_profit_margin,
        d.result_prod_time,
    ];
    let rows: Vec<Vec<String>> = vec![
        vec![result.name.clone()],
        vec![format!("{:.2}", result.price), cur.clone()],
        vec![format!("{:.2}", result.total_cost), cur.clone()],
        vec![format!("{:.2}", result.net_profit), cur.clone()],
        vec![format!("{:.2}", result.profit_percent)],
        vec![format!("{:.2}", result.duration_minutes)],
    ];
    let label_w = templates
        .iter()
        .map(|t| lang::str_width(lang::prefix_before_value(t)))
        .max()
        .unwrap_or(0)
        + 2;
    for (t, r) in templates.iter().zip(rows.iter()) {
        let refs: Vec<&str> = r.iter().map(String::as_str).collect();
        println!("{}", lang::fmt_aligned(t, &refs, label_w));
    }
    println!();
}

/// Parse every selected file into a `(file, ProductResult)` pair, printing the
/// stats for each. Files that fail to read or parse are reported and skipped.
/// Returns `None` if no product could be parsed.
fn parse_selected(
    txt_files: &[PathBuf],
    indices: &[usize],
    lang: &Lang,
) -> Option<Vec<(PathBuf, ProductResult)>> {
    let d = lang.dict();
    let mut items: Vec<(PathBuf, ProductResult)> = Vec::new();
    for &idx in indices {
        let file = &txt_files[idx];
        let content = match fs::read_to_string(file) {
            Ok(c) => c,
            Err(e) => {
                print_error(&lang::fmt(
                    d.err_read_file,
                    &[&file.display().to_string(), &e.to_string()],
                ));
                continue;
            }
        };
        let product = match parse_content(&content, lang) {
            Ok(p) => p,
            Err(errors) => {
                for err in &errors {
                    print_error(&lang::fmt(d.file_colon_msg, &[&file.display().to_string(), err]));
                }
                continue;
            }
        };
        let result = compute_result(&product);
        println!();
        print_product_stats(&result, lang);
        items.push((file.clone(), result));
    }
    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

/// List the `.txt` definitions in `folder`, let the user pick one or several
/// with an interactive checkbox menu, show each product's computed stats, then
/// ask for goals. A single selection keeps the original 100%-of-goal flow;
/// multiple selections trigger a random-percentage split (see
/// [`ask_goal_and_calculate_multi`]).
pub fn show_interactive_menu(folder: &Path, lang: &Lang) {
    let d = lang.dict();
    let txt_files = collect_txt_files(folder);
    if txt_files.is_empty() {
        print_warn(&lang::fmt(d.warn_no_txt_files, &[&folder.display().to_string()]));
        return;
    }

    let labels: Vec<String> = txt_files
        .iter()
        .map(|p| {
            p.file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        })
        .collect();

    let selection = match MultiSelect::new()
        .with_prompt(d.menu_prompt.to_string())
        .items(&labels)
        .interact()
    {
        Ok(indices) => indices,
        Err(e) => {
            print_error(&lang::fmt(d.err_menu, &[&e.to_string()]));
            return;
        }
    };

    if selection.is_empty() {
        print_warn(d.warn_no_selection);
        return;
    }

    let items = match parse_selected(&txt_files, &selection, lang) {
        Some(items) => items,
        None => return,
    };

    if items.len() == 1 {
        let (file, result) = &items[0];
        ask_goal_and_calculate(result, file, lang);
    } else {
        ask_goal_and_calculate_multi(folder, items, lang);
    }
}

// ---------------------------------------------------------------------------
// Small parsing helpers (for get_result_summary)
// ---------------------------------------------------------------------------

/// First parseable number (run of digits / at most one dot) in `s`.
#[allow(dead_code)]
fn first_number(s: &str) -> Option<f64> {
    let mut iter = s.char_indices().peekable();
    while let Some(&(start, c)) = iter.peek() {
        if c.is_ascii_digit() || c == '.' {
            let mut end = start;
            let mut has_digit = false;
            while let Some(&(i, ch)) = iter.peek() {
                if ch.is_ascii_digit() {
                    has_digit = true;
                    end = i + ch.len_utf8();
                    iter.next();
                } else if ch == '.' {
                    end = i + 1;
                    iter.next();
                } else {
                    break;
                }
            }
            if has_digit {
                if let Ok(v) = s[start..end].parse::<f64>() {
                    return Some(v);
                }
            }
        } else {
            iter.next();
        }
    }
    None
}

/// First number appearing after `needle` in `s`.
fn number_after(s: &str, needle: &str) -> Option<f64> {
    let idx = s.find(needle)?;
    first_number(&s[idx + needle.len()..])
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use crate::parser::Cost;

    const EN: Lang = Lang::En;

    #[test]
    fn result_file_path_appends_resultado() {
        let p = Path::new("test/test_data/foo.txt");
        let r = result_file_path(p);
        assert_eq!(r, Path::new("test/test_data/foo.simulation_results.txt"));
    }

    #[test]
    fn first_number_handles_basic_cases() {
        assert_eq!(first_number("abc 3.35 USD"), Some(3.35));
        assert_eq!(first_number("no numbers here"), None);
        assert_eq!(first_number("100%"), Some(100.0));
        assert_eq!(first_number("x .5 y"), Some(0.5));
    }

    #[test]
    fn number_after_finds_following_number() {
        assert_eq!(number_after("Required sales: 498", "Required sales:"), Some(498.0));
        assert_eq!(number_after("Monthly goal: 100.00 USD → Required sales: 30", "Required sales:"), Some(30.0));
        assert_eq!(number_after("nothing here", "Required sales:"), None);
    }

    #[test]
    fn compute_result_sums_costs_and_converts_hours() {
        let product = ProductDefinition {
            name: "X".into(),
            sale_price: 10.0,
            sale_currency: Currency::new("USD"),
            production_time: 1.5,
            production_time_unit: TimeUnit::Hours,
            costs: vec![
                Cost { price: 2.0, currency: Currency::new("USD"), description: "a".into() },
                Cost { price: 3.0, currency: Currency::new("USD"), description: "b".into() },
            ],
        };
        let r = compute_result(&product);
        assert!((r.total_cost - 5.0).abs() < 1e-9);
        assert!((r.net_profit - 5.0).abs() < 1e-9);
        assert!((r.profit_percent - 50.0).abs() < 1e-9);
        assert!((r.duration_minutes - 90.0).abs() < 1e-9);
        assert_eq!(r.currency, Currency::new("USD"));
    }

    #[test]
    fn write_then_read_result_summary_roundtrip() {
        let dir = env::temp_dir().join("parse_sim_test_roundtrip");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("prod.txt");

        let product = ProductDefinition {
            name: "Coffee".into(),
            sale_price: 4.5,
            sale_currency: Currency::new("USD"),
            production_time: 5.0,
            production_time_unit: TimeUnit::Mins,
            costs: vec![Cost {
                price: 1.15,
                currency: Currency::new("USD"),
                description: "beans".into(),
            }],
        };
        let result = compute_result(&product);
        // net_profit = 4.5 - 1.15 = 3.35
        write_result_file(&file, &result, 100.0, 1200.0, 30, 358, 8, 2, &EN).unwrap();

        let summary = get_result_summary(&file, &EN).expect("summary should exist");
        assert!(summary.contains("3.35 USD"), "summary was: {}", summary);
        assert!(summary.contains("74.44%"), "summary was: {}", summary);
        assert!(summary.contains("30 sales (month)"), "summary was: {}", summary);
        assert!(summary.contains("358 sales (year)"), "summary was: {}", summary);

        // The result file should also record the workday / parallel settings.
        let written = fs::read_to_string(result_file_path(&file)).unwrap();
        assert!(
            written.contains("2 parallel products in 8 workday hours"),
            "result file missing suffix, was:\n{}",
            written
        );
        assert!(written.contains("🕐 Workday:"), "missing workday line, was:\n{}", written);
        assert!(written.contains("🧵 Parallel products:"), "missing parallel line, was:\n{}", written);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_then_read_result_summary_roundtrip_spanish() {
        // The same round-trip in Spanish must succeed using Spanish anchors.
        let dir = env::temp_dir().join("parse_sim_test_roundtrip_es");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("prod.txt");

        let product = ProductDefinition {
            name: "Coffee".into(),
            sale_price: 4.5,
            sale_currency: Currency::new("USD"),
            production_time: 5.0,
            production_time_unit: TimeUnit::Mins,
            costs: vec![Cost {
                price: 1.15,
                currency: Currency::new("USD"),
                description: "beans".into(),
            }],
        };
        let result = compute_result(&product);
        write_result_file(&file, &result, 100.0, 1200.0, 30, 358, 8, 2, &Lang::Es).unwrap();

        let summary = get_result_summary(&file, &Lang::Es).expect("summary should exist");
        assert!(summary.contains("3.35 USD"), "es summary was: {}", summary);
        assert!(summary.contains("74.44%"), "es summary was: {}", summary);
        assert!(summary.contains("30 ventas (mes)"), "es summary was: {}", summary);
        assert!(summary.contains("358 ventas (año)"), "es summary was: {}", summary);

        let written = fs::read_to_string(result_file_path(&file)).unwrap();
        assert!(written.contains("🕐 Jornada laboral:"), "es missing jornada, was:\n{}", written);
        assert!(written.contains("🧵 Productos en paralelo:"), "es missing paralelo, was:\n{}", written);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn get_result_summary_returns_none_when_no_file() {
        let dir = env::temp_dir().join("parse_sim_test_nosuch");
        let file = dir.join("does_not_exist.txt");
        assert_eq!(get_result_summary(&file, &EN), None);
    }

    #[test]
    fn write_totals_file_records_aggregate_rows() {
        let dir = env::temp_dir().join("parse_sim_test_totals");
        fs::create_dir_all(&dir).unwrap();

        // 177 monthly sales / 2120 annual sales, 1185 / 17640 minutes,
        // 8 workday hours, 3 parallel products.
        write_totals_file(&dir, 2, 177, 2120, 1185.0, 17640.0, 8, 3, &EN).unwrap();

        let written = fs::read_to_string(totals_file_path(&dir)).unwrap();
        assert!(written.contains("Totals (2 products)"), "was:\n{}", written);
        assert!(written.contains("177"), "was:\n{}", written);
        assert!(written.contains("1185.00 min"), "was:\n{}", written);
        assert!(written.contains("2120"), "was:\n{}", written);
        assert!(written.contains("17640.00 min"), "was:\n{}", written);
        assert!(written.contains("3 parallel products in 8 workday hours"), "was:\n{}", written);
        assert!(written.contains("Workday:"), "was:\n{}", written);
        assert!(written.contains("Parallel products:"), "was:\n{}", written);

        // The totals file ends with `.simulation_results.txt`, so it must be
        // ignored by collect_txt_files (never treated as a product definition).
        let collected = collect_txt_files(&dir);
        assert!(
            !collected.iter().any(|p| p == &totals_file_path(&dir)),
            "totals file should not be collected as a product, got {:?}", collected
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_totals_file_works_in_spanish() {
        let dir = env::temp_dir().join("parse_sim_test_totals_es");
        fs::create_dir_all(&dir).unwrap();

        write_totals_file(&dir, 3, 500, 6000, 900.0, 10800.0, 6, 2, &Lang::Es).unwrap();

        let written = fs::read_to_string(totals_file_path(&dir)).unwrap();
        assert!(written.contains("Totales (3 productos)"), "was:\n{}", written);
        assert!(written.contains("Jornada laboral:"), "was:\n{}", written);
        assert!(written.contains("Productos en paralelo:"), "was:\n{}", written);

        let _ = fs::remove_dir_all(&dir);
    }

    fn make_result(name: &str, net_profit: f64, duration_minutes: f64, currency: Currency) -> ProductResult {
        ProductResult {
            name: name.into(),
            price: net_profit,
            currency,
            total_cost: 0.0,
            net_profit,
            profit_percent: 100.0,
            duration_minutes,
        }
    }

    #[test]
    fn compute_product_shares_normalizes_to_100_percent() {
        let a = make_result("A", 6.0, 5.0, Currency::new("USD"));
        let b = make_result("B", 5.0, 10.0, Currency::new("USD"));
        let results: Vec<&ProductResult> = vec![&a, &b];

        // Raw rolls 70 / 30 -> normalized 0.7 / 0.3.
        let shares = compute_product_shares(&results, &[70, 30], 1000.0, 12000.0);

        assert_eq!(shares.len(), 2);
        assert_eq!(shares[0].raw_percent, 70);
        assert_eq!(shares[1].raw_percent, 30);

        let sum: f64 = shares.iter().map(|s| s.share).sum();
        assert!((sum - 1.0).abs() < 1e-9, "shares must sum to 1.0, got {}", sum);
        assert!((shares[0].share - 0.7).abs() < 1e-9);
        assert!((shares[1].share - 0.3).abs() < 1e-9);

        // Goal is split per the normalized share (currencies ignored).
        assert!((shares[0].monthly_goal - 700.0).abs() < 1e-9);
        assert!((shares[1].monthly_goal - 300.0).abs() < 1e-9);
        assert!((shares[0].annual_goal - 8400.0).abs() < 1e-9);
        assert!((shares[1].annual_goal - 3600.0).abs() < 1e-9);

        // Sales = ceil(share_goal / net_profit).
        assert_eq!(shares[0].monthly_sales, 117); // ceil(700/6)
        assert_eq!(shares[1].monthly_sales, 60);  // ceil(300/5)
        assert_eq!(shares[0].annual_sales, 1400); // ceil(8400/6)
        assert_eq!(shares[1].annual_sales, 720);  // ceil(3600/5)

        // Minutes = duration * sales.
        assert!((shares[0].monthly_minutes - 5.0 * 117.0).abs() < 1e-9);
        assert!((shares[1].monthly_minutes - 10.0 * 60.0).abs() < 1e-9);
    }

    #[test]
    fn compute_product_shares_handles_extreme_rolls_and_zero_goal() {
        let a = make_result("A", 10.0, 1.0, Currency::new("USD"));
        let b = make_result("B", 10.0, 1.0, Currency::new("USD"));
        let results: Vec<&ProductResult> = vec![&a, &b];

        // Extremes of the allowed range: 1 and 70.
        let shares = compute_product_shares(&results, &[1, 70], 710.0, 0.0);

        assert!((shares[0].share - 1.0 / 71.0).abs() < 1e-9);
        assert!((shares[1].share - 70.0 / 71.0).abs() < 1e-9);
        // 710/71 = 10 -> 1 sale; 710*70/71 = 700 -> 70 sales.
        assert_eq!(shares[0].monthly_sales, 1);
        assert_eq!(shares[1].monthly_sales, 70);
        // Annual goal of 0 -> 0 sales.
        assert_eq!(shares[0].annual_sales, 0);
        assert_eq!(shares[1].annual_sales, 0);
    }

    #[test]
    #[should_panic(expected = "same length")]
    fn compute_product_shares_panics_on_length_mismatch() {
        let a = make_result("A", 10.0, 1.0, Currency::new("USD"));
        let results: Vec<&ProductResult> = vec![&a];
        let _ = compute_product_shares(&results, &[1, 2], 100.0, 0.0);
    }

    #[test]
    fn parallel_range_zero_minutes_forces_one() {
        assert_eq!(parallel_range(0.0, 0.0, 8), (1, 1));
    }

    #[test]
    fn parallel_range_monthly_binding_cap_to_one_workday() {
        // 60 h monthly, 365 h annual, 1 h/workday: monthly binds.
        // min = 60/30 = 2 (30 workdays), max = 60/1 = 60 (1 workday).
        assert_eq!(parallel_range(60.0 * 60.0, 365.0 * 60.0, 1), (2, 60));
    }

    #[test]
    fn parallel_range_annual_binding() {
        // 10 h monthly, 200 h annual, 8 h/workday: annual binds.
        // min = ceil(200/(365*8)) = 1, max = floor(200/8) = 25.
        assert_eq!(parallel_range(10.0 * 60.0, 200.0 * 60.0, 8), (1, 25));
    }

    #[test]
    fn parallel_range_typical_multi_product_totals() {
        // 800 h monthly, 1600 h annual, 8 h/workday: monthly binds.
        // min = ceil(800/240) = 4, max = floor(800/8) = 100.
        assert_eq!(parallel_range(800.0 * 60.0, 1600.0 * 60.0, 8), (4, 100));
    }

    #[test]
    fn parallel_range_min_never_exceeds_max() {
        let (min, max) = parallel_range(100.0, 999_999.0, 3);
        assert!(min <= max && min >= 1, "got ({min}, {max})");
    }

    /// Byte offset of the value column for a `fmt_aligned` line: since the
    /// label is space-padded to `label_w`, the value always starts at byte
    /// offset `label_w` (unless the label itself was already wider, in which
    /// case no padding was added and the value starts right after the label).
    fn value_offset(line: &str, label_w: usize) -> usize {
        line.char_indices()
            .enumerate()
            .skip_while(|(i, (_, c))| *i < label_w.min(line.chars().count()) && *c == ' ')
            .next()
            .map(|(_, (bi, _))| bi)
            .unwrap_or(line.len())
    }

    #[test]
    fn print_product_stats_aligns_value_column() {
        // Two products whose labels have very different widths; their value
        // columns must start at the same byte offset.
        let r1 = make_result("Cerveza", 2.01, 0.2, Currency::new("USD"));
        let r1 = ProductResult {
            price: 2.7,
            total_cost: 0.69,
            profit_percent: 74.44,
            ..r1
        };
        let r2 = make_result("American burger", 10.88, 12.0, Currency::new("USD"));
        let r2 = ProductResult {
            price: 16.0,
            total_cost: 5.12,
            profit_percent: 68.0,
            ..r2
        };

        let cases = [&r1, &r2];
        let mut prev_off: Option<usize> = None;
        for r in &cases {
            let d = Lang::En.dict();
            let templates = [
                d.result_product,
                d.result_sale_price,
                d.result_total_cost,
                d.result_net_profit_unit,
                d.result_profit_margin,
                d.result_prod_time,
            ];
            let label_w = templates
                .iter()
                .map(|t| lang::str_width(lang::prefix_before_value(t)))
                .max()
                .unwrap_or(0)
                + 2;
            let rows: Vec<Vec<String>> = vec![
                vec![r.name.clone()],
                vec![format!("{:.2}", r.price), r.currency.to_string()],
                vec![format!("{:.2}", r.total_cost), r.currency.to_string()],
                vec![format!("{:.2}", r.net_profit), r.currency.to_string()],
                vec![format!("{:.2}", r.profit_percent)],
                vec![format!("{:.2}", r.duration_minutes)],
            ];
            let rendered: Vec<String> = templates
                .iter()
                .zip(rows.iter())
                .map(|(t, row)| {
                    let refs: Vec<&str> = row.iter().map(String::as_str).collect();
                    lang::fmt_aligned(t, &refs, label_w)
                })
                .collect();

            // No tabs anywhere — alignment is terminal-independent.
            for line in &rendered {
                assert!(!line.contains('\t'), "stats line must not contain tabs: {line:?}");
            }

            // Within one product, all six value columns must align.
            let first_off = value_offset(&rendered[0], label_w);
            for line in &rendered {
                assert_eq!(
                    value_offset(line, label_w),
                    first_off,
                    "stats value columns must align:\n{}",
                    rendered.join("\n")
                );
            }
            if let Some(po) = prev_off {
                assert_eq!(first_off, po, "value column must be identical across products");
            }
            prev_off = Some(first_off);
        }
    }

    #[test]
    fn per_product_and_totals_lines_align_value_column() {
        let d = Lang::En.dict();
        // Pad product names to the longest name's width.
        let names = ["Cerveza", "American burger"];
        let name_w = names.iter().map(|n| lang::str_width(n)).max().unwrap();
        let padded: Vec<String> = names.iter().map(|n| lang::pad_to(n, name_w)).collect();

        // split_item: the 🎲 column starts at the same offset for both rows.
        let split_lines: Vec<String> = padded
            .iter()
            .map(|p| lang::fmt(d.split_item, &[p, "5", "9.26"]))
            .collect();
        for l in &split_lines {
            assert!(!l.contains('\t'), "split_item must not contain tabs: {l:?}");
        }
        let dice_offsets: Vec<usize> = split_lines
            .iter()
            .map(|l| l.find('🎲').expect("dice emoji present"))
            .collect();
        assert_eq!(
            dice_offsets[0], dice_offsets[1],
            "🎲 column must align across product names of different lengths"
        );

        // per_product_monthly / per_product_annual: every numeric column must
        // right-align across all rows in the block (this is the case the user
        // reported: `254.24` vs `5084.75` shifted the following columns).
        let per_w = [d.per_product_monthly, d.per_product_annual]
            .iter()
            .map(|t| lang::str_width(lang::prefix_before_value(t)))
            .max()
            .unwrap_or(0)
            + 2;
        let per_rows: Vec<(&'static str, Vec<String>)> = vec![
            (d.per_product_monthly, vec!["254.24".into(), "USD".into(), "24".into(), "288.00".into(), "4.80".into()]),
            (d.per_product_annual,   vec!["5084.75".into(), "USD".into(), "468".into(), "5616.00".into(), "93.60".into()]),
            (d.per_product_monthly, vec!["4745.76".into(), "USD".into(), "493".into(), "493.00".into(), "8.22".into()]),
            (d.per_product_annual,   vec!["94915.25".into(), "USD".into(), "9846".into(), "9846.00".into(), "164.10".into()]),
        ];
        let per_lines = lang::fmt_block(&per_rows, per_w);
        assert_eq!(per_lines.len(), 4);
        for l in &per_lines {
            assert!(!l.contains('\t'), "per-product line must not contain tabs: {l:?}");
        }
        // The currency column ("USD") must start at the same byte offset in
        // every row — that only happens if the amount column ({0}) was
        // right-aligned to a common width.
        let eur_offsets: Vec<usize> = per_lines
            .iter()
            .map(|l| l.find("USD").expect("USD present"))
            .collect();
        let first = eur_offsets[0];
        for o in &eur_offsets {
            assert_eq!(*o, first, "USD column must align across all rows:\n{}", per_lines.join("\n"));
        }
        // Likewise the "sales" word must start at the same offset.
        let sales_offsets: Vec<usize> = per_lines
            .iter()
            .map(|l| l.find("sales").or_else(|| l.find("ventas")).expect("sales word present"))
            .collect();
        let first = sales_offsets[0];
        for o in &sales_offsets {
            assert_eq!(*o, first, "sales column must align across all rows:\n{}", per_lines.join("\n"));
        }

        // Totals lines: every numeric column must right-align.
        let tot_w = [d.total_monthly_sales, d.total_annual_sales]
            .iter()
            .map(|t| lang::str_width(lang::prefix_before_value(t)))
            .max()
            .unwrap_or(0)
            + 2;
        let totals_rows: Vec<(&'static str, Vec<String>)> = vec![
            (d.total_monthly_sales, vec!["517".into(), "781.00".into(), "13.02".into(), "3".into(), "10".into(), "1.30".into()]),
            (d.total_annual_sales,  vec!["10314".into(), "15462.00".into(), "257.70".into(), "3".into(), "10".into(), "25.77".into()]),
        ];
        let tot_lines = lang::fmt_block(&totals_rows, tot_w);
        for l in &tot_lines {
            assert!(!l.contains('\t'), "totals line must not contain tabs: {l:?}");
        }
        // The 🕐 emoji must start at the same offset in both totals rows.
        let clock_offsets: Vec<usize> = tot_lines
            .iter()
            .map(|l| l.find('🕐').expect("clock present"))
            .collect();
        assert_eq!(
            clock_offsets[0], clock_offsets[1],
            "🕐 column must align across totals rows:\n{}", tot_lines.join("\n")
        );
    }
}
