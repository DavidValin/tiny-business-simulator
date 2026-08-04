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
    collect_txt_files, compute_product_shares, compute_result, parallel_range,
    write_result_file_monthly, write_totals_file_monthly, ProductResult,
};

/// Workdays assumed per month when deriving the production capacity in minutes.
const WORKDAYS_PER_MONTH: f64 = 22.0;

/// Localized month abbreviations for the current language.
fn months(lang: &Lang) -> [&'static str; 12] {
    lang.months_abbr()
}

// ---------------------------------------------------------------------------
// Slider model
// ---------------------------------------------------------------------------

/// Kinds of sliders shown in the sidebar.
///
/// `YearlyPercent(i)` — the product's yearly % (Products tab). The value is
/// *derived* (the mean of the 12 monthly %) and shown as a slider; editing it
/// propagates to every month where the product isn't month-locked.
///
/// `MonthPercent(i)` — the product's % for the currently-selected month
/// (Graph tab). Editing it only affects that month.
///
/// `MonthSelector` — the Jan..Dec `<select>` at the top of the Graph sidebar.
#[derive(Clone, Copy, PartialEq)]
enum SliderKind {
    YearlyPercent(usize),
    MonthPercent(usize),
    MonthSelector,
    WorkdayHours,
    Parallel,
    MonthlyGoal,
    YearlyGoal,
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

/// Human-readable value readout for a slider's track line. For the month
/// selector the readout is the month name rather than a raw number.
fn slider_readout(s: &Slider, lang: &Lang) -> String {
    match s.kind {
        SliderKind::MonthSelector => format!(" {}", lang.months_abbr()[s.value as usize]),
        _ => format!(" {}{}", s.value, s.suffix),
    }
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

/// Which of the two main-area tabs is active.  `Products` (the default, shown
/// first) lists the per-product simulation values (the same values written to
/// each `*.simulation_results.txt` on export) in a scrollable view; `Graph`
/// shows the 12-month chart.
#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Products,
    Graph,
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
    /// Currently-selected month for the Graph tab's `<select>` (0 = Jan).
    selected_month: usize,
    /// Flat slider list rebuilt whenever the tab or selected month changes.
    /// Yearly slider values are derived from `monthly_pct` (mean of the 12
    /// months); monthly slider values are `monthly_pct[p][selected_month]`.
    sliders: Vec<Slider>,
    selected: usize,
    /// Top visible entry index in the sidebar's scrollable slider list.
    scroll: usize,
    /// Folder the products were loaded from (for the totals export file).
    folder: PathBuf,
    /// Transient "exported to <path>" / error message shown in the footer.
    status: Option<String>,
    /// Active main-area tab.
    tab: Tab,
    /// Top visible line index of the Products scrollable view.
    product_scroll: usize,
    /// Interface language (used by the Products view to render the
    /// same per-product lines that are written to the export files).
    lang: Lang,
    /// Which region (main content vs sidebar) currently receives Up/Down.
    active_region: Region,
}

impl AppState {
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

    /// Compute one month's achievable (capacity-capped) sales totals for month
    /// `m`, using that month's percentage distribution.
    fn month_totals_for(&self, m: usize) -> MonthTotals {
        let monthly_goal = self.slider_value(SliderKind::MonthlyGoal) as f64;
        let workday_hours = self.slider_value(SliderKind::WorkdayHours).max(1) as f64;
        let parallel = self.slider_value(SliderKind::Parallel).max(1) as f64;

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

        let mut total_units = 0i64;
        let mut total_amount = 0.0f64;
        let mut total_profit = 0.0f64;
        let mut total_cost = 0.0f64;
        for (s, (_, p)) in req_sales.iter().zip(self.products.iter()) {
            let units = (*s as f64 * scale).floor() as i64;
            total_units += units;
            total_amount += units as f64 * p.price;
            total_profit += units as f64 * p.net_profit;
            total_cost += units as f64 * p.total_cost;
        }

        MonthTotals {
            units: total_units,
            amount: total_amount,
            profit: total_profit,
            cost: total_cost,
            required_minutes,
            capacity_minutes,
        }
    }

    /// Convenience: the selected month's totals (used by the chart title etc.).
    #[allow(dead_code)]
    fn month_totals(&self) -> MonthTotals {
        self.month_totals_for(self.selected_month)
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

    let sel = state.selected_month;
    let mt = &months[sel];
    let d = state.lang.dict();
    let mnames = state.lang.months_abbr();
    let stats = format!(
        "  |   {0}: {1:.0}   {2}: n={3} \u{00a4}={4:.0} ({5} {6:.0})   {7}: n={8} \u{00a4}={9:.0} ({10} {11:.0})",
        d.tui_axis_max, max_with_headroom,
        mnames[sel], mt.units, mt.amount, d.tui_profit, mt.profit,
        d.tui_yearly, yearly_units, yearly_amount, d.tui_profit, yearly_profit,
    );
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
        Span::raw(stats),
    ]);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let buf = frame.buffer_mut();

    // Axis-max label at the top-left of the inner area.
    let axis_label = format!("\u{2191} {} {:.0}", d.tui_max, max_with_headroom);
    let axis_w = axis_label.chars().count().min(inner.width as usize) as u16;
    buf.set_string(
        inner.x,
        inner.y,
        &axis_label,
        Style::default().fg(Color::Gray).add_modifier(Modifier::DIM),
    );
    let _ = axis_w;

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

