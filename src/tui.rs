// Realtime TUI simulator.
//
// Replaces the old dialoguer interactive menu with a full-screen ratatui
// interface driven by crossterm.  Fixed inputs (the only thing the user must
// supply) are the monthly and yearly net-profit goals; everything else is
// tweakable in realtime via on-screen sliders.
//
// Layout
// ------
//   left  : yearly bar chart, 12 months.  Each month is a group of 2 bars:
//           total sales NUMBER (units, cyan) and total sales AMOUNT
//           (currency revenue, yellow).
//   right : sidebar listing the navigable sliders:
//             - one percentage slider per product
//             - workday hours
//             - parallel products
//             - monthly net-profit goal
//             - yearly net-profit goal
//
// Keys
// -----
//   Up/Down    move focus between sidebar sliders
//   Left/Right decrement / increment the focused slider by its step
//   Ctrl+H     full-screen help overlay
//   q / Esc    quit
//
// Simulation model
// ----------------
// Each month's net-profit target is `monthly_goal`.  It is split across
// products by the normalized per-product percentages, giving a required sales
// count per product (`ceil(share * goal / net_profit)`).  Workday hours and
// parallel products define a monthly production CAPACITY in minutes:
//
//     capacity_minutes = workday_hours * WORKDAYS_PER_MONTH * 60 * parallel
//
// If the required production minutes exceed capacity, sales are scaled down so
// they fit (the goal cannot be met that month); otherwise the required sales
// stand.  This makes all three "simulation" parameters affect the chart in
// realtime: the percentages change the product mix (and thus the unit/amount
// totals and the minutes needed), while workday hours / parallel products
// change how much of that requirement can actually be produced.  The yearly
// goal is shown as a reference target next to the 12 * monthly sum.

use std::io;
use std::path::{Path, PathBuf};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyEvent};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Terminal;

use crate::lang::{self, Lang};
use crate::parser::parse_content;
use crate::simulator::{
    collect_txt_files, compute_product_shares, compute_result,
    write_result_file_monthly, write_totals_file_monthly, ProductResult,
};

/// Workdays assumed per month when deriving the production capacity in minutes.
const WORKDAYS_PER_MONTH: f64 = 22.0;

/// Localized month abbreviations for the current language.
#[allow(dead_code)]
fn months(lang: &Lang) -> [&'static str; 12] {
    lang.months_abbr()
}

// ---------------------------------------------------------------------------
// Slider model
// ---------------------------------------------------------------------------

/// Kinds of sliders shown in the sidebar.
///
/// `YearlyPercent(i)` — the product's yearly % (Full Year period). The value
/// is *derived* (the mean of the 12 monthly %) and shown as a slider; editing
/// it propagates to every month where the product isn't month-locked.
///
/// `MonthPercent(i)` — the product's % for the currently-selected month
/// (a Month period). Editing it only affects that month.
///
/// Full-Year global minimum / target sliders:
///   `MinWorkdayHours`, `MinParallel`, `MinMonthlyNetProfit`,
///   `TargetYearlyNetProfit`.
///
/// Per-month override sliders (a Month period `m`):
///   `MonthWorkdayHours(m)`, `MonthParallel(m)`, `MonthNetProfit(m)`.
#[derive(Clone, Copy, PartialEq)]
enum SliderKind {
    YearlyPercent(usize),
    MonthPercent(usize),
    MinWorkdayHours,
    MinParallel,
    MinMonthlyNetProfit,
    TargetYearlyNetProfit,
    MonthWorkdayHours(usize),
    MonthParallel(usize),
    MonthNetProfit(usize),
}

#[derive(Clone)]
struct Slider {
    kind: SliderKind,
    label: String,
    value: i64,
    min: i64,
    max: i64,
    step: i64,
    /// Suffix appended to the numeric readout (e.g. "%", " h", "").
    suffix: &'static str,
    /// For product-percentage sliders: when true this product's % is frozen and
    /// is excluded from the redistribution triggered by changing another
    /// product's %. Ignored for non-percent sliders.
    locked: bool,
}

impl Slider {
    fn clamp(&mut self) {
        if self.value < self.min {
            self.value = self.min;
        }
        if self.value > self.max {
            self.value = self.max;
        }
    }

    fn dec(&mut self) {
        self.value -= self.step;
        self.clamp();
    }

    fn inc(&mut self) {
        self.value += self.step;
        self.clamp();
    }
}

/// Human-readable value readout for a slider's track line.
fn slider_readout(s: &Slider, _lang: &Lang) -> String {
    format!(" {}{}", s.value, s.suffix)
}

/// Whether a slider is a product-percentage slider (and thus shows the
/// right-aligned "lock" checkbox on its track line).
fn is_product_pct(s: &Slider) -> bool {
    matches!(s.kind, SliderKind::YearlyPercent(_) | SliderKind::MonthPercent(_))
}

/// Short lock-checkbox label for a product-percentage slider.
fn slider_lock_label<'a>(s: &Slider, lang: &Lang) -> &'a str {
    let d = lang.dict();
    match s.kind {
        SliderKind::YearlyPercent(_) => d.tui_lock_year,
        SliderKind::MonthPercent(_) => d.tui_lock_month,
        _ => "",
    }
}

// ---------------------------------------------------------------------------
// TUI state
// ---------------------------------------------------------------------------

/// Which of the two main-area sub-tabs is active.  `Products` (the default,
/// shown first) lists the per-product simulation values (the same values
/// written to each `*.simulation_results.txt` on export) in a scrollable
/// view; `Graph` shows the 12-month chart.
#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Products,
    Graph,
}

/// The currently-selected top-level period tab.  `FullYear` (the default when
/// the program opens) shows the global minimum/target settings and the yearly
/// percentage sliders; `Month(m)` shows that month's override settings and
/// that month's percentage sliders, and (on the Graph sub-tab) borders that
/// month's bars on the always-full-year chart.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Period {
    FullYear,
    Month(usize),
}

impl Period {
    /// The month index if this is a `Month`, else `None`.
    fn month(self) -> Option<usize> {
        match self {
            Period::FullYear => None,
            Period::Month(m) => Some(m),
        }
    }

    /// Previous / next period for the `[` / `]` keys.  Order:
    /// Full Year, Jan, Feb, ..., Dec.
    fn prev(self) -> Period {
        match self {
            Period::FullYear => Period::Month(11),
            Period::Month(0) => Period::FullYear,
            Period::Month(m) => Period::Month(m - 1),
        }
    }

    fn next(self) -> Period {
        match self {
            Period::FullYear => Period::Month(0),
            Period::Month(11) => Period::FullYear,
            Period::Month(m) => Period::Month(m + 1),
        }
    }
}

/// Which on-screen region currently receives Up/Down scroll/navigation.  `Main`
/// is the main content area (the Products list or the Graph); `Sidebar` is the
/// product-parameters sliders panel.  Toggled with the Tab key.
#[derive(Clone, Copy, PartialEq)]
enum Region {
    Main,
    Sidebar,
}

struct AppState {
    /// (file path, computed result) for every product with positive net profit.
    products: Vec<(PathBuf, ProductResult)>,
    /// Per-product × per-month sales % (source of truth). Each month column
    /// sums to 100 across products. Indexed `[product][month]`.
    monthly_pct: Vec<[i64; 12]>,
    /// Per-product × per-month lock. Indexed `[product][month]`. A month entry
    /// is effectively locked when `month_locked[p][m] || yearly_locked[p]`.
    month_locked: Vec<[bool; 12]>,
    /// Per-product yearly lock. Freezes the product's % in ALL 12 months
    /// (month checkboxes render checked + greyed, uneditable) and pins its
    /// yearly % (which stays equal to the mean of the frozen months).
    yearly_locked: Vec<bool>,
    /// Currently-selected top-level period (Full Year or a month 0..11).
    period: Period,
    /// Flat slider list rebuilt whenever the period or sub-tab changes.
    /// Yearly slider values are derived from `monthly_pct` (mean of the 12
    /// months); monthly slider values are `monthly_pct[p][month]`.
    sliders: Vec<Slider>,
    selected: usize,
    /// Top visible entry index in the sidebar's scrollable slider list.
    scroll: usize,
    /// Folder the products were loaded from (for the totals export file).
    folder: PathBuf,
    /// Transient "exported to <path>" / error message shown in the footer.
    status: Option<String>,
    /// Active main-area sub-tab (Products / Graph).
    tab: Tab,
    /// Top visible line index of the Products scrollable view.
    product_scroll: usize,
    /// Interface language (used by the Products view to render the
    /// same per-product lines that are written to the export files).
    lang: Lang,
    /// Which region (main content vs sidebar) currently receives Up/Down.
    active_region: Region,
    /// Whether the full-screen help overlay (Ctrl+H) is currently shown.
    show_help: bool,
    /// Top visible line index of the help overlay's scrollable text.
    help_scroll: usize,
    /// Full-Year global minimum / target settings.
    settings: GlobalSettings,
    /// Per-month override arrays (workday hours, parallel products, monthly
    /// net profit goal). Clamped to be at least the global minimums.
    month_overrides: MonthOverrides,
}

/// Full-Year global minimum / target settings.  Per-month overrides
/// (below) are clamped to be at least these minimums.  All fields are `i64`
/// so the struct is `Copy`.
#[derive(Clone, Copy)]
struct GlobalSettings {
    /// "min. workday hours" (default 8).
    min_workday_hours: i64,
    /// "min. paralell products" (default 1).
    min_parallel: i64,
    /// "min. monthly net profit" (default 500).
    min_monthly_net_profit: i64,
    /// "target yearly net profit" (default 500).  Reference target only;
    /// there is no per-month override for it.
    target_yearly_net_profit: i64,
}

/// Default global settings (the values the program opens with).
const DEFAULT_MIN_WORKDAY_HOURS: i64 = 8;
const DEFAULT_MIN_PARALLEL: i64 = 1;
const DEFAULT_MIN_MONTHLY_NET_PROFIT: i64 = 500;
const DEFAULT_TARGET_YEARLY_NET_PROFIT: i64 = 500;

/// Per-month override arrays (workday hours, parallel products, monthly net
/// profit goal).  Each entry is clamped to be at least the corresponding
/// global minimum.  Defaults equal the global minimums.
#[derive(Clone)]
struct MonthOverrides {
    workday: [i64; 12],
    parallel: [i64; 12],
    net_profit: [i64; 12],
}

impl Default for MonthOverrides {
    fn default() -> Self {
        MonthOverrides {
            workday: [DEFAULT_MIN_WORKDAY_HOURS; 12],
            parallel: [DEFAULT_MIN_PARALLEL; 12],
            net_profit: [DEFAULT_MIN_MONTHLY_NET_PROFIT; 12],
        }
    }
}

impl AppState {
    #[allow(dead_code)]
    fn slider_value(&self, kind: SliderKind) -> i64 {
        self.sliders
            .iter()
            .find(|s| s.kind == kind)
            .map(|s| s.value)
            .unwrap_or(0)
    }

    /// Yearly % for product `idx` = `round(mean of the 12 monthly %)`. When the
    /// product is yearly-locked the value is still the mean (the months are all
    /// frozen, so the mean is stable).
    fn yearly_pct(&self, idx: usize) -> i64 {
        let sum: i64 = self.monthly_pct[idx].iter().sum();
        let n = 12i64;
        (sum as f64 / n as f64).round() as i64
    }

    /// The 12 monthly % for product `idx`.
    #[allow(dead_code)]
    fn monthly_pcts(&self, idx: usize) -> [i64; 12] {
        self.monthly_pct[idx]
    }

    /// Effective workday hours for month `m`: the per-month override clamped
    /// to be at least `min_workday_hours`.
    fn workday_hours(&self, m: usize) -> i64 {
        self.month_overrides.workday[m].max(self.settings.min_workday_hours).max(1)
    }

    /// Effective parallel products for month `m`: the per-month override
    /// clamped to be at least `min_parallel`.
    fn parallel(&self, m: usize) -> i64 {
        self.month_overrides.parallel[m].max(self.settings.min_parallel).max(1)
    }

    /// Effective monthly net-profit goal for month `m`: the per-month override
    /// clamped to be at least `min_monthly_net_profit`.
    fn monthly_goal(&self, m: usize) -> i64 {
        self.month_overrides.net_profit[m].max(self.settings.min_monthly_net_profit).max(0)
    }

    /// Clamp every per-month override array up to the corresponding global
    /// minimum. Called after a minimum slider changes so the invariant
    /// `override >= min` is restored.
    fn clamp_overrides_to_mins(&mut self) {
        for m in 0..12 {
            if self.month_overrides.workday[m] < self.settings.min_workday_hours {
                self.month_overrides.workday[m] = self.settings.min_workday_hours;
            }
            if self.month_overrides.parallel[m] < self.settings.min_parallel {
                self.month_overrides.parallel[m] = self.settings.min_parallel;
            }
            if self.month_overrides.net_profit[m] < self.settings.min_monthly_net_profit {
                self.month_overrides.net_profit[m] = self.settings.min_monthly_net_profit;
            }
        }
    }

    /// Normalized share (0..=1) for product `idx` in month `m`. If the month's
    /// total is zero the products are split equally.
    fn share_for_month(&self, idx: usize, m: usize) -> f64 {
        let total: i64 = (0..self.products.len()).map(|i| self.monthly_pct[i][m].max(0)).sum();
        if total <= 0 {
            return 1.0 / self.products.len() as f64;
        }
        self.monthly_pct[idx][m].max(0) as f64 / total as f64
    }

    /// Is product `idx` editable in month `m`? False when month-locked OR
    /// yearly-locked.
    fn editable_in_month(&self, idx: usize, m: usize) -> bool {
        !self.month_locked[idx][m] && !self.yearly_locked[idx]
    }

    /// Per-product achievable (capacity-capped) sales for month `m`. When the
    /// required production minutes exceed the monthly capacity, each product's
    /// sales are scaled down proportionally so the total fits.
    fn capped_product_sales(&self, m: usize) -> Vec<i64> {
        self.compute_month(m).capped_sales
    }

    /// Core computation for one month: required (uncapped) sales per product,
    /// the capacity scale factor, and the resulting capped sales. Uses that
    /// month's own workday hours, parallel products and net-profit goal.
    fn compute_month(&self, m: usize) -> MonthComputation {
        let monthly_goal = self.monthly_goal(m) as f64;
        let workday_hours = self.workday_hours(m) as f64;
        let parallel = self.parallel(m) as f64;

        let capacity_minutes = workday_hours * WORKDAYS_PER_MONTH * 60.0 * parallel;

        let mut req_sales: Vec<i64> = Vec::with_capacity(self.products.len());
        let mut required_minutes = 0.0;
        for (i, (_, p)) in self.products.iter().enumerate() {
            let target_profit = self.share_for_month(i, m) * monthly_goal;
            let s = if p.net_profit > 0.0 {
                ((target_profit / p.net_profit).ceil() as i64).max(0)
            } else {
                0
            };
            req_sales.push(s);
            required_minutes += s as f64 * p.duration_minutes;
        }

        let scale = if required_minutes > capacity_minutes && required_minutes > 0.0 {
            capacity_minutes / required_minutes
        } else {
            1.0
        };

        let capped_sales: Vec<i64> = req_sales
            .iter()
            .map(|s| (*s as f64 * scale).floor() as i64)
            .collect();

        MonthComputation {
            capped_sales,
            required_minutes,
            capacity_minutes,
        }
    }

