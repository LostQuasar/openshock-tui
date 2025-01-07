use std::{ cell, cmp::Ordering };
use ratatui::{ layout::*, style::*, text::Span, widgets::*, Frame };

#[derive(PartialEq)]
pub(crate) enum GaugeFormat {
    Percentage,
    Time,
    CountDown,
}
pub(crate) struct GaugeObject {
    pub(crate) title: cell::Cell<String>,
    pub(crate) min: u16,
    pub(crate) fill: cell::Cell<u16>,
    pub(crate) max: cell::Cell<u16>,
    pub(crate) format: GaugeFormat,
    pub(crate) font_color: Color,
    pub(crate) bar_color: Color,
}

pub(crate) fn render_gauge(gauge: &GaugeObject, selected: bool, area: Rect, f: &mut Frame<'_>) {
    let label;
    match gauge.format {
        GaugeFormat::Percentage => {
            label = Span::styled(
                format!("{:.1}%", gauge.fill.get()),
                Style::new().fg(gauge.font_color)
            );
        }
        GaugeFormat::Time | GaugeFormat::CountDown => {
            let string = match gauge.fill.get().cmp(&10) {
                Ordering::Less => format!("{:.1}00 ms", gauge.fill.get()),
                Ordering::Greater | Ordering::Equal =>
                    format!("{:.1} s", (gauge.fill.get() as f32) / 10.0),
            };

            label = Span::styled(string, Style::new().fg(gauge.font_color));
        }
    }
    let layout = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(area);
    f.render_widget(ratatui::widgets::Clear, area);
    if (gauge.format != GaugeFormat::CountDown) | (gauge.fill.get() != 0) {
        let title = gauge.title.take();
        gauge.title.set(title.clone());
        let mut title_widget = Block::default().title(title).fg(gauge.bar_color);
        if selected {
            title_widget = title_widget.bold();
        }
        f.render_widget(title_widget, area);
        f.render_widget(
            Gauge::default()
                .label(label)
                .gauge_style(gauge.bar_color)
                .ratio(f64::from(gauge.fill.get() + 1) / f64::from(gauge.max.get() + 1)),
            layout[1]
        );
    }
}