    // Reserve 2 rows at the bottom for the bar labels (n/$) and month labels.
    let label_rows: u16 = 2;
    let bar_h = inner.height.saturating_sub(label_rows);
    if bar_h == 0 {
        return;
    }
    let bar_bottom = inner.y + bar_h - 1;

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
            render_bar_value(buf, n_x, bar_bottom, bar_width, &mt.units.to_string(), Color::Cyan);
        }
        let total_h = (amount / max * bar_h as f64).round() as i64;
        let total_h = total_h.clamp(0, bar_h as i64);
        let mut profit_h = (profit / max * bar_h as f64).round() as i64;
        profit_h = profit_h.clamp(0, total_h);
        if profit_h >= 1 {
            render_bar_value(buf, d_x, bar_bottom, bar_width, &format!("{:.0}", profit), Color::Green);
        }
        let cost_h = total_h - profit_h;
        // Total $ (sales amount) at the TOP of the $ bar.
        if total_h >= 1 {
            let y_top = bar_bottom.saturating_sub((total_h - 1) as u16);
            let top_bg = if cost_h >= 1 { Color::Yellow } else { Color::Green };
            render_bar_value(buf, d_x, y_top, bar_width, &format!("{:.0}", amount), top_bg);
        }

        // Bar labels (n / $) just under the bars.
        let n_label_x = n_x + bar_width / 2;
        let d_label_x = d_x + bar_width / 2;
        if bar_bottom + 1 < inner.y + inner.height {
            buf.set_string(n_label_x, bar_bottom + 1, "n", label_style);
            buf.set_string(d_label_x, bar_bottom + 1, "$", label_style);
        }
        // Month label centered under the group; the selected month is highlighted.
        let m = mnames[g as usize];
        let m_w = m.chars().count() as u16;
        let m_x = group_x + group_width.saturating_sub(m_w) / 2;
        if bar_bottom + 2 < inner.y + inner.height {
            let ms = if g as usize == state.selected_month {
                sel_month_style
            } else {
                month_style
            };
            buf.set_string(m_x, bar_bottom + 2, m, ms);
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

/// Render the two main-area tabs (`Graph`, `Product details`) across `area`.
/// The active tab is shown inverted/bold; the inactive one is dim.
fn render_tab_bar(frame: &mut ratatui::Frame, area: Rect, state: &AppState) {
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

/// Render the scrollable per-product details view, mirroring the lines written
/// to each product's `*.simulation_results.txt` by `simulator::write_result_file`:
/// product stats (name, sale price, total cost, net profit/unit, profit margin,
/// production time), the monthly and annual goal + sales + time breakdown, and
/// the shared workday / parallel settings.  All products are concatenated and
/// the view is scrollable with Up/Down while this tab is active.
fn render_product_details(frame: &mut ratatui::Frame, area: Rect, state: &mut AppState) {
    let active = state.active_region == Region::Main;
    let border_style = if active {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(state.lang.dict().tui_tab_products)
        .title_style(Style::default().add_modifier(Modifier::BOLD))
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
    let yearly_goal = state.slider_value(SliderKind::YearlyGoal) as f64;
    // Per-month shares; per-product annual sales = sum of the 12 months.
    let per_month: Vec<Vec<crate::simulator::ProductShare>> =
        (0..12).map(|m| month_shares(state, m)).collect();
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
        // Annual sales for this product = sum of its 12 monthly sales.
        let annual_sales: i64 = (0..12).map(|m| per_month[m][k].monthly_sales).sum();

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
    let workday_hours = state.slider_value(SliderKind::WorkdayHours);
    let parallel = state.slider_value(SliderKind::Parallel).max(1);
    // Per-month, per-product shares.
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
            let style = if m == state.selected_month { goal_style } else { month_style };
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

        // Shared workday / parallel settings.
        lines.push(Line::from(lang::fmt_aligned(
            d.result_workday,
            &[&workday_hours.to_string()],
            label_w,
        )));
        lines.push(Line::from(lang::fmt_aligned(
            d.result_parallel,
            &[&parallel.to_string()],
            label_w,
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


/// Render `text` centered within a `width`-cell-wide bar column at row `y`,
/// clipped to the column width.  Drawn as black bold text on a `bg` colored
/// cell so the value reads as embedded in the bar.
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
    let chars: Vec<char> = text.chars().take(width as usize).collect();
    if chars.is_empty() {
        return;
    }
    let s: String = chars.into_iter().collect();
    let w = s.chars().count() as u16;
    let start_x = x + (width.saturating_sub(w)) / 2;
    let style = Style::default()
        .fg(Color::Black)
        .bg(bg)
        .add_modifier(Modifier::BOLD);
    buf.set_string(start_x, y, &s, style);
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

    // --- Left column: Monthly + Settings ---
    let mut left: Vec<Line<'static>> = Vec::new();
    left.push(Line::from(Span::styled(d.tui_label_monthly, sub)));
    left.push(Line::from(vec![
        Span::styled(lbl(d.tui_label_sales, lw), val),
        Span::styled(format!("{}", t.monthly.sales), val),
    ]));
    left.push(Line::from(vec![
        Span::styled(lbl(d.tui_label_min, lw), val),
        Span::styled(format!("{:.0}", t.monthly.minutes), val),
    ]));
    left.push(Line::from(vec![
        Span::styled(lbl(d.tui_label_hours, lw), val),
        Span::styled(format!("{:.1}", t.monthly.hours), val),
    ]));
    left.push(Line::from(vec![
        Span::styled(lbl(d.tui_label_workdays, lw), val),
        Span::styled(format!("{:.2}", t.monthly.workdays), val),
    ]));
    left.push(Line::from(""));
    left.push(Line::from(Span::styled(d.tui_label_settings, sub)));
    left.push(Line::from(vec![
        Span::styled(lbl(d.tui_label_workday, lw), val),
        Span::styled(format!("{} {}", t.workday_hours, d.tui_suffix_hours), val),
    ]));
    left.push(Line::from(vec![
        Span::styled(lbl(d.tui_label_parallel, lw), val),
        Span::styled(format!("{}", t.parallel), val),
    ]));

    // --- Right column: Yearly + Yearly ref ---
    let mut right: Vec<Line<'static>> = Vec::new();
    right.push(Line::from(Span::styled(d.tui_label_yearly, sub)));
    right.push(Line::from(vec![
        Span::styled(lbl(d.tui_label_sales, lw), val),
        Span::styled(format!("{}", t.annual.sales), val),
    ]));
    right.push(Line::from(vec![
        Span::styled(lbl(d.tui_label_min, lw), val),
        Span::styled(format!("{:.0}", t.annual.minutes), val),
    ]));
    right.push(Line::from(vec![
        Span::styled(lbl(d.tui_label_hours, lw), val),
        Span::styled(format!("{:.1}", t.annual.hours), val),
    ]));
    right.push(Line::from(vec![
        Span::styled(lbl(d.tui_label_workdays, lw), val),
        Span::styled(format!("{:.2}", t.annual.workdays), val),
    ]));
    right.push(Line::from(""));
    // Yearly reference: 12 x monthly goal vs the yearly goal slider.
    let monthly = state.slider_value(SliderKind::MonthlyGoal);
    let yearly_target = state.slider_value(SliderKind::YearlyGoal);
    let year_sum = monthly * 12;
    let (mark, mark_style) = if year_sum >= yearly_target {
        ("\u{2714}", Style::default().fg(Color::Green))
    } else {
        ("\u{2716}", Style::default().fg(Color::Red))
    };
    right.push(Line::from(Span::styled(d.tui_label_yearly_ref, sub)));
    right.push(Line::from(vec![
        Span::styled(lbl(d.tui_label_12x_mo, lw), val),
        Span::styled(format!("{}", year_sum), val),
    ]));
    right.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(mark.to_string(), mark_style),
        Span::styled(format!(" {}  {}", d.tui_label_goal, yearly_target), val),
    ]));

    (left, right)
}

fn draw(frame: &mut ratatui::Frame, state: &mut AppState) {
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
    // Split the main area into a one-row tab bar and the content below it.
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(chart_area);
    render_tab_bar(frame, main_chunks[0], state);
    let content_area = main_chunks[1];
    match state.tab {
        Tab::Products => render_product_details(frame, content_area, state),
        Tab::Graph => render_chart(frame, content_area, state),
    }

    // Sidebar layout:
    //   Graph tab:    [Month selector (fixed, bordered)] [Products (scroll)]
    //                 [Settings] [Totals]
    //   Products tab: [Products (scroll)] [Settings] [Totals]
    let sidebar_area = body_chunks[1];
    let total_sliders = state.sliders.len();
    let n_products = state.products.len();
    let has_month_selector = state.tab == Tab::Graph;
    // On Graph tab slider 0 is MonthSelector; product sliders follow.
    let products_start = if has_month_selector { 1 } else { 0 };
    let settings_start = products_start + n_products;

    // Desired dynamic inner padding for every sidebar region.
    let sidebar_inner_w = sidebar_area.width.saturating_sub(2) as usize;
    let desired_pad = ((sidebar_inner_w / 12) as u16).min(4);

    let totals_region_h: u16 = 13;
    let settings_min_h: u16 = 7;
    // Month selector: 1 slider × 3 lines + 2 border = 5 rows.
    let month_selector_region_h: u16 = if has_month_selector { 5 } else { 0 };

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

    // Totals: both columns have the same fixed structure (9 lines each).
    let (totals_left, totals_right) = build_totals_columns(state);
    let totals_needed = totals_left.len().max(totals_right.len()) as u16;

    // Compute the padding each region can afford.
    let mut sidebar_pad: u16 = 0;
    let mut settings_region_h: u16 = settings_min_h;
    for &p in (0..=desired_pad).rev().collect::<Vec<_>>().iter() {
        let s_h = (2 + settings_needed + p * 2).max(settings_min_h);
        let products_available = sidebar_area
            .height
            .saturating_sub(s_h + totals_region_h + month_selector_region_h);
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
    let sidebar_chunks = if has_month_selector {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(month_selector_region_h),
                Constraint::Min(6),
                Constraint::Length(settings_region_h),
                Constraint::Length(totals_region_h),
            ])
            .split(sidebar_area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(6),
                Constraint::Length(settings_region_h),
                Constraint::Length(totals_region_h),
            ])
            .split(sidebar_area)
    };

    let sidebar_active = state.active_region == Region::Sidebar;
    let mut region_idx = 0usize;

    // --- Month selector region (Graph tab only) ---
    if has_month_selector {
        let ms_area = sidebar_chunks[region_idx];
        region_idx += 1;
        let ms_block = Block::default()
            .borders(Borders::ALL)
            .title(state.lang.dict().tui_sidebar_month)
            .title_style(Style::default().add_modifier(Modifier::BOLD))
            .border_style(if sidebar_active {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            });
        let ms_inner = ms_block.inner(ms_area);
        frame.render_widget(ms_block, ms_area);
        if ms_inner.height > 0 && ms_inner.width >= 1 {
            // 1-char right padding so content doesn't touch the right border.
            let ms_content_w = ms_inner.width.saturating_sub(1);
            let ms_lines = build_slider_lines(state, 0, 1, ms_content_w);
            let ms_content =
                Rect::new(ms_inner.x, ms_inner.y, ms_content_w, ms_inner.height);
            frame.render_widget(Paragraph::new(ms_lines), ms_content);
        }
    }

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
    let top_title: String = match state.tab {
        Tab::Products => state.lang.dict().tui_products_yearly.to_string(),
        Tab::Graph => lang::fmt(state.lang.dict().tui_month_pct_sales, &[state.lang.months_abbr()[state.selected_month]]),
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
    let settings_block = Block::default()
        .borders(Borders::ALL)
        .title(state.lang.dict().tui_sidebar_settings)
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
    let region_name = match state.active_region {
        Region::Main => d.tui_region_main,
        Region::Sidebar => d.tui_region_sidebar,
    };
    let footer_text = match &state.status {
        Some(msg) => lang::fmt(d.tui_footer_status, &[msg, region_name]),
        None => lang::fmt(d.tui_footer, &[region_name]),
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

fn handle_key(state: &mut AppState, key: &KeyEvent, lang: &Lang) -> bool {
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
    // Shift+Tab (BackTab): switch the main-area tab (Products <-> Graph) and
    // rebuild the sidebar slider list for the new tab.
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
                            let m = state.selected_month;
                            state.month_locked[p][m] = !state.month_locked[p][m];
                            state.rebuild_sliders();
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

/// Apply a ±`dir` step to the focused slider. Handles the month selector
/// (changes `selected_month` and rebuilds the Graph sidebar), yearly % sliders
/// (propagates to all months), monthly % sliders (redistributes within the
/// selected month), and plain settings sliders (simple inc/dec).
fn adjust_slider(state: &mut AppState, dir: i64) {
    let Some(s) = state.sliders.get(state.selected).cloned() else {
        return;
    };
    match s.kind {
        SliderKind::MonthSelector => {
            let v = (s.value + dir).clamp(0, 11);
            state.selected_month = v as usize;
            state.rebuild_sliders();
        }
        SliderKind::YearlyPercent(p) => {
            let target = s.value + dir * s.step;
            edit_yearly(state, p, target);
            state.rebuild_sliders();
        }
        SliderKind::MonthPercent(p) => {
            let m = state.selected_month;
            let target = state.monthly_pct[p][m] + dir * s.step;
            redistribute_month(state, m, p, target);
            state.rebuild_sliders();
        }
        _ => {
            if let Some(s) = state.sliders.get_mut(state.selected) {
                if dir < 0 {
                    s.dec();
                } else {
                    s.inc();
                }
            }
        }
    }
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
/// month's percentage distribution. Only the `monthly_*` fields of the
/// returned [`ProductShare`] are meaningful (annual fields are zeroed). The
/// sales counts are the *required* (uncapped) figures — capacity capping is
/// applied separately in [`AppState::month_totals_for`] for the chart.
fn month_shares(state: &AppState, m: usize) -> Vec<crate::simulator::ProductShare> {
    let monthly_goal = state.slider_value(SliderKind::MonthlyGoal) as f64;
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
}

/// All totals shown in the sidebar's bottom region (and written to the totals
/// file on export). `monthly` reflects the currently-selected month; `annual`
/// is the sum of all 12 months.
struct Totals {
    monthly: PeriodTotals,
    annual: PeriodTotals,
    workday_hours: i64,
    parallel: i64,
}

fn compute_totals(state: &AppState) -> Totals {
    let workday_hours = state.slider_value(SliderKind::WorkdayHours);
    let parallel = state.slider_value(SliderKind::Parallel).max(1);
    let wh = workday_hours as f64;
    let pp = parallel.max(1) as f64;

    // Selected month's totals.
    let sel = state.selected_month;
    let mshares = month_shares(state, sel);
    let mut m_sales = 0i64;
    let mut m_min = 0.0f64;
    for s in &mshares {
        m_sales += s.monthly_sales;
        m_min += s.monthly_minutes;
    }
    let m_hours = m_min / 60.0;

    // Annual = sum of all 12 months.
    let mut a_sales = 0i64;
    let mut a_min = 0.0f64;
    for m in 0..12 {
        for s in month_shares(state, m) {
            a_sales += s.monthly_sales;
            a_min += s.monthly_minutes;
        }
    }
    let a_hours = a_min / 60.0;

    Totals {
        monthly: PeriodTotals {
            sales: m_sales,
            minutes: m_min,
            hours: m_hours,
            workdays: m_hours / (wh * pp),
        },
        annual: PeriodTotals {
            sales: a_sales,
            minutes: a_min,
            hours: a_hours,
            workdays: a_hours / (wh * pp),
        },
        workday_hours,
        parallel,
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
    let workday_hours = state.slider_value(SliderKind::WorkdayHours);
    // Use the selected month as the representative monthly load, and the sum
    // of all 12 months as the annual load.
    let mshares = month_shares(state, state.selected_month);
    let total_monthly_minutes: f64 = mshares.iter().map(|s| s.monthly_minutes).sum();
    let mut total_annual_minutes = 0.0f64;
    for m in 0..12 {
        for s in month_shares(state, m) {
            total_annual_minutes += s.monthly_minutes;
        }
    }
    let (p_min, p_max) = parallel_range(total_monthly_minutes, total_annual_minutes, workday_hours);

    for s in state.sliders.iter_mut() {
        if let SliderKind::Parallel = s.kind {
            s.min = p_min;
            s.max = p_max;
            if s.value < p_min {
                s.value = p_min;
            }
            if s.value > p_max {
                s.value = p_max;
            }
            s.label = lang::fmt(state.lang.dict().tui_parallel_label, &[&p_min.to_string(), &p_max.to_string()]);
        }
    }
}

/// Write the per-product `*.simulation_results.txt` files (12 monthly rows +
/// annual sum) and the aggregate `totals.simulation_results.txt` (12 monthly
/// rows + annual) in `state.folder`. The current per-month percentages,
/// monthly/yearly goals, workday hours and parallel products drive the split.
/// Returns a human-readable status string.
fn export_results(state: &AppState, lang: &Lang) -> String {
    let workday_hours = state.slider_value(SliderKind::WorkdayHours);
    let parallel = state.slider_value(SliderKind::Parallel).max(1);

    // Per-month, per-product shares.
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
    out.push_str("# tiny-business-simulator state v1\n");
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

    let wh = state.slider_value(SliderKind::WorkdayHours);
    let par = state.slider_value(SliderKind::Parallel);
    let mg = state.slider_value(SliderKind::MonthlyGoal);
    let yg = state.slider_value(SliderKind::YearlyGoal);
    out.push_str(&format!("workday_hours {}\n", wh));
    out.push_str(&format!("parallel {}\n", par));
    out.push_str(&format!("monthly_goal {}\n", mg));
    out.push_str(&format!("yearly_goal {}\n", yg));
    out.push_str(&format!("selected_month {}\n", state.selected_month));

    let _ = std::fs::write(path, out);
}

/// State loaded from `.simulation_state`. Fields not present in the file (or
/// `None` if the file doesn't exist) fall back to defaults.
struct LoadedState {
    monthly_pct: Vec<[i64; 12]>,
    month_locked: Vec<[bool; 12]>,
    yearly_locked: Vec<bool>,
    workday_hours: i64,
    parallel: i64,
    monthly_goal: i64,
    yearly_goal: i64,
    selected_month: usize,
}

/// Try to load `.simulation_state` from `folder` and match its per-product
/// entries to the `products` list (by file name). Returns `None` if the file
/// doesn't exist or contains no usable data. Each month's percentages are
/// normalized to sum to 100 in case products were added/removed since the
/// state was saved.
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
    let mut workday_hours = 8i64;
    let mut parallel = 1i64;
    let mut monthly_goal = 1000i64;
    let mut yearly_goal = 12000i64;
    let mut selected_month = 0usize;
    let mut found_any = false;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Settings lines.
        if let Some(rest) = line.strip_prefix("workday_hours ") {
            workday_hours = rest.trim().parse().unwrap_or(workday_hours);
            found_any = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("parallel ") {
            parallel = rest.trim().parse().unwrap_or(parallel);
            found_any = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("monthly_goal ") {
            monthly_goal = rest.trim().parse().unwrap_or(monthly_goal);
            found_any = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("yearly_goal ") {
            yearly_goal = rest.trim().parse().unwrap_or(yearly_goal);
            found_any = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("selected_month ") {
            let v: usize = rest.trim().parse().unwrap_or(0);
            selected_month = v.min(11);
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
                    // Apply the rounding drift to the first non-locked product.
                    for i in 0..n {
                        if !month_locked[i][m] && !yearly_locked[i] {
                            monthly_pct[i][m] += diff;
                            break;
                        }
                    }
                }
            } else if n > 0 {
                // All zero: equal split.
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
        workday_hours,
        parallel,
        monthly_goal,
        yearly_goal,
        selected_month,
    })
}

/// Default settings slider for a given kind (used on first build before any
/// prior slider state exists).
fn default_settings_slider(kind: SliderKind, lang: &Lang) -> Slider {
    let d = lang.dict();
    match kind {
        SliderKind::WorkdayHours => Slider {
            kind,
            label: d.tui_slider_workday.into(),
            value: 8,
            min: 1,
            max: 24,
            step: 1,
            suffix: " h",
            locked: false,
        },
        SliderKind::Parallel => Slider {
            kind,
            label: d.tui_slider_parallel.into(),
            value: 1,
            min: 1,
            max: 200,
            step: 1,
            suffix: "",
            locked: false,
        },
        SliderKind::MonthlyGoal => Slider {
            kind,
            label: d.tui_slider_monthly_goal.into(),
            value: 1000,
            min: 0,
            max: 1_000_000,
            step: 100,
            suffix: "",
            locked: false,
        },
        SliderKind::YearlyGoal => Slider {
            kind,
            label: d.tui_slider_yearly_goal.into(),
            value: 12000,
            min: 0,
            max: 10_000_000,
            step: 1000,
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
    /// Rebuild the flat `sliders` view from the current tab + selected month.
    /// The top region holds yearly sliders (Products tab) or the month selector
    /// + monthly sliders (Graph tab); the bottom 4 sliders are always the
    /// settings, preserved from the previous build (so their values / the
    /// parallel slider's dynamic min/max/label survive across rebuilds).
    fn rebuild_sliders(&mut self) {
        let mut sliders: Vec<Slider> = Vec::new();
        match self.tab {
            Tab::Products => {
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
            }
            Tab::Graph => {
                sliders.push(Slider {
                    kind: SliderKind::MonthSelector,
                    label: self.lang.dict().tui_slider_month.into(),
                    value: self.selected_month as i64,
                    min: 0,
                    max: 11,
                    step: 1,
                    suffix: "",
                    locked: false,
                });
                let m = self.selected_month;
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
            }
        }
        // Preserve the 4 settings sliders from the previous build.
        for kind in [
            SliderKind::WorkdayHours,
            SliderKind::Parallel,
            SliderKind::MonthlyGoal,
            SliderKind::YearlyGoal,
        ] {
            if let Some(old) = self.sliders.iter().find(|s| s.kind == kind).cloned() {
                sliders.push(old);
            } else {
                sliders.push(default_settings_slider(kind, &self.lang));
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

    let (monthly_pct, month_locked, yearly_locked, selected_month) = match &loaded {
        Some(l) => (
            l.monthly_pct.clone(),
            l.month_locked.clone(),
            l.yearly_locked.clone(),
            l.selected_month,
        ),
        None => (default_monthly, vec![[false; 12]; n], vec![false; n], 0),
    };

    let mut state = AppState {
        sliders: Vec::new(),
        monthly_pct,
        month_locked,
        yearly_locked,
        selected_month,
        folder: folder.to_path_buf(),
        products,
        selected: 0,
        scroll: 0,
        status: None,
        tab: Tab::Products,
        product_scroll: 0,
        lang: *lang,
        active_region: Region::Main,
    };
    // Apply loaded settings (workday, parallel, goals) if present.
    if let Some(l) = &loaded {
        state.rebuild_sliders();
        for s in state.sliders.iter_mut() {
            match s.kind {
                SliderKind::WorkdayHours => s.value = l.workday_hours,
                SliderKind::Parallel => s.value = l.parallel,
                SliderKind::MonthlyGoal => s.value = l.monthly_goal,
                SliderKind::YearlyGoal => s.value = l.yearly_goal,
                _ => {}
            }
        }
    } else {
        state.rebuild_sliders();
    }
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Currency;

    #[allow(dead_code)]
    fn prod(name: &str, price: f64, cost: f64, dur: f64) -> ProductResult {
        ProductResult {
            name: name.into(),
            price,
            currency: Currency::new("USD"),
            total_cost: cost,
            net_profit: price - cost,
            profit_percent: 100.0,
            duration_minutes: dur,
        }
    }

    /// Wrap product results with dummy file paths for AppState construction.
    fn wrap(results: Vec<ProductResult>) -> Vec<(PathBuf, ProductResult)> {
        results
            .into_iter()
            .enumerate()
            .map(|(i, r)| (PathBuf::from(format!("p{}.txt", i)), r))
            .collect()
    }

    fn make_state(products: Vec<ProductResult>) -> AppState {
        let n = products.len();
        let wrapped = wrap(products);
        let base = 100 / n.max(1) as i64;
        let extra = 100 - base * n.max(1) as i64;
        let monthly_pct: Vec<[i64; 12]> = (0..n)
            .map(|i| {
                let v = base + if (i as i64) < extra { 1 } else { 0 };
                [v; 12]
            })
            .collect();
        let mut state = AppState {
            products: wrapped,
            monthly_pct,
            month_locked: vec![[false; 12]; n],
            yearly_locked: vec![false; n],
            selected_month: 0,
            sliders: Vec::new(),
            selected: 0,
            scroll: 0,
            folder: PathBuf::from("."),
            status: None,
            tab: Tab::Products,
            product_scroll: 0,
            lang: Lang::En,
            active_region: Region::Main,
        };
        state.rebuild_sliders();
        state
    }

    /// Set a settings slider value by kind (workday / parallel / goals).
    fn set_setting(state: &mut AppState, kind: SliderKind, value: i64) {
        if let Some(s) = state.sliders.iter_mut().find(|s| s.kind == kind) {
            s.value = value;
        }
    }

    /// Read a settings slider value by kind.
    #[allow(dead_code)]
    fn get_setting(state: &AppState, kind: SliderKind) -> i64 {
        state.slider_value(kind)
    }

    #[test]
    fn share_for_month_normalizes_percentages() {
        let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
        let state = make_state(products);
        // Equal 50/50 split -> equal shares in every month.
        for m in 0..12 {
            assert!((state.share_for_month(0, m) - 0.5).abs() < 1e-9);
            assert!((state.share_for_month(1, m) - 0.5).abs() < 1e-9);
        }
    }

    #[test]
    fn share_for_month_all_zero_splits_equally() {
        let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
        let mut state = make_state(products);
        // Zero every month for every product -> equal split fallback.
        for p in &mut state.monthly_pct {
            *p = [0; 12];
        }
        assert!((state.share_for_month(0, 0) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn yearly_pct_is_mean_of_months() {
        let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
        let mut state = make_state(products);
        // Make Jan 80/20 and the rest 50/50 -> mean for A = (80 + 11*50)/12.
        state.monthly_pct[0][0] = 80;
        state.monthly_pct[1][0] = 20;
        let expected: f64 = (80.0 + 11.0 * 50.0) / 12.0;
        assert!((state.yearly_pct(0) as f64 - expected.round()).abs() < 1e-9);
    }

    #[test]
    fn month_totals_scales_down_when_capacity_exceeds() {
        // One product, net profit 5, duration 60 min.  Goal 1000 -> 200 sales
        // -> 12000 required minutes.  With 1h/day, 1 parallel, 22 days ->
        // capacity = 1*22*60 = 1320 min.  Scale = 1320/12000 = 0.11 ->
        // floor(200*0.11) = 22 units, amount = 22*10 = 220.
        let products = vec![prod("A", 10.0, 5.0, 60.0)];
        let mut state = make_state(products);
        set_setting(&mut state, SliderKind::MonthlyGoal, 1000);
        set_setting(&mut state, SliderKind::WorkdayHours, 1);
        set_setting(&mut state, SliderKind::Parallel, 1);
        let mt = state.month_totals();
        assert_eq!(mt.units, 22);
        assert!((mt.amount - 220.0).abs() < 1e-9);
        assert!((mt.required_minutes - 12000.0).abs() < 1e-9);
        assert!((mt.capacity_minutes - 1320.0).abs() < 1e-9);
    }

    #[test]
    fn month_totals_unchanged_when_capacity_sufficient() {
        let products = vec![prod("A", 10.0, 5.0, 60.0)];
        let mut state = make_state(products);
        set_setting(&mut state, SliderKind::MonthlyGoal, 1000);
        set_setting(&mut state, SliderKind::WorkdayHours, 24);
        set_setting(&mut state, SliderKind::Parallel, 10);
        let mt = state.month_totals();
        assert_eq!(mt.units, 200);
        assert!((mt.amount - 2000.0).abs() < 1e-9);
        assert!((mt.profit - 1000.0).abs() < 1e-9);
        assert!((mt.cost - 1000.0).abs() < 1e-9);
    }

    #[test]
    fn month_totals_differ_per_month() {
        // Two products. Jan: A=80/B=20. Feb: A=20/B=80. With equal net profit
        // and duration, the unit totals are identical, but the per-product
        // mix differs — verify via month_shares that Jan and Feb differ.
        let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
        let mut state = make_state(products);
        set_setting(&mut state, SliderKind::MonthlyGoal, 1000);
        state.monthly_pct[0][0] = 80;
        state.monthly_pct[1][0] = 20;
        state.monthly_pct[0][1] = 20;
        state.monthly_pct[1][1] = 80;
        let jan = month_shares(&state, 0);
        let feb = month_shares(&state, 1);
        assert_ne!(jan[0].monthly_sales, feb[0].monthly_sales);
        assert_eq!(jan[0].monthly_sales, 160); // ceil(0.8*1000/5)
        assert_eq!(feb[0].monthly_sales, 40);  // ceil(0.2*1000/5)
    }

    #[test]
    fn slider_clamps_and_steps() {
        let mut s = Slider {
            kind: SliderKind::WorkdayHours,
            label: "x".into(),
            value: 8,
            min: 1,
            max: 24,
            step: 1,
            suffix: "",
            locked: false,
        };
        s.inc();
        assert_eq!(s.value, 9);
        for _ in 0..100 {
            s.inc();
        }
        assert_eq!(s.value, 24);
        for _ in 0..100 {
            s.dec();
        }
        assert_eq!(s.value, 1);
    }

    #[test]
    fn fit_bar_width_fills_available_width() {
        assert_eq!(fit_bar_width(130, 1, 2), 4);
        assert_eq!(fit_bar_width(58, 1, 2), 1);
        assert_eq!(fit_bar_width(10, 1, 2), 1);
    }

    #[test]
    fn slider_track_fills_proportionally() {
        let s = Slider {
            kind: SliderKind::WorkdayHours,
            label: "x".into(),
            value: 12,
            min: 0,
            max: 24,
            step: 1,
            suffix: "",
            locked: false,
        };
        let t = slider_track(&s, 10);
        assert_eq!(t.chars().count(), 10);
        let filled = t.chars().filter(|c| *c == '\u{2588}').count();
        assert_eq!(filled, 5);
    }

    #[test]
    fn initial_monthly_percentages_sum_to_100() {
        let products = vec![
            prod("A", 10.0, 5.0, 5.0),
            prod("B", 10.0, 5.0, 5.0),
            prod("C", 10.0, 5.0, 5.0),
        ];
        let state = make_state(products);
        for m in 0..12 {
            let sum: i64 = state.monthly_pct.iter().map(|p| p[m]).sum();
            assert_eq!(sum, 100, "month {} sums to {}", m, sum);
        }
    }

    // --- redistribute_month (within a single month) -------------------------

    #[test]
    fn redistribute_month_keeps_total_at_100() {
        // 50/30/20. Raise A to 60 -> remainder 40 split EQUALLY across the two
        // non-locked others (including zeros): 20 / 20.
        let products = vec![
            prod("A", 10.0, 5.0, 5.0),
            prod("B", 10.0, 5.0, 5.0),
            prod("C", 10.0, 5.0, 5.0),
        ];
        let mut state = make_state(products);
        state.monthly_pct[0][0] = 50;
        state.monthly_pct[1][0] = 30;
        state.monthly_pct[2][0] = 20;
        redistribute_month(&mut state, 0, 0, 60);
        assert_eq!(state.monthly_pct[0][0], 60);
        assert_eq!(state.monthly_pct[1][0], 20);
        assert_eq!(state.monthly_pct[2][0], 20);
        let sum: i64 = state.monthly_pct.iter().map(|p| p[0]).sum();
        assert_eq!(sum, 100);
    }

    #[test]
    fn redistribute_month_includes_zero_products() {
        // A=50, B=30, C=0. Raise A to 60 -> remainder 40 split equally across
        // ALL non-locked others (including C at 0): B=20, C=20. (New behaviour:
        // zeros are receivers, unlike the old yearly-only redistribute.)
        let products = vec![
            prod("A", 10.0, 5.0, 5.0),
            prod("B", 10.0, 5.0, 5.0),
            prod("C", 10.0, 5.0, 5.0),
        ];
        let mut state = make_state(products);
        state.monthly_pct[0][0] = 50;
        state.monthly_pct[1][0] = 30;
        state.monthly_pct[2][0] = 0;
        redistribute_month(&mut state, 0, 0, 60);
        assert_eq!(state.monthly_pct[0][0], 60);
        assert_eq!(state.monthly_pct[1][0], 20);
        assert_eq!(state.monthly_pct[2][0], 20);
    }

    #[test]
    fn redistribute_month_freezes_locked_product() {
        // A=50, B=30, C=20. Lock B (month-lock) at 30, raise A to 60: remainder
        // 10 goes only to the non-locked C.
        let products = vec![
            prod("A", 10.0, 5.0, 5.0),
            prod("B", 10.0, 5.0, 5.0),
            prod("C", 10.0, 5.0, 5.0),
        ];
        let mut state = make_state(products);
        state.monthly_pct[0][0] = 50;
        state.monthly_pct[1][0] = 30;
        state.monthly_pct[2][0] = 20;
        state.month_locked[1][0] = true;
        redistribute_month(&mut state, 0, 0, 60);
        assert_eq!(state.monthly_pct[0][0], 60);
        assert_eq!(state.monthly_pct[1][0], 30); // frozen
        assert_eq!(state.monthly_pct[2][0], 10); // absorbed the whole remainder
    }

    #[test]
    fn redistribute_month_clamped_by_locked_room() {
        // Lock B=30 and C=50 (locked_sum=80). Raise A above the available 20:
        // it clamps to 20 so the sum stays exactly 100.
        let products = vec![
            prod("A", 10.0, 5.0, 5.0),
            prod("B", 10.0, 5.0, 5.0),
            prod("C", 10.0, 5.0, 5.0),
        ];
        let mut state = make_state(products);
        state.monthly_pct[0][0] = 20;
        state.monthly_pct[1][0] = 30;
        state.monthly_pct[2][0] = 50;
        state.month_locked[1][0] = true;
        state.month_locked[2][0] = true;
        redistribute_month(&mut state, 0, 0, 90);
        assert_eq!(state.monthly_pct[0][0], 20);
        assert_eq!(state.monthly_pct[1][0], 30);
        assert_eq!(state.monthly_pct[2][0], 50);
        let sum: i64 = state.monthly_pct.iter().map(|p| p[0]).sum();
        assert_eq!(sum, 100);
    }

    #[test]
    fn redistribute_month_locked_changed_is_noop() {
        let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
        let mut state = make_state(products);
        state.month_locked[0][0] = true;
        let before = state.monthly_pct[0][0];
        redistribute_month(&mut state, 0, 0, 80);
        assert_eq!(state.monthly_pct[0][0], before);
    }

    #[test]
    fn redistribute_month_yearly_locked_is_frozen() {
        // Yearly-lock on B freezes it in every month.
        let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
        let mut state = make_state(products);
        state.yearly_locked[1] = true;
        let before = state.monthly_pct[1][3];
        redistribute_month(&mut state, 3, 0, 60);
        assert_eq!(state.monthly_pct[1][3], before);
        // B is frozen at 50, so A is clamped to 100 - 50 = 50 (not 60).
        assert_eq!(state.monthly_pct[0][3], 50);
        assert_eq!(state.monthly_pct[1][3], 50);
    }

    // --- edit_yearly (propagation across all 12 months) ---------------------

    #[test]
    fn edit_yearly_propagates_to_all_months() {
        // 2 products 50/50. Edit yearly A to 80: every month becomes A=80/B=20
        // (no month-locks), and the yearly mean recompute = 80.
        let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
        let mut state = make_state(products);
        edit_yearly(&mut state, 0, 80);
        for m in 0..12 {
            assert_eq!(state.monthly_pct[0][m], 80, "month {}", m);
            assert_eq!(state.monthly_pct[1][m], 20, "month {}", m);
        }
        assert_eq!(state.yearly_pct(0), 80);
    }

    #[test]
    fn edit_yearly_skips_month_locked_months() {
        // Lock A in month 3 at 50. Edit yearly A to 80: month 3 stays 50, the
        // other 11 months become 80. Yearly mean = (11*80 + 50)/12 ≈ 77.5,
        // NOT 80 — the "unless locked" behaviour.
        let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
        let mut state = make_state(products);
        state.month_locked[0][3] = true;
        edit_yearly(&mut state, 0, 80);
        assert_eq!(state.monthly_pct[0][3], 50); // month-locked, unchanged
        for m in 0..12 {
            if m != 3 {
                assert_eq!(state.monthly_pct[0][m], 80, "month {}", m);
            }
        }
        let expected: f64 = (11.0 * 80.0 + 50.0) / 12.0;
        assert!((state.yearly_pct(0) as f64 - expected.round()).abs() < 1e-9);
        assert_ne!(state.yearly_pct(0), 80);
    }

    #[test]
    fn edit_yearly_locked_product_is_noop() {
        let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
        let mut state = make_state(products);
        state.yearly_locked[0] = true;
        let before: Vec<[i64; 12]> = state.monthly_pct.clone();
        edit_yearly(&mut state, 0, 80);
        assert_eq!(state.monthly_pct, before);
    }

    #[test]
    fn monthly_edit_recomputes_yearly_as_mean() {
        // 2 products 50/50. Edit month 0 A to 80 -> month 0 = 80/20, other
        // months stay 50/50. Yearly A = (80 + 11*50)/12 ≈ 52.5 -> 52 (rounded).
        let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
        let mut state = make_state(products);
        state.selected_month = 0;
        redistribute_month(&mut state, 0, 0, 80);
        let expected: f64 = (80.0 + 11.0 * 50.0) / 12.0;
        assert!((state.yearly_pct(0) as f64 - expected.round()).abs() < 1e-9);
    }

    #[test]
    fn yearly_lock_renders_month_checkbox_checked_and_greyed() {
        // When a product is yearly-locked, its monthly slider in the Graph tab
        // must render locked (checked) regardless of month_locked.
        let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
        let mut state = make_state(products);
        state.yearly_locked[0] = true;
        state.tab = Tab::Graph;
        state.rebuild_sliders();
        let a_month = state
            .sliders
            .iter()
            .find(|s| matches!(s.kind, SliderKind::MonthPercent(0)))
            .unwrap();
        assert!(a_month.locked, "monthly slider must be locked when yearly-locked");
        // Toggling its month-lock via Space must be ignored (yearly-locked).
        // (Covered by handle_key logic; here we just assert the effective lock.)
    }

    // --- parallel range / totals / export -----------------------------------

    #[test]
    fn parallel_slider_range_caps_to_workday_budget() {
        let products = vec![prod("A", 10.0, 5.0, 60.0)];
        let mut state = make_state(products);
        set_setting(&mut state, SliderKind::MonthlyGoal, 1000);
        set_setting(&mut state, SliderKind::YearlyGoal, 12000);
        set_setting(&mut state, SliderKind::WorkdayHours, 8);
        set_setting(&mut state, SliderKind::Parallel, 1);
        update_parallel_range(&mut state);
        let p = state
            .sliders
            .iter()
            .find(|s| s.kind == SliderKind::Parallel)
            .unwrap();
        assert_eq!(p.min, 1);
        assert_eq!(p.max, 25);
        assert!(p.value >= p.min && p.value <= p.max);
    }

    #[test]
    fn parallel_slider_clamps_when_goal_raises_min() {
        let products = vec![prod("A", 10.0, 5.0, 60.0)];
        let mut state = make_state(products);
        set_setting(&mut state, SliderKind::MonthlyGoal, 100000);
        set_setting(&mut state, SliderKind::YearlyGoal, 0);
        set_setting(&mut state, SliderKind::WorkdayHours, 1);
        set_setting(&mut state, SliderKind::Parallel, 1);
        update_parallel_range(&mut state);
        let p = state
            .sliders
            .iter()
            .find(|s| s.kind == SliderKind::Parallel)
            .unwrap();
        assert_eq!(p.min, 667);
        assert_eq!(p.value, 667);
        assert!(p.max >= p.min);
    }

    #[test]
    fn compute_totals_monthly_and_annual() {
        // Two products, equal 50/50 split, monthly goal 1000, all 12 months
        // equal. A: net 5, dur 5 -> 100 sales / 500 min. B: net 10, dur 10 ->
        // 50 sales / 500 min. Monthly totals: 150 sales, 1000 min. Annual =
        // 12 * monthly (all months equal).
        let products = vec![prod("A", 6.0, 1.0, 5.0), prod("B", 12.0, 2.0, 10.0)];
        let mut state = make_state(products);
        set_setting(&mut state, SliderKind::MonthlyGoal, 1000);
        set_setting(&mut state, SliderKind::YearlyGoal, 12000);
        set_setting(&mut state, SliderKind::WorkdayHours, 8);
        set_setting(&mut state, SliderKind::Parallel, 1);
        let t = compute_totals(&state);
        assert_eq!(t.monthly.sales, 150);
        assert!((t.monthly.minutes - 1000.0).abs() < 1e-6);
        assert_eq!(t.annual.sales, 150 * 12);
        assert!((t.annual.minutes - 12000.0).abs() < 1e-6);
        assert_eq!(t.workday_hours, 8);
        assert_eq!(t.parallel, 1);
    }

    #[test]
    fn export_results_writes_12_monthly_rows_and_totals() {
        let dir = std::env::temp_dir().join("tui_export_test_monthly");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let products = vec![
            prod("Coffee", 4.5, 1.15, 5.0),
            prod("Tea", 3.0, 0.8, 4.0),
        ];
        let mut paths: Vec<(PathBuf, ProductResult)> = Vec::new();
        for (i, r) in products.into_iter().enumerate() {
            let f = dir.join(format!("p{}.txt", i));
            std::fs::write(&f, "+ stub\n").unwrap();
            paths.push((f, r));
        }
        let n = paths.len();
        let base = 100 / n as i64;
        let extra = 100 - base * n as i64;
        let monthly_pct: Vec<[i64; 12]> = (0..n)
            .map(|i| {
                let v = base + if (i as i64) < extra { 1 } else { 0 };
                [v; 12]
            })
            .collect();
        let mut state = AppState {
            products: paths,
            folder: dir.clone(),
            monthly_pct,
            month_locked: vec![[false; 12]; n],
            yearly_locked: vec![false; n],
            selected_month: 0,
            sliders: Vec::new(),
            selected: 0,
            scroll: 0,
            status: None,
            tab: Tab::Products,
            product_scroll: 0,
            lang: Lang::En,
            active_region: Region::Main,
        };
        state.rebuild_sliders();
        set_setting(&mut state, SliderKind::MonthlyGoal, 500);

        let status = export_results(&state, &Lang::En);
        assert!(status.contains("exported"), "status was: {}", status);

        let product_file = std::fs::read_to_string(dir.join("p0.simulation_results.txt")).unwrap();
        // All 12 month abbreviations appear in the per-product file.
        for m in ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"] {
            assert!(product_file.contains(m), "per-product file missing month {}, was:\n{}", m, product_file);
        }
        assert!(product_file.contains("Annual goal"), "missing annual row");

        let totals_file = std::fs::read_to_string(dir.join("totals.simulation_results.txt")).unwrap();
        for m in ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"] {
            assert!(totals_file.contains(m), "totals file missing month {}, was:\n{}", m, totals_file);
        }
        assert!(totals_file.contains("Total annual sales"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Find the leftmost x at row `y` whose horizontal run of cells matches
    /// `needle`.
    fn find_in_row(buf: &ratatui::buffer::Buffer, y: u16, needle: &str) -> Option<u16> {
        let chars: Vec<char> = needle.chars().collect();
        let w = buf.area.width;
        if y >= buf.area.height {
            return None;
        }
        for x in 0..w.saturating_sub(chars.len() as u16) {
            let mut ok = true;
            for (i, c) in chars.iter().enumerate() {
                if buf[((x + i as u16), y)].symbol() != &c.to_string() {
                    ok = false;
                    break;
                }
            }
            if ok {
                return Some(x);
            }
        }
        None
    }

    #[test]
    fn settings_renders_two_columns_side_by_side() {
        use ratatui::backend::TestBackend;
        let products = vec![
            prod("A", 10.0, 5.0, 5.0),
            prod("B", 10.0, 5.0, 5.0),
        ];
        let mut state = make_state(products);
        update_parallel_range(&mut state);

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut state)).unwrap();
        let buf = terminal.backend().buffer().clone();

        let settings_y = (0..buf.area.height)
            .find_map(|y| find_in_row(&buf, y, "Settings").map(|_| y))
            .expect("Settings title not rendered");
        let header = (settings_y..buf.area.height)
            .find_map(|y| {
                let w = find_in_row(&buf, y, "Workday")?;
                let m = find_in_row(&buf, y, "Monthly")?;
                Some((y, w, m))
            })
            .expect("Workday/Monthly not rendered side by side in Settings");
        let (_, workday_x, monthly_x) = header;
        assert!(monthly_x > workday_x);

        let y = header.0;
        let mut found_sep = false;
        for x in (workday_x + 1)..monthly_x {
            if buf[(x, y)].symbol() == "\u{2502}" {
                found_sep = true;
                break;
            }
        }
        assert!(found_sep, "no vertical separator between the two settings columns");
    }

    #[test]
    fn totals_renders_two_columns_side_by_side() {
        use ratatui::backend::TestBackend;
        let products = vec![
            prod("A", 10.0, 5.0, 5.0),
            prod("B", 10.0, 5.0, 5.0),
        ];
        let mut state = make_state(products);
        update_parallel_range(&mut state);

        let backend = TestBackend::new(80, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut state)).unwrap();
        let buf = terminal.backend().buffer().clone();

        let totals_y = (0..buf.area.height)
            .find_map(|y| find_in_row(&buf, y, "Totals").map(|_| y))
            .expect("Totals title not rendered");
        let header_y = (totals_y..buf.area.height)
            .find_map(|y| {
                let m = find_in_row(&buf, y, "Monthly")?;
                let yr = find_in_row(&buf, y, "Yearly")?;
                Some((y, m, yr))
            })
            .expect("Monthly/Yearly headers not rendered side by side in Totals");
        let (_, monthly_x, yearly_x) = header_y;
        assert!(yearly_x > monthly_x);

        let y = header_y.0;
        let mut found_sep = false;
        for x in (monthly_x + 1)..yearly_x {
            if buf[(x, y)].symbol() == "\u{2502}" {
                found_sep = true;
                break;
            }
        }
        assert!(found_sep, "no vertical separator between the two totals columns");
    }

    /// Verify the sidebar padding is clamped so all product rows fit: with many
    /// products on a short terminal, the padding must drop to 0 so every row is
    /// visible.
    #[test]
    fn sidebar_padding_clamps_to_fit_all_product_rows() {
        use ratatui::backend::TestBackend;
        let products: Vec<ProductResult> = (0..10)
            .map(|i| prod(&format!("P{}", i), 10.0, 5.0, 5.0))
            .collect();
        let mut state = make_state(products);
        update_parallel_range(&mut state);

        let backend = TestBackend::new(80, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut state)).unwrap();
        let buf = terminal.backend().buffer().clone();

        let products_y = (0..buf.area.height)
            .find_map(|y| find_in_row(&buf, y, "Products").map(|_| y))
            .expect("Products title not rendered");
        let p0_row = (products_y + 1..buf.area.height)
            .find_map(|y| find_in_row(&buf, y, "% P0").map(|_| y))
            .expect("% P0 not rendered");
        assert_eq!(
            p0_row, products_y + 1,
            "padding should be 0 when products don't fit otherwise, got p0_row={} products_y={}",
            p0_row, products_y
        );
    }

    #[test]
    fn graph_sidebar_renders_month_selector() {
        use ratatui::backend::TestBackend;
        let products = vec![
            prod("A", 10.0, 5.0, 5.0),
            prod("B", 10.0, 5.0, 5.0),
        ];
        let mut state = make_state(products);
        state.tab = Tab::Graph;
        state.selected_month = 5; // Jun
        state.rebuild_sliders();
        update_parallel_range(&mut state);

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut state)).unwrap();
        let buf = terminal.backend().buffer().clone();

        // The Graph sidebar's top-block title carries the selected month name.
        assert!(
            find_in_row(&buf, 0, "Jun (% sales)").is_some()
                || (0..buf.area.height).any(|y| find_in_row(&buf, y, "Jun").is_some()),
            "selected month Jun not rendered in Graph sidebar title"
        );
        // The month selector slider label "Month" is rendered.
        assert!(
            (0..buf.area.height).any(|y| find_in_row(&buf, y, "Month").is_some()),
            "Month selector not rendered"
        );
    }

    #[test]
    fn save_then_load_state_roundtrip() {
        let dir = std::env::temp_dir().join("tui_state_roundtrip_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Two product definition files.
        let products = vec![
            prod("Coffee", 4.5, 1.15, 5.0),
            prod("Tea", 3.0, 0.8, 4.0),
        ];
        let mut paths: Vec<(PathBuf, ProductResult)> = Vec::new();
        for (i, r) in products.into_iter().enumerate() {
            let f = dir.join(format!("p{}.txt", i));
            std::fs::write(&f, "+ stub\n").unwrap();
            paths.push((f, r));
        }
        let n = paths.len();
        let base = 100 / n as i64;
        let extra = 100 - base * n as i64;
        let monthly_pct: Vec<[i64; 12]> = (0..n)
            .map(|i| {
                let v = base + if (i as i64) < extra { 1 } else { 0 };
                [v; 12]
            })
            .collect();
        let mut state = AppState {
            products: paths.clone(),
            folder: dir.clone(),
            monthly_pct,
            month_locked: vec![[false; 12]; n],
            yearly_locked: vec![false; n],
            selected_month: 3,
            sliders: Vec::new(),
            selected: 0,
            scroll: 0,
            status: None,
            tab: Tab::Products,
            product_scroll: 0,
            lang: Lang::En,
            active_region: Region::Main,
        };
        state.rebuild_sliders();

        // Customize: product 0 = 80%, product 1 = 20% in Jan (month 0).
        state.monthly_pct[0][0] = 80;
        state.monthly_pct[1][0] = 20;
        // Lock product 0 in month 3.
        state.month_locked[0][3] = true;
        // Yearly-lock product 1.
        state.yearly_locked[1] = true;
        // Change a setting.
        for s in state.sliders.iter_mut() {
            if s.kind == SliderKind::MonthlyGoal {
                s.value = 500;
            }
            if s.kind == SliderKind::WorkdayHours {
                s.value = 6;
            }
        }

        // Save state.
        save_state(&state);

        // Load state back.
        let loaded = load_state(&dir, &paths).expect("state should load");
        assert_eq!(loaded.monthly_pct[0][0], 80);
        assert_eq!(loaded.monthly_pct[1][0], 20);
        assert!(loaded.month_locked[0][3]);
        assert!(loaded.yearly_locked[1]);
        assert_eq!(loaded.selected_month, 3);
        assert_eq!(loaded.monthly_goal, 500);
        assert_eq!(loaded.workday_hours, 6);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_state_returns_none_when_no_file() {
        let dir = std::env::temp_dir().join("tui_state_nofile_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let products = vec![prod("A", 10.0, 5.0, 5.0)];
        let paths = wrap(products);
        assert!(load_state(&dir, &paths).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_state_normalizes_when_product_added() {
        let dir = std::env::temp_dir().join("tui_state_normalize_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Save state with 2 products (p0=80, p1=20 in Jan).
        std::fs::write(dir.join("p0.txt"), "+ stub\n").unwrap();
        std::fs::write(dir.join("p1.txt"), "+ stub\n").unwrap();
        std::fs::write(
            dir.join(".simulation_state"),
            "p0.txt 80 80 80 80 80 80 80 80 80 80 80 80 0 0 0 0 0 0 0 0 0 0 0 0 0\n\
             p1.txt 20 20 20 20 20 20 20 20 20 20 20 20 0 0 0 0 0 0 0 0 0 0 0 0 1\n",
        )
        .unwrap();

        // Now load with 3 products (p2 was added since the save).
        let products = vec![
            prod("A", 10.0, 5.0, 5.0),
            prod("B", 10.0, 5.0, 5.0),
            prod("C", 10.0, 5.0, 5.0),
        ];
        let paths: Vec<(PathBuf, ProductResult)> = products
            .into_iter()
            .enumerate()
            .map(|(i, r)| (dir.join(format!("p{}.txt", i)), r))
            .collect();
        // Create p2.txt so the file exists.
        std::fs::write(dir.join("p2.txt"), "+ stub\n").unwrap();

        let loaded = load_state(&dir, &paths).expect("state should load");
        // p0 and p1 keep their saved percentages.
        assert_eq!(loaded.monthly_pct[0][0], 80);
        assert_eq!(loaded.monthly_pct[1][0], 20);
        // p2 was not in the save → 0. After normalization the month sums to
        // 100 (80+20+0=100, already correct).
        assert_eq!(loaded.monthly_pct[2][0], 0);
        let sum: i64 = loaded.monthly_pct.iter().map(|p| p[0]).sum();
        assert_eq!(sum, 100);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