    /// Compute one month's achievable (capacity-capped) sales totals for month
    /// `m`, using that month's percentage distribution.
    fn month_totals_for(&self, m: usize) -> MonthTotals {
        let mc = self.compute_month(m);

        let mut total_units = 0i64;
        let mut total_amount = 0.0f64;
        let mut total_profit = 0.0f64;
        let mut total_cost = 0.0f64;
        let mut achieved_minutes = 0.0f64;
        for (units, (_, p)) in mc.capped_sales.iter().zip(self.products.iter()) {
            total_units += units;
            total_amount += *units as f64 * p.price;
            total_profit += *units as f64 * p.net_profit;
            total_cost += *units as f64 * p.total_cost;
            achieved_minutes += *units as f64 * p.duration_minutes;
        }

        MonthTotals {
            units: total_units,
            amount: total_amount,
            profit: total_profit,
            cost: total_cost,
            required_minutes: mc.required_minutes,
            capacity_minutes: mc.capacity_minutes,
            achieved_minutes,
        }
    }

    /// Convenience: the selected period's month totals (used by the chart title
    /// etc.). For `Full Year` this falls back to January as the representative
    /// monthly load (the totals sidebar uses the annual sum directly).
    #[allow(dead_code)]
    fn month_totals(&self) -> MonthTotals {
        let m = self.period.month().unwrap_or(0);
        self.month_totals_for(m)
    }
}

/// One month's achievable (capacity-capped) totals.
#[allow(dead_code)]
struct MonthTotals {
    units: i64,
    amount: f64,
    profit: f64,
    cost: f64,
    required_minutes: f64,
    capacity_minutes: f64,
    /// Actual production minutes after capacity capping (sum of
    /// `capped_units * duration` per product).
    achieved_minutes: f64,
}

