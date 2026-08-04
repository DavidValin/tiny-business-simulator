// Product simulator core: file collection, result computation, result-file
// writing, and the multi-product share split.
//
// All user-facing strings are pulled from [`crate::lang`] and rendered in the
// language selected at startup via `--lang`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::lang::{self, Lang};
use crate::parser::{Currency, ProductDefinition, TimeUnit};

const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

pub(crate) fn print_error(msg: &str) {
    eprintln!("{}{}{}", RED, msg, RESET);
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
// Result file (write)
// ---------------------------------------------------------------------------

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

/// Write a per-product `*.simulation_results.txt` with a **12-month breakdown**.
///
/// Writes one row per month (Jan..Dec) using [`Dict::result_month_row`], followed
/// by the annual goal / time (annual = sum of the 12 months).  Each month's goal,
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

/// Path of the aggregate totals file written in `folder`:
/// `totals.simulation_results.txt`. Excluded from [`collect_txt_files`] by the
/// `.simulation_results.txt` suffix rule, so it is never treated as a product
/// definition on the next run.
fn totals_file_path(folder: &Path) -> PathBuf {
    folder.join("totals.simulation_results.txt")
}

/// Write the aggregate `totals.simulation_results.txt` with a **12-month
/// breakdown**: one row per month (Jan..Dec) using [`Dict::total_month_row`],
/// followed by the annual total row.
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

/// Month abbreviations used in the per-month export rows. Delegates to the
/// language's localized month names.
pub(crate) fn months_abbr(lang: &Lang) -> [&'static str; 12] {
    lang.months_abbr()
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


#[cfg(test)]
#[path = "../test/simulator_tests.rs"]
mod tests;
