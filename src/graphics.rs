use crate::db::Kline;
use plotters::prelude::*;

pub fn draw_equity_chart(
    klines: &[Kline],
    equity_curve: &[(f32, f32)],
    tf: &str,
    epoch: usize,
    window_size: usize,
    leverage: f64,
    capital_percent: f64,
    is_start_idx: usize,
    oos_start_idx: usize,
    capital_final: f32,
    max_dd: f32,
    profit_factor: f64,
    winrate: f64,
    img_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    draw_equity_chart_in_dir(
        "", klines, equity_curve, tf, epoch, window_size, leverage, capital_percent,
        is_start_idx, oos_start_idx, capital_final, max_dd, profit_factor, winrate, img_name
    )
}

pub fn draw_equity_chart_in_dir(
    base_folder: &str,
    klines: &[Kline],
    equity_curve: &[(f32, f32)],
    tf: &str,
    epoch: usize,
    window_size: usize,
    leverage: f64,
    _capital_percent: f64,
    is_start_idx: usize,
    oos_start_idx: usize,
    capital_final: f32,
    max_dd: f32,
    profit_factor: f64,
    winrate: f64,
    img_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let target_dir = if base_folder.is_empty() || base_folder == "Astro EVO" {
        format!("{}/Epochs", tf)
    } else {
        format!("{}/{}/Epochs", base_folder, tf)
    };
    std::fs::create_dir_all(&target_dir)?;
    
    let filename_png = format!("{}/{}", target_dir, img_name);
    println!("Graficando curvas de equidad en {} (2880 x 1800)...", filename_png);

    let root_area = BitMapBackend::new(&filename_png, (2880, 1800)).into_drawing_area();
    let dark_bg = RGBColor(12, 13, 18);
    root_area.fill(&dark_bg)?;

    let min_x = is_start_idx as f32;
    let max_x = klines.len() as f32;

    let min_y = equity_curve.iter().map(|&(_, y)| y).fold(f32::INFINITY, f32::min);
    let max_y = equity_curve.iter().map(|&(_, y)| y).fold(f32::NEG_INFINITY, f32::max);
    
    let chart_max_y = max_y + (max_y - min_y).max(1000.0) * 0.05;
    let chart_min_y = (min_y - (max_y - min_y).max(1000.0) * 0.05).max(0.0);

    let label_color = RGBColor(138, 153, 173);

    let mut chart = ChartBuilder::on(&root_area)
        .margin(60)
        .x_label_area_size(80)
        .y_label_area_size(120)
        .build_cartesian_2d(min_x..max_x, chart_min_y..chart_max_y)?;

    chart.configure_mesh()
        .x_labels(30)
        .y_labels(30)
        .label_style(("sans-serif", 24).into_font().color(&label_color))
        .axis_style(&label_color)
        .bold_line_style(&label_color.mix(0.12))
        .light_line_style(&label_color.mix(0.04))
        .draw()?;

    let eq_at_oos_start = equity_curve.iter()
        .find(|&&(x, _)| x == oos_start_idx as f32)
        .map(|&(_, y)| y)
        .unwrap_or(10000.0f32);

    let is_trend_color = RGBColor(0x00, 0xd2, 0xff).mix(0.75);
    chart.draw_series(LineSeries::new(
        vec![
            (is_start_idx as f32, 10000.0f32),
            (oos_start_idx as f32, eq_at_oos_start),
        ],
        is_trend_color.stroke_width(3),
    ))?
    .label(format!("Tendencia IS (Retorno: ${:.2})", eq_at_oos_start - 10000.0))
    .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], is_trend_color.stroke_width(3)));

    let oos_trend_color = RGBColor(0x3b, 0x82, 0xf6).mix(0.75);
    chart.draw_series(LineSeries::new(
        vec![
            (oos_start_idx as f32, eq_at_oos_start),
            ((klines.len() - 1) as f32, capital_final),
        ],
        oos_trend_color.stroke_width(3),
    ))?
    .label(format!("Tendencia OOS (Retorno: ${:.2})", capital_final - eq_at_oos_start))
    .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], oos_trend_color.stroke_width(3)));

    let global_trend_color = RGBColor(0x75, 0x33, 0xef).mix(0.85);
    chart.draw_series(LineSeries::new(
        vec![
            (is_start_idx as f32, 10000.0f32),
            ((klines.len() - 1) as f32, capital_final),
        ],
        global_trend_color.stroke_width(3),
    ))?
    .label("Retorno Lineal Teórico Global")
    .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], global_trend_color.stroke_width(3)));

    let real_equity_color = RGBColor(192, 192, 192);
    chart.draw_series(LineSeries::new(
        equity_curve.iter().map(|&(x, y)| (x, y)),
        real_equity_color.stroke_width(4),
    ))?
    .label(format!("Curva Equidad Real Astro EVO (Capital: ${:.2})", capital_final))
    .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], real_equity_color.stroke_width(4)));

    let oos_x = oos_start_idx as f32;
    chart.draw_series(std::iter::once(PathElement::new(
        vec![(oos_x, chart_min_y), (oos_x, chart_max_y)],
        WHITE.mix(0.25).stroke_width(2),
    )))?;

    chart.configure_series_labels()
        .background_style(&dark_bg.mix(0.85))
        .border_style(&label_color.mix(0.3))
        .label_font(("sans-serif", 28).into_font().color(&WHITE))
        .draw()?;

    let font_title = ("sans-serif", 40).into_font().color(&WHITE);
    let font_subtitle = ("sans-serif", 24).into_font().color(&label_color);
    
    root_area.draw_text(
        &format!("Estrategia Convolucional Astro EVO - Época {} - {}", epoch, tf),
        &font_title,
        (100, 50),
    )?;

    let subtitle_text = if epoch == 9999 {
        format!("Backtest de Online Learning: 20-04-2020 a la Última Vela Disponible")
    } else {
        format!("In-Sample: 20-04-2020 a 01-01-2023 | Out-of-Sample: 01-01-2023 en adelante")
    };

    root_area.draw_text(
        &subtitle_text,
        &font_subtitle,
        (100, 105),
    )?;

    root_area.draw_text(
        &format!("Apalancamiento: {}X | Capital Inicial: $10,000.00 | Ventana W: {}", leverage, window_size),
        &font_subtitle,
        (100, 140),
    )?;

    let font_stats = ("sans-serif", 28).into_font().color(&WHITE);
    let stats_box_color = RGBColor(20, 22, 32);
    
    root_area.draw(&Rectangle::new(
        [(2100, 1460), (2750, 1660)],
        ShapeStyle::from(&stats_box_color).filled()
    ))?;
    
    root_area.draw(&Rectangle::new(
        [(2100, 1460), (2750, 1660)],
        ShapeStyle::from(&label_color.mix(0.15)).stroke_width(1)
    ))?;

    root_area.draw_text(
        &format!("Capital Final: ${:.2}", capital_final),
        &font_stats,
        (2130, 1490),
    )?;
    root_area.draw_text(
        &format!("Max Drawdown: {:.2}%", max_dd),
        &font_stats,
        (2130, 1530),
    )?;
    root_area.draw_text(
        &format!("Profit Factor: {:.4}", profit_factor),
        &font_stats,
        (2130, 1570),
    )?;
    root_area.draw_text(
        &format!("Winrate: {:.2}%", winrate),
        &font_stats,
        (2130, 1610),
    )?;

    let is_center_x = (is_start_idx + oos_start_idx) / 2;
    let oos_center_x = (oos_start_idx + klines.len()) / 2;
    
    chart.draw_series(std::iter::once(Text::new(
        "IN-SAMPLE",
        (is_center_x as f32, chart_max_y - (chart_max_y - chart_min_y) * 0.1),
        ("sans-serif", 28).into_font().color(&WHITE.mix(0.4)),
    )))?;

    chart.draw_series(std::iter::once(Text::new(
        "OUT-OF-SAMPLE",
        (oos_center_x as f32, chart_max_y - (chart_max_y - chart_min_y) * 0.1),
        ("sans-serif", 28).into_font().color(&RED.mix(0.5)),
    )))?;

    root_area.present()?;
    Ok(())
}