/// Intermediate per-month computation: the required (uncapped) sales, the
/// capacity, and the resulting capped sales per product.
struct MonthComputation {
    capped_sales: Vec<i64>,
    required_minutes: f64,
    capacity_minutes: f64,
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// How wide (in terminal cells) the 12-group chart tries to render each bar.
///
/// `chart_width` is the inner width (borders excluded). Each of the 12 groups
/// contains 2 bars + 1 inter-bar gap; groups are separated by `group_gap`.
/// Solving `12*(2*w + bar_gap) + 11*group_gap = chart_width` for `w`, clamped
/// to a minimum of 1.
fn fit_bar_width(chart_width: u16, bar_gap: u16, group_gap: u16) -> u16 {
    let fixed = 12u16 * bar_gap + 11 * group_gap;
    let remaining = chart_width.saturating_sub(fixed);
    let w = remaining / (12 * 2);
    w.max(1)
}

/// Render the 12-month chart directly into the frame buffer.
///
/// Each month draws two columns: `n` (units, cyan, single-color with a
/// fractional top cell) and `$` (amount, **two-toned**: the bottom portion is
/// net profit in green, the top portion is cost in yellow). ratatui's
/// `BarChart` widget only supports one style per bar, so the chart is drawn
/// manually to get the two-tone `$` bar.
fn render_chart(frame: &mut ratatui::Frame, area: Rect, state: &AppState) {
    // Compute each month's totals once (the chart now shows 12 distinct
    // months whose % distributions may differ). Yearly = sum of the 12.
    let months: Vec<MonthTotals> = (0..12).map(|m| state.month_totals_for(m)).collect();
    let yearly_units: i64 = months.iter().map(|m| m.units).sum();
    let yearly_amount: f64 = months.iter().map(|m| m.amount).sum();
    let yearly_profit: f64 = months.iter().map(|m| m.profit).sum();

    let max_val = months
        .iter()
        .map(|m| (m.units as f64).max(m.amount))
        .fold(1.0f64, f64::max);
    let max_with_headroom = (max_val * 1.25).max(max_val + 1.0);

    let sel = state.period.month();
    let d = state.lang.dict();
    let mnames = state.lang.months_abbr();
    // The stats line (axis max + selected-month + yearly figures) is rendered
    // below the legend, as the first line inside the bordered region, so it is
    // easy to read instead of being crammed into the title next to the legend.
    // For a Month period the selected month's figures are shown; for Full Year
    // only the yearly figures appear.
    let stats = match sel {
        Some(m) => {
            let mt = &months[m];
            format!(
                "  {0}: {1:.0}   {2}: n={3} \u{00a4}={4:.0} ({5} {6:.0} {7} {8:.0})   {9}: n={10} \u{00a4}={11:.0} ({12} {13:.0})",
                d.tui_axis_max, max_with_headroom,
                mnames[m], mt.units, mt.amount, d.tui_profit, mt.profit, d.tui_cost, mt.cost,
                d.tui_yearly, yearly_units, yearly_amount, d.tui_profit, yearly_profit,
            )
        }
        None => format!(
            "  {0}: {1:.0}   {2}: n={3} \u{00a4}={4:.0} ({5} {6:.0})",
            d.tui_axis_max, max_with_headroom,
            d.tui_yearly, yearly_units, yearly_amount, d.tui_profit, yearly_profit,
        ),
    };
    let active = state.active_region == Region::Main;
    let border_style = if active {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    // Build the title as styled spans so each legend marker inherits the
    // color of its bar region: cyan = units (n), green = profit ($),
    // yellow = cost ($).
    let bold = Modifier::BOLD;
    let title = Line::from(vec![
        Span::raw(format!("{}  ", d.tui_yearly_sales)),
        Span::styled(d.tui_legend_units, Style::default().fg(Color::Cyan).add_modifier(bold)),
        Span::styled(d.tui_legend_profit, Style::default().fg(Color::Green).add_modifier(bold)),
        Span::styled(d.tui_legend_cost, Style::default().fg(Color::Yellow).add_modifier(bold)),
    ]);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let buf = frame.buffer_mut();

    // Stats line: first row inside the bordered region, right below the legend.
    let stats_rows: u16 = 1;
    if inner.height >= stats_rows {
        buf.set_string(
            inner.x,
            inner.y,
            &stats,
            Style::default().fg(Color::White),
        );
    }

    // Axis-max label: moved onto the stats line, right-aligned with a 2-space
    // margin before the right border so it never collides with it.
    let chart_top = inner.y + stats_rows;
    let axis_label = format!("\u{2191} {} {:.0}", d.tui_max, max_with_headroom);
    let axis_x = inner.x
        + inner
            .width
            .saturating_sub(axis_label.chars().count() as u16 + 2);
    buf.set_string(
        axis_x,
        inner.y,
        &axis_label,
        Style::default().fg(Color::Gray).add_modifier(Modifier::DIM),
    );

    let bar_gap: u16 = 1;
    let group_gap: u16 = 2;
    // 1-char margin on each side so bars never touch the border, then center
    // the resulting chart block horizontally within the available width.
    let chart_margin: u16 = 1;
    let avail_width = inner.width.saturating_sub(chart_margin * 2);
    let bar_width = fit_bar_width(avail_width, bar_gap, group_gap);
    let group_width = 2 * bar_width + bar_gap;
    let total_chart_width: u16 =
        (12u32 * group_width as u32 + 11 * group_gap as u32) as u16;
    let chart_x = inner.x + chart_margin + avail_width.saturating_sub(total_chart_width) / 2;

    // Reserve 2 rows at the bottom for the bar labels (n/$) and month labels,
    // and `stats_rows` at the top for the stats line.
    let label_rows: u16 = 2;
    let bar_h = inner.height.saturating_sub(label_rows).saturating_sub(stats_rows);
    if bar_h == 0 {
        return;
    }
    let bar_bottom = chart_top + bar_h - 1;

    let cyan = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let yellow = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let green = Style::default().fg(Color::Green).add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(Color::DarkGray);
    let month_style = Style::default().fg(Color::White);
    let sel_month_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);

    let max = max_with_headroom;

    for g in 0..12u16 {
        let group_x = chart_x + g * (group_width + group_gap);
        let n_x = group_x;
        let d_x = group_x + bar_width + bar_gap;
        let mt = &months[g as usize];
        let units = mt.units as f64;
        let amount = mt.amount;
        let profit = mt.profit;

        // n bar: single color, fractional top cell.
        draw_bar_column(buf, n_x, bar_bottom, bar_width, units, max, bar_h, cyan, None);
        // $ bar: two-tone (green profit at the bottom, yellow cost on top).
        draw_bar_column(buf, d_x, bar_bottom, bar_width, amount, max, bar_h, yellow, Some((profit, green)));

        // Render the numeric value at the base of each bar / region.
        let n_h = (units / max * bar_h as f64).round() as i64;
        if n_h >= 1 {
            render_bar_value(buf, n_x, bar_bottom, bar_width, &compact_value(units), Color::Cyan);
        }
        let total_h = (amount / max * bar_h as f64).round() as i64;
        let total_h = total_h.clamp(0, bar_h as i64);
        let mut profit_h = (profit / max * bar_h as f64).round() as i64;
        profit_h = profit_h.clamp(0, total_h);
        if profit_h >= 1 {
            render_bar_value(buf, d_x, bar_bottom, bar_width, &compact_value(profit), Color::Green);
        }
        let cost_h = total_h - profit_h;
        // Cost ($) at the TOP of the yellow cost region, matching the legend.
        if cost_h >= 1 {
            let y_top = bar_bottom.saturating_sub((total_h - 1) as u16);
            render_bar_value(buf, d_x, y_top, bar_width, &compact_value(mt.cost), Color::Yellow);
        } else if total_h >= 1 && profit_h >= 1 {
            // No cost region: show total amount at the top (all-profit bar).
            let y_top = bar_bottom.saturating_sub((total_h - 1) as u16);
            render_bar_value(buf, d_x, y_top, bar_width, &compact_value(amount), Color::Green);
        }

        // Bar labels (n / $) just under the bars.
        let n_label_x = n_x + bar_width / 2;
        let d_label_x = d_x + bar_width / 2;
        if bar_bottom + 1 < inner.y + inner.height {
            buf.set_string(n_label_x, bar_bottom + 1, "n", label_style);
            buf.set_string(d_label_x, bar_bottom + 1, "$", label_style);
        }
        // Month label centered under the group; the selected month is
        // highlighted (only when a Month period is active).
        let m = mnames[g as usize];
        let m_w = m.chars().count() as u16;
        let m_x = group_x + group_width.saturating_sub(m_w) / 2;
        if bar_bottom + 2 < inner.y + inner.height {
            let ms = if sel == Some(g as usize) {
                sel_month_style
            } else {
                month_style
            };
            buf.set_string(m_x, bar_bottom + 2, m, ms);
        }

        // When a Month period is active, mark that month's bar group with a
        // downward arrow at the top, pointing into the bars (skipped for Full
        // Year, where no month is selected). The arrow sits in the inter-bar
        // gap at the top row of the bar area so it never overlaps a bar or its
        // value label.
        if sel == Some(g as usize) {
            let arrow_st = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
            let arrow_x = group_x + bar_width + bar_gap / 2;
            let arrow_y = chart_top;
            if arrow_y >= inner.y && arrow_y < inner.y + inner.height
                && arrow_x >= inner.x && arrow_x < inner.x + inner.width
            {
                buf.set_string(arrow_x, arrow_y, "\u{25bc}", arrow_st);
            }
        }
    }
}

/// Draw one bar column (possibly multi-cell wide) into `buf`.
///
/// With `profit_split = Some((profit, profit_style))` the bar is two-toned:
/// the bottom `profit` portion is drawn in `profit_style` and the remainder
/// (cost) in `top_style`. Without a split the bar is single-color with a
/// fractional top cell for smoother heights.
#[allow(clippy::too_many_arguments)]
fn draw_bar_column(
    buf: &mut ratatui::buffer::Buffer,
    x: u16,
    bottom_y: u16,
    width: u16,
    value: f64,
    max: f64,
    bar_h: u16,
    top_style: Style,
    profit_split: Option<(f64, Style)>,
) {
    if bar_h == 0 || max <= 0.0 || value <= 0.0 {
        return;
    }
    let fill_row = |buf: &mut ratatui::buffer::Buffer, y: u16, ch: char, style: Style| {
        let s = ch.to_string().repeat(width as usize);
        buf.set_string(x, y, &s, style);
    };

    match profit_split {
        Some((profit, profit_style)) => {
            let total = (value / max * bar_h as f64).round() as i64;
            let total = total.clamp(0, bar_h as i64);
            if total == 0 {
                return;
            }
            let mut profit_h = (profit / max * bar_h as f64).round() as i64;
            profit_h = profit_h.clamp(0, total);
            // bottom portion: profit (green)
            for i in 0..profit_h {
                let y = bottom_y.saturating_sub(i as u16);
                fill_row(buf, y, '\u{2588}', profit_style);
            }
            // top portion: cost (yellow)
            for i in profit_h..total {
                let y = bottom_y.saturating_sub(i as u16);
                fill_row(buf, y, '\u{2588}', top_style);
            }
        }
        None => {
            let total_float = value / max * bar_h as f64;
            let full = total_float.floor() as i64;
            let frac = total_float - full as f64;
            for i in 0..full {
                let y = bottom_y.saturating_sub(i as u16);
                fill_row(buf, y, '\u{2588}', top_style);
            }
            if frac > 0.0 && full < bar_h as i64 {
                if let Some(ch) = frac_char(frac) {
                    let y = bottom_y.saturating_sub(full as u16);
                    fill_row(buf, y, ch, top_style);
                }
            }
        }
    }
}

/// Render the top-level period tab bar (`Full Year`, `Jan`..`Dec`) across
/// `area`. The active period is shown inverted/bold; the inactive ones are
/// dim. The labels come from `tui_tab_full_year` and the language's month
/// abbreviations.
fn render_period_bar(frame: &mut ratatui::Frame, area: Rect, state: &AppState) {
    let d = state.lang.dict();
    let mnames = state.lang.months_abbr();
    // 13 labels: Full Year, then Jan..Dec.
    let labels: Vec<&'static str> = std::iter::once(d.tui_tab_full_year)
        .chain(mnames.iter().copied())
        .collect();
    let active_idx = match state.period {
        Period::FullYear => 0,
        Period::Month(m) => m + 1,
    };
    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, label) in labels.iter().enumerate() {
        let active = i == active_idx;
        let style = if active {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(format!(" {} ", label), style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Render a full-width horizontal separator rule across `area`.
fn render_separator(frame: &mut ratatui::Frame, area: Rect) {
    let buf = frame.buffer_mut();
    let style = Style::default().fg(Color::DarkGray);
    let mut x = area.x;
    while x < area.x + area.width {
        buf.set_string(x, area.y, "\u{2500}", style);
        x += 1;
    }
}

/// Render the sub-tab bar (`Products`, `Graph`) across `area`. The active
/// sub-tab is shown inverted/bold; the inactive one is dim.
fn render_subtab_bar(frame: &mut ratatui::Frame, area: Rect, state: &AppState) {
    let d = state.lang.dict();
    let tabs = [(Tab::Products, d.tui_tab_products), (Tab::Graph, d.tui_tab_graph)];
    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, (tab, label)) in tabs.iter().enumerate() {
        let active = state.tab == *tab;
        let style = if active {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(format!(" {} ", label), style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Render the scrollable per-product details view.  For the Full Year period
/// this mirrors the lines written to each product's `*.simulation_results.txt`
/// (stats + 12 monthly rows + annual + workday/parallel) with two donut
/// graphs per product.  For a Month period it shows each product's stats plus
/// that single month's row only.  All products are concatenated and the view
/// is scrollable with Up/Down while this sub-tab is active.
fn render_product_details(frame: &mut ratatui::Frame, area: Rect, state: &mut AppState) {
    match state.period {
        Period::Month(m) => render_product_details_month(frame, area, state, m),
        Period::FullYear => render_product_details_full_year(frame, area, state),
    }
}

/// Full-Year per-product details view (stats + 12 months + annual + donuts).
fn render_product_details_full_year(frame: &mut ratatui::Frame, area: Rect, state: &mut AppState) {
    let active = state.active_region == Region::Main;
    let border_style = if active {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    // Right strip reserved for the two per-product donut graphs (two columns,
    // right-aligned).  The text flows in the remaining left area.  A 1-char
    // right padding is reserved so the donuts never render flush against the
    // border.
    let right_pad: u16 = 1;
    let right_strip_w = (2 * DONUT_W + DONUT_GAP).min(inner.width.saturating_sub(right_pad));
    let text_w = inner.width.saturating_sub(right_strip_w + right_pad);
    // 1-char left padding for the product text.
    let pad = PROD_PAD as u16;
    let text_area = Rect::new(
        inner.x + pad,
        inner.y,
        text_w.saturating_sub(pad).max(1),
        inner.height,
    );
    let strip_x = inner.x + text_w;

    let lines = build_product_details_lines(state);
    let total = lines.len();
    let visible = inner.height as usize;
    // Clamp the scroll offset to the valid range, leaving the last page full
    // when possible.
    if total <= visible {
        state.product_scroll = 0;
    } else if state.product_scroll > total - visible {
        state.product_scroll = total - visible;
    }

    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((state.product_scroll as u16, 0)),
        text_area,
    );

    let buf = frame.buffer_mut();
    let n = state.products.len();
    let yearly_goal = state.settings.target_yearly_net_profit as f64;
    // Per-month, per-product capped sales (what the chart and donuts show).
    let capped_per_month: Vec<Vec<i64>> =
        (0..12).map(|m| state.capped_product_sales(m)).collect();
    let rule_style = Style::default().fg(Color::DarkGray);

    for k in 0..n {
        let product_start = k * LINES_PER_PRODUCT;
        let content_start = product_start + PROD_PAD;

        // Full-width separator rule across the inner width, drawn over the
        // blank line that build_product_details_lines inserts after each
        // product's padded area (the line at product_start + PRODUCT_AREA_H).
        if k + 1 < n {
            let sep_line = product_start + PRODUCT_AREA_H;
            let sep_y = inner.y as i64 + sep_line as i64 - state.product_scroll as i64;
            if sep_y >= inner.y as i64 && sep_y < (inner.y + inner.height) as i64 {
                let y = sep_y as u16;
                let mut x = inner.x;
                while x < inner.x + inner.width {
                    buf.set_string(x, y, "\u{2500}", rule_style);
                    x += 1;
                }
            }
        }

        // Two donut graphs, vertically centered within the product's padded
        // area (top pad + content + bottom pad).  Only draw when the whole
        // donut block is on screen so the ring never gets clipped mid-shape.
        let donut_top_line =
            content_start as i64 + (PRODUCT_CONTENT_LINES as i64 - DONUT_BLOCK_H as i64) / 2;
        let screen_y = inner.y as i64 + donut_top_line - state.product_scroll as i64;
        if screen_y < inner.y as i64
            || screen_y + DONUT_BLOCK_H as i64 > (inner.y + inner.height) as i64
        {
            continue;
        }
        let top_y = screen_y as u16;

        let (_, r) = &state.products[k];
        // Annual achievable (capped) sales for this product = sum of its 12
        // monthly capped sales — consistent with the chart, not the required
        // (uncapped) targets shown in the text lines.
        let annual_sales: i64 = (0..12).map(|m| capped_per_month[m][k]).sum();

        // Donut 1: profit margin (net profit / sale price, %).
        let margin_pct = r.profit_percent;
        // Donut 2: this product's yearly net profit vs the yearly goal slider.
        let yearly_profit = annual_sales as f64 * r.net_profit;
        let of_goal_pct = if yearly_goal > 0.0 {
            yearly_profit / yearly_goal * 100.0
        } else {
            0.0
        };

        let d1_x = strip_x;
        let d2_x = strip_x + DONUT_W + DONUT_GAP;
        let dd = state.lang.dict();
        draw_donut(buf, d1_x, top_y, margin_pct, dd.tui_donut_margin, inner);
        draw_donut(buf, d2_x, top_y, of_goal_pct, dd.tui_donut_vs_year, inner);
    }

    // Scroll indicators (up/down arrows) on the right edge, matching the
    // Products region's indicators.
    let can_up = state.product_scroll > 0;
    let can_down = total > visible && state.product_scroll + visible < total;
    let arrow_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    if inner.width >= 1 && inner.height >= 1 {
        let right_col = inner.x + inner.width.saturating_sub(1);
        if can_up {
            frame.render_widget(
                Paragraph::new("\u{2191}").style(arrow_style),
                Rect::new(right_col, inner.y, 1, 1),
            );
        }
        if can_down {
            frame.render_widget(
                Paragraph::new("\u{2193}").style(arrow_style),
                Rect::new(right_col, inner.y + inner.height.saturating_sub(1), 1, 1),
            );
        }
    }
}

/// Month-period per-product details view: each product shows its stats block
/// plus a single row for the selected month (and that month's workday /
/// parallel / net-profit override). No donuts, no annual row. Products are
/// separated by a full-width rule.
fn render_product_details_month(
    frame: &mut ratatui::Frame,
    area: Rect,
    state: &mut AppState,
    m: usize,
) {
    let active = state.active_region == Region::Main;
    let border_style = if active {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let pad = PROD_PAD as u16;
    let text_area = Rect::new(
        inner.x + pad,
        inner.y,
        inner.width.saturating_sub(pad * 2).max(1),
        inner.height,
    );

    let lines = build_product_details_lines_month(state, m);
    let total = lines.len();
    let visible = inner.height as usize;
    if total <= visible {
        state.product_scroll = 0;
    } else if state.product_scroll > total - visible {
        state.product_scroll = total - visible;
    }

    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((state.product_scroll as u16, 0)),
        text_area,
    );

    // Separator rules between products: one blank line per product boundary
    // (the last product has no trailing separator) is overdrawn with a rule.
    let buf = frame.buffer_mut();
    let n = state.products.len();
    let rule_style = Style::default().fg(Color::DarkGray);
    // Month-view layout per product: top pad (1) + content (9) + bottom pad (1)
    // = 11 lines, plus 1 separator blank = 12 lines per product.
    let lines_per_product_month = MONTH_PRODUCT_CONTENT_LINES + 2 * PROD_PAD + 1;
    for k in 0..n {
        if k + 1 < n {
            let sep_line = k * lines_per_product_month + (MONTH_PRODUCT_CONTENT_LINES + 2 * PROD_PAD);
            let sep_y = inner.y as i64 + sep_line as i64 - state.product_scroll as i64;
            if sep_y >= inner.y as i64 && sep_y < (inner.y + inner.height) as i64 {
                let y = sep_y as u16;
                let mut x = inner.x;
                while x < inner.x + inner.width {
                    buf.set_string(x, y, "\u{2500}", rule_style);
                    x += 1;
                }
            }
        }
    }

    // Scroll indicators.
    let can_up = state.product_scroll > 0;
    let can_down = total > visible && state.product_scroll + visible < total;
    let arrow_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    if inner.width >= 1 && inner.height >= 1 {
        let right_col = inner.x + inner.width.saturating_sub(1);
        if can_up {
            frame.render_widget(
                Paragraph::new("\u{2191}").style(arrow_style),
                Rect::new(right_col, inner.y, 1, 1),
            );
        }
        if can_down {
            frame.render_widget(
                Paragraph::new("\u{2193}").style(arrow_style),
                Rect::new(right_col, inner.y + inner.height.saturating_sub(1), 1, 1),
            );
        }
    }
}

/// Number of text lines describing one product in the month details view:
/// 6 stats + 1 blank + 1 month row + 1 blank + 1 workday + 1 parallel +
/// 1 net-profit = 11. Excludes padding and the separator blank.
const MONTH_PRODUCT_CONTENT_LINES: usize = 11;

/// Build the per-product lines for the Month-period Products view: stats block
/// + the selected month's row + that month's workday / parallel / net-profit
/// override settings. Products are separated by a blank line (overdrawn as a
/// rule by the renderer).
fn build_product_details_lines_month(state: &AppState, m: usize) -> Vec<Line<'static>> {
    let d = state.lang.dict();
    let workday_hours = state.workday_hours(m);
    let parallel = state.parallel(m);
    let net_profit_goal = state.monthly_goal(m);
    let per_month = month_shares(state, m);

    let all_templates = [
        d.result_product,
        d.result_sale_price,
        d.result_total_cost,
        d.result_net_profit_unit,
        d.result_profit_margin,
        d.result_prod_time,
        d.result_month_row,
        d.result_workday,
        d.result_parallel,
    ];
    let label_w = all_templates
        .iter()
        .map(|t| lang::str_width(lang::prefix_before_value(t)))
        .max()
        .unwrap_or(0)
        + 2;

    let header_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let goal_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let setting_style = Style::default().fg(Color::Green);

    let mut lines: Vec<Line<'static>> = Vec::new();
    for (i, (_, r)) in state.products.iter().enumerate() {
        let cur = r.currency.to_string();

        // Top padding.
        lines.push(Line::from(""));

        // Product stats block.
        let stats: Vec<(&'static str, Vec<String>)> = vec![
            (d.result_product, vec![r.name.clone()]),
            (d.result_sale_price, vec![format!("{:.2}", r.price), cur.clone()]),
            (d.result_total_cost, vec![format!("{:.2}", r.total_cost), cur.clone()]),
            (d.result_net_profit_unit, vec![format!("{:.2}", r.net_profit), cur.clone()]),
            (d.result_profit_margin, vec![format!("{:.2}", r.profit_percent)]),
            (d.result_prod_time, vec![format!("{:.2}", r.duration_minutes)]),
        ];
        for (k, (t, row)) in stats.iter().enumerate() {
            let refs: Vec<&str> = row.iter().map(String::as_str).collect();
            let text = lang::fmt_aligned(t, &refs, label_w);
            let style = if k == 0 { header_style } else { Style::default() };
            lines.push(Line::from(Span::styled(text, style)));
        }

        lines.push(Line::from(""));

        // The selected month's row.
        let s = &per_month[i];
        let hours = s.monthly_minutes / 60.0;
        let prefix = format!("  📆 {}:", crate::simulator::months_abbr(&state.lang)[m]);
        let text = lang::fmt_prefixed(
            d.result_month_row,
            &prefix,
            &[
                &format!("{:.2}", s.monthly_goal),
                &cur,
                &s.monthly_sales.to_string(),
                &format!("{:.2}", s.monthly_minutes),
                &format!("{:.2}", hours),
            ],
            label_w,
        );
        lines.push(Line::from(Span::styled(text, goal_style)));

        lines.push(Line::from(""));

        // The month's override settings.
        lines.push(Line::from(Span::styled(
            lang::fmt_aligned(d.result_workday, &[&workday_hours.to_string()], label_w),
            setting_style,
        )));
        lines.push(Line::from(Span::styled(
            lang::fmt_aligned(d.result_parallel, &[&parallel.to_string()], label_w),
            setting_style,
        )));
        lines.push(Line::from(Span::styled(
            lang::fmt_aligned(
                d.result_net_profit_unit,
                &[&format!("{:.2}", net_profit_goal as f64), &cur],
                label_w,
            ),
            setting_style,
        )));

        // Bottom padding.
        lines.push(Line::from(""));

        // Separator blank (overdrawn as a rule by the renderer, except after
        // the last product).
        if i + 1 < state.products.len() {
            lines.push(Line::from(""));
        }
    }

    lines
}

/// Width (cells) and height (rows) of one donut graph.  The ring is drawn with
/// braille dot patterns (2×4 sub-cell dots per cell) so it renders round and
/// smooth instead of blocky.  Because each braille dot is roughly square on
/// screen, a 2:1 cell width:height (`DONUT_W = 2 × DONUT_H`) yields a square
/// dot grid and thus a visually round ring.
const DONUT_W: u16 = 8;
const DONUT_H: u16 = 4;
/// One donut block = the ring + the percentage line + the caption line.
const DONUT_BLOCK_H: u16 = DONUT_H + 2;
/// Horizontal padding (cells) between the two donut graphs.
const DONUT_GAP: u16 = 4;
/// Per-product padding (cells/lines) on every side of the product's content
/// block in the details view.
const PROD_PAD: usize = 1;
/// Number of text lines that describe one product in the details view (stats +
/// 12 monthly rows + annual + workday/parallel), excluding padding and the
/// separator. 6 stats + 1 blank + 12 months + 1 blank + 2 annual + 1 blank +
/// 2 workday/parallel = 25.
const PRODUCT_CONTENT_LINES: usize = 25;
/// A product's padded area height: top pad + content + bottom pad.
const PRODUCT_AREA_H: usize = 2 * PROD_PAD + PRODUCT_CONTENT_LINES;
/// Lines per product in the scrollable Paragraph: padded area + one separator
/// blank line (except the last product, which has no trailing separator).
const LINES_PER_PRODUCT: usize = PRODUCT_AREA_H + 1;

/// Braille dot bit for sub-cell column `dc` (0=left, 1=right) and row `dr`
/// (0..4), per the U+2800..U+28FF mapping.
fn braille_bit(dc: usize, dr: usize) -> u8 {
    match (dr, dc) {
        (0, 0) => 0x01,
        (0, 1) => 0x02,
        (1, 0) => 0x04,
        (1, 1) => 0x08,
        (2, 0) => 0x10,
        (2, 1) => 0x20,
        (3, 0) => 0x40,
        (3, 1) => 0x80,
        _ => 0,
    }
}

/// Draw a round braille-dot donut ring into `buf` at `(x, y)`, filled clockwise
/// from the top by `pct`% of the ring, with the percentage and a caption
/// rendered below it.  The unfilled portion of the ring is drawn dim and the
/// filled arc bright; cells outside `inner` are clipped.  `pct` is clamped to
/// 0..=100 for the arc fill, while the printed percentage reflects the true
/// value (which may exceed 100).
fn draw_donut(
    buf: &mut ratatui::buffer::Buffer,
    x: u16,
    y: u16,
    pct: f64,
    caption: &str,
    inner: Rect,
) {
    let pct_c = pct.clamp(0.0, 100.0);
    // Dot grid resolution: 2 dots per cell column, 4 dots per cell row.
    let dots_w = 2 * DONUT_W as usize;
    let dots_h = 4 * DONUT_H as usize;
    let cx = dots_w as f64 / 2.0;
    let cy = dots_h as f64 / 2.0;
    let outer_r = (dots_w as f64 / 2.0).min(dots_h as f64 / 2.0);
    let inner_r = outer_r * 0.62;
    let filled = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    // Two passes so the dim ring shows fully and the filled arc is overlaid
    // bright on top (a cell is a single char/style, so a boundary cell ends up
    // bright with only its filled dots).
    for pass in 0..2 {
        for cr in 0..DONUT_H {
            let py = y + cr;
            if py < inner.y || py >= inner.y + inner.height {
                continue;
            }
            for cc in 0..DONUT_W {
                let px = x + cc;
                if px < inner.x || px >= inner.x + inner.width {
                    continue;
                }
                let mut ring_bits = 0u8;
                let mut fill_bits = 0u8;
                for dr in 0..4 {
                    for dc in 0..2 {
                        let dot_col = cc as usize * 2 + dc;
                        let dot_row = cr as usize * 4 + dr;
                        let dx = dot_col as f64 + 0.5 - cx;
                        let dy = dot_row as f64 + 0.5 - cy;
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist > outer_r || dist < inner_r {
                            continue;
                        }
                        ring_bits |= braille_bit(dc, dr);
                        // Angle from the top (12 o'clock), clockwise.
                        let mut ang = dx.atan2(-dy).to_degrees();
                        if ang < 0.0 {
                            ang += 360.0;
                        }
                        if ang <= pct_c * 3.6 {
                            fill_bits |= braille_bit(dc, dr);
                        }
                    }
                }
                if pass == 0 {
                    if ring_bits != 0 {
                        let ch = char::from_u32(0x2800 + ring_bits as u32).unwrap_or('\u{2800}');
                        buf.set_string(px, py, ch.to_string(), dim);
                    }
                } else if fill_bits != 0 {
                    let ch = char::from_u32(0x2800 + fill_bits as u32).unwrap_or('\u{2800}');
                    buf.set_string(px, py, ch.to_string(), filled);
                }
            }
        }
    }

    // Percentage label, centered under the ring.
    let label = format!("{:.0}%", pct);
    let label_y = y + DONUT_H;
    if label_y < inner.y + inner.height {
        let lw = label.chars().count() as u16;
        let lx = x + (DONUT_W.saturating_sub(lw)) / 2;
        let style = Style::default().fg(Color::White).add_modifier(Modifier::BOLD);
        clip_set_str(buf, lx, label_y, &label, style, inner);
    }
    // Caption, centered under the percentage.
    let cap_y = y + DONUT_H + 1;
    if cap_y < inner.y + inner.height {
        let cw = caption.chars().count() as u16;
        let cxp = x + (DONUT_W.saturating_sub(cw)) / 2;
        let style = Style::default().fg(Color::DarkGray);
        clip_set_str(buf, cxp, cap_y, caption, style, inner);
    }
}

/// Write `s` at `(x, y)` into `buf`, clipping any characters that fall outside
/// `inner` (column-wise and row-wise).
fn clip_set_str(
    buf: &mut ratatui::buffer::Buffer,
    x: u16,
    y: u16,
    s: &str,
    style: Style,
    inner: Rect,
) {
    if y < inner.y || y >= inner.y + inner.height {
        return;
    }
    let mut cx = x;
    for ch in s.chars() {
        if cx >= inner.x + inner.width {
            break;
        }
        if cx >= inner.x {
            buf.set_string(cx, y, ch.to_string(), style);
        }
        cx += 1;
    }
}

/// Build the full list of lines for the Product-details view: every product's
/// export-file rows concatenated, separated by a blank line.  Each product
/// shows its stats, a 12-month breakdown (one row per month), the annual
/// goal/time (annual = sum of the 12 months), and the shared workday/parallel
/// settings.  The label column width is computed across all templates (just
/// like `write_result_file_monthly`) so the value columns line up.
fn build_product_details_lines(state: &AppState) -> Vec<Line<'static>> {
    let d = state.lang.dict();
    // The Full Year view shows the global minimum workday / parallel as the
    // reference settings (each month's own override is reflected in its
    // monthly minutes row). The annual time line uses these same reference
    // values for its workdays calculation.
    let workday_hours = state.settings.min_workday_hours;
    let parallel = state.settings.min_parallel.max(1);
    // Per-month, per-product shares (each month uses its own net-profit goal
    // override via month_shares).
    let per_month: Vec<Vec<crate::simulator::ProductShare>> =
        (0..12).map(|m| month_shares(state, m)).collect();

    let all_templates = [
        d.result_product,
        d.result_sale_price,
        d.result_total_cost,
        d.result_net_profit_unit,
        d.result_profit_margin,
        d.result_prod_time,
        d.result_month_row,
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

    let header_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let goal_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let month_style = Style::default();
    let setting_style = Style::default().fg(Color::Green);

    let selected_month = state.period.month();

    let mut lines: Vec<Line<'static>> = Vec::new();
    for (i, (_, r)) in state.products.iter().enumerate() {
        let cur = r.currency.to_string();

        // Top padding (1 blank line).
        lines.push(Line::from(""));

        // Product stats block.
        let stats: Vec<(&'static str, Vec<String>)> = vec![
            (d.result_product, vec![r.name.clone()]),
            (d.result_sale_price, vec![format!("{:.2}", r.price), cur.clone()]),
            (d.result_total_cost, vec![format!("{:.2}", r.total_cost), cur.clone()]),
            (d.result_net_profit_unit, vec![format!("{:.2}", r.net_profit), cur.clone()]),
            (d.result_profit_margin, vec![format!("{:.2}", r.profit_percent)]),
            (d.result_prod_time, vec![format!("{:.2}", r.duration_minutes)]),
        ];
        for (k, (t, row)) in stats.iter().enumerate() {
            let refs: Vec<&str> = row.iter().map(String::as_str).collect();
            let text = lang::fmt_aligned(t, &refs, label_w);
            let style = if k == 0 { header_style } else { Style::default() };
            lines.push(Line::from(Span::styled(text, style)));
        }

        lines.push(Line::from(""));

        // 12 monthly rows.
        let mut annual_goal = 0.0f64;
        let mut annual_sales = 0i64;
        let mut annual_minutes = 0.0f64;
        for m in 0..12 {
            let s = &per_month[m][i];
            annual_goal += s.monthly_goal;
            annual_sales += s.monthly_sales;
            annual_minutes += s.monthly_minutes;
            let hours = s.monthly_minutes / 60.0;
            let prefix = format!("  📆 {}:", crate::simulator::months_abbr(&state.lang)[m]);
            let text = lang::fmt_prefixed(
                d.result_month_row,
                &prefix,
                &[
                    &format!("{:.2}", s.monthly_goal),
                    &cur,
                    &s.monthly_sales.to_string(),
                    &format!("{:.2}", s.monthly_minutes),
                    &format!("{:.2}", hours),
                ],
                label_w,
            );
            let style = if selected_month == Some(m) { goal_style } else { month_style };
            lines.push(Line::from(Span::styled(text, style)));
        }

        lines.push(Line::from(""));

        // Annual goal + time (annual = sum of the 12 months).
        lines.push(Line::from(Span::styled(
            lang::fmt_aligned(
                d.result_annual_goal,
                &[&format!("{:.2}", annual_goal), &cur, &annual_sales.to_string()],
                label_w,
            ),
            goal_style,
        )));
        lines.push(Line::from(time_line_rendered(
            d.result_annual_time,
            annual_minutes,
            parallel,
            workday_hours,
            label_w,
        )));

        lines.push(Line::from(""));

        // Reference workday / parallel settings (the Full Year minimums).
        lines.push(Line::from(Span::styled(
            lang::fmt_aligned(d.result_workday, &[&workday_hours.to_string()], label_w),
            setting_style,
        )));
        lines.push(Line::from(Span::styled(
            lang::fmt_aligned(d.result_parallel, &[&parallel.to_string()], label_w),
            setting_style,
        )));

        // Bottom padding (1 blank line).
        lines.push(Line::from(""));

        // Separator between products (not after the last one): a single blank
        // line that render_product_details overdraws with a full-width rule.
        if i + 1 < state.products.len() {
            lines.push(Line::from(""));
        }
    }

    lines
}

/// Render a "time line" the same way `simulator::time_line` does for the
/// export files, returning the formatted string for display in the TUI.
fn time_line_rendered(
    template: &str,
    minutes: f64,
    parallel: i64,
    workday_hours: i64,
    label_width: usize,
) -> String {
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


/// Format a number compactly for display inside narrow bar columns.
///
/// Values < 1000 are shown as-is. Larger values use K/M suffixes so they
/// stay short enough to fit in 2–4 cells, avoiding truncation that would
/// make a growing value appear to shrink (e.g. 8800 → "88" vs "9K").
fn compact_value(v: f64) -> String {
    let v_abs = v.abs();
    if v_abs >= 1_000_000.0 {
        format!("{:.1}M", v / 1_000_000.0)
    } else if v_abs >= 1_000.0 {
        format!("{}K", (v / 1_000.0).round() as i64)
    } else {
        format!("{:.0}", v)
    }
}

/// Render `text` centered within a `width`-cell-wide bar column at row `y`.
/// Drawn as black bold text on a `bg` colored cell so the value reads as
/// embedded in the bar.  If the text is wider than the bar it overflows to
/// the right (into the inter-bar gap) rather than being truncated, so the
/// value is always correct.
fn render_bar_value(
    buf: &mut ratatui::buffer::Buffer,
    x: u16,
    y: u16,
    width: u16,
    text: &str,
    bg: Color,
) {
    if width == 0 || text.is_empty() {
        return;
    }
    let w = text.chars().count() as u16;
    let start_x = if w >= width {
        x
    } else {
        x + (width - w) / 2
    };
    let style = Style::default()
        .fg(Color::Black)
        .bg(bg)
        .add_modifier(Modifier::BOLD);
    buf.set_string(start_x, y, text, style);
}

/// Fractional block character for the top cell of a single-color bar.
fn frac_char(f: f64) -> Option<char> {
    if f <= 0.0 {
        return None;
    }
    if f >= 0.875 {
        return Some('\u{2588}');
    }
    let levels = [
        '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}',
    ];
    let idx = (f * 8.0).ceil() as usize;
    if idx == 0 {
        None
    } else {
        Some(levels[idx - 1])
    }
}


/// A textual slider track `width` cells wide, filled proportionally to the
/// value.  `width` is clamped to at least 1 so a degenerate column still shows
/// a single-cell track.
fn slider_track(s: &Slider, width: usize) -> String {
    let w = width.max(1);
    let span = (s.max - s.min).max(1) as f64;
    let frac = (s.value - s.min) as f64 / span;
    let filled = (frac * w as f64).round() as usize;
    let filled = filled.min(w);
    let mut out = String::with_capacity(w);
    for i in 0..w {
        if i < filled {
            out.push('\u{2588}');
        } else {
            out.push('\u{2591}');
        }
    }
    out
}

/// Lines for one slider entry (header + track + blank).  When the slider is a
/// product-percentage slider, a right-aligned "[x] lock values" / "[ ] lock
/// values" checkbox is appended to the header line within `width` cells, so it
/// sits on the same line as the product name, flush against the panel's right
/// border.  `width` is the inner (border-excluded) width of the panel that will
/// render these lines; pass 0 to suppress the checkbox (e.g. when the width is
/// unknown).
fn slider_entry_lines(s: &Slider, focused: bool, width: u16, lang: &Lang) -> Vec<Line<'static>> {
    let marker = if focused { "\u{25b6} " } else { "  " };
    let label_style = if focused {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let pct = is_product_pct(s);

    // Settings / month-selector sliders (non-percent): wrap the label after the
    // first word (the first word on its own header line, the remaining words
    // wrapped to the column width), left-aligned.  Product-percentage sliders
    // keep their single-line header with the right-aligned "lock" checkbox.
    if !pct {
        return settings_entry_lines(s, focused, width, marker, label_style, lang);
    }

    // Header line: marker + label (the "lock" checkbox moves to the track line
    // below, right-aligned within `width`).
    let mut header_spans: Vec<Span<'static>> = Vec::new();
    header_spans.push(Span::styled(marker.to_string(), label_style));
    header_spans.push(Span::styled(s.label.clone(), label_style));

    let mut lines = Vec::new();
    lines.push(Line::from(header_spans));
    let readout = slider_readout(s, lang);
    // Track width: prefer 10 cells, but shrink to whatever room is left after
    // the 2-cell indent and the readout so the track + readout never wraps to
    // a second line in a narrow column.
    let indent_w = 2usize;
    let readout_w = readout.chars().count();
    let max_track = 10usize;
    let avail_for_track = (width as usize).saturating_sub(indent_w + readout_w);
    let track_w = avail_for_track.clamp(1, max_track);
    let track = slider_track(s, track_w);
    let track_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    // Build the track line: indent + track + readout, optionally followed by
    // a right-aligned "[x] lock" / "[ ] lock" checkbox for product sliders.
    let mut track_spans: Vec<Span<'static>> = Vec::new();
    track_spans.push(Span::raw("  "));
    track_spans.push(Span::styled(track, track_style));
    track_spans.push(Span::styled(readout, label_style));

    if width > 0 {
        let box_str = if s.locked { "[x]" } else { "[ ]" };
        let lock_label = slider_lock_label(s, lang);
        let needed = box_str.chars().count() + lock_label.chars().count();
        let used = indent_w + track_w + readout_w;
        let avail = (width as usize).saturating_sub(used);
        if avail >= needed {
            let pad = avail - needed;
            let box_style = if s.locked {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else if focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let label_style_cb = box_style;
            track_spans.push(Span::raw(" ".repeat(pad)));
            track_spans.push(Span::styled(box_str.to_string(), box_style));
            track_spans.push(Span::styled(lock_label.to_string(), label_style_cb));
        }
    }

    lines.push(Line::from(track_spans));
    lines.push(Line::from(""));
    lines
}

/// Lines for a Settings (non-percent) slider entry.  The label is split after
/// the first word: the first word goes on its own header line (with the focus
/// marker), the remaining words are word-wrapped to the column width, all
/// left-aligned.  The track + readout line follows.  A trailing blank line
/// separates entries.
fn settings_entry_lines(
    s: &Slider,
    focused: bool,
    width: u16,
    marker: &str,
    label_style: Style,
    lang: &Lang,
) -> Vec<Line<'static>> {
    let w = width as usize;

    // Split the label into the first word and the remainder.
    let (first, rest) = match s.label.find(' ') {
        Some(i) => (s.label[..i].to_string(), s.label[i + 1..].trim().to_string()),
        None => (s.label.clone(), String::new()),
    };

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Header line: marker + first word, left-aligned.
    lines.push(Line::from(vec![
        Span::styled(marker.to_string(), label_style),
        Span::styled(first, label_style),
    ]));

    // Remaining words, word-wrapped to `w` and each line left-aligned.  Indent
    // each wrapped line by the marker width so it aligns with the first word
    // on the header line (rather than starting at the column's left edge).
    let marker_w = marker.chars().count();
    let indent: String = " ".repeat(marker_w);
    let wrap_w = w.saturating_sub(marker_w).max(1);
    if !rest.is_empty() {
        for chunk in word_wrap(&rest, wrap_w) {
            lines.push(Line::from(vec![
                Span::raw(indent.clone()),
                Span::styled(chunk, label_style),
            ]));
        }
    }

    // Track + readout line, left-aligned (2-space indent + track + readout).
    let readout = slider_readout(s, lang);
    let indent_w = 2usize;
    let readout_w = readout.chars().count();
    let max_track = 10usize;
    let avail_for_track = w.saturating_sub(indent_w + readout_w);
    let track_w = avail_for_track.clamp(1, max_track);
    let track = slider_track(s, track_w);
    let track_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(track, track_style),
        Span::styled(readout, label_style),
    ]));
    lines.push(Line::from(""));
    lines
}

/// Greedy word-wrap `text` to lines of at most `width` display cells, breaking
/// at spaces.  A single word longer than `width` is hard-split at `width`.
fn word_wrap(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let word_w = lang::str_width(word);
        if cur.is_empty() {
            if word_w <= width {
                cur.push_str(word);
            } else {
                // Hard-split an over-long word.
                let mut s = word;
                while lang::str_width(s) > width {
                    let mut take = 0usize;
                    let mut wlen = 0usize;
                    for (i, c) in s.char_indices() {
                        let cw = lang::str_width(&c.to_string());
                        if wlen + cw > width {
                            break;
                        }
                        wlen += cw;
                        take = i + c.len_utf8();
                    }
                    lines.push(s[..take].to_string());
                    s = &s[take..];
                }
                cur.push_str(s);
            }
        } else if lang::str_width(&cur) + 1 + word_w <= width {
            cur.push(' ');
            cur.push_str(word);
        } else {
            lines.push(std::mem::take(&mut cur));
            if word_w <= width {
                cur.push_str(word);
            } else {
                let mut s = word;
                while lang::str_width(s) > width {
                    let mut take = 0usize;
                    let mut wlen = 0usize;
                    for (i, c) in s.char_indices() {
                        let cw = lang::str_width(&c.to_string());
                        if wlen + cw > width {
                            break;
                        }
                        wlen += cw;
                        take = i + c.len_utf8();
                    }
                    lines.push(s[..take].to_string());
                    s = &s[take..];
                }
                cur.push_str(s);
            }
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Lines for a range of slider entries (no header — the surrounding `Block`
/// title serves as the header).  `start..end` are indices into
/// `state.sliders`; the focused entry (`state.selected`) is highlighted.
/// `width` is the inner (border-excluded) width of the rendering panel, used
/// to right-align the per-product "lock values" checkbox.
fn build_slider_lines(state: &AppState, start: usize, end: usize, width: u16) -> Vec<Line<'static>> {
    let lang = state.lang;
    let mut lines: Vec<Line<'static>> = Vec::new();
    let end = end.min(state.sliders.len());
    for i in start..end {
        let s = &state.sliders[i];
        lines.extend(slider_entry_lines(s, i == state.selected, width, &lang));
    }
    lines
}

/// Lines for the sidebar's bottom region: every total value that is also
/// written to `totals.simulation_results.txt` (monthly + annual sales, time in
/// minutes/hours, workdays, plus the workday/parallel settings), followed by
/// the 12 x monthly vs yearly-goal reference.
#[allow(dead_code)]
fn build_totals_lines(state: &AppState) -> Vec<Line<'static>> {
    let (left, right) = build_totals_columns(state);
    let mut lines = left;
    lines.extend(right);
    lines
}

/// Build the two Totals columns as separate line vectors so the renderer can
/// place them side by side.  Left column: Monthly + Settings.  Right column:
/// Yearly + Yearly ref.  Each column is self-contained and wraps to its own
/// width when rendered with `Paragraph::wrap`.  Labels are kept short so they
/// fit narrow columns without wrapping the value onto its own line.
fn build_totals_columns(state: &AppState) -> (Vec<Line<'static>>, Vec<Line<'static>>) {
    let t = compute_totals(state);
    let d = state.lang.dict();
    let sub = Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan);
    let val = Style::default();

    // Helper: pad a label to a fixed width.
    let lbl = |s: &str, w: usize| -> String {
        let pad = w.saturating_sub(lang::str_width(s));
        format!("  {}{}", s, " ".repeat(pad))
    };
    let lw = 11; // label column width

    // A periodic block (Monthly or Yearly): header + sales/min/hours/workdays.
    let period_block = |header: &'static str, p: &PeriodTotals| -> Vec<Line<'static>> {
        vec![
            Line::from(Span::styled(header, sub)),
            Line::from(vec![
                Span::styled(lbl(d.tui_label_sales, lw), val),
                Span::styled(format!("{}", p.sales), val),
            ]),
            Line::from(vec![
                Span::styled(lbl(d.tui_label_min, lw), val),
                Span::styled(format!("{:.0}", p.minutes), val),
            ]),
            Line::from(vec![
                Span::styled(lbl(d.tui_label_hours, lw), val),
                Span::styled(format!("{:.1}", p.hours), val),
            ]),
            Line::from(vec![
                Span::styled(lbl(d.tui_label_workdays, lw), val),
                Span::styled(format!("{:.2}", p.workdays), val),
            ]),
        ]
    };

    // Yearly reference: sum of the 12 monthly net-profit goals vs the target
    // yearly net profit.
    let year_sum: i64 = (0..12).map(|m| state.monthly_goal(m)).sum();
    let yearly_target = state.settings.target_yearly_net_profit;
    let (mark, mark_style) = if year_sum >= yearly_target {
        ("\u{2714}", Style::default().fg(Color::Green))
    } else {
        ("\u{2716}", Style::default().fg(Color::Red))
    };
    let ref_block: Vec<Line<'static>> = {
        let mut v = vec![Line::from(Span::styled(d.tui_label_yearly_ref, sub))];
        v.push(Line::from(vec![
            Span::styled(lbl(d.tui_label_12x_mo, lw), val),
            Span::styled(format!("{}", year_sum), val),
        ]));
        v.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(mark.to_string(), mark_style),
            Span::styled(format!(" {}  {}", d.tui_label_goal, yearly_target), val),
        ]));
        v
    };

