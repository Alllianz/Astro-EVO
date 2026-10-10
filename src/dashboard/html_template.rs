use super::models::{DashboardModelParams, DashboardSummaryMetrics};
use super::data_exporter::ExportedChartData;

pub fn render_html_dashboard(
    params: &DashboardModelParams,
    metrics: &DashboardSummaryMetrics,
    chart_data: &ExportedChartData,
) -> String {
    format!(r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Astro EVO - Dashboard Dual (Porcentual vs Nominal) - {tf} {leverage:.0}X</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.1/dist/chart.umd.min.js"></script>
    <style>
        :root {{
            --bg-black: #050507;
            --card-black: #0d0e13;
            --card-black-hover: #13151c;
            --border-black: #1c1e27;
            --border-highlight: #2c303f;
            --text-white: #ffffff;
            --text-white-muted: #c8cdd8;
            --text-gray: #777e90;
            --pure-white: #ffffff;
            --imperial-red: #e60026;
            --imperial-red-glow: rgba(230, 0, 38, 0.35);
            --imperial-red-dark: #8b0015;
            --band-black: #000000;
            --band-white: #ffffff;
            --band-red: #d0001f;
        }}

        * {{
            box-sizing: border-box;
            margin: 0;
            padding: 0;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
        }}

        /* Eliminar spinners/flechitas de inputs numéricos */
        input[type="number"]::-webkit-outer-spin-button,
        input[type="number"]::-webkit-inner-spin-button {{
            -webkit-appearance: none;
            margin: 0;
        }}
        input[type="number"] {{
            -moz-appearance: textfield;
        }}

        body {{
            background-color: var(--bg-black);
            color: var(--text-white);
            min-height: 100vh;
            padding-bottom: 60px;
        }}

        /* Franja Tricolor Histórica Horizontal (Schwarz-Weiß-Rot 1:1:1) */
        .flag-strip {{
            display: flex;
            flex-direction: column;
            height: 12px;
            width: 100%;
            box-shadow: 0 4px 20px rgba(0, 0, 0, 0.9);
        }}
        .flag-strip .band-1 {{ height: 4px; width: 100%; background-color: var(--band-black); }}
        .flag-strip .band-2 {{ height: 4px; width: 100%; background-color: var(--band-white); }}
        .flag-strip .band-3 {{ height: 4px; width: 100%; background-color: var(--band-red); }}

        /* Layout Widescreen optimizado para 2880 x 1800 */
        .container {{
            width: 100%;
            max-width: 100%;
            padding: 24px 36px;
        }}

        /* Header Táctico y Agresivo (Sin redondeos) */
        .header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 20px 28px;
            background: #0a0b10;
            border: 1px solid var(--border-black);
            border-left: 5px solid var(--imperial-red);
            margin-bottom: 16px;
            box-shadow: 0 8px 30px rgba(0, 0, 0, 0.7);
        }}

        .header-title h1 {{
            font-size: 24px;
            font-weight: 900;
            letter-spacing: 1px;
            text-transform: uppercase;
            color: var(--text-white);
            display: flex;
            align-items: center;
            gap: 12px;
        }}

        .header-title p {{
            color: var(--text-gray);
            font-size: 13px;
            margin-top: 4px;
            letter-spacing: 0.3px;
        }}

        .badge-tricolor {{
            display: inline-flex;
            align-items: center;
            background: #000000;
            border: 1px solid var(--imperial-red);
            color: var(--pure-white);
            padding: 8px 18px;
            font-size: 13px;
            font-weight: 900;
            letter-spacing: 1.2px;
            text-transform: uppercase;
            box-shadow: 0 0 15px rgba(230, 0, 38, 0.2);
        }}

        /* Barra de Controles Táctica */
        .control-panel {{
            background: #0a0b10;
            border: 1px solid var(--border-black);
            padding: 16px 24px;
            margin-bottom: 16px;
            display: flex;
            flex-wrap: wrap;
            align-items: center;
            justify-content: space-between;
            gap: 20px;
            box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
        }}

        .control-inputs-group {{
            display: flex;
            flex-wrap: wrap;
            align-items: center;
            gap: 24px;
        }}

        .filter-group {{
            display: flex;
            align-items: center;
            gap: 10px;
        }}

        .filter-label {{
            font-size: 12px;
            font-weight: 800;
            letter-spacing: 0.8px;
            text-transform: uppercase;
            color: var(--text-white-muted);
            display: flex;
            align-items: center;
            gap: 6px;
        }}

        .input-box-wrapper {{
            display: flex;
            align-items: center;
            background: #000000;
            border: 1px solid #2d303f;
            padding: 6px 12px;
            transition: border-color 0.2s ease;
        }}

        .input-box-wrapper:focus-within {{
            border-color: var(--imperial-red);
            box-shadow: 0 0 8px var(--imperial-red-glow);
        }}

        .input-box-wrapper input[type="date"],
        .input-box-wrapper input[type="number"] {{
            background: transparent;
            border: none;
            color: var(--pure-white);
            font-size: 14px;
            font-weight: 800;
            outline: none;
            color-scheme: dark;
        }}

        .input-box-wrapper input[type="number"] {{
            width: 90px;
            text-align: right;
        }}

        .input-suffix {{
            color: var(--text-gray);
            font-size: 13px;
            font-weight: 700;
            margin-left: 4px;
        }}

        .preset-buttons {{
            display: flex;
            gap: 6px;
        }}

        .preset-btn {{
            background: #000000;
            border: 1px solid #2d303f;
            color: var(--text-white-muted);
            padding: 7px 14px;
            font-size: 11px;
            font-weight: 800;
            letter-spacing: 0.6px;
            text-transform: uppercase;
            cursor: pointer;
            transition: all 0.15s ease;
        }}

        .preset-btn:hover {{
            background: #181a24;
            color: #ffffff;
            border-color: #ffffff;
        }}

        .preset-btn.active {{
            background: var(--imperial-red);
            color: #ffffff;
            border-color: var(--imperial-red);
            box-shadow: 0 0 10px var(--imperial-red-glow);
        }}

        .filter-status-badge {{
            font-size: 12px;
            font-weight: 700;
            color: var(--text-gray);
            background: #000000;
            padding: 7px 14px;
            border: 1px solid #20222d;
            letter-spacing: 0.5px;
        }}

        /* Selector de Vistas Táctico */
        .view-controls {{
            display: flex;
            gap: 6px;
            margin-bottom: 16px;
        }}

        .view-btn {{
            background: #0a0b10;
            border: 1px solid var(--border-black);
            color: var(--text-gray);
            padding: 10px 22px;
            font-size: 12px;
            font-weight: 900;
            letter-spacing: 0.8px;
            text-transform: uppercase;
            cursor: pointer;
            transition: all 0.15s ease;
        }}

        .view-btn:hover {{
            background: #14161f;
            color: var(--text-white);
        }}

        .view-btn.active {{
            background: #000000;
            border: 1px solid var(--imperial-red);
            color: var(--pure-white);
            box-shadow: inset 0 0 12px rgba(230, 0, 38, 0.25), 0 0 10px rgba(230, 0, 38, 0.2);
        }}

        /* Tarjetas Comparativas Duales */
        .comparison-grid {{
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 16px;
            margin-bottom: 16px;
        }}

        @media screen and (max-width: 1024px) {{
            .comparison-grid {{
                grid-template-columns: 1fr;
            }}
        }}

        .mode-card {{
            background: #0a0b10;
            border: 1px solid var(--border-black);
            padding: 20px 24px;
            box-shadow: 0 6px 20px rgba(0, 0, 0, 0.6);
            position: relative;
        }}

        .mode-card.card-pct {{
            border-top: 3px solid var(--pure-white);
        }}

        .mode-card.card-nom {{
            border-top: 3px solid var(--imperial-red);
        }}

        .mode-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 16px;
            padding-bottom: 10px;
            border-bottom: 1px solid var(--border-black);
        }}

        .mode-header h2 {{
            font-size: 16px;
            font-weight: 900;
            letter-spacing: 0.8px;
            text-transform: uppercase;
            color: var(--pure-white);
        }}

        .mode-kpis {{
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 12px;
        }}

        .mini-kpi {{
            background: #000000;
            border: 1px solid #1c1e27;
            padding: 14px 16px;
        }}

        .mini-kpi .kpi-label {{
            font-size: 11px;
            color: var(--text-gray);
            text-transform: uppercase;
            font-weight: 800;
            letter-spacing: 0.6px;
            margin-bottom: 4px;
        }}

        .mini-kpi .kpi-val {{
            font-size: 24px;
            font-weight: 900;
            letter-spacing: 0.5px;
            color: var(--text-white);
        }}

        .mini-kpi .kpi-sub {{
            font-size: 11px;
            color: var(--text-gray);
            margin-top: 4px;
            font-weight: 600;
        }}

        /* Sección del Gráfico Principal Widescreen */
        .chart-section {{
            background: #0a0b10;
            border: 1px solid var(--border-black);
            padding: 24px;
            margin-bottom: 16px;
            box-shadow: 0 8px 30px rgba(0, 0, 0, 0.7);
        }}

        .chart-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 16px;
            padding-bottom: 12px;
            border-bottom: 1px solid var(--border-black);
        }}

        .chart-header h2 {{
            font-size: 16px;
            font-weight: 900;
            letter-spacing: 0.8px;
            text-transform: uppercase;
            color: var(--text-white);
            display: flex;
            align-items: center;
            gap: 8px;
        }}

        .legend-tags {{
            display: flex;
            gap: 18px;
            font-size: 12px;
            font-weight: 800;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }}

        .legend-tag {{
            display: flex;
            align-items: center;
            gap: 6px;
        }}

        .dot-white {{ width: 8px; height: 8px; background: var(--pure-white); box-shadow: 0 0 6px rgba(255,255,255,0.7); }}
        .dot-red {{ width: 8px; height: 8px; background: var(--imperial-red); box-shadow: 0 0 6px var(--imperial-red-glow); }}
        .dot-gray {{ width: 8px; height: 8px; background: #666; }}

        .chart-canvas-wrapper {{
            position: relative;
            width: 100%;
            height: 620px;
        }}

        .chart-dd-wrapper {{
            position: relative;
            width: 100%;
            height: 180px;
            margin-top: 14px;
        }}

        /* Grid de Detalles */
        .details-grid {{
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 16px;
            margin-bottom: 16px;
        }}

        @media screen and (max-width: 1024px) {{
            .details-grid {{
                grid-template-columns: 1fr;
            }}
        }}

        .details-panel {{
            background: #0a0b10;
            border: 1px solid var(--border-black);
            padding: 20px 24px;
        }}

        .details-panel h3 {{
            font-size: 14px;
            font-weight: 900;
            letter-spacing: 0.8px;
            text-transform: uppercase;
            color: var(--pure-white);
            margin-bottom: 14px;
            padding-bottom: 10px;
            border-bottom: 1px solid var(--border-black);
            display: flex;
            align-items: center;
            justify-content: space-between;
        }}

        .stat-table {{
            width: 100%;
            border-collapse: collapse;
        }}

        .stat-table tr {{
            border-bottom: 1px solid #161822;
        }}

        .stat-table td {{
            padding: 9px 0;
            font-size: 13px;
        }}

        .stat-table td:first-child {{
            color: var(--text-gray);
            font-weight: 600;
        }}

        .stat-table td:nth-child(2) {{
            color: var(--text-white);
            font-weight: 800;
            text-align: right;
        }}

        .stat-table td:nth-child(3) {{
            color: var(--imperial-red);
            font-weight: 800;
            text-align: right;
        }}

        .stat-table th {{
            font-size: 11px;
            font-weight: 900;
            text-transform: uppercase;
            letter-spacing: 0.8px;
            color: var(--text-gray);
            padding-bottom: 8px;
        }}

        .stat-table th:first-child {{ text-align: left; }}
        .stat-table th:nth-child(2) {{ text-align: right; color: var(--pure-white); }}
        .stat-table th:nth-child(3) {{ text-align: right; color: var(--imperial-red); }}

        .pill-badge {{
            display: inline-block;
            padding: 4px 10px;
            font-size: 11px;
            font-weight: 900;
            letter-spacing: 0.8px;
            text-transform: uppercase;
            background: #000000;
            border: 1px solid #2d303f;
            color: var(--pure-white);
        }}

        .pill-red {{
            background: #000000;
            color: var(--imperial-red);
            border: 1px solid var(--imperial-red);
        }}

        .footer {{
            text-align: center;
            color: var(--text-gray);
            font-size: 12px;
            font-weight: 700;
            letter-spacing: 0.5px;
            margin-top: 30px;
            text-transform: uppercase;
        }}

        /* Botón Flotante de Impresión / PDF */
        .print-btn {{
            position: fixed;
            bottom: 24px;
            right: 24px;
            z-index: 9999;
            background: #000000;
            color: var(--pure-white);
            border: 1px solid var(--imperial-red);
            padding: 10px 20px;
            font-size: 12px;
            font-weight: 800;
            letter-spacing: 0.8px;
            text-transform: uppercase;
            cursor: pointer;
            box-shadow: 0 4px 20px rgba(0, 0, 0, 0.8), 0 0 12px var(--imperial-red-glow);
            display: flex;
            align-items: center;
            gap: 8px;
            transition: all 0.2s ease;
        }}

        .print-btn:hover {{
            background: var(--imperial-red);
            color: #ffffff;
            box-shadow: 0 6px 25px rgba(230, 0, 38, 0.5);
            transform: translateY(-2px);
        }}

        /* Estilos Optimizados para Impresión y PDF */
        /* Estilos Optimizados para Impresión y PDF (1 Sola Página A4 Completa - Sin Bordes Blancos) */
        @media print {{
            @page {{
                size: A4 portrait;
                margin: 0 !important;
            }}

            *, *::before, *::after {{
                -webkit-print-color-adjust: exact !important;
                print-color-adjust: exact !important;
                color-adjust: exact !important;
                box-sizing: border-box !important;
            }}

            html {{
                background-color: var(--bg-black) !important;
                background: #050507 !important;
                margin: 0 !important;
                padding: 0 !important;
                width: 100% !important;
                height: 100% !important;
            }}

            body {{
                background-color: var(--bg-black) !important;
                background: #050507 !important;
                color: var(--text-white) !important;
                padding: 0 !important;
                margin: 0 !important;
                width: 100% !important;
                height: 100% !important;
                min-height: 296mm !important;
                font-size: 11px !important;
            }}

            .flag-strip {{
                box-shadow: none !important;
                height: 4px !important;
                margin-bottom: 0 !important;
            }}
            .flag-strip .band-1, .flag-strip .band-2, .flag-strip .band-3 {{
                height: 1.33px !important;
            }}

            .container {{
                padding: 7mm 8mm 7mm 8mm !important;
                margin: 0 auto !important;
                width: 100% !important;
                max-width: 100% !important;
                background-color: var(--bg-black) !important;
                background: #050507 !important;
                min-height: 296mm !important;
                height: 296mm !important;
                box-sizing: border-box !important;
                display: flex !important;
                flex-direction: column !important;
                justify-content: space-between !important;
            }}

            .preset-buttons,
            .view-controls,
            .print-btn {{
                display: none !important;
            }}

            .header {{
                padding: 7px 12px !important;
                margin-bottom: 0 !important;
                box-shadow: none !important;
                break-inside: avoid !important;
                page-break-inside: avoid !important;
            }}

            .header-title h1 {{
                font-size: 13.5px !important;
            }}

            .header-title p {{
                font-size: 8.5px !important;
            }}

            .badge-tricolor {{
                padding: 2.5px 6px !important;
                font-size: 8.5px !important;
            }}

            .control-panel {{
                padding: 5px 10px !important;
                margin-bottom: 0 !important;
                box-shadow: none !important;
                break-inside: avoid !important;
                page-break-inside: avoid !important;
                gap: 8px !important;
            }}

            .control-inputs-group {{
                gap: 10px !important;
            }}

            .filter-label {{
                font-size: 8.5px !important;
            }}

            .input-box-wrapper {{
                border: none !important;
                padding: 0 !important;
                background: transparent !important;
            }}

            .input-box-wrapper input {{
                font-size: 9.5px !important;
                color: #ffffff !important;
            }}

            .filter-status-badge {{
                padding: 2px 6px !important;
                font-size: 8.5px !important;
            }}

            /* COMPARISON CARDS EN 2 COLUMNAS LADO A LADO */
            .comparison-grid {{
                display: grid !important;
                grid-template-columns: 1fr 1fr !important;
                gap: 6px !important;
                margin-bottom: 0 !important;
                break-inside: avoid !important;
                page-break-inside: avoid !important;
            }}

            .mode-card {{
                padding: 8px 12px !important;
                box-shadow: none !important;
                break-inside: avoid !important;
                page-break-inside: avoid !important;
            }}

            .mode-header {{
                margin-bottom: 5px !important;
                padding-bottom: 3px !important;
            }}

            .mode-header h2 {{
                font-size: 10.5px !important;
            }}

            .mode-kpis {{
                gap: 6px !important;
            }}

            .mini-kpi {{
                padding: 5px 7px !important;
            }}

            .mini-kpi .kpi-label {{
                font-size: 7.5px !important;
                margin-bottom: 1px !important;
            }}

            .mini-kpi .kpi-val {{
                font-size: 12.5px !important;
            }}

            .mini-kpi .kpi-sub {{
                font-size: 7.5px !important;
                margin-top: 1px !important;
            }}

            /* SECCIÓN DE GRÁFICO PANORÁMICO COMPACTO */
            .chart-section {{
                padding: 7px 10px !important;
                margin-bottom: 0 !important;
                box-shadow: none !important;
                break-inside: avoid !important;
                page-break-inside: avoid !important;
                overflow: hidden !important;
            }}

            .chart-header {{
                margin-bottom: 3px !important;
                padding-bottom: 2px !important;
            }}

            .chart-header h2 {{
                font-size: 10px !important;
            }}

            .legend-tags {{
                font-size: 8px !important;
                gap: 8px !important;
            }}

            .chart-canvas-wrapper {{
                height: 290px !important;
                max-height: 290px !important;
                position: relative !important;
                width: 100% !important;
            }}

            .chart-dd-wrapper {{
                height: 75px !important;
                max-height: 75px !important;
                margin-top: 3px !important;
                position: relative !important;
                width: 100% !important;
            }}

            .chart-canvas-wrapper canvas,
            .chart-dd-wrapper canvas {{
                width: 100% !important;
                height: 100% !important;
                display: block !important;
            }}

            /* TABLAS EN 2 COLUMNAS LADO A LADO */
            .details-grid {{
                display: grid !important;
                grid-template-columns: 1fr 1fr !important;
                gap: 6px !important;
                margin-bottom: 0 !important;
                break-inside: avoid !important;
                page-break-inside: avoid !important;
            }}

            .details-panel {{
                padding: 7px 12px !important;
                box-shadow: none !important;
                break-inside: avoid !important;
                page-break-inside: avoid !important;
            }}

            .details-panel h3 {{
                font-size: 9.5px !important;
                margin-bottom: 4px !important;
                padding-bottom: 3px !important;
            }}

            .stat-table td {{
                padding: 2.8px 0 !important;
                font-size: 8.5px !important;
            }}

            .stat-table th {{
                padding-bottom: 2.5px !important;
                font-size: 8px !important;
            }}

            .pill-badge {{
                padding: 1px 4px !important;
                font-size: 7.5px !important;
            }}

            .footer {{
                margin-top: 0 !important;
                padding-top: 2px !important;
                font-size: 7.5px !important;
                break-inside: avoid !important;
                page-break-inside: avoid !important;
            }}
        }}
    </style>
</head>
<body>

    <!-- Botón Flotante para Imprimir / Guardar en PDF -->
    <button class="print-btn" onclick="window.print()" title="Imprimir o Guardar como PDF">
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M6 9V2h12v7M6 18H4a2 2 0 01-2-2v-5a2 2 0 012-2h16a2 2 0 012 2v5a2 2 0 01-2 2h-2M6 14h12v8H6z"/></svg>
        <span>Imprimir / PDF</span>
    </button>

    <!-- Franja Tricolor Histórica Horizontal (Schwarz-Weiß-Rot) -->
    <div class="flag-strip">
        <div class="band-1"></div>
        <div class="band-2"></div>
        <div class="band-3"></div>
    </div>

    <div class="container">
        <!-- Header -->
        <header class="header">
            <div class="header-title">
                <h1>
                    <span>ASTRO EVO</span>
                    <span style="color: var(--imperial-red);">//</span>
                    <span style="font-weight: 400;">Dashboard Dual de Rendimiento</span>
                </h1>
                <p>Simulación Walk-Forward &bull; Capital Porcentual (Compuesto) vs Capital Nominal (Margen Fijo)</p>
            </div>
            <div>
                <span class="badge-tricolor">{tf} &bull; {leverage:.0}X APALANCAMIENTO</span>
            </div>
        </header>

        <!-- Dynamic Control Panel Bar -->
        <div class="control-panel">
            <div class="control-inputs-group">
                <!-- 1. Inicio Backtest (Solo Fecha + 2 Botones) -->
                <div class="filter-group">
                    <div class="filter-label">
                        <span style="color: var(--imperial-red);">📅</span>
                        <span>Inicio Backtest:</span>
                    </div>
                    <div class="input-box-wrapper">
                        <input type="date" id="startDatePicker" value="{min_date_str}" min="{min_date_str}" max="{max_date_str}" onchange="onControlChange()">
                    </div>
                    <div class="preset-buttons">
                        <button class="preset-btn active" id="btnPresetAll" onclick="setPresetDate('{min_date_str}')">Todo el historial</button>
                        <button class="preset-btn" id="btnPreset1Y" onclick="setRelativePreset(365)">Último año</button>
                    </div>
                </div>

                <!-- 2. Capital Inicial (Sin flechitas) -->
                <div class="filter-group">
                    <div class="filter-label">
                        <span style="color: var(--imperial-red);">💰</span>
                        <span>Capital Inicial:</span>
                    </div>
                    <div class="input-box-wrapper">
                        <span style="color: var(--text-gray); font-weight: 800; font-size: 14px; margin-right: 4px;">$</span>
                        <input type="number" id="initCapInput" value="10000" min="1" step="100" oninput="onControlChange()">
                    </div>
                </div>

                <!-- 3. Capital / Trade (Valor numérico directo sin barra) -->
                <div class="filter-group">
                    <div class="filter-label">
                        <span style="color: var(--imperial-red);">⚡</span>
                        <span>Capital / Trade:</span>
                    </div>
                    <div class="input-box-wrapper">
                        <input type="number" id="riskPctInput" value="{base_cap_pct:.1}" min="0.1" max="100" step="0.5" oninput="onControlChange()">
                        <span class="input-suffix">%</span>
                    </div>
                </div>
            </div>

            <!-- Rango de Velas Activo -->
            <div class="filter-status-badge" id="filterStatusBadge">
                Rango: <span id="labelRange" style="color: #ffffff; font-weight: 900;">{min_date_str} ➔ {max_date_str}</span>
            </div>
        </div>

        <!-- View Mode Selector -->
        <div class="view-controls">
            <button class="view-btn active" id="btnDual" onclick="switchView('dual')">⚡ Ambos Modos (Comparativa Dual)</button>
            <button class="view-btn" id="btnPct" onclick="switchView('pct')">📈 Modo Porcentual (Interés Compuesto)</button>
            <button class="view-btn" id="btnNom" onclick="switchView('nom')">📊 Modo Nominal (Margen Fijo)</button>
        </div>

        <!-- Dual Comparison Cards -->
        <div class="comparison-grid">
            <!-- Porcentual Card -->
            <div class="mode-card card-pct">
                <div class="mode-header">
                    <h2>📈 Modo Porcentual (Interés Compuesto)</h2>
                    <span class="pill-badge" id="badgeCardPct">{base_cap_pct:.1}% Asignación / Trade</span>
                </div>
                <div class="mode-kpis">
                    <div class="mini-kpi">
                        <div class="kpi-label">Total Profit (Capital Final)</div>
                        <div class="kpi-val" id="valCapPct" style="color: #ffffff;">${cap_final_pct:.2}</div>
                        <div class="kpi-sub">Retorno: <span id="subRetPct">+{ret_pct_pct:.2}%</span></div>
                    </div>
                    <div class="mini-kpi">
                        <div class="kpi-label">Ganancia Neta ($)</div>
                        <div class="kpi-val" id="valNetPct" style="color: #ffffff;">${net_profit_pct:.2}</div>
                        <div class="kpi-sub">Capital Base: <span id="subBaseCapPct">$10,000.00</span></div>
                    </div>
                    <div class="mini-kpi">
                        <div class="kpi-label">Max Drawdown (%)</div>
                        <div class="kpi-val" id="valDDPct" style="color: var(--imperial-red);">{max_dd_pct_pct:.2}%</div>
                        <div class="kpi-sub">Max DD Nominal: <span id="subDDNomPct">${max_dd_pct_dollar:.2}</span></div>
                    </div>
                    <div class="mini-kpi">
                        <div class="kpi-label">Win Rate &amp; Profit Factor</div>
                        <div class="kpi-val">{winrate_pct:.2}%</div>
                        <div class="kpi-sub">Profit Factor: {pf_pct:.2}</div>
                    </div>
                </div>
            </div>

            <!-- Nominal Card -->
            <div class="mode-card card-nom">
                <div class="mode-header">
                    <h2>📊 Modo Nominal (Margen Fijo)</h2>
                    <span class="pill-badge pill-red" id="badgeCardNom">{base_cap_pct:.1}% Fijo / Trade</span>
                </div>
                <div class="mode-kpis">
                    <div class="mini-kpi">
                        <div class="kpi-label">Total Profit (Capital Final)</div>
                        <div class="kpi-val" id="valCapNom" style="color: #ffffff;">${cap_final_nom:.2}</div>
                        <div class="kpi-sub">Retorno: <span id="subRetNom">+{ret_nom_pct:.2}%</span></div>
                    </div>
                    <div class="mini-kpi">
                        <div class="kpi-label">Ganancia Neta ($)</div>
                        <div class="kpi-val" id="valNetNom" style="color: #ffffff;">${net_profit_nom:.2}</div>
                        <div class="kpi-sub">Capital Base: <span id="subBaseCapNom">$10,000.00</span></div>
                    </div>
                    <div class="mini-kpi">
                        <div class="kpi-label">Max Drawdown (%)</div>
                        <div class="kpi-val" id="valDDNom" style="color: var(--imperial-red);">{max_dd_nom_pct:.2}%</div>
                        <div class="kpi-sub">Max DD Nominal: <span id="subDDNom">${max_dd_nom_dollar:.2}</span></div>
                    </div>
                    <div class="mini-kpi">
                        <div class="kpi-label">Win Rate &amp; Profit Factor</div>
                        <div class="kpi-val">{winrate_nom:.2}%</div>
                        <div class="kpi-sub">Profit Factor: {pf_nom:.2}</div>
                    </div>
                </div>
            </div>
        </div>

        <!-- Main Chart Section (Panoramic Widescreen) -->
        <div class="chart-section">
            <div class="chart-header">
                <h2>
                    <span style="color: var(--imperial-red); font-size: 18px;">📈</span>
                    <span id="chartTitle">Curvas de Equidad Comparadas (Porcentual vs Nominal)</span>
                </h2>
                <div class="legend-tags">
                    <div class="legend-tag" id="legPct"><div class="dot-white"></div><span>Porcentual (Compuesto)</span></div>
                    <div class="legend-tag" id="legNom"><div class="dot-red"></div><span>Nominal (Fijo)</span></div>
                    <div class="legend-tag"><div class="dot-gray"></div><span>Split IS / OOS</span></div>
                </div>
            </div>

            <div class="chart-canvas-wrapper">
                <canvas id="equityChart"></canvas>
            </div>

            <div class="chart-dd-wrapper">
                <canvas id="ddChart"></canvas>
            </div>
        </div>

        <!-- Details Grid -->
        <div class="details-grid">
            <div class="details-panel">
                <h3>
                    <span>Comparativa del Período Seleccionado</span>
                    <span class="pill-badge pill-red">Porcentual vs Nominal</span>
                </h3>
                <table class="stat-table">
                    <thead>
                        <tr>
                            <th>Métrica</th>
                            <th>Porcentual</th>
                            <th>Nominal</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr>
                            <td>Capital Base de Inicio</td>
                            <td id="tblBaseCapPct">$10,000.00</td>
                            <td id="tblBaseCapNom">$10,000.00</td>
                        </tr>
                        <tr>
                            <td>Total Profit / Capital Final ($)</td>
                            <td id="tblCapPct">${cap_final_pct:.2}</td>
                            <td id="tblCapNom">${cap_final_nom:.2}</td>
                        </tr>
                        <tr>
                            <td>Retorno Total (%)</td>
                            <td id="tblRetPct">+{ret_pct_pct:.2}%</td>
                            <td id="tblRetNom">+{ret_nom_pct:.2}%</td>
                        </tr>
                        <tr>
                            <td>Ganancia Neta ($)</td>
                            <td id="tblNetPct">${net_profit_pct:.2}</td>
                            <td id="tblNetNom">${net_profit_nom:.2}</td>
                        </tr>
                        <tr>
                            <td>Max Drawdown (%)</td>
                            <td id="tblDDPct">{max_dd_pct_pct:.2}%</td>
                            <td id="tblDDNom">{max_dd_nom_pct:.2}%</td>
                        </tr>
                        <tr>
                            <td>Retorno In-Sample Histórico (IS)</td>
                            <td id="tblIsRetPct">{is_ret_pct:.2}%</td>
                            <td id="tblIsRetNom">{is_ret_nom:.2}%</td>
                        </tr>
                        <tr>
                            <td>Retorno Out-Of-Sample Histórico (OOS)</td>
                            <td id="tblOosRetPct">{oos_ret_pct:.2}%</td>
                            <td id="tblOosRetNom">{oos_ret_nom:.2}%</td>
                        </tr>
                        <tr>
                            <td>Win Rate Global (%)</td>
                            <td id="tblWinratePct">{winrate_pct:.2}%</td>
                            <td id="tblWinrateNom">{winrate_nom:.2}%</td>
                        </tr>
                        <tr>
                            <td>Profit Factor Global</td>
                            <td id="tblPfPct">{pf_pct:.2}</td>
                            <td id="tblPfNom">{pf_nom:.2}</td>
                        </tr>
                        <tr>
                            <td>Trades Globales (Longs / Shorts)</td>
                            <td id="tblTradesPct">{trades_pct} ({longs_pct}/{shorts_pct})</td>
                            <td id="tblTradesNom">{trades_nom} ({longs_nom}/{shorts_nom})</td>
                        </tr>
                        <tr>
                            <td>Liquidaciones</td>
                            <td id="tblLiqPct">{liq_pct}</td>
                            <td id="tblLiqNom">{liq_nom}</td>
                        </tr>
                        <tr>
                            <td>Ratio Pendientes Histórico (Slope IS/OOS)</td>
                            <td id="tblSlopePct">{slope_pct:.4}</td>
                            <td id="tblSlopeNom">{slope_nom:.4}</td>
                        </tr>
                    </tbody>
                </table>
            </div>

            <div class="details-panel">
                <h3>
                    <span>Hiperparámetros de la Red CNN-1D</span>
                    <span class="pill-badge">Astro EVO Engine</span>
                </h3>
                <table class="stat-table">
                    <tr>
                        <td>Timeframe (TF)</td>
                        <td colspan="2"><span class="pill-badge">{tf}</span></td>
                    </tr>
                    <tr>
                        <td>Apalancamiento Óptimo</td>
                        <td colspan="2"><span class="pill-badge pill-red">{leverage:.0}X</span></td>
                    </tr>
                    <tr>
                        <td>Asignación de Capital Base</td>
                        <td colspan="2">{base_cap_pct:.1}%</td>
                    </tr>
                    <tr>
                        <td>Ventana Temporal (W)</td>
                        <td colspan="2">{window_size} velas</td>
                    </tr>
                    <tr>
                        <td>Kernel Convolucional (K)</td>
                        <td colspan="2">{kernel_size}</td>
                    </tr>
                    <tr>
                        <td>Horizonte de Predicción (H)</td>
                        <td colspan="2">{horizon} velas</td>
                    </tr>
                    <tr>
                        <td>Weight Decay (L2)</td>
                        <td colspan="2">{weight_decay:.6}</td>
                    </tr>
                    <tr>
                        <td>L1 Lasso Regularization</td>
                        <td colspan="2">{l1_lambda:.6}</td>
                    </tr>
                    <tr>
                        <td>Épocas Pre-Entrenamiento IS</td>
                        <td colspan="2">{epoch} épocas</td>
                    </tr>
                    <tr>
                        <td>Fecha Inicio IS</td>
                        <td colspan="2">{is_start_date} (Vela #{is_start_idx})</td>
                    </tr>
                    <tr>
                        <td>Fecha Inicio OOS</td>
                        <td colspan="2">{oos_start_date} (Vela #{oos_start_idx})</td>
                    </tr>
                </table>
            </div>
        </div>

        <div class="footer">
            <p>Generado automáticamente por Astro EVO Engine &bull; Arquitectura Neuronal Convolucional 1D &bull; Re-normalización Dinámica en Tiempo Real</p>
        </div>
    </div>

    <!-- Scripts de Gráficos y Filtrado por Fecha y Capital -->
    <script>
        const rawLabels = {labels_json};
        const rawIsoDates = {iso_dates_json};
        const rawTimestamps = {timestamps_json};
        const rawEquityPct = {equity_pct_json};
        const rawEquityNom = {equity_nom_json};
        const oosDateStr = "{oos_start_date}";
        const isDateStr = "{is_start_date}";
        const baseCapitalPercent = {base_cap_pct};
        const baseLiqPct = {liq_pct};
        const baseLiqNom = {liq_nom};

        let currentStartIndex = 0;
        let currentMode = 'dual';

        // Plugin divisor IS / OOS dinámico
        const verticalLinePlugin = {{
            id: 'verticalLinePlugin',
            afterDraw: (chart) => {{
                if (!chart.chartArea) return;
                const {{ ctx, chartArea: {{ top, bottom }}, scales: {{ x }} }} = chart;
                
                const activeLabels = chart.data.labels;
                let oosLocalIdx = -1;
                for (let i = 0; i < activeLabels.length; i++) {{
                    const globalIdx = currentStartIndex + i;
                    if (globalIdx < rawIsoDates.length && rawIsoDates[globalIdx] >= oosDateStr) {{
                        oosLocalIdx = i;
                        break;
                    }}
                }}

                if (oosLocalIdx >= 0) {{
                    const oosPixel = x.getPixelForValue(oosLocalIdx);
                    if (oosPixel >= chart.chartArea.left && oosPixel <= chart.chartArea.right) {{
                        ctx.save();
                        ctx.beginPath();
                        ctx.setLineDash([6, 6]);
                        ctx.strokeStyle = 'rgba(230, 0, 38, 0.85)';
                        ctx.lineWidth = 2;
                        ctx.moveTo(oosPixel, top);
                        ctx.lineTo(oosPixel, bottom);
                        ctx.stroke();

                        ctx.fillStyle = '#ffffff';
                        ctx.font = 'bold 11px sans-serif';
                        ctx.fillText('OUT-OF-SAMPLE (OOS) ➔', oosPixel + 8, top + 20);
                        ctx.restore();
                    }}
                }}
            }}
        }};

        // Inicializar Gráficos de Chart.js
        const ctxEq = document.getElementById('equityChart').getContext('2d');
        const gradientEqPct = ctxEq.createLinearGradient(0, 0, 0, 580);
        gradientEqPct.addColorStop(0, 'rgba(255, 255, 255, 0.22)');
        gradientEqPct.addColorStop(0.5, 'rgba(255, 255, 255, 0.05)');
        gradientEqPct.addColorStop(1, 'rgba(5, 5, 7, 0.0)');

        const equityChart = new Chart(ctxEq, {{
            type: 'line',
            data: {{
                labels: [],
                datasets: [
                    {{
                        label: 'Porcentual (Compuesto)',
                        data: [],
                        borderColor: '#ffffff',
                        borderWidth: 2.5,
                        pointRadius: 0,
                        pointHoverRadius: 5,
                        pointHoverBackgroundColor: '#ffffff',
                        pointHoverBorderColor: '#e60026',
                        fill: true,
                        backgroundColor: gradientEqPct,
                        tension: 0.12
                    }},
                    {{
                        label: 'Nominal (Fijo)',
                        data: [],
                        borderColor: '#e60026',
                        borderWidth: 2.2,
                        borderDash: [5, 4],
                        pointRadius: 0,
                        pointHoverRadius: 5,
                        pointHoverBackgroundColor: '#e60026',
                        pointHoverBorderColor: '#ffffff',
                        fill: false,
                        tension: 0.12
                    }}
                ]
            }},
            options: {{
                responsive: true,
                maintainAspectRatio: false,
                interaction: {{
                    intersect: false,
                    mode: 'index',
                }},
                plugins: {{
                    legend: {{ display: false }},
                    tooltip: {{
                        backgroundColor: '#0a0b10',
                        titleColor: '#ffffff',
                        bodyColor: '#ffffff',
                        borderColor: '#e60026',
                        borderWidth: 1,
                        padding: 12,
                        callbacks: {{
                            label: (context) => ' ' + context.dataset.label + ': $' + parseFloat(context.parsed.y).toLocaleString(undefined, {{ minimumFractionDigits: 2, maximumFractionDigits: 2 }})
                        }}
                    }}
                }},
                scales: {{
                    x: {{
                        grid: {{ color: 'rgba(255, 255, 255, 0.04)' }},
                        ticks: {{
                            color: '#777e90',
                            maxTicksLimit: 14,
                            font: {{ size: 11, weight: 'bold' }}
                        }}
                    }},
                    y: {{
                        grid: {{ color: 'rgba(255, 255, 255, 0.06)' }},
                        ticks: {{
                            color: '#ffffff',
                            font: {{ size: 11, weight: 'bold' }},
                            callback: (value) => '$' + Number(value).toLocaleString()
                        }}
                    }}
                }}
            }},
            plugins: [verticalLinePlugin]
        }});

        const ctxDd = document.getElementById('ddChart').getContext('2d');
        const gradientDd = ctxDd.createLinearGradient(0, 0, 0, 160);
        gradientDd.addColorStop(0, 'rgba(230, 0, 38, 0.0)');
        gradientDd.addColorStop(1, 'rgba(230, 0, 38, 0.45)');

        const ddChart = new Chart(ctxDd, {{
            type: 'line',
            data: {{
                labels: [],
                datasets: [
                    {{
                        label: 'Drawdown Porcentual (%)',
                        data: [],
                        borderColor: '#ffffff',
                        borderWidth: 1.8,
                        pointRadius: 0,
                        fill: false,
                        tension: 0.1
                    }},
                    {{
                        label: 'Drawdown Nominal (%)',
                        data: [],
                        borderColor: '#e60026',
                        borderWidth: 1.8,
                        pointRadius: 0,
                        fill: true,
                        backgroundColor: gradientDd,
                        tension: 0.1
                    }}
                ]
            }},
            options: {{
                responsive: true,
                maintainAspectRatio: false,
                interaction: {{
                    intersect: false,
                    mode: 'index',
                }},
                plugins: {{
                    legend: {{ display: false }},
                    tooltip: {{
                        backgroundColor: '#0a0b10',
                        titleColor: '#ffffff',
                        bodyColor: '#ffffff',
                        borderColor: '#e60026',
                        borderWidth: 1,
                        padding: 10,
                        callbacks: {{
                            label: (context) => ' ' + context.dataset.label + ': ' + context.parsed.y.toFixed(2) + '%'
                        }}
                    }}
                }},
                scales: {{
                    x: {{ display: false }},
                    y: {{
                        grid: {{ color: 'rgba(255, 255, 255, 0.04)' }},
                        ticks: {{
                            color: '#e60026',
                            font: {{ size: 10, weight: 'bold' }},
                            callback: (value) => value + '%'
                        }}
                    }}
                }}
            }},
            plugins: [verticalLinePlugin]
        }});

        function onControlChange() {{
            const riskVal = parseFloat(document.getElementById('riskPctInput').value) || baseCapitalPercent;
            document.getElementById('badgeCardPct').innerText = riskVal.toFixed(1) + '% Asignación / Trade';
            document.getElementById('badgeCardNom').innerText = riskVal.toFixed(1) + '% Fijo / Trade';
            applyDataFilter();
        }}

        // Función Principal de Re-normalización dinámica
        function applyDataFilter() {{
            const dateValue = document.getElementById('startDatePicker').value;
            let startIdx = rawIsoDates.findIndex(d => d >= dateValue);
            if (startIdx < 0) startIdx = 0;
            if (startIdx >= rawEquityPct.length) startIdx = rawEquityPct.length - 1;
            currentStartIndex = startIdx;

            const initCap = parseFloat(document.getElementById('initCapInput').value) || 10000.0;
            const riskPct = parseFloat(document.getElementById('riskPctInput').value) || baseCapitalPercent;
            const riskScale = (baseCapitalPercent > 0) ? (riskPct / baseCapitalPercent) : 1.0;
            const capScale = initCap / 10000.0;

            const sliceLabels = rawLabels.slice(startIdx);
            const normPct = [];
            const normNom = [];
            const ddPct = [];
            const ddNom = [];

            let peakP = initCap;
            let peakN = initCap;
            let maxDDPct = 0.0;
            let maxDDNom = 0.0;
            let maxDDNomDollar = 0.0;
            let maxDDPctDollar = 0.0;

            let curValPct = initCap;
            let curValNom = initCap;
            let isLiquidatedPct = false;
            let isLiquidatedNom = false;

            normPct.push(curValPct);
            normNom.push(curValNom);
            ddPct.push(0.0);
            ddNom.push(0.0);

            for (let i = startIdx + 1; i < rawEquityPct.length; i++) {{
                // 1. Recálculo Nominal: margen fijo sobre Capital Inicial (se detiene si se funde)
                if (!isLiquidatedNom) {{
                    const deltaNomRaw = rawEquityNom[i] - rawEquityNom[i - 1];
                    const deltaNom = deltaNomRaw * capScale * riskScale;
                    curValNom = curValNom + deltaNom;
                    if (curValNom <= 0.0) {{
                        curValNom = 0.0;
                        isLiquidatedNom = true;
                    }}
                }} else {{
                    curValNom = 0.0;
                }}

                // 2. Recálculo Porcentual: margen sobre Capital Acumulado (Interés Compuesto, se detiene si se funde)
                if (!isLiquidatedPct) {{
                    const prevRawPct = rawEquityPct[i - 1];
                    const curRawPct = rawEquityPct[i];
                    const stepReturnRate = prevRawPct > 0 ? ((curRawPct - prevRawPct) / prevRawPct) * riskScale : 0.0;
                    curValPct = curValPct * (1.0 + stepReturnRate);
                    if (curValPct <= 0.0) {{
                        curValPct = 0.0;
                        isLiquidatedPct = true;
                    }}
                }} else {{
                    curValPct = 0.0;
                }}

                if (curValPct > peakP) peakP = curValPct;
                if (curValNom > peakN) peakN = curValNom;

                const curDDPct = peakP > 0 ? ((peakP - curValPct) / peakP) * 100.0 : 0.0;
                const curDDNom = initCap > 0 ? ((peakN - curValNom) / initCap) * 100.0 : 0.0;

                if (curDDPct > maxDDPct) maxDDPct = curDDPct;
                if (curDDNom > maxDDNom) maxDDNom = curDDNom;
                if ((peakP - curValPct) > maxDDPctDollar) maxDDPctDollar = peakP - curValPct;
                if ((peakN - curValNom) > maxDDNomDollar) maxDDNomDollar = peakN - curValNom;

                normPct.push(curValPct);
                normNom.push(curValNom);
                ddPct.push(-curDDPct);
                ddNom.push(-curDDNom);
            }}

            const finalP = normPct[normPct.length - 1] || initCap;
            const finalN = normNom[normNom.length - 1] || initCap;
            const retP = initCap > 0 ? ((finalP - initCap) / initCap) * 100.0 : 0.0;
            const retN = initCap > 0 ? ((finalN - initCap) / initCap) * 100.0 : 0.0;
            const netProfitP = finalP - initCap;
            const netProfitN = finalN - initCap;

            // 1. Encontrar índices locales para IS y OOS
            let isStartLocalIdx = 0;
            let oosLocalIdx = -1;
            for (let i = 0; i < sliceLabels.length; i++) {{
                const gIdx = startIdx + i;
                if (rawIsoDates[gIdx] >= isDateStr && isStartLocalIdx === 0 && gIdx > startIdx) {{
                    isStartLocalIdx = i;
                }}
                if (rawIsoDates[gIdx] >= oosDateStr && oosLocalIdx < 0) {{
                    oosLocalIdx = i;
                }}
            }}

            // Retorno IS Dinámico
            let isRetP = 0.0;
            let isRetN = 0.0;
            if (oosLocalIdx > 0 && oosLocalIdx < normPct.length) {{
                const eqAtIsStartP = normPct[isStartLocalIdx] || initCap;
                const eqAtIsStartN = normNom[isStartLocalIdx] || initCap;
                const eqAtOosStartP = normPct[oosLocalIdx] || initCap;
                const eqAtOosStartN = normNom[oosLocalIdx] || initCap;
                isRetP = eqAtIsStartP > 0 ? ((eqAtOosStartP - eqAtIsStartP) / eqAtIsStartP) * 100.0 : 0.0;
                isRetN = eqAtIsStartN > 0 ? ((eqAtOosStartN - eqAtIsStartN) / eqAtIsStartN) * 100.0 : 0.0;
            }} else if (oosLocalIdx <= 0) {{
                isRetP = 0.0;
                isRetN = 0.0;
            }} else {{
                isRetP = retP;
                isRetN = retN;
            }}

            // Retorno OOS Dinámico
            let oosRetP = 0.0;
            let oosRetN = 0.0;
            if (oosLocalIdx >= 0 && oosLocalIdx < normPct.length) {{
                const eqAtOosStartP = normPct[oosLocalIdx] || initCap;
                const eqAtOosStartN = normNom[oosLocalIdx] || initCap;
                oosRetP = eqAtOosStartP > 0 ? ((finalP - eqAtOosStartP) / eqAtOosStartP) * 100.0 : 0.0;
                oosRetN = eqAtOosStartN > 0 ? ((finalN - eqAtOosStartN) / eqAtOosStartN) * 100.0 : 0.0;
            }}

            // Slope Ratio Dinámico
            const isCandles = (oosLocalIdx > isStartLocalIdx) ? (oosLocalIdx - isStartLocalIdx) : 1;
            const oosCandles = (normPct.length > oosLocalIdx && oosLocalIdx >= 0) ? (normPct.length - oosLocalIdx) : 1;
            const slopeIsP = isRetP / isCandles;
            const slopeOosP = oosRetP / oosCandles;
            const slopeRatioP = (slopeIsP > 0 && slopeOosP > 0) ? (slopeIsP / slopeOosP) : (slopeOosP <= 0 ? 0.0 : 99.0);

            const slopeIsN = isRetN / isCandles;
            const slopeOosN = oosRetN / oosCandles;
            const slopeRatioN = (slopeIsN > 0 && slopeOosN > 0) ? (slopeIsN / slopeOosN) : (slopeOosN <= 0 ? 0.0 : 99.0);

            // Liquidaciones Dinámicas
            const liqCountP = isLiquidatedPct ? (baseLiqPct > 0 ? baseLiqPct : 1) : baseLiqPct;
            const liqCountN = isLiquidatedNom ? (baseLiqNom > 0 ? baseLiqNom : 1) : baseLiqNom;

            // Actualizar Tarjetas KPIs
            document.getElementById('valCapPct').innerText = '$' + finalP.toLocaleString(undefined, {{ minimumFractionDigits: 2, maximumFractionDigits: 2 }});
            document.getElementById('subRetPct').innerText = (retP >= 0 ? '+' : '') + retP.toFixed(2) + '%';
            document.getElementById('valNetPct').innerText = '$' + netProfitP.toLocaleString(undefined, {{ minimumFractionDigits: 2, maximumFractionDigits: 2 }});
            document.getElementById('subBaseCapPct').innerText = '$' + initCap.toLocaleString(undefined, {{ minimumFractionDigits: 2, maximumFractionDigits: 2 }});
            document.getElementById('valDDPct').innerText = maxDDPct.toFixed(2) + '%';
            document.getElementById('subDDNomPct').innerText = '$' + maxDDPctDollar.toLocaleString(undefined, {{ minimumFractionDigits: 2, maximumFractionDigits: 2 }});

            document.getElementById('valCapNom').innerText = '$' + finalN.toLocaleString(undefined, {{ minimumFractionDigits: 2, maximumFractionDigits: 2 }});
            document.getElementById('subRetNom').innerText = (retN >= 0 ? '+' : '') + retN.toFixed(2) + '%';
            document.getElementById('valNetNom').innerText = '$' + netProfitN.toLocaleString(undefined, {{ minimumFractionDigits: 2, maximumFractionDigits: 2 }});
            document.getElementById('subBaseCapNom').innerText = '$' + initCap.toLocaleString(undefined, {{ minimumFractionDigits: 2, maximumFractionDigits: 2 }});
            document.getElementById('valDDNom').innerText = maxDDNom.toFixed(2) + '%';
            document.getElementById('subDDNom').innerText = '$' + maxDDNomDollar.toLocaleString(undefined, {{ minimumFractionDigits: 2, maximumFractionDigits: 2 }});

            // Actualizar Tabla Comparativa
            document.getElementById('tblBaseCapPct').innerText = '$' + initCap.toLocaleString(undefined, {{ minimumFractionDigits: 2, maximumFractionDigits: 2 }});
            document.getElementById('tblBaseCapNom').innerText = '$' + initCap.toLocaleString(undefined, {{ minimumFractionDigits: 2, maximumFractionDigits: 2 }});
            document.getElementById('tblCapPct').innerText = '$' + finalP.toLocaleString(undefined, {{ minimumFractionDigits: 2, maximumFractionDigits: 2 }});
            document.getElementById('tblCapNom').innerText = '$' + finalN.toLocaleString(undefined, {{ minimumFractionDigits: 2, maximumFractionDigits: 2 }});
            document.getElementById('tblRetPct').innerText = (retP >= 0 ? '+' : '') + retP.toFixed(2) + '%';
            document.getElementById('tblRetNom').innerText = (retN >= 0 ? '+' : '') + retN.toFixed(2) + '%';
            document.getElementById('tblNetPct').innerText = '$' + netProfitP.toLocaleString(undefined, {{ minimumFractionDigits: 2, maximumFractionDigits: 2 }});
            document.getElementById('tblNetNom').innerText = '$' + netProfitN.toLocaleString(undefined, {{ minimumFractionDigits: 2, maximumFractionDigits: 2 }});
            document.getElementById('tblDDPct').innerText = maxDDPct.toFixed(2) + '%';
            document.getElementById('tblDDNom').innerText = maxDDNom.toFixed(2) + '%';
            document.getElementById('tblIsRetPct').innerText = (isRetP >= 0 ? '+' : '') + isRetP.toFixed(2) + '%';
            document.getElementById('tblIsRetNom').innerText = (isRetN >= 0 ? '+' : '') + isRetN.toFixed(2) + '%';
            document.getElementById('tblOosRetPct').innerText = (oosRetP >= 0 ? '+' : '') + oosRetP.toFixed(2) + '%';
            document.getElementById('tblOosRetNom').innerText = (oosRetN >= 0 ? '+' : '') + oosRetN.toFixed(2) + '%';
            document.getElementById('tblLiqPct').innerText = isLiquidatedPct ? (liqCountP + ' (Liquidada)') : String(liqCountP);
            document.getElementById('tblLiqNom').innerText = isLiquidatedNom ? (liqCountN + ' (Liquidada)') : String(liqCountN);
            document.getElementById('tblSlopePct').innerText = slopeRatioP.toFixed(4);
            document.getElementById('tblSlopeNom').innerText = slopeRatioN.toFixed(4);

            // Actualizar Fecha en Badge
            const curStartDate = rawIsoDates[startIdx];
            const curEndDate = rawIsoDates[rawIsoDates.length - 1];
            document.getElementById('labelRange').innerText = curStartDate + ' ➔ ' + curEndDate;

            // Actualizar Chart.js
            equityChart.data.labels = sliceLabels;
            equityChart.data.datasets[0].data = normPct;
            equityChart.data.datasets[1].data = normNom;

            ddChart.data.labels = sliceLabels;
            ddChart.data.datasets[0].data = ddPct;
            ddChart.data.datasets[1].data = ddNom;

            equityChart.update();
            ddChart.update();
        }}

        function setPresetDate(dateValue) {{
            document.getElementById('startDatePicker').value = dateValue;
            clearPresetActive();
            if (dateValue === '{min_date_str}') {{
                document.getElementById('btnPresetAll').classList.add('active');
            }}
            applyDataFilter();
        }}

        function setRelativePreset(days) {{
            const lastTs = rawTimestamps[rawTimestamps.length - 1] || Date.now();
            const targetTs = lastTs - (days * 24 * 60 * 60 * 1000);
            let targetIdx = rawTimestamps.findIndex(ts => ts >= targetTs);
            if (targetIdx < 0) targetIdx = 0;

            const targetIso = rawIsoDates[targetIdx];
            document.getElementById('startDatePicker').value = targetIso;

            clearPresetActive();
            if (days === 365) {{
                document.getElementById('btnPreset1Y').classList.add('active');
            }}

            applyDataFilter();
        }}

        function clearPresetActive() {{
            document.querySelectorAll('.preset-btn').forEach(btn => btn.classList.remove('active'));
        }}

        // Switch View Modes
        function switchView(mode) {{
            currentMode = mode;
            document.getElementById('btnDual').classList.remove('active');
            document.getElementById('btnPct').classList.remove('active');
            document.getElementById('btnNom').classList.remove('active');

            if (mode === 'dual') {{
                document.getElementById('btnDual').classList.add('active');
                document.getElementById('chartTitle').innerText = 'Curvas de Equidad Comparadas (Porcentual vs Nominal)';
                equityChart.data.datasets[0].hidden = false;
                equityChart.data.datasets[1].hidden = false;
                ddChart.data.datasets[0].hidden = false;
                ddChart.data.datasets[1].hidden = false;
                document.getElementById('legPct').style.display = 'flex';
                document.getElementById('legNom').style.display = 'flex';
            }} else if (mode === 'pct') {{
                document.getElementById('btnPct').classList.add('active');
                document.getElementById('chartTitle').innerText = 'Curva de Equidad Porcentual (Interés Compuesto)';
                equityChart.data.datasets[0].hidden = false;
                equityChart.data.datasets[1].hidden = true;
                ddChart.data.datasets[0].hidden = false;
                ddChart.data.datasets[1].hidden = true;
                document.getElementById('legPct').style.display = 'flex';
                document.getElementById('legNom').style.display = 'none';
            }} else if (mode === 'nom') {{
                document.getElementById('btnNom').classList.add('active');
                document.getElementById('chartTitle').innerText = 'Curva de Equidad Nominal (Margen Fijo)';
                equityChart.data.datasets[0].hidden = true;
                equityChart.data.datasets[1].hidden = false;
                ddChart.data.datasets[0].hidden = true;
                ddChart.data.datasets[1].hidden = false;
                document.getElementById('legPct').style.display = 'none';
                document.getElementById('legNom').style.display = 'flex';
            }}
            equityChart.update();
            ddChart.update();
        }}

        // Redimensionar Gráficos limpiamente al Imprimir o Exportar PDF
        window.addEventListener('beforeprint', () => {{
            if (typeof equityChart !== 'undefined' && equityChart) {{
                equityChart.resize();
            }}
            if (typeof ddChart !== 'undefined' && ddChart) {{
                ddChart.resize();
            }}
        }});
        window.addEventListener('afterprint', () => {{
            if (typeof equityChart !== 'undefined' && equityChart) {{
                equityChart.resize();
            }}
            if (typeof ddChart !== 'undefined' && ddChart) {{
                ddChart.resize();
            }}
        }});

        // Carga Inicial Completa
        applyDataFilter();
    </script>
</body>
</html>"#,
        tf = params.tf,
        leverage = params.leverage,
        min_date_str = chart_data.min_date_str,
        max_date_str = chart_data.max_date_str,
        is_start_date = params.is_start_date,
        oos_start_date = params.oos_start_date,
        is_start_idx = params.is_start_idx,
        oos_start_idx = params.oos_start_idx,
        base_cap_pct = params.capital_percent,
        cap_final_pct = metrics.capital_final_pct,
        ret_pct_pct = metrics.total_return_pct_pct,
        net_profit_pct = metrics.net_profit_pct,
        max_dd_pct_pct = metrics.max_dd_pct_pct,
        max_dd_pct_dollar = metrics.max_dd_pct_dollar,
        winrate_pct = metrics.winrate_pct,
        pf_pct = metrics.profit_factor_pct,
        cap_final_nom = metrics.capital_final_nom,
        ret_nom_pct = metrics.total_return_nom_pct,
        net_profit_nom = metrics.net_profit_nom,
        max_dd_nom_pct = metrics.max_dd_nom_pct,
        max_dd_nom_dollar = metrics.max_dd_nom_dollar,
        winrate_nom = metrics.winrate_nom,
        pf_nom = metrics.profit_factor_nom,
        is_ret_pct = metrics.is_return_pct_pct,
        is_ret_nom = metrics.is_return_nom_pct,
        oos_ret_pct = metrics.oos_return_pct_pct,
        oos_ret_nom = metrics.oos_return_nom_pct,
        trades_pct = metrics.total_trades_pct,
        longs_pct = metrics.longs_count_pct,
        shorts_pct = metrics.shorts_count_pct,
        trades_nom = metrics.total_trades_nom,
        longs_nom = metrics.longs_count_nom,
        shorts_nom = metrics.shorts_count_nom,
        liq_pct = metrics.liquidations_pct,
        liq_nom = metrics.liquidations_nom,
        slope_pct = metrics.slope_ratio_pct,
        slope_nom = metrics.slope_ratio_nom,
        window_size = params.window_size,
        kernel_size = params.kernel_size,
        horizon = params.horizon,
        weight_decay = params.weight_decay,
        l1_lambda = params.l1_lambda,
        epoch = params.epoch,
        labels_json = chart_data.labels_json,
        iso_dates_json = chart_data.iso_dates_json,
        timestamps_json = chart_data.timestamps_json,
        equity_pct_json = chart_data.equity_pct_json,
        equity_nom_json = chart_data.equity_nom_json,
    )
}
