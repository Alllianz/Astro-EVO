use super::exporter::IntelligenceComparisonExport;

pub fn render_intelligence_dashboard_html(data: &IntelligenceComparisonExport) -> String {
    let json_data = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());

    format!(r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Astro EVO - Comparativa de Niveles de Inteligencia ({tf} {leverage:.0}X)</title>
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
            --gold: #ffb700;
            --cyan: #00f0ff;
            --green: #00ff66;
            --magenta: #ff0055;
            --purple: #b026ff;
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

        body {{
            background-color: var(--bg-black);
            color: var(--text-white);
            min-height: 100vh;
            padding-bottom: 60px;
        }}

        /* Franja Tricolor Histórica Horizontal (Schwarz-Weiß-Rot) */
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

        .container {{
            width: 100%;
            max-width: 100%;
            padding: 24px 36px;
        }}

        /* Header Táctico */
        .header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 20px 28px;
            background: #0a0b10;
            border: 1px solid var(--border-black);
            border-left: 5px solid var(--cyan);
            margin-bottom: 20px;
            box-shadow: 0 8px 30px rgba(0, 0, 0, 0.7);
        }}

        .header-title h1 {{
            font-size: 22px;
            font-weight: 900;
            letter-spacing: 1px;
            text-transform: uppercase;
            color: var(--text-white);
            display: flex;
            align-items: center;
            gap: 12px;
        }}

        .header-title p {{
            font-size: 13px;
            color: var(--text-gray);
            margin-top: 4px;
        }}

        .badge-live {{
            display: inline-flex;
            align-items: center;
            gap: 6px;
            padding: 5px 12px;
            background: rgba(0, 240, 255, 0.12);
            border: 1px solid var(--cyan);
            color: var(--cyan);
            font-size: 11px;
            font-weight: 800;
            letter-spacing: 1px;
            text-transform: uppercase;
        }}

        .badge-best {{
            display: inline-flex;
            align-items: center;
            gap: 8px;
            padding: 8px 16px;
            background: rgba(255, 183, 0, 0.15);
            border: 1px solid var(--gold);
            color: var(--gold);
            font-size: 13px;
            font-weight: 900;
            letter-spacing: 1px;
            text-transform: uppercase;
            box-shadow: 0 0 15px rgba(255, 183, 0, 0.25);
        }}

        .pill-badge {{
            padding: 3px 8px;
            font-size: 10px;
            font-weight: 800;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            border-radius: 2px;
        }}

        /* Winner Model Master Showcase */
        .winner-master-card {{
            background: linear-gradient(135deg, #0a0b12 0%, #111420 100%);
            border: 1px solid #232738;
            border-left: 6px solid var(--gold);
            padding: 24px;
            margin-bottom: 24px;
            box-shadow: 0 10px 40px rgba(0, 0, 0, 0.7);
        }}

        .winner-header-bar {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            flex-wrap: wrap;
            gap: 12px;
            margin-bottom: 18px;
            padding-bottom: 14px;
            border-bottom: 1px solid var(--border-black);
        }}

        .winner-header-title {{
            display: flex;
            align-items: center;
            gap: 12px;
            flex-wrap: wrap;
        }}

        .winner-header-title h2 {{
            font-size: 18px;
            font-weight: 900;
            letter-spacing: 1px;
            text-transform: uppercase;
            color: var(--text-white);
        }}

        .winner-dual-grid {{
            display: grid;
            grid-template-columns: 1fr 1fr 1fr;
            gap: 16px;
        }}

        @media (max-width: 1200px) {{
            .winner-dual-grid {{
                grid-template-columns: 1fr;
            }}
        }}

        .winner-box {{
            background: #08090e;
            border: 1px solid var(--border-black);
            padding: 18px 20px;
            display: flex;
            flex-direction: column;
            gap: 12px;
            position: relative;
            transition: all 0.2s ease;
        }}

        .winner-box.box-pct {{
            border-top: 3px solid var(--cyan);
            background: linear-gradient(180deg, rgba(0, 240, 255, 0.05) 0%, #08090e 100%);
        }}

        .winner-box.box-nom {{
            border-top: 3px solid var(--imperial-red);
            background: linear-gradient(180deg, rgba(230, 0, 38, 0.05) 0%, #08090e 100%);
        }}

        .winner-box.box-arch {{
            border-top: 3px solid var(--gold);
            background: linear-gradient(180deg, rgba(255, 183, 0, 0.05) 0%, #08090e 100%);
        }}

        .winner-box.active-mode-card {{
            border-color: var(--cyan);
            box-shadow: 0 0 20px rgba(0, 240, 255, 0.2);
        }}

        .box-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            border-bottom: 1px solid rgba(255, 255, 255, 0.06);
            padding-bottom: 8px;
        }}

        .box-header h3 {{
            font-size: 13px;
            font-weight: 800;
            text-transform: uppercase;
            letter-spacing: 0.8px;
        }}

        .winner-stat-row {{
            display: flex;
            justify-content: space-between;
            align-items: baseline;
            font-size: 13px;
            border-bottom: 1px solid rgba(255, 255, 255, 0.03);
            padding-bottom: 4px;
        }}

        .winner-stat-label {{
            color: var(--text-gray);
            font-size: 11px;
            font-weight: 700;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }}

        .winner-stat-val {{
            font-size: 14px;
            font-weight: 800;
            color: var(--text-white);
            font-family: monospace;
        }}

        .winner-hero-val {{
            font-size: 26px;
            font-weight: 900;
            line-height: 1.1;
            font-family: monospace;
            margin-top: 2px;
        }}

        .val-positive {{ color: var(--green) !important; }}
        .val-negative {{ color: var(--imperial-red) !important; }}
        .val-gold {{ color: var(--gold) !important; }}
        .val-cyan {{ color: var(--cyan) !important; }}

        /* Main Chart Section */
        .chart-section {{
            background: var(--card-black);
            border: 1px solid var(--border-black);
            padding: 22px;
            margin-bottom: 20px;
            box-shadow: 0 8px 30px rgba(0, 0, 0, 0.5);
        }}

        .chart-controls {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            flex-wrap: wrap;
            gap: 12px;
            margin-bottom: 16px;
            padding-bottom: 14px;
            border-bottom: 1px solid var(--border-black);
        }}

        .chart-title {{
            font-size: 16px;
            font-weight: 800;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            display: flex;
            align-items: center;
            gap: 10px;
        }}

        .btn-group {{
            display: flex;
            gap: 8px;
        }}

        .btn {{
            padding: 7px 14px;
            background: #14161f;
            border: 1px solid var(--border-black);
            color: var(--text-white-muted);
            font-size: 12px;
            font-weight: 700;
            cursor: pointer;
            transition: all 0.15s ease;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }}

        .btn:hover {{
            background: #1d212d;
            color: var(--text-white);
            border-color: var(--border-highlight);
        }}

        .btn.active {{
            background: var(--cyan);
            color: #000000;
            border-color: var(--cyan);
            font-weight: 900;
            box-shadow: 0 0 12px rgba(0, 240, 255, 0.4);
        }}

        .level-toggles {{
            display: flex;
            flex-wrap: wrap;
            gap: 8px;
            margin-bottom: 14px;
            padding: 10px 14px;
            background: #08090d;
            border: 1px solid var(--border-black);
        }}

        .level-chip {{
            display: flex;
            align-items: center;
            gap: 6px;
            padding: 4px 10px;
            background: #11131a;
            border: 1px solid var(--border-black);
            font-size: 11px;
            font-weight: 700;
            cursor: pointer;
            user-select: none;
            transition: all 0.15s ease;
        }}

        .level-chip.active {{
            border-color: currentColor;
            background: rgba(255, 255, 255, 0.05);
        }}

        .level-chip .dot {{
            width: 8px;
            height: 8px;
            border-radius: 50%;
            background-color: currentColor;
        }}

        .canvas-container {{
            position: relative;
            height: 480px;
            width: 100%;
        }}

        /* Secondary Grid (Radar + Bar + Neural Inspector) */
        .secondary-grid {{
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 20px;
            margin-bottom: 20px;
        }}

        @media (max-width: 1200px) {{
            .secondary-grid {{
                grid-template-columns: 1fr;
            }}
        }}

        .sub-card {{
            background: var(--card-black);
            border: 1px solid var(--border-black);
            padding: 20px;
        }}

        .sub-card-title {{
            font-size: 14px;
            font-weight: 800;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            margin-bottom: 14px;
            color: var(--text-white);
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}

        .radar-container {{
            position: relative;
            height: 320px;
            width: 100%;
        }}

        /* Neural Layer Inspector Box */
        .layer-box {{
            display: flex;
            flex-direction: column;
            gap: 12px;
        }}

        .layer-node {{
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 12px 16px;
            background: #090a0f;
            border: 1px solid var(--border-black);
            border-left: 4px solid var(--cyan);
        }}

        .layer-node.dense-node {{
            border-left-color: var(--gold);
        }}

        .layer-node.gap-node {{
            border-left-color: var(--magenta);
        }}

        .layer-name {{
            font-size: 13px;
            font-weight: 800;
            color: var(--text-white);
        }}

        .layer-detail {{
            font-size: 12px;
            color: var(--text-white-muted);
            font-family: monospace;
        }}

        .layer-params {{
            font-size: 12px;
            font-weight: 700;
            color: var(--cyan);
            font-family: monospace;
        }}

        /* Table Section & Toolbar */
        .table-section {{
            background: var(--card-black);
            border: 1px solid var(--border-black);
            padding: 22px;
            margin-bottom: 20px;
            overflow-x: auto;
        }}

        .table-toolbar {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            flex-wrap: wrap;
            gap: 12px;
            margin-bottom: 16px;
            padding: 12px 16px;
            background: #08090e;
            border: 1px solid var(--border-black);
        }}

        .quick-sort-bar {{
            display: flex;
            flex-wrap: wrap;
            gap: 6px;
            align-items: center;
        }}

        .quick-sort-btn {{
            padding: 5px 11px;
            background: #11131a;
            border: 1px solid var(--border-black);
            color: var(--text-white-muted);
            font-size: 11px;
            font-weight: 700;
            cursor: pointer;
            transition: all 0.15s ease;
            text-transform: uppercase;
            letter-spacing: 0.3px;
        }}

        .quick-sort-btn:hover {{
            background: #1a1e2b;
            color: var(--text-white);
            border-color: var(--border-highlight);
        }}

        .quick-sort-btn.active {{
            background: var(--cyan);
            color: #000;
            border-color: var(--cyan);
            font-weight: 900;
            box-shadow: 0 0 10px rgba(0, 240, 255, 0.3);
        }}

        table {{
            width: 100%;
            border-collapse: collapse;
            font-size: 12px;
            text-align: left;
        }}

        th {{
            background: #08090d;
            padding: 12px 14px;
            color: var(--text-gray);
            font-weight: 800;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            border-bottom: 2px solid var(--border-black);
            white-space: nowrap;
        }}

        th.sortable {{
            cursor: pointer;
            user-select: none;
            transition: all 0.15s ease;
        }}

        th.sortable:hover {{
            color: var(--text-white);
            background: #151824 !important;
        }}

        th.sorted-active {{
            color: var(--cyan) !important;
            border-bottom: 2px solid var(--cyan) !important;
            background: #0e121e !important;
        }}

        .sort-icon {{
            display: inline-block;
            margin-left: 4px;
            font-size: 10px;
            opacity: 0.6;
        }}

        th.sorted-active .sort-icon {{
            opacity: 1;
            color: var(--cyan);
        }}

        td {{
            padding: 12px 14px;
            border-bottom: 1px solid var(--border-black);
            color: var(--text-white-muted);
            white-space: nowrap;
        }}

        td.sorted-cell {{
            background: rgba(0, 240, 255, 0.035);
            font-weight: 800;
        }}

        tr:hover td {{
            background: #11131a;
            color: var(--text-white);
        }}

        tr.winner-row td {{
            background: rgba(255, 183, 0, 0.08);
            border-bottom-color: rgba(255, 183, 0, 0.3);
        }}

        .rank-badge {{
            display: inline-block;
            padding: 3px 8px;
            font-weight: 900;
            font-size: 11px;
            text-align: center;
        }}

        .rank-1 {{ background: var(--gold); color: #000; }}
        .rank-2 {{ background: #c0c0c0; color: #000; }}
        .rank-3 {{ background: #cd7f32; color: #000; }}
        .rank-other {{ background: #1c1e27; color: var(--text-white-muted); }}

        .neurons-pill {{
            font-family: monospace;
            font-weight: 700;
            color: var(--cyan);
            background: rgba(0, 240, 255, 0.1);
            padding: 3px 8px;
            border: 1px solid rgba(0, 240, 255, 0.2);
        }}
    </style>
</head>
<body>

    <!-- Franja Tricolor Histórica -->
    <div class="flag-strip">
        <div class="band-1"></div>
        <div class="band-2"></div>
        <div class="band-3"></div>
    </div>

    <div class="container">

        <!-- Header -->
        <div class="header">
            <div class="header-title">
                <h1>🧬 ASTRO EVO — COMPARATIVA DE NIVELES DE INTELIGENCIA ({tf})</h1>
                <p>Evaluación de Mutación de Arquitectura Neuronal (L ➔ 2L ➔ 2L) | Apalancamiento: {leverage:.0}X | Capital: {capital_percent:.1}% | Ventana W={window_size}, K={kernel_size}, H={horizon}</p>
            </div>
            <div style="display: flex; gap: 12px; align-items: center; flex-wrap: wrap;">
                <div class="badge-live">⚡ WALK-FORWARD CAUSAL</div>
                <div class="badge-best" id="bestModelBadge">🏆 MEJOR NIVEL: L={best_level} ({best_neurons_desc})</div>
            </div>
        </div>

        <!-- Showcase del Modelo Ganador (Métricas Unificadas: Total Profit y Modo Porcentual Completo) -->
        <div class="winner-master-card">
            <div class="winner-header-bar">
                <div class="winner-header-title">
                    <h2>🏆 RESUMEN COMPLETO DEL MODELO GANADOR (L={best_level})</h2>
                    <span class="pill-badge" style="background: rgba(255, 183, 0, 0.2); border: 1px solid var(--gold); color: var(--gold);">Arquitectura Óptima: {best_neurons_desc}</span>
                </div>
                <div style="display: flex; gap: 10px; align-items: center;">
                    <span style="font-size: 11px; color: var(--text-gray); font-weight: 800;">MÉTRICA UNIFICADA:</span>
                    <span class="badge-live">TOTAL PROFIT (CAPITAL FINAL)</span>
                </div>
            </div>

            <div class="winner-dual-grid">
                <!-- Tarjeta Ganador Modo Porcentual -->
                <div class="winner-box box-pct" id="cardWinnerPct">
                    <div class="box-header">
                        <h3 style="color: var(--cyan);">🚀 Modo Porcentual (Interés Compuesto)</h3>
                        <span class="pill-badge" style="background: rgba(0, 240, 255, 0.15); border: 1px solid var(--cyan); color: var(--cyan);">{capital_percent:.1}% Compuesto</span>
                    </div>
                    <div>
                        <div class="winner-stat-label">Total Profit (Capital Final)</div>
                        <div class="winner-hero-val val-positive" id="winCapPct">$0.00</div>
                    </div>
                    <div class="winner-stat-row">
                        <span class="winner-stat-label">Ganancia Neta ($)</span>
                        <span class="winner-stat-val val-positive" id="winNetPct">$0.00</span>
                    </div>
                    <div class="winner-stat-row">
                        <span class="winner-stat-label">Retorno Total Acumulado</span>
                        <span class="winner-stat-val val-positive" id="winRetPct">+0.00%</span>
                    </div>
                    <div class="winner-stat-row">
                        <span class="winner-stat-label">Max Drawdown (%)</span>
                        <span class="winner-stat-val val-negative" id="winDDPct">0.00%</span>
                    </div>
                    <div class="winner-stat-row">
                        <span class="winner-stat-label">Max Drawdown Nominal ($)</span>
                        <span class="winner-stat-val val-negative" id="winDDNomPct">-$0.00</span>
                    </div>
                    <div class="winner-stat-row">
                        <span class="winner-stat-label">Win Rate &amp; Profit Factor</span>
                        <span class="winner-stat-val val-cyan"><span id="winWinratePct">0.00%</span> | PF: <span id="winPFPct">0.00</span></span>
                    </div>
                    <div class="winner-stat-row">
                        <span class="winner-stat-label">Trades (Longs / Shorts)</span>
                        <span class="winner-stat-val" style="color: var(--text-white-muted);" id="winTradesPct">0 (0L / 0S)</span>
                    </div>
                </div>

                <!-- Tarjeta Ganador Modo Nominal -->
                <div class="winner-box box-nom" id="cardWinnerNom">
                    <div class="box-header">
                        <h3 style="color: var(--imperial-red);">📊 Modo Nominal (Margen Fijo)</h3>
                        <span class="pill-badge" style="background: rgba(230, 0, 38, 0.15); border: 1px solid var(--imperial-red); color: var(--imperial-red);">{capital_percent:.1}% Fijo</span>
                    </div>
                    <div>
                        <div class="winner-stat-label">Total Profit (Capital Final)</div>
                        <div class="winner-hero-val val-positive" id="winCapNom">$0.00</div>
                    </div>
                    <div class="winner-stat-row">
                        <span class="winner-stat-label">Ganancia Neta ($)</span>
                        <span class="winner-stat-val val-positive" id="winNetNom">$0.00</span>
                    </div>
                    <div class="winner-stat-row">
                        <span class="winner-stat-label">Retorno Total Acumulado</span>
                        <span class="winner-stat-val val-positive" id="winRetNom">+0.00%</span>
                    </div>
                    <div class="winner-stat-row">
                        <span class="winner-stat-label">Max Drawdown (%)</span>
                        <span class="winner-stat-val val-negative" id="winDDNom">0.00%</span>
                    </div>
                    <div class="winner-stat-row">
                        <span class="winner-stat-label">Max Drawdown Nominal ($)</span>
                        <span class="winner-stat-val val-negative" id="winDDNomDollar">-$0.00</span>
                    </div>
                    <div class="winner-stat-row">
                        <span class="winner-stat-label">Win Rate &amp; Profit Factor</span>
                        <span class="winner-stat-val val-cyan"><span id="winWinrateNom">0.00%</span> | PF: <span id="winPFNom">0.00</span></span>
                    </div>
                    <div class="winner-stat-row">
                        <span class="winner-stat-label">Trades (Longs / Shorts)</span>
                        <span class="winner-stat-val" style="color: var(--text-white-muted);" id="winTradesNom">0 (0L / 0S)</span>
                    </div>
                </div>

                <!-- Tarjeta Ganador Arquitectura y Robustez Neuronal -->
                <div class="winner-box box-arch">
                    <div class="box-header">
                        <h3 style="color: var(--gold);">🧠 Arquitectura &amp; Robustez</h3>
                        <span class="pill-badge" style="background: rgba(255, 183, 0, 0.15); border: 1px solid var(--gold); color: var(--gold);">Walk-Forward</span>
                    </div>
                    <div>
                        <div class="winner-stat-label">Topología Neuronal (L ➔ 2L ➔ 2L)</div>
                        <div class="winner-hero-val val-gold" id="winNeuronsDesc">{best_neurons_desc}</div>
                    </div>
                    <div class="winner-stat-row">
                        <span class="winner-stat-label">Parámetros Entrenables</span>
                        <span class="winner-stat-val val-gold" id="winParams">0 pesos</span>
                    </div>
                    <div class="winner-stat-row">
                        <span class="winner-stat-label">Linealidad R² Global</span>
                        <span class="winner-stat-val" style="color: #ffffff;" id="winR2">0.0000</span>
                    </div>
                    <div class="winner-stat-row">
                        <span class="winner-stat-label">R² In-Sample / Out-Of-Sample</span>
                        <span class="winner-stat-val" style="font-size: 11px; color: var(--text-gray);" id="winR2IS_OOS">IS: 0.000 | OOS: 0.000</span>
                    </div>
                    <div class="winner-stat-row">
                        <span class="winner-stat-label">Sortino Ratio &amp; Slope</span>
                        <span class="winner-stat-val" id="winSortinoSlope">Sortino: 0.00 | Slope: 0.000</span>
                    </div>
                    <div class="winner-stat-row">
                        <span class="winner-stat-label">Ranking Multicriterio</span>
                        <span class="winner-stat-val val-cyan" id="winScore">#1 (Score: 0.00)</span>
                    </div>
                </div>
            </div>
        </div>

        <!-- Multi-Curve Equity Chart -->
        <div class="chart-section">
            <div class="chart-controls">
                <div class="chart-title">
                    📈 Comparativa de Curvas de Equidad Multi-Inteligencia
                </div>
                <div class="btn-group">
                    <button class="btn active" id="btnNominal" onclick="switchCurveMode('nominal')">Nominal ($)</button>
                    <button class="btn" id="btnPercent" onclick="switchCurveMode('percent')">Porcentual (%)</button>
                    <button class="btn" onclick="toggleAllLevels(true)">Ver Todos</button>
                    <button class="btn" onclick="toggleAllLevels(false)">Ocultar Todos</button>
                </div>
            </div>

            <!-- Level Selector Chips -->
            <div class="level-toggles" id="levelChipsContainer"></div>

            <div class="canvas-container">
                <canvas id="equityChart"></canvas>
            </div>
        </div>

        <!-- Secondary Grid: Radar + Parameters vs Return + Neural Layer Inspector -->
        <div class="secondary-grid">
            <div class="sub-card">
                <div class="sub-card-title">
                    <span>🎯 Análisis Multicriterio (Radar de Rendimiento)</span>
                </div>
                <div class="radar-container">
                    <canvas id="radarChart"></canvas>
                </div>
            </div>

            <div class="sub-card">
                <div class="sub-card-title">
                    <span>🧠 Inspector de Arquitectura Neuronal</span>
                    <select id="levelInspectorSelect" onchange="updateNeuralInspector(this.value)" style="background: #14161f; color: var(--text-white); border: 1px solid var(--border-black); padding: 4px 8px; font-weight: 700;"></select>
                </div>
                <div class="layer-box" id="neuralLayersContainer"></div>
            </div>
        </div>

        <!-- Comparative Metrics Table -->
        <div class="table-section">
            <div class="sub-card-title" style="margin-bottom: 12px;">
                <span>📊 Tabla Comparativa Completa de Niveles de Inteligencia</span>
                <span id="sortStatusLabel" style="font-size: 11px; color: var(--cyan); font-weight: 800;">Ordenado por: Ranking General (Menor es Mejor)</span>
            </div>

            <!-- Table Toolbar (Quick Sort & View Columns) -->
            <div class="table-toolbar">
                <div class="quick-sort-bar">
                    <span style="font-size: 11px; color: var(--text-gray); font-weight: 800; margin-right: 4px;">ORDENAR POR:</span>
                    <button class="quick-sort-btn active" id="btnSortRank" onclick="applyQuickSort('rank')">🏆 Ranking</button>
                    <button class="quick-sort-btn" id="btnSortR2" onclick="applyQuickSort('r2')">📈 Mejor R²</button>
                    <button class="quick-sort-btn" id="btnSortWinrate" onclick="applyQuickSort('winrate')">🎯 Win Rate</button>
                    <button class="quick-sort-btn" id="btnSortTotNom" onclick="applyQuickSort('total_profit_nom')">💰 Total Profit Nom</button>
                    <button class="quick-sort-btn" id="btnSortTotPct" onclick="applyQuickSort('total_profit_pct')">🚀 Total Profit Compuesto</button>
                    <button class="quick-sort-btn" id="btnSortNetNom" onclick="applyQuickSort('net_profit_nom')">💵 Net Profit</button>
                    <button class="quick-sort-btn" id="btnSortRetNom" onclick="applyQuickSort('return_nom')">📊 Retorno %</button>
                    <button class="quick-sort-btn" id="btnSortDD" onclick="applyQuickSort('dd')">🛡️ Menor Drawdown</button>
                    <button class="quick-sort-btn" id="btnSortPF" onclick="applyQuickSort('pf')">⚡ Profit Factor</button>
                    <button class="quick-sort-btn" id="btnSortSortino" onclick="applyQuickSort('sortino')">⭐ Sortino</button>
                </div>

                <div style="display: flex; gap: 8px; align-items: center;">
                    <span style="font-size: 11px; color: var(--text-gray); font-weight: 800;">VISTA:</span>
                    <button class="btn active" id="btnViewDual" onclick="switchTableView('dual')" style="padding: 4px 10px; font-size: 11px;">⚡ Completa (Dual)</button>
                    <button class="btn" id="btnViewNom" onclick="switchTableView('nominal')" style="padding: 4px 10px; font-size: 11px;">📊 Solo Nominal</button>
                    <button class="btn" id="btnViewPct" onclick="switchTableView('percent')" style="padding: 4px 10px; font-size: 11px;">🚀 Solo Porcentual</button>
                </div>
            </div>

            <table>
                <thead>
                    <tr>
                        <th class="sortable" onclick="handleHeaderSort('rank')">Rank <span class="sort-icon" id="icon_rank">▲</span></th>
                        <th class="sortable" onclick="handleHeaderSort('level')">Nivel (L) <span class="sort-icon" id="icon_level">⇅</span></th>
                        <th>Neuronas (L ➔ 2L ➔ 2L)</th>
                        <th class="sortable" onclick="handleHeaderSort('params')">Parámetros <span class="sort-icon" id="icon_params">⇅</span></th>
                        
                        <!-- Columnas Nominales -->
                        <th class="col-nom sortable" onclick="handleHeaderSort('total_profit_nom')">Total Profit Nom ($) <span class="sort-icon" id="icon_total_profit_nom">⇅</span></th>
                        <th class="col-nom sortable" onclick="handleHeaderSort('net_profit_nom')">Net Profit Nom ($) <span class="sort-icon" id="icon_net_profit_nom">⇅</span></th>
                        <th class="col-nom sortable" onclick="handleHeaderSort('return_nom')">Retorno Nom % <span class="sort-icon" id="icon_return_nom">⇅</span></th>
                        <th class="col-nom sortable" onclick="handleHeaderSort('dd_nom')">Max DD Nom % <span class="sort-icon" id="icon_dd_nom">⇅</span></th>
                        <th class="col-nom sortable" onclick="handleHeaderSort('winrate_nom')">Win Rate Nom % <span class="sort-icon" id="icon_winrate_nom">⇅</span></th>
                        <th class="col-nom sortable" onclick="handleHeaderSort('pf_nom')">PF Nom <span class="sort-icon" id="icon_pf_nom">⇅</span></th>

                        <!-- Columnas Porcentuales (Interés Compuesto) -->
                        <th class="col-pct sortable" onclick="handleHeaderSort('total_profit_pct')">Total Profit Comp. ($) <span class="sort-icon" id="icon_total_profit_pct">⇅</span></th>
                        <th class="col-pct sortable" onclick="handleHeaderSort('return_pct')">Retorno Comp. % <span class="sort-icon" id="icon_return_pct">⇅</span></th>
                        <th class="col-pct sortable" onclick="handleHeaderSort('dd_pct')">Max DD Comp. % <span class="sort-icon" id="icon_dd_pct">⇅</span></th>
                        <th class="col-pct sortable" onclick="handleHeaderSort('winrate_pct')">Win Rate Comp. % <span class="sort-icon" id="icon_winrate_pct">⇅</span></th>
                        <th class="col-pct sortable" onclick="handleHeaderSort('pf_pct')">PF Comp. <span class="sort-icon" id="icon_pf_pct">⇅</span></th>

                        <!-- Métricas Causalidad y Robustez -->
                        <th class="sortable" onclick="handleHeaderSort('trades')">Trades <span class="sort-icon" id="icon_trades">⇅</span></th>
                        <th class="sortable" onclick="handleHeaderSort('sortino')">Sortino <span class="sort-icon" id="icon_sortino">⇅</span></th>
                        <th class="sortable" onclick="handleHeaderSort('r2')">R² Global <span class="sort-icon" id="icon_r2">⇅</span></th>
                        <th>R² IS / OOS</th>
                        <th class="sortable" onclick="handleHeaderSort('slope')">Slope Ratio <span class="sort-icon" id="icon_slope">⇅</span></th>
                        <th class="sortable" onclick="handleHeaderSort('score')">Score Ponderado <span class="sort-icon" id="icon_score">⇅</span></th>
                    </tr>
                </thead>
                <tbody id="metricsTableBody"></tbody>
            </table>
        </div>

    </div>

    <script>
        const rawData = {json_data};
        let currentMode = 'nominal';
        let equityChart = null;
        let radarChart = null;
        let activeLevelVisibility = {{}};
        let currentSortKey = 'rank';
        let currentSortAsc = true;
        let currentTableView = 'dual';

        function initDashboard() {{
            if (!rawData || !rawData.levels || rawData.levels.length === 0) return;

            // Inicializar visibilidad de niveles
            rawData.levels.forEach((lvl) => {{
                activeLevelVisibility[lvl.level] = true;
            }});

            populateWinnerShowcase();
            renderLevelChips();
            initEquityChart();
            initRadarChart();
            populateInspectorSelect();
            updateNeuralInspector(rawData.levels[0].level);
            populateTable();
        }}

        function getTotalProfitNom(lvl) {{
            if (lvl.total_profit_nom !== undefined && lvl.total_profit_nom !== null) {{
                return lvl.total_profit_nom;
            }}
            if (lvl.eq_nom && lvl.eq_nom.length > 0) {{
                return lvl.eq_nom[lvl.eq_nom.length - 1];
            }}
            return lvl.net_profit_nom + 10000.0;
        }}

        function getTotalProfitPct(lvl) {{
            if (lvl.total_profit_pct !== undefined && lvl.total_profit_pct !== null) {{
                return lvl.total_profit_pct;
            }}
            if (lvl.eq_pct && lvl.eq_pct.length > 0) {{
                return lvl.eq_pct[lvl.eq_pct.length - 1];
            }}
            return lvl.net_profit_pct + 10000.0;
        }}

        function populateWinnerShowcase() {{
            const best = rawData.levels[0];
            if (!best) return;

            document.getElementById('bestModelBadge').innerText = `🏆 MEJOR NIVEL: L=${{best.level}} (${{best.neurons_desc}})`;

            // 1. Métricas Porcentuales (Compuesto) del Modelo Ganador
            const capPct = getTotalProfitPct(best);
            const netPct = best.net_profit_pct;
            document.getElementById('winCapPct').innerText = `$${{capPct.toLocaleString('en-US', {{minimumFractionDigits: 2, maximumFractionDigits: 2}})}}`;
            document.getElementById('winNetPct').innerText = `$${{netPct.toLocaleString('en-US', {{minimumFractionDigits: 2, maximumFractionDigits: 2}})}}`;
            document.getElementById('winRetPct').innerText = `${{best.total_return_pct_pct >= 0 ? '+' : ''}}${{best.total_return_pct_pct.toFixed(2)}}%`;
            document.getElementById('winDDPct').innerText = `${{best.max_dd_pct_pct.toFixed(2)}}%`;
            document.getElementById('winDDNomPct').innerText = `-$${{best.max_dd_pct_dollar.toLocaleString('en-US', {{minimumFractionDigits: 2, maximumFractionDigits: 2}})}}`;
            document.getElementById('winWinratePct').innerText = `${{best.winrate_pct.toFixed(2)}}%`;
            document.getElementById('winPFPct').innerText = best.profit_factor_pct >= 99 ? '∞' : best.profit_factor_pct.toFixed(2);
            document.getElementById('winTradesPct').innerText = `${{best.total_trades_pct}} (${{best.longs_count_pct}}L / ${{best.shorts_count_pct}}S)`;

            // 2. Métricas Nominales (Margen Fijo) del Modelo Ganador (Unificadas a Total Profit)
            const capNom = getTotalProfitNom(best);
            const netNom = best.net_profit_nom;
            document.getElementById('winCapNom').innerText = `$${{capNom.toLocaleString('en-US', {{minimumFractionDigits: 2, maximumFractionDigits: 2}})}}`;
            document.getElementById('winNetNom').innerText = `$${{netNom.toLocaleString('en-US', {{minimumFractionDigits: 2, maximumFractionDigits: 2}})}}`;
            document.getElementById('winRetNom').innerText = `${{best.total_return_nom_pct >= 0 ? '+' : ''}}${{best.total_return_nom_pct.toFixed(2)}}%`;
            document.getElementById('winDDNom').innerText = `${{best.max_dd_nom_pct.toFixed(2)}}%`;
            document.getElementById('winDDNomDollar').innerText = `-$${{best.max_dd_nom_dollar.toLocaleString('en-US', {{minimumFractionDigits: 2, maximumFractionDigits: 2}})}}`;
            document.getElementById('winWinrateNom').innerText = `${{best.winrate_nom.toFixed(2)}}%`;
            document.getElementById('winPFNom').innerText = best.profit_factor_nom >= 99 ? '∞' : best.profit_factor_nom.toFixed(2);
            document.getElementById('winTradesNom').innerText = `${{best.total_trades_nom}} (${{best.longs_count_nom}}L / ${{best.shorts_count_nom}}S)`;

            // 3. Arquitectura y Robustez Neuronal
            document.getElementById('winNeuronsDesc').innerText = best.neurons_desc;
            document.getElementById('winParams').innerText = `${{best.total_params.toLocaleString()}} pesos`;
            document.getElementById('winR2').innerText = best.curve_r2_nom.toFixed(4);
            document.getElementById('winR2IS_OOS').innerText = `IS: ${{best.r2_is_nom.toFixed(3)}} | OOS: ${{best.r2_oos_nom.toFixed(3)}}`;
            document.getElementById('winSortinoSlope').innerText = `Sortino: ${{best.sortino_nom.toFixed(2)}} | Slope: ${{best.slope_ratio_nom.toFixed(3)}}`;
            document.getElementById('winScore').innerText = `#1 (Score: ${{best.weighted_avg_rank.toFixed(2)}})`;
        }}

        function renderLevelChips() {{
            const container = document.getElementById('levelChipsContainer');
            container.innerHTML = '';

            rawData.levels.forEach((lvl) => {{
                const chip = document.createElement('div');
                chip.className = `level-chip ${{activeLevelVisibility[lvl.level] ? 'active' : ''}}`;
                chip.style.color = lvl.color_hex;
                chip.innerHTML = `<span class="dot"></span> Nivel ${{lvl.level}} (${{lvl.neurons_desc}}) — ${{lvl.total_params}} params`;
                chip.onclick = () => {{
                    activeLevelVisibility[lvl.level] = !activeLevelVisibility[lvl.level];
                    chip.classList.toggle('active');
                    updateEquityChartDatasets();
                }};
                container.appendChild(chip);
            }});
        }}

        function toggleAllLevels(visible) {{
            rawData.levels.forEach((lvl) => {{
                activeLevelVisibility[lvl.level] = visible;
            }});
            const chips = document.querySelectorAll('.level-chip');
            chips.forEach(c => {{
                if (visible) c.classList.add('active');
                else c.classList.remove('active');
            }});
            updateEquityChartDatasets();
        }}

        function switchCurveMode(mode) {{
            currentMode = mode;
            document.getElementById('btnNominal').classList.toggle('active', mode === 'nominal');
            document.getElementById('btnPercent').classList.toggle('active', mode === 'percent');

            // Resaltar la tarjeta activa del ganador
            const cardPct = document.getElementById('cardWinnerPct');
            const cardNom = document.getElementById('cardWinnerNom');
            if (cardPct && cardNom) {{
                cardPct.classList.toggle('active-mode-card', mode === 'percent');
                cardNom.classList.toggle('active-mode-card', mode === 'nominal');
            }}

            updateEquityChartDatasets();
        }}

        function getChartDatasets() {{
            return rawData.levels.map((lvl) => {{
                const series = currentMode === 'nominal' ? lvl.eq_nom : lvl.eq_pct;
                return {{
                    label: `Nivel ${{lvl.level}} (${{lvl.neurons_desc}})`,
                    data: series,
                    borderColor: lvl.color_hex,
                    backgroundColor: 'transparent',
                    borderWidth: 2,
                    pointRadius: 0,
                    pointHoverRadius: 5,
                    tension: 0.1,
                    hidden: !activeLevelVisibility[lvl.level],
                }};
            }});
        }}

        function initEquityChart() {{
            const ctx = document.getElementById('equityChart').getContext('2d');
            equityChart = new Chart(ctx, {{
                type: 'line',
                data: {{
                    labels: rawData.dates,
                    datasets: getChartDatasets()
                }},
                options: {{
                    responsive: true,
                    maintainAspectRatio: false,
                    interaction: {{
                        mode: 'index',
                        intersect: false,
                    }},
                    plugins: {{
                        legend: {{
                            display: false,
                        }},
                        tooltip: {{
                            backgroundColor: '#0a0b10',
                            titleColor: '#ffffff',
                            bodyColor: '#c8cdd8',
                            borderColor: '#2c303f',
                            borderWidth: 1,
                            padding: 12,
                            callbacks: {{
                                label: function(context) {{
                                    let label = context.dataset.label || '';
                                    let val = context.parsed.y;
                                    return `${{label}}: $${{val.toLocaleString('en-US', {{minimumFractionDigits: 2, maximumFractionDigits: 2}})}}`;
                                }}
                            }}
                        }}
                    }},
                    scales: {{
                        x: {{
                            grid: {{ color: '#14161f' }},
                            ticks: {{ color: '#777e90', maxTicksLimit: 12 }}
                        }},
                        y: {{
                            grid: {{ color: '#14161f' }},
                            ticks: {{
                                color: '#777e90',
                                callback: function(value) {{
                                    return '$' + value.toLocaleString();
                                }}
                            }}
                        }}
                    }}
                }}
            }});
        }}

        function updateEquityChartDatasets() {{
            if (!equityChart) return;
            equityChart.data.datasets = getChartDatasets();
            equityChart.update();
        }}

        function initRadarChart() {{
            const ctx = document.getElementById('radarChart').getContext('2d');
            const topLevels = rawData.levels.slice(0, 5);

            const datasets = topLevels.map((lvl) => {{
                const normProfit = Math.min(100, Math.max(0, (lvl.net_profit_nom / 50000) * 100));
                const normWinrate = lvl.winrate_nom;
                const normR2 = lvl.curve_r2_nom * 100;
                const normResilience = Math.max(0, 100 - lvl.max_dd_nom_pct);
                const normSortino = Math.min(100, (lvl.sortino_nom / 3.0) * 100);

                return {{
                    label: `L=${{lvl.level}} (${{lvl.neurons_desc}})`,
                    data: [normProfit, normWinrate, normR2, normResilience, normSortino],
                    borderColor: lvl.color_hex,
                    backgroundColor: lvl.color_hex + '22',
                    borderWidth: 2,
                    pointBackgroundColor: lvl.color_hex,
                }};
            }});

            radarChart = new Chart(ctx, {{
                type: 'radar',
                data: {{
                    labels: ['Net Profit', 'Win Rate %', 'Linealidad R²', 'Resistencia DD', 'Sortino Ratio'],
                    datasets: datasets
                }},
                options: {{
                    responsive: true,
                    maintainAspectRatio: false,
                    plugins: {{
                        legend: {{
                            position: 'top',
                            labels: {{ color: '#c8cdd8', font: {{ size: 11 }} }}
                        }}
                    }},
                    scales: {{
                        r: {{
                            angleLines: {{ color: '#1c1e27' }},
                            grid: {{ color: '#1c1e27' }},
                            pointLabels: {{ color: '#c8cdd8', font: {{ size: 11, weight: 'bold' }} }},
                            ticks: {{ display: false, max: 100, min: 0 }}
                        }}
                    }}
                }}
            }});
        }}

        function populateInspectorSelect() {{
            const sel = document.getElementById('levelInspectorSelect');
            sel.innerHTML = '';
            rawData.levels.forEach(lvl => {{
                const opt = document.createElement('option');
                opt.value = lvl.level;
                opt.text = `Nivel ${{lvl.level}} (${{lvl.neurons_desc}}) — ${{lvl.total_params}} params`;
                sel.appendChild(opt);
            }});
        }}

        function updateNeuralInspector(levelVal) {{
            const lvl = rawData.levels.find(l => l.level == levelVal) || rawData.levels[0];
            const container = document.getElementById('neuralLayersContainer');

            container.innerHTML = `
                <div class="layer-node">
                    <div>
                        <div class="layer-name">🔷 Capa 1: Convolución 1D (Conv1D)</div>
                        <div class="layer-detail">Entrada: ${{rawData.tf === '1H' ? 35 : 34}} Canales ➔ Salida: ${{lvl.conv1_filters}} Filtros (Kernel K=${{rawData.kernel_size}})</div>
                    </div>
                    <div class="layer-params">${{lvl.conv1_params.toLocaleString()}} pesos</div>
                </div>

                <div class="layer-node">
                    <div>
                        <div class="layer-name">🔷 Capa 2: Convolución 1D Profunda (Conv1D)</div>
                        <div class="layer-detail">Entrada: ${{lvl.conv1_filters}} Canales ➔ Salida: ${{lvl.conv2_filters}} Filtros (Kernel K=${{rawData.kernel_size}})</div>
                    </div>
                    <div class="layer-params">${{lvl.conv2_params.toLocaleString()}} pesos</div>
                </div>

                <div class="layer-node gap-node">
                    <div>
                        <div class="layer-name">🔄 Capa 3: Global Average Pooling (GAP)</div>
                        <div class="layer-detail">Compresión Temporal: Ventana W=${{rawData.window_size}} ➔ Vector ${{lvl.dense_neurons}}D</div>
                    </div>
                    <div class="layer-params">0 pesos (Invariante)</div>
                </div>

                <div class="layer-node dense-node">
                    <div>
                        <div class="layer-name">⭐ Capa 4: Densa / Clasificación (Softmax)</div>
                        <div class="layer-detail">Entrada: ${{lvl.dense_neurons}} Neuronas ➔ Salida: 3 Clases [LONG, SHORT, FLAT]</div>
                    </div>
                    <div class="layer-params">${{lvl.dense_params.toLocaleString()}} pesos</div>
                </div>

                <div style="display: flex; justify-content: space-between; padding: 10px 16px; background: #06070a; border: 1px solid var(--border-black); font-size: 13px; font-weight: 800;">
                    <span style="color: var(--text-white);">TOTAL PARÁMETROS ENTRENABLES</span>
                    <span style="color: var(--gold); font-family: monospace;">${{lvl.total_params.toLocaleString()}} PARÁMETROS</span>
                </div>
            `;
        }}

        // ===== LÓGICA DE ORDENAMIENTO INTERACTIVO DE LA TABLA =====
        function getMetricValue(lvl, key) {{
            switch (key) {{
                case 'rank': return lvl.weighted_avg_rank;
                case 'level': return lvl.level;
                case 'params': return lvl.total_params;
                case 'total_profit_nom': return getTotalProfitNom(lvl);
                case 'net_profit_nom': return lvl.net_profit_nom;
                case 'return_nom': return lvl.total_return_nom_pct;
                case 'dd_nom': return lvl.max_dd_nom_pct;
                case 'winrate_nom': return lvl.winrate_nom;
                case 'pf_nom': return lvl.profit_factor_nom;
                case 'total_profit_pct': return getTotalProfitPct(lvl);
                case 'return_pct': return lvl.total_return_pct_pct;
                case 'dd_pct': return lvl.max_dd_pct_pct;
                case 'winrate_pct': return lvl.winrate_pct;
                case 'pf_pct': return lvl.profit_factor_pct;
                case 'trades': return lvl.total_trades_nom;
                case 'sortino': return lvl.sortino_nom;
                case 'r2': return lvl.curve_r2_nom;
                case 'slope': return lvl.slope_ratio_nom;
                case 'score': return lvl.weighted_avg_rank;
                default: return 0;
            }}
        }}

        function isLowerBetter(key) {{
            return key === 'rank' || key === 'score' || key === 'dd_nom' || key === 'dd_pct' || key === 'dd' || key === 'params' || key === 'level';
        }}

        function handleHeaderSort(key) {{
            if (currentSortKey === key) {{
                currentSortAsc = !currentSortAsc;
            }} else {{
                currentSortKey = key;
                currentSortAsc = isLowerBetter(key);
            }}
            updateSortUI();
            populateTable();
        }}

        function applyQuickSort(key) {{
            let mappedKey = key;
            if (key === 'winrate') mappedKey = 'winrate_nom';
            if (key === 'dd') mappedKey = 'dd_nom';
            if (key === 'pf') mappedKey = 'pf_nom';

            if (currentSortKey === mappedKey) {{
                currentSortAsc = !currentSortAsc;
            }} else {{
                currentSortKey = mappedKey;
                currentSortAsc = isLowerBetter(mappedKey);
            }}
            updateSortUI();
            populateTable();
        }}

        function updateSortUI() {{
            // Actualizar botones de Quick Sort
            const buttons = document.querySelectorAll('.quick-sort-btn');
            buttons.forEach(b => b.classList.remove('active'));

            if (currentSortKey === 'rank') {{
                const b = document.getElementById('btnSortRank'); if (b) b.classList.add('active');
            }} else if (currentSortKey === 'r2') {{
                const b = document.getElementById('btnSortR2'); if (b) b.classList.add('active');
            }} else if (currentSortKey === 'winrate_nom' || currentSortKey === 'winrate_pct') {{
                const b = document.getElementById('btnSortWinrate'); if (b) b.classList.add('active');
            }} else if (currentSortKey === 'total_profit_nom') {{
                const b = document.getElementById('btnSortTotNom'); if (b) b.classList.add('active');
            }} else if (currentSortKey === 'total_profit_pct') {{
                const b = document.getElementById('btnSortTotPct'); if (b) b.classList.add('active');
            }} else if (currentSortKey === 'net_profit_nom') {{
                const b = document.getElementById('btnSortNetNom'); if (b) b.classList.add('active');
            }} else if (currentSortKey === 'return_nom') {{
                const b = document.getElementById('btnSortRetNom'); if (b) b.classList.add('active');
            }} else if (currentSortKey === 'dd_nom' || currentSortKey === 'dd_pct') {{
                const b = document.getElementById('btnSortDD'); if (b) b.classList.add('active');
            }} else if (currentSortKey === 'pf_nom' || currentSortKey === 'pf_pct') {{
                const b = document.getElementById('btnSortPF'); if (b) b.classList.add('active');
            }} else if (currentSortKey === 'sortino') {{
                const b = document.getElementById('btnSortSortino'); if (b) b.classList.add('active');
            }}

            // Actualizar encabezados e iconos
            const allTh = document.querySelectorAll('th.sortable');
            allTh.forEach(th => {{
                th.classList.remove('sorted-active');
            }});

            const allIcons = document.querySelectorAll('.sort-icon');
            allIcons.forEach(icon => {{
                icon.innerText = '⇅';
            }});

            const activeIcon = document.getElementById(`icon_${{currentSortKey}}`);
            if (activeIcon) {{
                activeIcon.innerText = currentSortAsc ? '▲' : '▼';
                if (activeIcon.parentElement) {{
                    activeIcon.parentElement.classList.add('sorted-active');
                }}
            }}

            const statusLabel = document.getElementById('sortStatusLabel');
            if (statusLabel) {{
                const dir = currentSortAsc ? 'Ascendente (Menor primero)' : 'Descendente (Mayor primero)';
                statusLabel.innerText = `Ordenado por: ${{currentSortKey.toUpperCase()}} [${{dir}}]`;
            }}
        }}

        function switchTableView(mode) {{
            currentTableView = mode;
            document.getElementById('btnViewDual').classList.toggle('active', mode === 'dual');
            document.getElementById('btnViewNom').classList.toggle('active', mode === 'nominal');
            document.getElementById('btnViewPct').classList.toggle('active', mode === 'percent');

            const nomCols = document.querySelectorAll('.col-nom');
            const pctCols = document.querySelectorAll('.col-pct');

            nomCols.forEach(col => {{
                col.style.display = (mode === 'dual' || mode === 'nominal') ? '' : 'none';
            }});
            pctCols.forEach(col => {{
                col.style.display = (mode === 'dual' || mode === 'percent') ? '' : 'none';
            }});
        }}

        function populateTable() {{
            const tbody = document.getElementById('metricsTableBody');
            tbody.innerHTML = '';

            // Copia ordenada de niveles
            const sortedLevels = [...rawData.levels].sort((a, b) => {{
                const valA = getMetricValue(a, currentSortKey);
                const valB = getMetricValue(b, currentSortKey);
                if (valA < valB) return currentSortAsc ? -1 : 1;
                if (valA > valB) return currentSortAsc ? 1 : -1;
                return 0;
            }});

            sortedLevels.forEach((lvl, currentIdx) => {{
                const tr = document.createElement('tr');
                if (lvl.level === rawData.levels[0].level) {{
                    tr.className = 'winner-row';
                }}

                let rankBadgeClass = 'rank-other';
                if (currentIdx === 0) rankBadgeClass = 'rank-1';
                else if (currentIdx === 1) rankBadgeClass = 'rank-2';
                else if (currentIdx === 2) rankBadgeClass = 'rank-3';

                const totalNom = getTotalProfitNom(lvl);
                const totalPct = getTotalProfitPct(lvl);

                const totalNomFmt = `$${{totalNom.toLocaleString('en-US', {{minimumFractionDigits: 2, maximumFractionDigits: 2}})}}`;
                const netNomFmt = `$${{lvl.net_profit_nom.toLocaleString('en-US', {{minimumFractionDigits: 2, maximumFractionDigits: 2}})}}`;
                const totalPctFmt = `$${{totalPct.toLocaleString('en-US', {{minimumFractionDigits: 2, maximumFractionDigits: 2}})}}`;

                const profitClassNom = totalNom >= 10000.0 ? 'val-positive' : 'val-negative';
                const profitClassPct = totalPct >= 10000.0 ? 'val-positive' : 'val-negative';

                tr.innerHTML = `
                    <td class="${{currentSortKey === 'rank' ? 'sorted-cell' : ''}}"><span class="rank-badge ${{rankBadgeClass}}">#${{currentIdx + 1}}</span></td>
                    <td class="${{currentSortKey === 'level' ? 'sorted-cell' : ''}}" style="font-weight: 800; color: ${{lvl.color_hex}};">L=${{lvl.level}}</td>
                    <td><span class="neurons-pill">${{lvl.neurons_desc}}</span></td>
                    <td class="${{currentSortKey === 'params' ? 'sorted-cell' : ''}}" style="font-family: monospace; font-weight: 700;">${{lvl.total_params.toLocaleString()}}</td>
                    
                    <!-- Métricas Nominales -->
                    <td class="col-nom ${{currentSortKey === 'total_profit_nom' ? 'sorted-cell' : ''}} ${{profitClassNom}}" style="font-weight: 800; font-family: monospace;">${{totalNomFmt}}</td>
                    <td class="col-nom ${{currentSortKey === 'net_profit_nom' ? 'sorted-cell' : ''}} ${{profitClassNom}}" style="font-family: monospace;">${{netNomFmt}}</td>
                    <td class="col-nom ${{currentSortKey === 'return_nom' ? 'sorted-cell' : ''}} ${{profitClassNom}}">${{lvl.total_return_nom_pct >= 0 ? '+' : ''}}${{lvl.total_return_nom_pct.toFixed(2)}}%</td>
                    <td class="col-nom ${{currentSortKey === 'dd_nom' ? 'sorted-cell' : ''}} val-negative" style="font-weight: 700;">${{lvl.max_dd_nom_pct.toFixed(2)}}%</td>
                    <td class="col-nom ${{currentSortKey === 'winrate_nom' ? 'sorted-cell' : ''}}" style="font-weight: 700;">${{lvl.winrate_nom.toFixed(2)}}%</td>
                    <td class="col-nom ${{currentSortKey === 'pf_nom' ? 'sorted-cell' : ''}}">${{lvl.profit_factor_nom >= 99 ? '∞' : lvl.profit_factor_nom.toFixed(2)}}</td>

                    <!-- Métricas Porcentuales (Compuesto) -->
                    <td class="col-pct ${{currentSortKey === 'total_profit_pct' ? 'sorted-cell' : ''}} ${{profitClassPct}}" style="font-weight: 800; font-family: monospace;">${{totalPctFmt}}</td>
                    <td class="col-pct ${{currentSortKey === 'return_pct' ? 'sorted-cell' : ''}} ${{profitClassPct}}">${{lvl.total_return_pct_pct >= 0 ? '+' : ''}}${{lvl.total_return_pct_pct.toFixed(2)}}%</td>
                    <td class="col-pct ${{currentSortKey === 'dd_pct' ? 'sorted-cell' : ''}} val-negative" style="font-weight: 700;">${{lvl.max_dd_pct_pct.toFixed(2)}}%</td>
                    <td class="col-pct ${{currentSortKey === 'winrate_pct' ? 'sorted-cell' : ''}}" style="font-weight: 700;">${{lvl.winrate_pct.toFixed(2)}}%</td>
                    <td class="col-pct ${{currentSortKey === 'pf_pct' ? 'sorted-cell' : ''}}">${{lvl.profit_factor_pct >= 99 ? '∞' : lvl.profit_factor_pct.toFixed(2)}}</td>

                    <!-- Métricas Robustez -->
                    <td class="${{currentSortKey === 'trades' ? 'sorted-cell' : ''}}">${{lvl.total_trades_nom}} (${{lvl.longs_count_nom}}L / ${{lvl.shorts_count_nom}}S)</td>
                    <td class="${{currentSortKey === 'sortino' ? 'sorted-cell' : ''}}">${{lvl.sortino_nom.toFixed(2)}}</td>
                    <td class="${{currentSortKey === 'r2' ? 'sorted-cell' : ''}}" style="font-weight: 700;">${{lvl.curve_r2_nom.toFixed(4)}}</td>
                    <td style="color: var(--text-gray); font-size: 11px;">IS: ${{lvl.r2_is_nom.toFixed(3)}} | OOS: ${{lvl.r2_oos_nom.toFixed(3)}}</td>
                    <td class="${{currentSortKey === 'slope' ? 'sorted-cell' : ''}}">${{lvl.slope_ratio_nom.toFixed(3)}}</td>
                    <td class="${{currentSortKey === 'score' ? 'sorted-cell' : ''}}" style="font-weight: 900; color: var(--cyan);">${{lvl.weighted_avg_rank.toFixed(2)}}</td>
                `;
                tbody.appendChild(tr);
            }});

            switchTableView(currentTableView);
        }}

        window.onload = initDashboard;
    </script>
</body>
</html>
"#,
        tf = data.tf,
        leverage = data.leverage,
        capital_percent = data.capital_percent,
        window_size = data.window_size,
        kernel_size = data.kernel_size,
        horizon = data.horizon,
        best_level = data.best_level,
        best_neurons_desc = data.best_neurons_desc,
        json_data = json_data
    )
}