    // --- Left column: period totals ---
    //   Month period  -> Monthly (+ goal-achievement indicator)
    //   Full Year     -> Yearly
    let mut left: Vec<Line<'static>> = match state.period {
        Period::Month(m) => {
            let mut block = period_block(d.tui_label_monthly, &t.monthly);
            // Goal-achievement line: green check if the month's achieved net
            // profit meets the month's net-profit goal, red cross otherwise.
            let m_goal = state.monthly_goal(m);
            let (m_mark, m_mark_style) = if t.monthly.profit >= m_goal as f64 {
                ("\u{2714}", Style::default().fg(Color::Green))
            } else {
                ("\u{2716}", Style::default().fg(Color::Red))
            };
            block.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(m_mark.to_string(), m_mark_style),
                Span::styled(format!(" {}  {}", d.tui_label_goal, m_goal), val),
            ]));
            block
        }
        Period::FullYear => period_block(d.tui_label_yearly, &t.annual),
    };

    // --- Right column: ---
    //   Month period  -> Yearly + Yearly ref
    //   Full Year     -> Yearly ref only (left already shows yearly)
    let mut right: Vec<Line<'static>> = Vec::new();
    if state.period == Period::FullYear {
        right.extend(ref_block);
    } else {
        right.extend(period_block(d.tui_label_yearly, &t.annual));
        right.push(Line::from(""));
        right.extend(ref_block);
    }

    // Pad the shorter column so both columns are the same height (the
    // surrounding bordered region sizes to the taller one).
    while left.len() < right.len() {
        left.push(Line::from(""));
    }
    while right.len() < left.len() {
        right.push(Line::from(""));
    }

    (left, right)
}

