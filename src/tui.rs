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
    collect_txt_files, compute_product_shares, compute_result, parallel_range, write_result_file,
    write_totals_file, ProductResult,
};

/// Workdays assumed per month when deriving the production capacity in minutes.
const WORKDAYS_PER_MONTH: f64 = 22.0;

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

// ---------------------------------------------------------------------------
// Slider model
// ---------------------------------------------------------------------------

/// Kinds of sliders shown in the sidebar.  The product-percentage sliders are
/// identified by the product index.
#[derive(Clone, Copy, PartialEq)]
enum SliderKind {
    Percent(usize),
    WorkdayHours,
    Parallel,
    MonthlyGoal,
    YearlyGoal,
}

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

    fn percent_for(&self, idx: usize) -> i64 {
        self.slider_value(SliderKind::Percent(idx))
    }

    /// Normalized share (0..=1) for product `idx`, from the raw percentage
    /// sliders.  If every slider is zero the products are split equally.
    fn share_for(&self, idx: usize) -> f64 {
        let total: i64 = (0..self.products.len()).map(|i| self.percent_for(i).max(0)).sum();
        if total <= 0 {
            return 1.0 / self.products.len() as f64;
        }
        self.percent_for(idx).max(0) as f64 / total as f64
    }

    /// Compute one month's achievable (capacity-capped) sales totals.
    ///
    /// `profit` is the portion of `amount` that is net profit and `cost` the
    /// portion that is total cost (`amount = profit + cost`).
    fn month_totals(&self) -> MonthTotals {
        let monthly_goal = self.slider_value(SliderKind::MonthlyGoal) as f64;
        let workday_hours = self.slider_value(SliderKind::WorkdayHours).max(1) as f64;
        let parallel = self.slider_value(SliderKind::Parallel).max(1) as f64;

        let capacity_minutes = workday_hours * WORKDAYS_PER_MONTH * 60.0 * parallel;

        // Required sales per product from the goal split.
        let mut req_sales: Vec<i64> = Vec::with_capacity(self.products.len());
        let mut required_minutes = 0.0;
        for (i, (_, p)) in self.products.iter().enumerate() {
            let target_profit = self.share_for(i) * monthly_goal;
            let s = if p.net_profit > 0.0 {
                ((target_profit / p.net_profit).ceil() as i64).max(0)
            } else {
                0
            };
            req_sales.push(s);
            required_minutes += s as f64 * p.duration_minutes;
        }

        // Capacity cap: if the required production time exceeds what the
        // workday/parallel setup can deliver in a month, scale sales down.
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
    let mt = state.month_totals();
    let units = mt.units as f64;
    let amount = mt.amount;
    let profit = mt.profit;

    let max_val = units.max(amount).max(1.0);
    let max_with_headroom = (max_val * 1.25).max(max_val + 1.0);

    let yearly_units = mt.units * 12;
    let yearly_amount = amount * 12.0;
    let title = format!(
        "Yearly sales  \u{25a0} units (n)   \u{25a0} amount ($)   \u{25a0} profit ($)   |   axis max: {:.0}   monthly: n={} \u{00a4}={:.0} (profit {:.0})   yearly: n={} \u{00a4}={:.0}",
        max_with_headroom, mt.units, amount, profit, yearly_units, yearly_amount,
    );
    let active = state.active_region == Region::Main;
    let border_style = if active {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().add_modifier(Modifier::BOLD))
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let buf = frame.buffer_mut();

    // Axis-max label at the top-left of the inner area.
    let axis_label = format!("\u{2191} max {:.0}", max_with_headroom);
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
    let bar_width = fit_bar_width(inner.width, bar_gap, group_gap);
    let group_width = 2 * bar_width + bar_gap;

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

    let max = max_with_headroom;

    for g in 0..12u16 {
        let group_x = inner.x + g * (group_width + group_gap);
        let n_x = group_x;
        let d_x = group_x + bar_width + bar_gap;

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
        let cost_h = total_h - profit_h;
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
        // Month label centered under the group.
        let m = MONTHS[g as usize];
        let m_w = m.chars().count() as u16;
        let m_x = group_x + group_width.saturating_sub(m_w) / 2;
        if bar_bottom + 2 < inner.y + inner.height {
            buf.set_string(m_x, bar_bottom + 2, m, month_style);
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
    let tabs = [(Tab::Products, "Products"), (Tab::Graph, "Graph")];
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
        .title("Products")
        .title_style(Style::default().add_modifier(Modifier::BOLD))
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    // Right strip reserved for the two per-product donut graphs (two columns,
    // right-aligned).  The text flows in the remaining left area.
    let right_strip_w = (2 * DONUT_W + DONUT_GAP).min(inner.width);
    let text_w = inner.width.saturating_sub(right_strip_w);
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
    let shares = product_shares(state);
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
        let s = &shares[k];

        // Donut 1: profit margin (net profit / sale price, %).
        let margin_pct = r.profit_percent;
        // Donut 2: this product's yearly net profit vs the yearly goal slider.
        let yearly_profit = s.annual_sales as f64 * r.net_profit;
        let of_goal_pct = if yearly_goal > 0.0 {
            yearly_profit / yearly_goal * 100.0
        } else {
            0.0
        };

        let d1_x = strip_x;
        let d2_x = strip_x + DONUT_W + DONUT_GAP;
        draw_donut(buf, d1_x, top_y, margin_pct, "margin", inner);
        draw_donut(buf, d2_x, top_y, of_goal_pct, "vs year", inner);
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
/// monthly + annual + workday/parallel), excluding padding and the separator.
const PRODUCT_CONTENT_LINES: usize = 15;
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
/// export-file rows concatenated, separated by a blank line.  The label column
/// width is computed across all templates (just like `write_result_file`) so
/// the value columns line up.
fn build_product_details_lines(state: &AppState) -> Vec<Line<'static>> {
    let d = state.lang.dict();
    let workday_hours = state.slider_value(SliderKind::WorkdayHours);
    let parallel = state.slider_value(SliderKind::Parallel).max(1);
    let shares = product_shares(state);

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

    let header_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let goal_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);

    let mut lines: Vec<Line<'static>> = Vec::new();
    for (i, ((_, r), s)) in state.products.iter().zip(shares.iter()).enumerate() {
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

        // Monthly goal + time.
        lines.push(Line::from(Span::styled(
            lang::fmt_aligned(
                d.result_monthly_goal,
                &[&format!("{:.2}", s.monthly_goal), &cur, &s.monthly_sales.to_string()],
                label_w,
            ),
            goal_style,
        )));
        lines.push(Line::from(time_line_rendered(
            d.result_monthly_time,
            s.monthly_minutes,
            parallel,
            workday_hours,
            label_w,
        )));

        lines.push(Line::from(""));

        // Annual goal + time.
        lines.push(Line::from(Span::styled(
            lang::fmt_aligned(
                d.result_annual_goal,
                &[&format!("{:.2}", s.annual_goal), &cur, &s.annual_sales.to_string()],
                label_w,
            ),
            goal_style,
        )));
        lines.push(Line::from(time_line_rendered(
            d.result_annual_time,
            s.annual_minutes,
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
fn slider_entry_lines(s: &Slider, focused: bool, width: u16) -> Vec<Line<'static>> {
    let marker = if focused { "\u{25b6} " } else { "  " };
    let label_style = if focused {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let is_percent = matches!(s.kind, SliderKind::Percent(_));

    // Settings sliders (non-percent): wrap the label after the first word (the
    // first word on its own header line, the remaining words wrapped to the
    // column width), left-aligned.  Product-percentage sliders keep their
    // single-line header with the right-aligned "lock values" checkbox.
    if !is_percent {
        return settings_entry_lines(s, focused, width, marker, label_style);
    }

    // Header line: marker + label (the "lock values" checkbox moves to the
    // track line below, right-aligned within `width`).
    let mut header_spans: Vec<Span<'static>> = Vec::new();
    header_spans.push(Span::styled(marker.to_string(), label_style));
    header_spans.push(Span::styled(s.label.clone(), label_style));

    let mut lines = Vec::new();
    lines.push(Line::from(header_spans));
    let readout = format!(" {}{}", s.value, s.suffix);
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
    // a right-aligned "[x] lock values" / "[ ] lock values" checkbox for
    // product-percentage sliders.
    let mut track_spans: Vec<Span<'static>> = Vec::new();
    track_spans.push(Span::raw("  "));
    track_spans.push(Span::styled(track, track_style));
    track_spans.push(Span::styled(readout, label_style));

    if is_percent && width > 0 {
        let box_str = if s.locked { "[x]" } else { "[ ]" };
        let lock_label = " lock values";
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
            let label_style_cb = if s.locked {
                Style::default().fg(Color::Green)
            } else if focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };
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
    let readout = format!(" {}{}", s.value, s.suffix);
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
    let mut lines: Vec<Line<'static>> = Vec::new();
    let end = end.min(state.sliders.len());
    for i in start..end {
        let s = &state.sliders[i];
        lines.extend(slider_entry_lines(s, i == state.selected, width));
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
    let sub = Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan);
    let val = Style::default();

    // --- Left column: Monthly + Settings ---
    let mut left: Vec<Line<'static>> = Vec::new();
    left.push(Line::from(Span::styled("Monthly", sub)));
    left.push(Line::from(vec![
        Span::styled("  sales    ", val),
        Span::styled(format!("{}", t.monthly.sales), val),
    ]));
    left.push(Line::from(vec![
        Span::styled("  min      ", val),
        Span::styled(format!("{:.0}", t.monthly.minutes), val),
    ]));
    left.push(Line::from(vec![
        Span::styled("  hours    ", val),
        Span::styled(format!("{:.1}", t.monthly.hours), val),
    ]));
    left.push(Line::from(vec![
        Span::styled("  workdays ", val),
        Span::styled(format!("{:.2}", t.monthly.workdays), val),
    ]));
    left.push(Line::from(""));
    left.push(Line::from(Span::styled("Settings", sub)));
    left.push(Line::from(vec![
        Span::styled("  workday  ", val),
        Span::styled(format!("{} h", t.workday_hours), val),
    ]));
    left.push(Line::from(vec![
        Span::styled("  parallel ", val),
        Span::styled(format!("{}", t.parallel), val),
    ]));

    // --- Right column: Yearly + Yearly ref ---
    let mut right: Vec<Line<'static>> = Vec::new();
    right.push(Line::from(Span::styled("Yearly", sub)));
    right.push(Line::from(vec![
        Span::styled("  sales    ", val),
        Span::styled(format!("{}", t.annual.sales), val),
    ]));
    right.push(Line::from(vec![
        Span::styled("  min      ", val),
        Span::styled(format!("{:.0}", t.annual.minutes), val),
    ]));
    right.push(Line::from(vec![
        Span::styled("  hours    ", val),
        Span::styled(format!("{:.1}", t.annual.hours), val),
    ]));
    right.push(Line::from(vec![
        Span::styled("  workdays ", val),
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
    right.push(Line::from(Span::styled("Yearly ref", sub)));
    right.push(Line::from(vec![
        Span::styled("  12x mo   ", val),
        Span::styled(format!("{}", year_sum), val),
    ]));
    right.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(mark.to_string(), mark_style),
        Span::styled(format!(" goal  {}", yearly_target), val),
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

    // Sidebar, split into three stacked regions:
    //   1. Products  — per-product percentage sliders (scrollable)
    //   2. Settings  — workday hours, parallel, monthly & yearly goals
    //   3. Totals    — every aggregate value (also written to the totals file)
    let sidebar_area = body_chunks[1];
    let n_percent = state.products.len();
    let total_sliders = state.sliders.len();

    // Desired dynamic inner padding for every sidebar region: scales with the
    // sidebar width so wider terminals get more breathing room (0..=4 cells).
    let sidebar_inner_w = sidebar_area.width.saturating_sub(2) as usize;
    let desired_pad = ((sidebar_inner_w / 12) as u16).min(4);

    // The Settings region height is content-driven: just enough to fit its
    // wrapped lines + border + padding (clamped to a 7-row minimum so the title
    // and a couple of lines always show).  Totals is fixed at 13 rows.  Products
    // (Min) absorbs ALL remaining vertical space, so resizing the terminal
    // taller grows Products, not Settings.
    let totals_region_h: u16 = 13;
    let settings_min_h: u16 = 7;

    // Products: each entry is 3 lines (header + track + blank).
    let products_needed = (n_percent * 3) as u16;

    // Settings: build the actual wrapped lines for both columns and take the
    // taller one.  The column width depends on the padding, but the line count
    // only grows when the column is narrower (more wrapping), so compute it
    // with the *desired* padding width first; if that doesn't fit we reduce the
    // padding below, which only makes columns wider (fewer wraps), so the
    // estimate is conservative.
    let settings_inner_w = sidebar_inner_w;
    let settings_col_w = settings_inner_w
        .saturating_sub(1 /* separator */ + desired_pad as usize * 2) / 2;
    let mid = (n_percent + 2).min(total_sliders);
    let settings_left_lines = build_slider_lines(state, n_percent, mid, settings_col_w as u16);
    let settings_right_lines = build_slider_lines(state, mid, total_sliders, settings_col_w as u16);
    let settings_needed = settings_left_lines.len().max(settings_right_lines.len()) as u16;

    // Totals: both columns have the same fixed structure (9 lines each).
    let (totals_left, totals_right) = build_totals_columns(state);
    let totals_needed = totals_left.len().max(totals_right.len()) as u16;

    // Compute the padding each region can afford at a few candidate padding
    // values, and pick the largest padding that lets every region fit all its
    // rows.  Settings height grows with the padding (so the padding is real,
    // not clipped); Products and Totals are clamped by their available height.
    // Try from the desired padding down to 0.
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

    let sidebar_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),
            Constraint::Length(settings_region_h),
            Constraint::Length(totals_region_h),
        ])
        .split(sidebar_area);

    // --- Region 1: Products (scrollable) ---
    let top_area = sidebar_chunks[0];
    // Inner area minus the 2-cell border and the dynamic sidebar padding on
    // all sides. Each entry is 3 lines.
    let top_inner_h = top_area.height.saturating_sub(2 + sidebar_pad * 2) as usize;
    let entry_h = 3usize;
    let visible_entries = (top_inner_h / entry_h).max(1);
    // Auto-scroll only while focus is inside the products range.
    if state.selected < n_percent {
        if state.selected < state.scroll {
            state.scroll = state.selected;
        } else if state.selected >= state.scroll + visible_entries {
            state.scroll = state.selected + 1 - visible_entries;
        }
        if state.scroll + visible_entries > n_percent {
            state.scroll = n_percent.saturating_sub(visible_entries);
        }
    }
    let start = state.scroll;
    let end = (start + visible_entries).min(n_percent);
    let top_inner_w = top_area.width.saturating_sub(2 + sidebar_pad * 2);
    let product_lines = build_slider_lines(state, start, end, top_inner_w);
    let sidebar_active = state.active_region == Region::Sidebar;
    let products_block = Block::default()
        .borders(Borders::ALL)
        .title("Products")
        .title_style(Style::default().add_modifier(Modifier::BOLD))
        .border_style(if sidebar_active {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        });
    frame.render_widget(products_block, top_area);
    // Render the content inset by the border (1) + dynamic padding on all
    // sides so the sliders get more breathing room on wider terminals.
    let products_content = Rect::new(
        top_area.x + 1 + sidebar_pad,
        top_area.y + 1 + sidebar_pad,
        top_inner_w,
        top_inner_h as u16,
    );
    frame.render_widget(Paragraph::new(product_lines), products_content);

    // Scroll indicators for the products region: an up-arrow at the top edge
    // when entries are scrolled off above, and a down-arrow at the bottom edge
    // when more entries remain below the visible window.
    let can_up = state.scroll > 0;
    let can_down = end < n_percent && n_percent > visible_entries;
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

    // --- Region 2: Settings (workday, parallel, monthly & yearly goals) ---
    // Two internal columns separated by a 1-cell vertical rule, each holding
    // two of the four settings sliders so the region is compact.  Labels wrap
    // to the column width via Paragraph's Wrap, and the track shrinks to fit
    // so the track + readout stays on one line.
    let settings_block = Block::default()
        .borders(Borders::ALL)
        .title("Settings")
        .title_style(Style::default().add_modifier(Modifier::BOLD))
        .border_style(if sidebar_active {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        });
    let settings_inner = settings_block.inner(sidebar_chunks[1]);
    frame.render_widget(settings_block, sidebar_chunks[1]);
    if settings_inner.height > 0 && settings_inner.width >= 3 {
        // [left col | 1-cell rule | right col].  Min(1) on the sides lets the
        // Length(1) middle win its exact 1 cell and the rest splits evenly.
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

        // Draw the vertical separator rule down the middle column.
        let sep_style = Style::default().fg(Color::DarkGray);
        let buf = frame.buffer_mut();
        for y in sep_area.top()..sep_area.bottom() {
            if y < buf.area.height {
                buf.set_string(sep_area.x, y, "\u{2502}", sep_style);
            }
        }

        // Left column: first two settings sliders; right column: the next two.
        // Each column is inset by the dynamic sidebar padding on all sides
        // (left, right, top, bottom) for a uniform inner margin that grows with
        // the terminal width.
        let pad = sidebar_pad;
        let left_inner = Rect::new(
            left_area.x + pad,
            left_area.y + pad,
            left_area.width.saturating_sub(pad * 2),
            left_area.height.saturating_sub(pad * 2),
        );
        let right_inner = Rect::new(
            right_area.x + pad,
            right_area.y + pad,
            right_area.width.saturating_sub(pad * 2),
            right_area.height.saturating_sub(pad * 2),
        );
        let mid = (n_percent + 2).min(total_sliders);
        let left_lines = build_slider_lines(state, n_percent, mid, left_inner.width);
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

    // --- Region 3: Totals ---
    // Two internal columns separated by a 1-cell vertical rule, mirroring the
    // Settings region.  Left: Monthly + Settings; Right: Yearly + Yearly ref.
    let bottom_block = Block::default()
        .borders(Borders::ALL)
        .title("Totals")
        .title_style(Style::default().add_modifier(Modifier::BOLD))
        .border_style(if sidebar_active {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        });
    let totals_inner = bottom_block.inner(sidebar_chunks[2]);
    frame.render_widget(bottom_block, sidebar_chunks[2]);
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

        // Vertical separator rule down the middle column.
        let sep_style = Style::default().fg(Color::DarkGray);
        let buf = frame.buffer_mut();
        for y in sep_area.top()..sep_area.bottom() {
            if y < buf.area.height {
                buf.set_string(sep_area.x, y, "\u{2502}", sep_style);
            }
        }

        // Each column is inset by the dynamic sidebar padding on all sides,
        // mirroring the Settings region.
        let pad = sidebar_pad;
        let left_inner = Rect::new(
            left_area.x + pad,
            left_area.y + pad,
            left_area.width.saturating_sub(pad * 2),
            left_area.height.saturating_sub(pad * 2),
        );
        let right_inner = Rect::new(
            right_area.x + pad,
            right_area.y + pad,
            right_area.width.saturating_sub(pad * 2),
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

    let region_name = match state.active_region {
        Region::Main => "main",
        Region::Sidebar => "sidebar",
    };
    let footer_text = match &state.status {
        Some(msg) => format!(
            "{}   |   region: {}   Tab region   Shift+Tab tab   \u{2191}/\u{2193} scroll/navigate   \u{2190}/\u{2192} adjust   Space lock   Ctrl+E export   q quit",
            msg, region_name
        ),
        None => format!(
            "region: {}   Tab region   Shift+Tab tab   \u{2191}/\u{2193} scroll/navigate   \u{2190}/\u{2192} adjust   Space lock   Ctrl+E export   q quit",
            region_name
        ),
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

/// Set the percentage slider at `changed_idx` to `new_value` (clamped to a
/// range that respects the locked products) and distribute the remaining
/// percentage **equally** across the other **non-locked** product-percentage
/// sliders whose current value is greater than zero. Locked products keep their
/// exact value and never receive any redistributed share. Products already at
/// 0% are left at 0% (they opt out of the split until the user raises them).
/// Rounding drift is corrected so the post-state sum is exactly 100 whenever at
/// least one other product is eligible. Non-percent sliders are left untouched.
fn redistribute_percent(sliders: &mut [Slider], changed_idx: usize, new_value: i64) {
    let target = match sliders.get(changed_idx).and_then(|s| match s.kind {
        SliderKind::Percent(p) => Some(p),
        _ => None,
    }) {
        Some(p) => p,
        None => return,
    };
    // A locked product is frozen: it cannot be moved, even by direct input.
    if sliders[changed_idx].locked {
        return;
    }

    // Sum of the OTHER locked products' values — these are frozen and carve out
    // a fixed chunk of the 100% pie that the changed product + the eligible
    // non-locked products must share.
    let locked_sum: i64 = sliders
        .iter()
        .filter(|s| matches!(s.kind, SliderKind::Percent(p) if p != target) && s.locked)
        .map(|s| s.value)
        .sum();

    // The changed product cannot exceed the room left by the locked products.
    let max_v = (100 - locked_sum).max(0);
    let v = new_value.clamp(0, max_v);

    // Set the changed product first.
    for s in sliders.iter_mut() {
        if matches!(s.kind, SliderKind::Percent(p) if p == target) {
            s.value = v;
        }
    }

    // Eligible receivers: non-locked, != changed, currently > 0.
    let mut eligible: Vec<usize> = Vec::new();
    for (i, s) in sliders.iter().enumerate() {
        if let SliderKind::Percent(p) = s.kind {
            if p != target && !s.locked && s.value > 0 {
                eligible.push(i);
            }
        }
    }

    let remainder = 100 - v - locked_sum;
    if remainder <= 0 || eligible.is_empty() {
        // No room to distribute, or no eligible receiver: zero out the
        // non-locked, non-changed products (the locked ones keep their value).
        for s in sliders.iter_mut() {
            if let SliderKind::Percent(p) = s.kind {
                if p != target && !s.locked {
                    s.value = 0;
                }
            }
        }
        return;
    }

    // Equal split of the remainder across eligible products.
    let n = eligible.len() as i64;
    let base = remainder / n;
    let extra = remainder - base * n;
    let mut new_vals: Vec<i64> = (0..eligible.len())
        .map(|i| base + if (i as i64) < extra { 1 } else { 0 })
        .collect();
    // Clamp negatives (can't happen with remainder>=0, but be safe).
    for v in new_vals.iter_mut() {
        if *v < 0 {
            *v = 0;
        }
    }
    // Fixup any residual rounding drift against the sum target.
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
    for (i, &si) in eligible.iter().enumerate() {
        sliders[si].value = new_vals[i].max(0);
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
    // Shift+Tab (BackTab): switch the main-area tab (Products <-> Graph).
    if key.code == KeyCode::BackTab {
        state.tab = match state.tab {
            Tab::Products => Tab::Graph,
            Tab::Graph => Tab::Products,
        };
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
            // Toggle the "lock values" checkbox of the focused product
            // percentage slider (ignored for non-percent sliders).
            if let Some(s) = state.sliders.get_mut(state.selected) {
                if matches!(s.kind, SliderKind::Percent(_)) {
                    s.locked = !s.locked;
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
        KeyCode::Left => {
            let is_percent = matches!(
                state.sliders.get(state.selected).map(|s| s.kind),
                Some(SliderKind::Percent(_))
            );
            if is_percent {
                let cur = state.sliders[state.selected].value;
                let step = state.sliders[state.selected].step;
                redistribute_percent(&mut state.sliders, state.selected, cur - step);
            } else if let Some(s) = state.sliders.get_mut(state.selected) {
                s.dec();
            }
        }
        KeyCode::Right => {
            let is_percent = matches!(
                state.sliders.get(state.selected).map(|s| s.kind),
                Some(SliderKind::Percent(_))
            );
            if is_percent {
                let cur = state.sliders[state.selected].value;
                let step = state.sliders[state.selected].step;
                redistribute_percent(&mut state.sliders, state.selected, cur + step);
            } else if let Some(s) = state.sliders.get_mut(state.selected) {
                s.inc();
            }
        }
        _ => {}
    }
    // Goals / percentages / workday hours changed the required production
    // time, so the parallel-products cap range may have shifted. Recompute
    // and clamp on every adjustment (cheap; no-op for pure navigation).
    update_parallel_range(state);
    false
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

/// Per-product split of the current goals, derived from the percentage sliders
/// and the monthly/yearly goal sliders. Shared by the export path and the
/// sidebar totals so both show identical numbers.
fn product_shares(state: &AppState) -> Vec<crate::simulator::ProductShare> {
    let monthly_goal = state.slider_value(SliderKind::MonthlyGoal) as f64;
    let annual_goal = state.slider_value(SliderKind::YearlyGoal) as f64;
    let raw_pcts: Vec<i64> = (0..state.products.len())
        .map(|i| state.percent_for(i).max(0))
        .collect();
    let results: Vec<&ProductResult> = state.products.iter().map(|(_, r)| r).collect();
    compute_product_shares(&results, &raw_pcts, monthly_goal, annual_goal)
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
/// file on export).
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

    let shares = product_shares(state);
    let mut m_sales = 0i64;
    let mut a_sales = 0i64;
    let mut m_min = 0.0f64;
    let mut a_min = 0.0f64;
    for s in &shares {
        m_sales += s.monthly_sales;
        a_sales += s.annual_sales;
        m_min += s.monthly_minutes;
        a_min += s.annual_minutes;
    }
    let m_hours = m_min / 60.0;
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
    let shares = product_shares(state);
    let total_monthly_minutes: f64 = shares.iter().map(|s| s.monthly_minutes).sum();
    let total_annual_minutes: f64 = shares.iter().map(|s| s.annual_minutes).sum();
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
            s.label = format!("Parallel products [{}..={}]", p_min, p_max);
        }
    }
}

/// Write the per-product `*.simulation_results.txt` files and the aggregate
/// `totals.simulation_results.txt` in `state.folder`, mirroring the original
/// dialoguer flow.  The current slider values (per-product percentages,
/// monthly/yearly goals, workday hours, parallel products) drive the split.
/// Returns a human-readable status string.
fn export_results(state: &AppState, lang: &Lang) -> String {
    let workday_hours = state.slider_value(SliderKind::WorkdayHours);
    let parallel = state.slider_value(SliderKind::Parallel).max(1);
    let shares = product_shares(state);

    let mut total_monthly_sales = 0i64;
    let mut total_annual_sales = 0i64;
    let mut total_monthly_minutes = 0.0f64;
    let mut total_annual_minutes = 0.0f64;

    for ((file, r), s) in state.products.iter().zip(&shares) {
        if let Err(e) = write_result_file(
            file,
            r,
            s.monthly_goal,
            s.annual_goal,
            s.monthly_sales,
            s.annual_sales,
            workday_hours,
            parallel,
            lang,
        ) {
            return format!("export error ({}): {}", r.name, e);
        }
        total_monthly_sales += s.monthly_sales;
        total_annual_sales += s.annual_sales;
        total_monthly_minutes += s.monthly_minutes;
        total_annual_minutes += s.annual_minutes;
    }

    if let Err(e) = write_totals_file(
        &state.folder,
        state.products.len(),
        total_monthly_sales,
        total_annual_sales,
        total_monthly_minutes,
        total_annual_minutes,
        workday_hours,
        parallel,
        lang,
    ) {
        return format!("export error (totals): {}", e);
    }

    format!(
        "exported {} product files + totals to {}",
        state.products.len(),
        state.folder.display()
    )
}

/// Build the sidebar slider list from the loaded products.
fn build_sliders(products: &[(PathBuf, ProductResult)]) -> Vec<Slider> {
    let mut sliders = Vec::new();
    let n = products.len() as i64;
    let base = 100 / n.max(1);
    let extra = 100 - base * n.max(1);
    for (i, (_, p)) in products.iter().enumerate() {
        let value = base + if (i as i64) < extra { 1 } else { 0 };
        sliders.push(Slider {
            kind: SliderKind::Percent(i),
            label: format!("% {}", p.name),
            value,
            min: 0,
            max: 100,
            step: 1,
            suffix: "%",
            locked: false,
        });
    }
    sliders.push(Slider {
        kind: SliderKind::WorkdayHours,
        label: "Workday hours".into(),
        value: 8,
        min: 1,
        max: 24,
        step: 1,
        suffix: " h",
        locked: false,
    });
    sliders.push(Slider {
        kind: SliderKind::Parallel,
        label: "Parallel products".into(),
        value: 1,
        min: 1,
        max: 200,
        step: 1,
        suffix: "",
        locked: false,
    });
    sliders.push(Slider {
        kind: SliderKind::MonthlyGoal,
        label: "Monthly net-profit goal".into(),
        value: 1000,
        min: 0,
        max: 1_000_000,
        step: 100,
        suffix: "",
        locked: false,
    });
    sliders.push(Slider {
        kind: SliderKind::YearlyGoal,
        label: "Yearly net-profit goal".into(),
        value: 12000,
        min: 0,
        max: 10_000_000,
        step: 1000,
        suffix: "",
        locked: false,
    });
    sliders
}

/// Entry point: parse products, enter the alternate screen, run the loop.
pub fn run(folder: &Path, lang: &Lang) {
    let products = load_products(folder, lang);
    if products.is_empty() {
        eprintln!("No products with a positive net profit were found in {}", folder.display());
        return;
    }

    let mut state = AppState {
        sliders: build_sliders(&products),
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
            currency: Currency::USD,
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

    fn make_state(products: Vec<ProductResult>, sliders: Vec<Slider>) -> AppState {
        AppState {
            products: wrap(products),
            folder: PathBuf::from("."),
            sliders,
            selected: 0,
            scroll: 0,
            status: None,
            tab: Tab::Products,
            product_scroll: 0,
            lang: Lang::En,
            active_region: Region::Main,
        }
    }

    #[test]
    fn share_for_normalizes_percentages() {
        let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
        let sliders = build_sliders(&wrap(products.clone()));
        let state = make_state(products, sliders);
        // Equal sliders -> equal shares.
        assert!((state.share_for(0) - 0.5).abs() < 1e-9);
        assert!((state.share_for(1) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn share_for_all_zero_splits_equally() {
        let products = vec![prod("A", 10.0, 5.0, 5.0), prod("B", 10.0, 5.0, 5.0)];
        let mut sliders = build_sliders(&wrap(products.clone()));
        for s in &mut sliders {
            if let SliderKind::Percent(_) = s.kind {
                s.value = 0;
            }
        }
        let state = make_state(products, sliders);
        assert!((state.share_for(0) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn month_totals_scales_down_when_capacity_exceeds() {
        // One product, net profit 5, duration 60 min.  Goal 1000 -> 200 sales
        // -> 12000 required minutes.  With 1h/day, 1 parallel, 22 days ->
        // capacity = 1*22*60 = 1320 min.  Scale = 1320/12000 = 0.11 ->
        // floor(200*0.11) = 22 units, amount = 22*10 = 220.
        let products = vec![prod("A", 10.0, 5.0, 60.0)];
        let mut sliders = build_sliders(&wrap(products.clone()));
        for s in &mut sliders {
            match s.kind {
                SliderKind::MonthlyGoal => s.value = 1000,
                SliderKind::WorkdayHours => s.value = 1,
                SliderKind::Parallel => s.value = 1,
                _ => {}
            }
        }
        let state = make_state(products, sliders);
        let mt = state.month_totals();
        assert_eq!(mt.units, 22);
        assert!((mt.amount - 220.0).abs() < 1e-9);
        assert!((mt.required_minutes - 12000.0).abs() < 1e-9);
        assert!((mt.capacity_minutes - 1320.0).abs() < 1e-9);
    }

    #[test]
    fn month_totals_unchanged_when_capacity_sufficient() {
        // Same product but 24h/day, 10 parallel -> capacity = 24*22*60*10
        // = 316800 >> 12000 -> no scaling, 200 units, amount 2000.
        let products = vec![prod("A", 10.0, 5.0, 60.0)];
        let mut sliders = build_sliders(&wrap(products.clone()));
        for s in &mut sliders {
            match s.kind {
                SliderKind::MonthlyGoal => s.value = 1000,
                SliderKind::WorkdayHours => s.value = 24,
                SliderKind::Parallel => s.value = 10,
                _ => {}
            }
        }
        let state = make_state(products, sliders);
        let mt = state.month_totals();
        assert_eq!(mt.units, 200);
        assert!((mt.amount - 2000.0).abs() < 1e-9);
        // amount = profit + cost; profit = units * net_profit = 200 * 5 = 1000,
        // cost = units * total_cost = 200 * 5 = 1000.
        assert!((mt.profit - 1000.0).abs() < 1e-9);
        assert!((mt.cost - 1000.0).abs() < 1e-9);
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
        // 12 groups x 2 bars + bar_gap(1) per group + group_gap(2) x 11.
        // fixed = 12*1 + 11*2 = 34.  With width 130: (130-34)/24 = 4.
        assert_eq!(fit_bar_width(130, 1, 2), 4);
        // Width just enough for 1-cell bars: fixed=34, need 24 more -> 58.
        assert_eq!(fit_bar_width(58, 1, 2), 1);
        // Too narrow: clamp to 1.
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

    fn pct_sliders(values: &[i64]) -> Vec<Slider> {
        values
            .iter()
            .enumerate()
            .map(|(i, &v)| Slider {
                kind: SliderKind::Percent(i),
                label: format!("p{}", i),
                value: v,
                min: 0,
                max: 100,
                step: 1,
                suffix: "%",
                locked: false,
            })
            .collect()
    }

    fn pct_values(sliders: &[Slider]) -> Vec<i64> {
        sliders
            .iter()
            .filter_map(|s| match s.kind {
                SliderKind::Percent(_) => Some(s.value),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn build_sliders_percentages_sum_to_100() {
        let products = vec![
            prod("A", 10.0, 5.0, 5.0),
            prod("B", 10.0, 5.0, 5.0),
            prod("C", 10.0, 5.0, 5.0),
        ];
        let sliders = build_sliders(&wrap(products));
        let sum: i64 = pct_values(&sliders).iter().sum();
        assert_eq!(sum, 100);
    }

    #[test]
    fn redistribute_keeps_total_at_100() {
        let mut sliders = pct_sliders(&[50, 30, 20]);
        // Bump product 0 from 50 to 60; remainder 40 split EQUALLY across the
        // two eligible others (both >0): 20 / 20.
        redistribute_percent(&mut sliders, 0, 60);
        let vals = pct_values(&sliders);
        assert_eq!(vals.iter().sum::<i64>(), 100);
        assert_eq!(vals[0], 60);
        assert_eq!(vals[1], 20);
        assert_eq!(vals[2], 20);
    }

    #[test]
    fn redistribute_skips_zero_products() {
        // A=50, B=30, C=0. Raise A to 60 -> remainder 40 split equally across
        // the eligible (non-zero) others, which is ONLY B. C stays at 0.
        let mut sliders = pct_sliders(&[50, 30, 0]);
        redistribute_percent(&mut sliders, 0, 60);
        let vals = pct_values(&sliders);
        assert_eq!(vals.iter().sum::<i64>(), 100);
        assert_eq!(vals[0], 60);
        assert_eq!(vals[1], 40);
        assert_eq!(vals[2], 0);
    }

    #[test]
    fn redistribute_all_others_zero_leaves_sum_below_100() {
        // A=100, B=0. Lower A to 50: no eligible other to absorb the 50
        // remainder, so B stays 0 and the sum is 50 (the user must raise B).
        let mut sliders = pct_sliders(&[100, 0]);
        redistribute_percent(&mut sliders, 0, 50);
        assert_eq!(pct_values(&sliders), vec![50, 0]);
    }

    #[test]
    fn redistribute_revives_a_zero_product_when_raised() {
        // A=100, B=0. Raise B to 25: A is the only eligible other (>0), so A
        // absorbs the 75 remainder.
        let mut sliders = pct_sliders(&[100, 0]);
        redistribute_percent(&mut sliders, 1, 25);
        assert_eq!(pct_values(&sliders), vec![75, 25]);
    }

    #[test]
    fn redistribute_freezes_locked_product_value() {
        // A=50, B=30, C=20. Lock B at 30, then raise A to 60: the remainder 10
        // must go ONLY to the non-locked, non-zero C (B is frozen and excluded).
        let mut sliders = pct_sliders(&[50, 30, 20]);
        sliders[1].locked = true;
        redistribute_percent(&mut sliders, 0, 60);
        let vals = pct_values(&sliders);
        assert_eq!(vals.iter().sum::<i64>(), 100);
        assert_eq!(vals[0], 60);
        assert_eq!(vals[1], 30); // frozen
        assert_eq!(vals[2], 10); // absorbed the whole remainder
    }

    #[test]
    fn redistribute_capped_by_locked_room() {
        // A=20, B=30, C=50. Lock B=30 and C=50 (locked_sum=80). Raise A above
        // the available 20: it must clamp to 20 (100 - 80), not exceed it, so
        // the sum stays exactly 100.
        let mut sliders = pct_sliders(&[20, 30, 50]);
        sliders[1].locked = true;
        sliders[2].locked = true;
        redistribute_percent(&mut sliders, 0, 90);
        let vals = pct_values(&sliders);
        assert_eq!(vals.iter().sum::<i64>(), 100);
        assert_eq!(vals[0], 20); // clamped to 100 - locked_sum
        assert_eq!(vals[1], 30); // frozen
        assert_eq!(vals[2], 50); // frozen
    }

    #[test]
    fn redistribute_locked_changed_slider_does_nothing() {
        // A=50, B=50. Lock A, then try to raise A to 80: it is frozen, so
        // nothing moves.
        let mut sliders = pct_sliders(&[50, 50]);
        sliders[0].locked = true;
        redistribute_percent(&mut sliders, 0, 80);
        assert_eq!(pct_values(&sliders), vec![50, 50]);
    }

    #[test]
    fn redistribute_locked_zero_excluded_from_receivers() {
        // A=50, B=50. Lock B at 50. Lower A to 20: remainder 30 would normally
        // go to B, but B is locked, so there is no eligible receiver and A
        // stays at 20 (sum below 100, the user must unlock B to fix it).
        let mut sliders = pct_sliders(&[50, 50]);
        sliders[1].locked = true;
        redistribute_percent(&mut sliders, 0, 20);
        assert_eq!(pct_values(&sliders), vec![20, 50]);
    }

    #[test]
    fn redistribute_ignores_non_percent_slider() {
        let mut sliders = pct_sliders(&[50, 50]);
        sliders.push(Slider {
            kind: SliderKind::WorkdayHours,
            label: "wh".into(),
            value: 8,
            min: 1,
            max: 24,
            step: 1,
            suffix: " h",
            locked: false,
        });
        let idx = sliders.len() - 1; // the workday slider
        redistribute_percent(&mut sliders, idx, 20);
        // Workday slider untouched, percentages untouched.
        assert_eq!(sliders[idx].value, 8);
        assert_eq!(pct_values(&sliders), vec![50, 50]);
    }

    #[test]
    fn parallel_slider_range_caps_to_workday_budget() {
        // One product: net profit 5, duration 60 min. Monthly goal 1000 ->
        // 200 sales -> 12000 monthly minutes (200 h). Yearly 12000 -> 2400
        // sales -> 144000 min (2400 h). Workday 8 h.
        //   monthly_p_cap = 200 / (30*8) = 0.833 -> min binds at annual?
        //   annual_p_cap  = 2400 / (365*8) = 0.822 -> monthly binds (0.833).
        //   min = ceil(0.833) = 1, max = floor(200/8) = 25.
        let products = vec![prod("A", 10.0, 5.0, 60.0)];
        let mut sliders = build_sliders(&wrap(products.clone()));
        for s in &mut sliders {
            match s.kind {
                SliderKind::MonthlyGoal => s.value = 1000,
                SliderKind::YearlyGoal => s.value = 12000,
                SliderKind::WorkdayHours => s.value = 8,
                SliderKind::Parallel => s.value = 1,
                _ => {}
            }
        }
        let mut state = make_state(products, sliders);
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
        // Large monthly goal + 1 h workday drives the min up. Monthly goal
        // 100000, net 5, dur 60 -> 20000 sales -> 1200000 min (20000 h).
        // workday 1 h: monthly_p_cap = 20000/30 = 666.67 -> min = 667.
        //   max = floor(20000/1) = 20000.
        let products = vec![prod("A", 10.0, 5.0, 60.0)];
        let mut sliders = build_sliders(&wrap(products.clone()));
        for s in &mut sliders {
            match s.kind {
                SliderKind::MonthlyGoal => s.value = 100000,
                SliderKind::YearlyGoal => s.value = 0,
                SliderKind::WorkdayHours => s.value = 1,
                SliderKind::Parallel => s.value = 1,
                _ => {}
            }
        }
        let mut state = make_state(products, sliders);
        update_parallel_range(&mut state);
        let p = state
            .sliders
            .iter()
            .find(|s| s.kind == SliderKind::Parallel)
            .unwrap();
        assert_eq!(p.min, 667);
        assert_eq!(p.value, 667); // clamped up to the new min
        assert!(p.max >= p.min);
    }

    #[test]
    fn compute_totals_matches_split_sums() {
        // Two products, equal 50/50 split, monthly goal 1000, yearly 12000.
        // A: net 5, dur 5 min -> share goal 500 -> 100 sales -> 500 min.
        // B: net 10, dur 10 min -> share goal 500 -> 50 sales -> 500 min.
        // Totals: monthly 150 sales, 1000 min, 16.67 h, workdays = 16.67/(8*1).
        let products = vec![prod("A", 6.0, 1.0, 5.0), prod("B", 12.0, 2.0, 10.0)];
        let mut sliders = build_sliders(&wrap(products.clone()));
        for s in &mut sliders {
            match s.kind {
                SliderKind::MonthlyGoal => s.value = 1000,
                SliderKind::YearlyGoal => s.value = 12000,
                SliderKind::WorkdayHours => s.value = 8,
                SliderKind::Parallel => s.value = 1,
                _ => {}
            }
        }
        let state = make_state(products, sliders);
        let t = compute_totals(&state);
        assert_eq!(t.monthly.sales, 150);
        assert!((t.monthly.minutes - 1000.0).abs() < 1e-6);
        assert!((t.monthly.hours - 16.6667).abs() < 1e-3);
        assert!((t.monthly.workdays - 16.6667 / 8.0).abs() < 1e-3);
        assert_eq!(t.workday_hours, 8);
        assert_eq!(t.parallel, 1);
    }

    #[test]
    fn export_results_writes_per_product_and_totals_files() {
        let dir = std::env::temp_dir().join("tui_export_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Two products with positive net profit.
        let products = vec![
            prod("Coffee", 4.5, 1.15, 5.0),
            prod("Tea", 3.0, 0.8, 4.0),
        ];
        // Place product definition files alongside so result files write next
        // to them.
        let mut paths: Vec<(PathBuf, ProductResult)> = Vec::new();
        for (i, r) in products.into_iter().enumerate() {
            let f = dir.join(format!("p{}.txt", i));
            std::fs::write(&f, "+ stub\n").unwrap();
            paths.push((f, r));
        }

        let mut sliders = build_sliders(&paths.clone());
        // Give the monthly goal a small known value.
        for s in &mut sliders {
            if let SliderKind::MonthlyGoal = s.kind {
                s.value = 500;
            }
            if let SliderKind::YearlyGoal = s.kind {
                s.value = 6000;
            }
        }
        let state = AppState {
            products: paths,
            folder: dir.clone(),
            sliders,
            selected: 0,
            scroll: 0,
            status: None,
            tab: Tab::Products,
            product_scroll: 0,
            lang: Lang::En,
            active_region: Region::Main,
        };

        let status = export_results(&state, &Lang::En);
        assert!(status.contains("exported"), "status was: {}", status);

        // Per-product result files exist.
        assert!(dir.join("p0.simulation_results.txt").exists());
        assert!(dir.join("p1.simulation_results.txt").exists());
        // Aggregate totals file exists.
        assert!(dir.join("totals.simulation_results.txt").exists());

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
        // Two products so the Settings sliders are at indices 2..6.
        let products = vec![
            prod("A", 10.0, 5.0, 5.0),
            prod("B", 10.0, 5.0, 5.0),
        ];
        let sliders = build_sliders(&wrap(products.clone()));
        let mut state = make_state(products, sliders);
        update_parallel_range(&mut state);

        // 120x40 terminal: sidebar inner ~34 cols, enough for two ~16-col
        // columns with 1-char padding on all sides.
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut state)).unwrap();
        let buf = terminal.backend().buffer().clone();

        // Scope the search to the Settings block: find its title row, then
        // look for a row below it that has both "Workday" (left col) and
        // "Monthly" (right col) — the right-column slider label "Monthly
        // net-profit goal" starts with "Monthly".
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
        assert!(
            monthly_x > workday_x,
            "Monthly (right col) must be right of Workday (left col): workday_x={} monthly_x={}",
            workday_x,
            monthly_x
        );

        // A vertical separator rule (U+2502) must exist between the two labels,
        // at an x strictly between them.
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
        let sliders = build_sliders(&wrap(products.clone()));
        let mut state = make_state(products, sliders);
        update_parallel_range(&mut state);

        let backend = TestBackend::new(80, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut state)).unwrap();
        let buf = terminal.backend().buffer().clone();

        // The Totals block's left column header is "Monthly", right column
        // header is "Yearly".  Both appear inside the Totals block; find the
        // Totals title row first to scope the search below it.  "Monthly" also
        // appears in the Products panel, so search row by row starting at the
        // Totals title and take the first row that has both labels.
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
        assert!(
            yearly_x > monthly_x,
            "Yearly (right col) must be right of Monthly (left col): monthly_x={} yearly_x={}",
            monthly_x,
            yearly_x
        );

        // A vertical separator rule must exist between them.
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
    /// products on a short terminal, the padding must drop to 0 (no top blank
    /// line inside the Products border) so every row is visible.
    #[test]
    fn sidebar_padding_clamps_to_fit_all_product_rows() {
        use ratatui::backend::TestBackend;
        // 10 products × 3 lines = 30 content lines needed.
        let products: Vec<ProductResult> = (0..10)
            .map(|i| prod(&format!("P{}", i), 10.0, 5.0, 5.0))
            .collect();
        let sliders = build_sliders(&wrap(products.clone()));
        let mut state = make_state(products, sliders);
        update_parallel_range(&mut state);

        // 80x40 terminal: sidebar inner ~22 cols (desired_pad=1), but Products
        // only gets 40-14-13=13 rows; 10 products need 30 lines, so padding
        // must clamp to 0.
        let backend = TestBackend::new(80, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut state)).unwrap();
        let buf = terminal.backend().buffer().clone();

        // The Products block top border row: find "Products" title, then check
        // the row immediately below the top border.  With pad=0 the first
        // product slider header appears right below the border (row+1); with
        // pad>=1 there's a blank row (row+1 blank, content at row+2).
        let products_y = (0..buf.area.height)
            .find_map(|y| find_in_row(&buf, y, "Products").map(|_| y))
            .expect("Products title not rendered");
        // The first product slider label is "% P0".
        let p0_row = (products_y + 1..buf.area.height)
            .find_map(|y| find_in_row(&buf, y, "% P0").map(|_| y))
            .expect("% P0 not rendered");
        // With padding clamped to 0, the content starts immediately after the
        // border: p0_row == products_y + 1.
        assert_eq!(
            p0_row, products_y + 1,
            "padding should be 0 (content right below border) when products don't fit otherwise, got p0_row={} products_y={}",
            p0_row, products_y
        );
    }
}