fn draw(frame: &mut ratatui::Frame, state: &mut AppState) {
    // Ctrl+H help overlay takes over the whole screen when active.
    if state.show_help {
        render_help(frame, frame.area(), state);
        return;
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(frame.area());

    let body = chunks[0];
    let footer = chunks[1];

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(body);

    let chart_area = body_chunks[0];
    // Split the main area into the period tab bar (1 row), a separator rule
    // (1 row), the Products/Graph sub-tab bar (1 row), and the content below.
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(chart_area);
    render_period_bar(frame, main_chunks[0], state);
    render_separator(frame, main_chunks[1]);
    render_subtab_bar(frame, main_chunks[2], state);
    let content_area = main_chunks[3];
    match state.tab {
        Tab::Products => render_product_details(frame, content_area, state),
        Tab::Graph => render_chart(frame, content_area, state),
    }

    // Sidebar layout:
    //   [Products (scroll)] [Settings] [Totals]
    // The settings shown depend on the period (Full Year = the 4 global
    // min/target sliders; a Month = that month's 3 override sliders).
    let sidebar_area = body_chunks[1];
    let total_sliders = state.sliders.len();
    let n_products = state.products.len();
    let products_start = 0usize;
    let settings_start = products_start + n_products;

    // Desired dynamic inner padding for every sidebar region.
    let sidebar_inner_w = sidebar_area.width.saturating_sub(2) as usize;
    let desired_pad = ((sidebar_inner_w / 12) as u16).min(4);

    let totals_region_h: u16 = 11;
    let settings_min_h: u16 = 7;

    // Products: each entry is 3 lines (header + track + blank).
    let products_needed = (n_products * 3) as u16;

    // Settings: build the actual wrapped lines for both columns and take the
    // taller one.
    let settings_inner_w = sidebar_inner_w;
    let settings_col_w = settings_inner_w
        .saturating_sub(1 /* separator */ + desired_pad as usize * 2) / 2;
    let mid = (settings_start + 2).min(total_sliders);
    let settings_left_lines = build_slider_lines(state, settings_start, mid, settings_col_w as u16);
    let settings_right_lines = build_slider_lines(state, mid, total_sliders, settings_col_w as u16);
    let settings_needed = settings_left_lines.len().max(settings_right_lines.len()) as u16;

    // Totals: both columns have the same height (build_totals_columns pads
    // the shorter one).
    let (totals_left, totals_right) = build_totals_columns(state);
    let totals_needed = totals_left.len().max(totals_right.len()) as u16;

    // Compute the padding each region can afford.
    let mut sidebar_pad: u16 = 0;
    let mut settings_region_h: u16 = settings_min_h;
    for &p in (0..=desired_pad).rev().collect::<Vec<_>>().iter() {
        let s_h = (2 + settings_needed + p * 2).max(settings_min_h);
        let products_available = sidebar_area
            .height
            .saturating_sub(s_h + totals_region_h);
        let products_max_pad = products_available
            .saturating_sub(2 + products_needed) / 2;
        let totals_max_pad = totals_region_h
            .saturating_sub(2 + totals_needed) / 2;
        if p <= products_max_pad && p <= totals_max_pad {
            sidebar_pad = p;
            settings_region_h = s_h;
            break;
        }
    }

    // Build the vertical constraints for the sidebar regions.
    let sidebar_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),
            Constraint::Length(settings_region_h),
            Constraint::Length(totals_region_h),
        ])
        .split(sidebar_area);

    let sidebar_active = state.active_region == Region::Sidebar;
    let mut region_idx = 0usize;

    // --- Products region (scrollable) ---
    let top_area = sidebar_chunks[region_idx];
    region_idx += 1;
    let top_inner_h = top_area.height.saturating_sub(2 + sidebar_pad * 2) as usize;
    let entry_h = 3usize;
    let visible_entries = (top_inner_h / entry_h).max(1);
    // Auto-scroll only while focus is inside the products range.
    if state.selected >= products_start && state.selected < settings_start {
        let rel = state.selected - products_start;
        if rel < state.scroll {
            state.scroll = rel;
        } else if rel >= state.scroll + visible_entries {
            state.scroll = rel + 1 - visible_entries;
        }
        if state.scroll + visible_entries > n_products {
            state.scroll = n_products.saturating_sub(visible_entries);
        }
    }
    let start = products_start + state.scroll;
    let end = (start + visible_entries).min(settings_start);
    let right_pad = sidebar_pad.max(1);
    let top_inner_w = top_area.width.saturating_sub(2 + sidebar_pad + right_pad);
    let product_lines = build_slider_lines(state, start, end, top_inner_w);
    let top_title: String = match state.period {
        Period::FullYear => state.lang.dict().tui_products_yearly.to_string(),
        Period::Month(m) => lang::fmt(
            state.lang.dict().tui_month_pct_sales,
            &[state.lang.months_abbr()[m]],
        ),
    };
    let products_block = Block::default()
        .borders(Borders::ALL)
        .title(top_title)
        .title_style(Style::default().add_modifier(Modifier::BOLD))
        .border_style(if sidebar_active {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        });
    frame.render_widget(products_block, top_area);
    let products_content = Rect::new(
        top_area.x + 1 + sidebar_pad,
        top_area.y + 1 + sidebar_pad,
        top_inner_w,
        top_inner_h as u16,
    );
    frame.render_widget(Paragraph::new(product_lines), products_content);

    // Scroll indicators for the products region.
    let can_up = state.scroll > 0;
    let can_down = end < settings_start && n_products > visible_entries;
    let arrow_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    if top_area.width >= 2 && top_area.height >= 2 {
        let right_col = top_area.x + top_area.width - 2;
        if can_up {
            frame.render_widget(
                Paragraph::new("\u{2191}").style(arrow_style),
                Rect::new(right_col, top_area.y + 1, 1, 1),
            );
        }
        if can_down {
            frame.render_widget(
                Paragraph::new("\u{2193}").style(arrow_style),
                Rect::new(right_col, top_area.y + top_area.height - 2, 1, 1),
            );
        }
    }

    // --- Settings region ---
    let settings_title: String = match state.period {
        Period::FullYear => state.lang.dict().tui_sidebar_settings.to_string(),
        Period::Month(m) => lang::fmt(
            state.lang.dict().tui_sidebar_settings_month,
            &[state.lang.months_abbr()[m]],
        ),
    };
    let settings_block = Block::default()
        .borders(Borders::ALL)
        .title(settings_title)
        .title_style(Style::default().add_modifier(Modifier::BOLD))
        .border_style(if sidebar_active {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        });
    let settings_inner = settings_block.inner(sidebar_chunks[region_idx]);
    frame.render_widget(settings_block, sidebar_chunks[region_idx]);
    region_idx += 1;
    if settings_inner.height > 0 && settings_inner.width >= 3 {
        let settings_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Min(1),
            ])
            .horizontal_margin(0)
            .split(settings_inner);
        let left_area = settings_cols[0];
        let sep_area = settings_cols[1];
        let right_area = settings_cols[2];

        let sep_style = Style::default().fg(Color::DarkGray);
        let buf = frame.buffer_mut();
        for y in sep_area.top()..sep_area.bottom() {
            if y < buf.area.height {
                buf.set_string(sep_area.x, y, "\u{2502}", sep_style);
            }
        }

        let pad = sidebar_pad;
        let right_pad = pad.max(1);
        let left_inner = Rect::new(
            left_area.x + pad,
            left_area.y + pad,
            left_area.width.saturating_sub(pad * 2),
            left_area.height.saturating_sub(pad * 2),
        );
        let right_inner = Rect::new(
            right_area.x + pad,
            right_area.y + pad,
            right_area.width.saturating_sub(pad + right_pad),
            right_area.height.saturating_sub(pad * 2),
        );
        let mid = (settings_start + 2).min(total_sliders);
        let left_lines = build_slider_lines(state, settings_start, mid, left_inner.width);
        let right_lines = build_slider_lines(state, mid, total_sliders, right_inner.width);
        frame.render_widget(
            Paragraph::new(left_lines).wrap(Wrap { trim: false }),
            left_inner,
        );
        frame.render_widget(
            Paragraph::new(right_lines).wrap(Wrap { trim: false }),
            right_inner,
        );
    }

    // --- Totals region ---
    let bottom_block = Block::default()
        .borders(Borders::ALL)
        .title(state.lang.dict().tui_sidebar_totals)
        .title_style(Style::default().add_modifier(Modifier::BOLD))
        .border_style(if sidebar_active {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        });
    let totals_inner = bottom_block.inner(sidebar_chunks[region_idx]);
    frame.render_widget(bottom_block, sidebar_chunks[region_idx]);
    if totals_inner.height > 0 && totals_inner.width >= 3 {
        let totals_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Min(1),
            ])
            .split(totals_inner);
        let left_area = totals_cols[0];
        let sep_area = totals_cols[1];
        let right_area = totals_cols[2];

        let sep_style = Style::default().fg(Color::DarkGray);
        let buf = frame.buffer_mut();
        for y in sep_area.top()..sep_area.bottom() {
            if y < buf.area.height {
                buf.set_string(sep_area.x, y, "\u{2502}", sep_style);
            }
        }

        let pad = sidebar_pad;
        let right_pad = pad.max(1);
        let left_inner = Rect::new(
            left_area.x + pad,
            left_area.y + pad,
            left_area.width.saturating_sub(pad * 2),
            left_area.height.saturating_sub(pad * 2),
        );
        let right_inner = Rect::new(
            right_area.x + pad,
            right_area.y + pad,
            right_area.width.saturating_sub(pad + right_pad),
            right_area.height.saturating_sub(pad * 2),
        );
        let (left_lines, right_lines) = build_totals_columns(state);
        frame.render_widget(
            Paragraph::new(left_lines).wrap(Wrap { trim: false }),
            left_inner,
        );
        frame.render_widget(
            Paragraph::new(right_lines).wrap(Wrap { trim: false }),
            right_inner,
        );
    }

    let d = state.lang.dict();
    let footer_text = match &state.status {
        Some(msg) => lang::fmt(d.tui_footer_status, &[msg]),
        None => d.tui_footer.to_string(),
    };
    frame.render_widget(
        Paragraph::new(footer_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center),
        footer,
    );
}

// ---------------------------------------------------------------------------
// Event loop
// ---------------------------------------------------------------------------

/// Set product `changed_prod`'s monthly % in `month` to `new_value` (clamped
/// to the room left by locked products in that month) and distribute the
/// remaining % **equally** across all other **non-locked** products in that
/// month (including ones currently at 0). A product is locked-in-month when
/// `month_locked[p][month] || yearly_locked[p]`. The changed product must
/// itself be editable (otherwise this is a no-op). Rounding drift is corrected
/// so the month sums to exactly 100 whenever at least one receiver exists.
fn redistribute_month(state: &mut AppState, month: usize, changed_prod: usize, new_value: i64) {
    if !state.editable_in_month(changed_prod, month) {
        return;
    }
    let n = state.products.len();

    // Frozen products in this month (locked, excluding the changed one). Their
    // values carve out a fixed chunk of the 100% pie.
    let frozen_sum: i64 = (0..n)
        .filter(|&q| q != changed_prod && !state.editable_in_month(q, month))
        .map(|q| state.monthly_pct[q][month])
        .sum();

    let max_v = (100 - frozen_sum).max(0);
    let v = new_value.clamp(0, max_v);
    state.monthly_pct[changed_prod][month] = v;

    // Receivers: every other non-locked product (including zeros).
    let receivers: Vec<usize> = (0..n)
        .filter(|&q| q != changed_prod && state.editable_in_month(q, month))
        .collect();

    let remainder = 100 - v - frozen_sum;
    if remainder <= 0 || receivers.is_empty() {
        // No room / no receivers: zero out the non-locked, non-changed products.
        for &q in &receivers {
            state.monthly_pct[q][month] = 0;
        }
        return;
    }

    // Equal split of the remainder across receivers, with rounding fixup.
    let n_r = receivers.len() as i64;
    let base = remainder / n_r;
    let extra = remainder - base * n_r;
    let mut new_vals: Vec<i64> = (0..receivers.len())
        .map(|i| base + if (i as i64) < extra { 1 } else { 0 })
        .collect();
    let mut diff = remainder - new_vals.iter().sum::<i64>();
    if diff != 0 {
        let step: i64 = if diff > 0 { 1 } else { -1 };
        let mut k = 0;
        while diff != 0 {
            let i = k % new_vals.len();
            new_vals[i] += step;
            diff -= step;
            k += 1;
        }
    }
    for (i, &q) in receivers.iter().enumerate() {
        state.monthly_pct[q][month] = new_vals[i].max(0);
    }
}

/// Edit a product's **yearly** % (Products tab). The yearly % is the mean of
/// the 12 monthly %, so "setting" it propagates the target value to every month
/// where the product is editable (non-month-locked, non-yearly-locked),
/// redistributing the remainder within each such month. Month-locked months
/// keep their overridden value, so the resulting yearly mean may differ from
/// the target — that is the "unless locked" behaviour.
fn edit_yearly(state: &mut AppState, changed_prod: usize, target_value: i64) {
    if state.yearly_locked[changed_prod] {
        return;
    }
    for m in 0..12 {
        redistribute_month(state, m, changed_prod, target_value);
    }
}

/// Build the styled line list for the help overlay. Lines starting with
/// `## ` are section headers (rendered bold + yellow, preceded by a blank
/// line); everything else is body text wrapped by the `Paragraph` widget.
fn build_help_lines(state: &AppState) -> Vec<Line<'static>> {
    let header_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let text = state.lang.dict().tui_help_text;
    let mut out: Vec<Line<'static>> = Vec::new();
    for raw in text.split('\n') {
        if let Some(rest) = raw.strip_prefix("## ") {
            if !out.is_empty() {
                out.push(Line::default());
            }
            out.push(Line::styled(rest.to_string(), header_style));
        } else {
            out.push(Line::from(raw.to_string()));
        }
    }
    out
}

/// Page-up / page-down step for the help overlay, derived from the current
/// terminal height (inner area, borders excluded).
fn help_page_size() -> usize {
    crossterm::terminal::size()
        .map(|(_, h)| (h.saturating_sub(2)) as usize)
        .unwrap_or(10)
        .max(1)
}

/// Render the full-screen help overlay (Ctrl+H). Clears the normal layout
/// and fills the whole terminal with a bordered, scrollable `Paragraph`.
fn render_help(frame: &mut ratatui::Frame, area: Rect, state: &mut AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(state.lang.dict().tui_help_title)
        .title_style(Style::default().add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let lines = build_help_lines(state);
    let total = lines.len();
    let visible = inner.height as usize;
    // Clamp the scroll offset to the valid range, leaving the last page full
    // when possible.
    if total <= visible {
        state.help_scroll = 0;
    } else if state.help_scroll > total - visible {
        state.help_scroll = total - visible;
    }

    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((state.help_scroll as u16, 0)),
        inner,
    );

    // Scroll indicators on the right edge.
    let can_up = state.help_scroll > 0;
    let can_down = total > visible && state.help_scroll + visible < total;
    let arrow_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    if inner.width >= 1 && inner.height >= 1 {
        let right_col = inner.x + inner.width.saturating_sub(1);
        if can_up {
            frame.render_widget(
                Paragraph::new("\u{2191}").style(arrow_style),
                Rect::new(right_col, inner.y, 1, 1),
            );
        }
        if can_down {
            frame.render_widget(
                Paragraph::new("\u{2193}").style(arrow_style),
                Rect::new(
                    right_col,
                    inner.y + inner.height.saturating_sub(1),
                    1,
                    1,
                ),
            );
        }
    }
}

fn handle_key(state: &mut AppState, key: &KeyEvent, lang: &Lang) -> bool {
    // Ctrl+H: toggle the full-screen help overlay. While the overlay is open
    // it swallows all keys: Esc/q/Ctrl+H close it, arrows scroll the text,
    // everything else is ignored (so the underlying layout is not disturbed).
    if key.modifiers.contains(event::KeyModifiers::CONTROL) && key.code == KeyCode::Char('h') {
        state.show_help = !state.show_help;
        state.help_scroll = 0;
        return false;
    }
    if state.show_help {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                state.show_help = false;
                state.help_scroll = 0;
            }
            KeyCode::Up => {
                if state.help_scroll > 0 {
                    state.help_scroll -= 1;
                }
            }
            KeyCode::Down => {
                state.help_scroll = state.help_scroll.saturating_add(1);
            }
            KeyCode::PageUp => {
                let page = help_page_size();
                state.help_scroll = state.help_scroll.saturating_sub(page);
            }
            KeyCode::PageDown => {
                let page = help_page_size();
                state.help_scroll = state.help_scroll.saturating_add(page);
            }
            KeyCode::Home => state.help_scroll = 0,
            KeyCode::End => state.help_scroll = usize::MAX,
            _ => {}
        }
        return false;
    }
    // Ctrl+E: export current simulation to the *.simulation_results.txt files.
    if key.modifiers.contains(event::KeyModifiers::CONTROL) && key.code == KeyCode::Char('e') {
        state.status = Some(export_results(state, lang));
        return false;
    }
    // Tab: toggle the active region (main content <-> sidebar sliders).
    if key.code == KeyCode::Tab && !key.modifiers.contains(event::KeyModifiers::SHIFT) {
        state.active_region = match state.active_region {
            Region::Main => Region::Sidebar,
            Region::Sidebar => Region::Main,
        };
        return false;
    }
    // Shift+Tab (BackTab): switch the main-area sub-tab (Products <-> Graph).
    if key.code == KeyCode::BackTab {
        state.tab = match state.tab {
            Tab::Products => Tab::Graph,
            Tab::Graph => Tab::Products,
        };
        state.selected = 0;
        state.scroll = 0;
        state.rebuild_sliders();
        return false;
    }
    // `[` / `]`: move the top-level period selection left / right
    // (Full Year, Jan, ..., Dec). The sidebar is rebuilt for the new period.
    if key.code == KeyCode::Char('[') {
        state.period = state.period.prev();
        state.selected = 0;
        state.scroll = 0;
        state.rebuild_sliders();
        update_parallel_range(state);
        return false;
    }
    if key.code == KeyCode::Char(']') {
        state.period = state.period.next();
        state.selected = 0;
        state.scroll = 0;
        state.rebuild_sliders();
        update_parallel_range(state);
        return false;
    }
    // Up/Down behaviour depends on the active region:
    //   - Main + Products tab -> scroll the product details list
    //   - Sidebar             -> navigate the sidebar sliders
    //   - Main + Graph tab    -> fall through to sidebar navigation (the chart
    //                            doesn't scroll, so arrows still drive sliders)
    if state.active_region == Region::Main && state.tab == Tab::Products {
        match key.code {
            KeyCode::Up => {
                if state.product_scroll > 0 {
                    state.product_scroll -= 1;
                }
                return false;
            }
            KeyCode::Down => {
                state.product_scroll = state.product_scroll.saturating_add(1);
                return false;
            }
            _ => {}
        }
    }
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return true,
        KeyCode::Char(' ') => {
            // Toggle the lock of the focused product slider.
            //  - YearlyPercent(P): toggle yearly_locked[P] (freezes P in all
            //    12 months; the month checkboxes render checked + greyed).
            //  - MonthPercent(P): toggle month_locked[P][selected_month], but
            //    only if P is not yearly-locked (otherwise it is greyed out).
            if let Some(s) = state.sliders.get(state.selected).cloned() {
                match s.kind {
                    SliderKind::YearlyPercent(p) => {
                        state.yearly_locked[p] = !state.yearly_locked[p];
                        state.rebuild_sliders();
                    }
                    SliderKind::MonthPercent(p) => {
                        if !state.yearly_locked[p] {
                            if let Some(m) = state.period.month() {
                                state.month_locked[p][m] = !state.month_locked[p][m];
                                state.rebuild_sliders();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        KeyCode::Up => {
            if state.selected == 0 {
                state.selected = state.sliders.len() - 1;
            } else {
                state.selected -= 1;
            }
        }
        KeyCode::Down => {
            state.selected = (state.selected + 1) % state.sliders.len();
        }
        KeyCode::Left => adjust_slider(state, -1),
        KeyCode::Right => adjust_slider(state, 1),
        _ => {}
    }
    // Goals / percentages / workday hours changed the required production
    // time, so the parallel-products cap range may have shifted. Recompute
    // and clamp on every adjustment (cheap; no-op for pure navigation).
    update_parallel_range(state);
    false
}

/// Apply a ±`dir` step to the focused slider. Handles yearly % sliders
/// (propagates to all months), monthly % sliders (redistributes within the
/// selected month), and the settings sliders (global min/target and per-month
/// overrides). After a settings change the slider values are synced back to
/// the state and overrides are clamped to the minimums.
fn adjust_slider(state: &mut AppState, dir: i64) {
    let Some(s) = state.sliders.get(state.selected).cloned() else {
        return;
    };
    match s.kind {
        SliderKind::YearlyPercent(p) => {
            let target = s.value + dir * s.step;
            edit_yearly(state, p, target);
            state.rebuild_sliders();
        }
        SliderKind::MonthPercent(p) => {
            if let Some(m) = state.period.month() {
                let target = state.monthly_pct[p][m] + dir * s.step;
                redistribute_month(state, m, p, target);
                state.rebuild_sliders();
            }
        }
        _ => {
            // A settings slider: inc/dec the slider (clamped to its min/max),
            // then sync the new value back into the state and clamp overrides
            // to the minimums.
            if let Some(s) = state.sliders.get_mut(state.selected) {
                if dir < 0 {
                    s.dec();
                } else {
                    s.inc();
                }
            }
            sync_settings_from_sliders(state);
            state.rebuild_sliders();
        }
    }
}

/// Copy the settings slider values back into the state (`settings` /
/// `month_overrides`) and clamp every per-month override up to its global
/// minimum. Called after a settings slider is adjusted.
fn sync_settings_from_sliders(state: &mut AppState) {
    for s in &state.sliders {
        match s.kind {
            SliderKind::MinWorkdayHours => state.settings.min_workday_hours = s.value,
            SliderKind::MinParallel => state.settings.min_parallel = s.value,
            SliderKind::MinMonthlyNetProfit => state.settings.min_monthly_net_profit = s.value,
            SliderKind::TargetYearlyNetProfit => {
                state.settings.target_yearly_net_profit = s.value
            }
            SliderKind::MonthWorkdayHours(m) => state.month_overrides.workday[m] = s.value,
            SliderKind::MonthParallel(m) => state.month_overrides.parallel[m] = s.value,
            SliderKind::MonthNetProfit(m) => state.month_overrides.net_profit[m] = s.value,
            _ => {}
        }
    }
    state.clamp_overrides_to_mins();
}

/// Load and compute every product definition in `folder`.  Products with
/// non-positive net profit are dropped (they cannot service a profit goal).
/// Returns `(file_path, result)` pairs alongside the folder path.
fn load_products(folder: &Path, lang: &Lang) -> Vec<(PathBuf, ProductResult)> {
    let files = collect_txt_files(folder);
    let mut out = Vec::new();
    for file in &files {
        let content = match std::fs::read_to_string(file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if let Ok(product) = parse_content(&content, lang) {
            let r = compute_result(&product);
            if r.net_profit > 0.0 {
                out.push((file.clone(), r));
            }
        }
    }
    out
}

/// Per-product split of the monthly goal for a single month `m`, using that
/// month's percentage distribution and that month's net-profit goal override.
/// Only the `monthly_*` fields of the returned [`ProductShare`] are meaningful
/// (annual fields are zeroed). The sales counts are the *required* (uncapped)
/// figures — capacity capping is applied separately in
/// [`AppState::month_totals_for`] for the chart.
fn month_shares(state: &AppState, m: usize) -> Vec<crate::simulator::ProductShare> {
    let monthly_goal = state.monthly_goal(m) as f64;
    let raw_pcts: Vec<i64> = (0..state.products.len())
        .map(|i| state.monthly_pct[i][m].max(0))
        .collect();
    let results: Vec<&ProductResult> = state.products.iter().map(|(_, r)| r).collect();
    compute_product_shares(&results, &raw_pcts, monthly_goal, 0.0)
}

/// Aggregate totals across all products for one period, mirroring the rows
/// written to `totals.simulation_results.txt`.
struct PeriodTotals {
    sales: i64,
    minutes: f64,
    hours: f64,
    workdays: f64,
    /// Achieved net profit (sum of capped_units * net_profit_per_unit). Used
    /// by the Monthly block's goal-achievement indicator.
    profit: f64,
}

/// All totals shown in the sidebar's bottom region. `monthly` reflects the
/// currently-selected month (when a Month period is active); `annual` is the
/// sum of all 12 months. Both use achievable (capacity-capped) figures so
/// they agree with the chart. Workdays are computed per-month (each month
/// using its own workday hours / parallel override) and summed for the annual
/// row.
struct Totals {
    monthly: PeriodTotals,
    annual: PeriodTotals,
}

fn compute_totals(state: &AppState) -> Totals {
    // Annual = sum of all 12 months' achievable (capped) totals. Workdays are
    // summed per-month so each month's own workday hours / parallel override
    // is respected.
    let mut a_sales = 0i64;
    let mut a_min = 0.0f64;
    let mut a_workdays = 0.0f64;
    let mut a_profit = 0.0f64;
    for m in 0..12 {
        let mt_m = state.month_totals_for(m);
        a_sales += mt_m.units;
        a_min += mt_m.achieved_minutes;
        a_profit += mt_m.profit;
        let wh = state.workday_hours(m) as f64;
        let pp = state.parallel(m) as f64;
        let m_hours = mt_m.achieved_minutes / 60.0;
        a_workdays += m_hours / (wh * pp);
    }
    let a_hours = a_min / 60.0;

    // Selected month's achievable (capped) totals — same computation the
    // chart uses, so the sidebar and chart always agree. For Full Year the
    // monthly column is unused by the display (it falls back to yearly), so
    // January is used as a harmless representative.
    let sel = state.period.month().unwrap_or(0);
    let mt = state.month_totals_for(sel);
    let m_hours = mt.achieved_minutes / 60.0;
    let wh_m = state.workday_hours(sel) as f64;
    let pp_m = state.parallel(sel) as f64;

    Totals {
        monthly: PeriodTotals {
            sales: mt.units,
            minutes: mt.achieved_minutes,
            hours: m_hours,
            workdays: m_hours / (wh_m * pp_m),
            profit: mt.profit,
        },
        annual: PeriodTotals {
            sales: a_sales,
            minutes: a_min,
            hours: a_hours,
            workdays: a_workdays,
            profit: a_profit,
        },
    }
}

/// Recompute the parallel-products slider's `[min, max]` from the current
/// goals / percentages / workday hours, mirroring the old dialoguer flow
/// (`simulator::parallel_range`):
///   * `min` = throughput needed to stay within 30 monthly / 365 yearly
///     workdays (the binding cap).
///   * `max` = throughput that brings the binding period down to 1 workday.
///
/// The slider value is clamped into the new range, and its label is refreshed
/// to show the caps.
fn update_parallel_range(state: &mut AppState) {
    // Only the per-month parallel override slider (shown when a Month period
    // is active) has a computed range. Its min is the global `min_parallel`
    // floor; its max is the throughput that brings that month's required
    // production down to ~1 hour of work. Crucially the max is derived from a
    // fixed 1-hour reference — NOT from the month's current workday hours —
    // so raising workday hours never shrinks the cap and clamps `parallel`
    // back down. Both sliders thus independently grow the monthly capacity
    // (workday_hours × 22 × 60 × parallel) toward the net-profit goal. The
    // state field is clamped into range so the displayed value and the
    // simulation agree.
    let Some(m) = state.period.month() else {
        return;
    };
    let monthly_minutes: f64 = month_shares(state, m).iter().map(|s| s.monthly_minutes).sum();
    let min_par = state.settings.min_parallel.max(1);
    let monthly_hours = monthly_minutes / 60.0;
    let max_par = (monthly_hours.floor() as i64).max(min_par);
    let lo = state.month_overrides.parallel[m].max(min_par);
    let clamped = lo.min(max_par).max(min_par);
    state.month_overrides.parallel[m] = clamped;
    for s in state.sliders.iter_mut() {
        if let SliderKind::MonthParallel(_) = s.kind {
            s.min = min_par;
            s.max = max_par;
            s.value = clamped;
            s.label = lang::fmt(
                state.lang.dict().tui_parallel_label,
                &[&min_par.to_string(), &max_par.to_string()],
            );
        }
    }
}

/// Write the per-product `*.simulation_results.txt` files (12 monthly rows +
/// annual sum) and the aggregate `totals.simulation_results.txt` (12 monthly
/// rows + annual) in `state.folder`. The current per-month percentages,
/// per-month net-profit goals, and the Full-Year minimum workday / parallel
/// (used as the reference settings rows) drive the split. Returns a
/// human-readable status string.
fn export_results(state: &AppState, lang: &Lang) -> String {
    let workday_hours = state.settings.min_workday_hours;
    let parallel = state.settings.min_parallel.max(1);

    // Per-month, per-product shares (each month uses its own net-profit goal
    // override via month_shares).
    let per_month: Vec<Vec<crate::simulator::ProductShare>> =
        (0..12).map(|m| month_shares(state, m)).collect();

    let mut total_monthly_sales = [0i64; 12];
    let mut total_monthly_minutes = [0.0f64; 12];
    let mut total_annual_sales = 0i64;
    let mut total_annual_minutes = 0.0f64;

    for (i, ((file, r), _)) in state.products.iter().zip(per_month[0].iter()).enumerate() {
        let mut monthly_goals = [0.0f64; 12];
        let mut monthly_sales = [0i64; 12];
        let mut monthly_minutes = [0.0f64; 12];
        let mut annual_goal = 0.0f64;
        let mut annual_sales = 0i64;
        let mut annual_minutes = 0.0f64;
        for m in 0..12 {
            let s = &per_month[m][i];
            monthly_goals[m] = s.monthly_goal;
            monthly_sales[m] = s.monthly_sales;
            monthly_minutes[m] = s.monthly_minutes;
            annual_goal += s.monthly_goal;
            annual_sales += s.monthly_sales;
            annual_minutes += s.monthly_minutes;
            total_monthly_sales[m] += s.monthly_sales;
            total_monthly_minutes[m] += s.monthly_minutes;
        }
        total_annual_sales += annual_sales;
        total_annual_minutes += annual_minutes;
        if let Err(e) = write_result_file_monthly(
            file,
            r,
            &monthly_goals,
            &monthly_sales,
            &monthly_minutes,
            annual_goal,
            annual_sales,
            annual_minutes,
            workday_hours,
            parallel,
            lang,
        ) {
            return lang::fmt(lang.dict().tui_export_error, &[&r.name, &e.to_string()]);
        }
    }
    // Silence unused-mut warning for per_month (borrowed mutably above).
    let _ = &per_month;

    if let Err(e) = write_totals_file_monthly(
        &state.folder,
        state.products.len(),
        &total_monthly_sales,
        &total_monthly_minutes,
        total_annual_sales,
        total_annual_minutes,
        workday_hours,
        parallel,
        lang,
    ) {
        return lang::fmt(lang.dict().tui_export_error_totals, &[&e.to_string()]);
    }

    // Persist percentages, locks, and settings so reopening the app restores
    // the user's custom distribution.
    save_state(state);

    lang::fmt(
        lang.dict().tui_exported,
        &[&state.products.len().to_string(), &state.folder.display().to_string()],
    )
}

// ---------------------------------------------------------------------------
// State persistence (save / load percentages, locks, and settings)
// ---------------------------------------------------------------------------

/// Hidden file written in the product folder alongside the export files. Not a
/// `.txt` file, so [`collect_txt_files`] never picks it up as a product
/// definition.
const STATE_FILE_NAME: &str = ".simulation_state";

fn state_file_path(folder: &Path) -> PathBuf {
    folder.join(STATE_FILE_NAME)
}

/// Persist the current percentages, locks, settings, and selected month to
/// `.simulation_state` in the product folder. Called during export so that
/// reopening the app restores the user's custom distribution.
fn save_state(state: &AppState) {
    let path = state_file_path(&state.folder);
    let mut out = String::new();
    out.push_str("# tiny-business-simulator state v2\n");
    out.push_str("# <file> <pct[0..11]> <mlock[0..11]> <ylock>\n");

    for (i, (file, _)) in state.products.iter().enumerate() {
        let fname = file
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        out.push_str(&fname);
        for m in 0..12 {
            out.push(' ');
            out.push_str(&state.monthly_pct[i][m].to_string());
        }
        for m in 0..12 {
            out.push(' ');
            out.push_str(if state.month_locked[i][m] { "1" } else { "0" });
        }
        out.push(' ');
        out.push_str(if state.yearly_locked[i] { "1" } else { "0" });
        out.push('\n');
    }

    out.push_str(&format!("min_workday_hours {}\n", state.settings.min_workday_hours));
    out.push_str(&format!("min_parallel {}\n", state.settings.min_parallel));
    out.push_str(&format!("min_monthly_net_profit {}\n", state.settings.min_monthly_net_profit));
    out.push_str(&format!("target_yearly_net_profit {}\n", state.settings.target_yearly_net_profit));
    out.push_str("month_workday");
    for m in 0..12 {
        out.push(' ');
        out.push_str(&state.month_overrides.workday[m].to_string());
    }
    out.push('\n');
    out.push_str("month_parallel");
    for m in 0..12 {
        out.push(' ');
        out.push_str(&state.month_overrides.parallel[m].to_string());
    }
    out.push('\n');
    out.push_str("month_net_profit");
    for m in 0..12 {
        out.push(' ');
        out.push_str(&state.month_overrides.net_profit[m].to_string());
    }
    out.push('\n');
    match state.period {
        Period::FullYear => out.push_str("period FullYear\n"),
        Period::Month(m) => out.push_str(&format!("period Month {}\n", m)),
    }

    let _ = std::fs::write(path, out);
}

/// State loaded from `.simulation_state`. Fields not present in the file (or
/// `None` if the file doesn't exist) fall back to defaults.
struct LoadedState {
    monthly_pct: Vec<[i64; 12]>,
    month_locked: Vec<[bool; 12]>,
    yearly_locked: Vec<bool>,
    settings: GlobalSettings,
    month_overrides: MonthOverrides,
    period: Period,
}

/// Try to load `.simulation_state` from `folder` and match its per-product
/// entries to the `products` list (by file name). Returns `None` if the file
/// doesn't exist or contains no usable data. Each month's percentages are
/// normalized to sum to 100 in case products were added/removed since the
/// state was saved. Backward-compatible with the v1 format (the old single
/// `workday_hours` / `parallel` / `monthly_goal` / `yearly_goal` /
/// `selected_month` keys are mapped onto the new global minimums + per-month
/// overrides).
fn load_state(folder: &Path, products: &[(PathBuf, ProductResult)]) -> Option<LoadedState> {
    let path = state_file_path(folder);
    let content = std::fs::read_to_string(&path).ok()?;
    let n = products.len();

    // Map file-name → index for matching stored entries to current products.
    let name_to_idx = |name: &str| -> Option<usize> {
        products.iter().position(|(f, _)| {
            f.file_name()
                .map(|s| s.to_string_lossy().as_ref() == name)
                .unwrap_or(false)
        })
    };

    let mut monthly_pct = vec![[0i64; 12]; n];
    let mut month_locked = vec![[false; 12]; n];
    let mut yearly_locked = vec![false; n];
    let mut settings = GlobalSettings {
        min_workday_hours: DEFAULT_MIN_WORKDAY_HOURS,
        min_parallel: DEFAULT_MIN_PARALLEL,
        min_monthly_net_profit: DEFAULT_MIN_MONTHLY_NET_PROFIT,
        target_yearly_net_profit: DEFAULT_TARGET_YEARLY_NET_PROFIT,
    };
    let mut month_overrides = MonthOverrides::default();
    let mut period = Period::FullYear;
    let mut found_any = false;

    // v1 backward-compat temporaries.
    let mut v1_selected_month: Option<usize> = None;

    let parse_arr12 = |rest: &str| -> Option<[i64; 12]> {
        let toks: Vec<&str> = rest.split_whitespace().collect();
        if toks.len() < 12 {
            return None;
        }
        let mut a = [0i64; 12];
        for m in 0..12 {
            a[m] = toks[m].parse().unwrap_or(0);
        }
        Some(a)
    };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // v2 settings lines.
        if let Some(rest) = line.strip_prefix("min_workday_hours ") {
            settings.min_workday_hours = rest.trim().parse().unwrap_or(settings.min_workday_hours);
            found_any = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("min_parallel ") {
            settings.min_parallel = rest.trim().parse().unwrap_or(settings.min_parallel);
            found_any = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("min_monthly_net_profit ") {
            settings.min_monthly_net_profit =
                rest.trim().parse().unwrap_or(settings.min_monthly_net_profit);
            found_any = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("target_yearly_net_profit ") {
            settings.target_yearly_net_profit =
                rest.trim().parse().unwrap_or(settings.target_yearly_net_profit);
            found_any = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("month_workday ") {
            if let Some(a) = parse_arr12(rest) {
                month_overrides.workday = a;
                found_any = true;
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("month_parallel ") {
            if let Some(a) = parse_arr12(rest) {
                month_overrides.parallel = a;
                found_any = true;
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("month_net_profit ") {
            if let Some(a) = parse_arr12(rest) {
                month_overrides.net_profit = a;
                found_any = true;
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("period ") {
            let r = rest.trim();
            period = if r == "FullYear" {
                Period::FullYear
            } else if let Some(m) = r.strip_prefix("Month ") {
                Period::Month(m.trim().parse::<usize>().unwrap_or(0).min(11))
            } else {
                Period::FullYear
            };
            found_any = true;
            continue;
        }
        // v1 backward-compat settings lines.
        if let Some(rest) = line.strip_prefix("workday_hours ") {
            settings.min_workday_hours = rest.trim().parse().unwrap_or(settings.min_workday_hours);
            found_any = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("parallel ") {
            settings.min_parallel = rest.trim().parse().unwrap_or(settings.min_parallel);
            found_any = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("monthly_goal ") {
            settings.min_monthly_net_profit =
                rest.trim().parse().unwrap_or(settings.min_monthly_net_profit);
            found_any = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("yearly_goal ") {
            settings.target_yearly_net_profit =
                rest.trim().parse().unwrap_or(settings.target_yearly_net_profit);
            found_any = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("selected_month ") {
            v1_selected_month = Some(rest.trim().parse::<usize>().unwrap_or(0).min(11));
            found_any = true;
            continue;
        }
        // Product line: "file pct[0..11] mlock[0..11] ylock" = 26 tokens.
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() >= 26 {
            if let Some(idx) = name_to_idx(tokens[0]) {
                for m in 0..12 {
                    monthly_pct[idx][m] = tokens[1 + m].parse().unwrap_or(0);
                }
                for m in 0..12 {
                    month_locked[idx][m] = tokens[13 + m] == "1";
                }
                yearly_locked[idx] = tokens[25] == "1";
                found_any = true;
            }
        }
    }

    if !found_any {
        return None;
    }

    // v1 backward-compat: if a selected_month was saved but no v2 `period`
    // line was present, restore that month as the active period.
    if matches!(period, Period::FullYear) {
        if let Some(m) = v1_selected_month {
            period = Period::Month(m);
        }
    }

    // Clamp overrides to the loaded minimums (the minimums may have changed).
    let mw = settings.min_workday_hours;
    let mp = settings.min_parallel;
    let mn = settings.min_monthly_net_profit;
    for m in 0..12 {
        if month_overrides.workday[m] < mw {
            month_overrides.workday[m] = mw;
        }
        if month_overrides.parallel[m] < mp {
            month_overrides.parallel[m] = mp;
        }
        if month_overrides.net_profit[m] < mn {
            month_overrides.net_profit[m] = mn;
        }
    }

    // Normalize each month to sum to exactly 100 (products may have been
    // added/removed since the state was saved).
    for m in 0..12 {
        let sum: i64 = monthly_pct.iter().map(|p| p[m]).sum();
        if sum != 100 {
            if sum > 0 {
                for i in 0..n {
                    monthly_pct[i][m] =
                        ((monthly_pct[i][m] as f64 / sum as f64) * 100.0).round() as i64;
                }
                let new_sum: i64 = monthly_pct.iter().map(|p| p[m]).sum();
                let diff = 100 - new_sum;
                if diff != 0 && n > 0 {
                    let mut applied = false;
                    for i in 0..n {
                        if !month_locked[i][m] && !yearly_locked[i] {
                            monthly_pct[i][m] += diff;
                            applied = true;
                            break;
                        }
                    }
                    if !applied {
                        monthly_pct[0][m] += diff;
                    }
                }
            } else if n > 0 {
                let base = 100 / n as i64;
                let extra = 100 - base * n as i64;
                for i in 0..n {
                    monthly_pct[i][m] = base + if (i as i64) < extra { 1 } else { 0 };
                }
            }
        }
    }

    Some(LoadedState {
        monthly_pct,
        month_locked,
        yearly_locked,
        settings,
        month_overrides,
        period,
    })
}

/// Build a settings slider for the given kind, reading its value / range from
/// `state`. Used by [`AppState::rebuild_sliders`].
fn make_settings_slider(kind: SliderKind, state: &AppState) -> Slider {
    let d = state.lang.dict();
    match kind {
        SliderKind::MinWorkdayHours => Slider {
            kind,
            label: d.tui_slider_min_workday.into(),
            value: state.settings.min_workday_hours,
            min: 1,
            max: 24,
            step: 1,
            suffix: " h",
            locked: false,
        },
        SliderKind::MinParallel => Slider {
            kind,
            label: d.tui_slider_min_parallel.into(),
            value: state.settings.min_parallel,
            min: 1,
            max: 200,
            step: 1,
            suffix: "",
            locked: false,
        },
        SliderKind::MinMonthlyNetProfit => Slider {
            kind,
            label: d.tui_slider_min_monthly_profit.into(),
            value: state.settings.min_monthly_net_profit,
            min: 0,
            max: 1_000_000,
            step: 100,
            suffix: "",
            locked: false,
        },
        SliderKind::TargetYearlyNetProfit => Slider {
            kind,
            label: d.tui_slider_target_yearly.into(),
            value: state.settings.target_yearly_net_profit,
            min: 0,
            max: 10_000_000,
            step: 1000,
            suffix: "",
            locked: false,
        },
        SliderKind::MonthWorkdayHours(m) => Slider {
            kind,
            label: d.tui_slider_month_workday.into(),
            value: state.month_overrides.workday[m].max(state.settings.min_workday_hours),
            min: state.settings.min_workday_hours.max(1),
            max: 24,
            step: 1,
            suffix: " h",
            locked: false,
        },
        SliderKind::MonthParallel(m) => {
            // The max is refined by `update_parallel_range`; start with a
            // generous upper bound so the slider is usable before that runs.
            // Uses the same 1-hour reference as `update_parallel_range` so the
            // cap does not depend on the current workday hours.
            let min_par = state.settings.min_parallel.max(1);
            let monthly_minutes: f64 =
                month_shares(state, m).iter().map(|s| s.monthly_minutes).sum();
            let max_par = ((monthly_minutes / 60.0).floor() as i64).max(min_par);
            let val = state.month_overrides.parallel[m].max(min_par).min(max_par);
            Slider {
                kind,
                label: lang::fmt(
                    d.tui_parallel_label,
                    &[&min_par.to_string(), &max_par.to_string()],
                ),
                value: val,
                min: min_par,
                max: max_par,
                step: 1,
                suffix: "",
                locked: false,
            }
        }
        SliderKind::MonthNetProfit(m) => Slider {
            kind,
            label: d.tui_slider_month_profit.into(),
            value: state
                .month_overrides
                .net_profit[m]
                .max(state.settings.min_monthly_net_profit),
            min: state.settings.min_monthly_net_profit.max(0),
            max: 1_000_000,
            step: 100,
            suffix: "",
            locked: false,
        },
        _ => Slider {
            kind,
            label: String::new(),
            value: 0,
            min: 0,
            max: 1,
            step: 1,
            suffix: "",
            locked: false,
        },
    }
}

impl AppState {
    /// Rebuild the flat `sliders` view from the current period.  Full Year:
    /// yearly % sliders + the 4 global min/target settings.  A Month: that
    /// month's monthly % sliders + the 3 per-month override settings.  Values
    /// are read from the state (percentages / settings / overrides).
    fn rebuild_sliders(&mut self) {
        let mut sliders: Vec<Slider> = Vec::new();
        match self.period {
            Period::FullYear => {
                for i in 0..self.products.len() {
                    sliders.push(Slider {
                        kind: SliderKind::YearlyPercent(i),
                        label: format!("% {}", self.products[i].1.name),
                        value: self.yearly_pct(i),
                        min: 0,
                        max: 100,
                        step: 1,
                        suffix: "%",
                        locked: self.yearly_locked[i],
                    });
                }
                for kind in [
                    SliderKind::MinWorkdayHours,
                    SliderKind::MinParallel,
                    SliderKind::MinMonthlyNetProfit,
                    SliderKind::TargetYearlyNetProfit,
                ] {
                    sliders.push(make_settings_slider(kind, self));
                }
            }
            Period::Month(m) => {
                for i in 0..self.products.len() {
                    let eff_locked = self.month_locked[i][m] || self.yearly_locked[i];
                    sliders.push(Slider {
                        kind: SliderKind::MonthPercent(i),
                        label: format!("% {}", self.products[i].1.name),
                        value: self.monthly_pct[i][m],
                        min: 0,
                        max: 100,
                        step: 1,
                        suffix: "%",
                        locked: eff_locked,
                    });
                }
                for kind in [
                    SliderKind::MonthWorkdayHours(m),
                    SliderKind::MonthParallel(m),
                    SliderKind::MonthNetProfit(m),
                ] {
                    sliders.push(make_settings_slider(kind, self));
                }
            }
        }
        self.sliders = sliders;
        // Keep `selected` in range.
        if self.selected >= self.sliders.len() {
            self.selected = 0;
        }
    }
}

/// Entry point: parse products, enter the alternate screen, run the loop.
pub fn run(folder: &Path, lang: &Lang) {
    let products = load_products(folder, lang);
    if products.is_empty() {
        eprintln!("{}", lang::fmt(lang.dict().tui_no_products, &[&folder.display().to_string()]));
        return;
    }

    let n = products.len();
    // Try to load saved state (percentages, locks, settings) from a previous
    // export. Falls back to equal distribution if no state file exists.
    let loaded = load_state(folder, &products);

    // Initial monthly distribution: equal split across products for every month.
    let base = 100 / n.max(1) as i64;
    let extra = 100 - base * n.max(1) as i64;
    let default_monthly: Vec<[i64; 12]> = (0..n)
        .map(|i| {
            let v = base + if (i as i64) < extra { 1 } else { 0 };
            [v; 12]
        })
        .collect();

    let (monthly_pct, month_locked, yearly_locked, period, settings, month_overrides) = match &loaded {
        Some(l) => (
            l.monthly_pct.clone(),
            l.month_locked.clone(),
            l.yearly_locked.clone(),
            l.period,
            l.settings,
            l.month_overrides.clone(),
        ),
        None => (
            default_monthly,
            vec![[false; 12]; n],
            vec![false; n],
            Period::FullYear,
            GlobalSettings {
                min_workday_hours: DEFAULT_MIN_WORKDAY_HOURS,
                min_parallel: DEFAULT_MIN_PARALLEL,
                min_monthly_net_profit: DEFAULT_MIN_MONTHLY_NET_PROFIT,
                target_yearly_net_profit: DEFAULT_TARGET_YEARLY_NET_PROFIT,
            },
            MonthOverrides::default(),
        ),
    };

    let mut state = AppState {
        sliders: Vec::new(),
        monthly_pct,
        month_locked,
        yearly_locked,
        period,
        folder: folder.to_path_buf(),
        products,
        selected: 0,
        scroll: 0,
        status: None,
        tab: Tab::Products,
        product_scroll: 0,
        lang: *lang,
        active_region: Region::Main,
        show_help: false,
        help_scroll: 0,
        settings,
        month_overrides,
    };
    state.rebuild_sliders();
    // Set the initial parallel-products cap range from the default goals.
    update_parallel_range(&mut state);

    enable_raw_mode().ok();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).ok();

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(_) => {
            disable_raw_mode().ok();
            return;
        }
    };

    loop {
        if terminal.draw(|f| draw(f, &mut state)).is_err() {
            break;
        }

        if !event::poll(std::time::Duration::from_millis(250)).unwrap_or(false) {
            continue;
        }
        let ev = match event::read() {
            Ok(ev) => ev,
            Err(_) => break,
        };
        if let Event::Key(k) = ev {
            if k.kind == KeyEventKind::Press && handle_key(&mut state, &k, lang) {
                break;
            }
        }
    }

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
}


#[cfg(test)]
#[path = "../test/tui_tests.rs"]
mod tests;
