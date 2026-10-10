import sqlite3

conn = sqlite3.connect('models.db')
rows = conn.execute("SELECT level, timeframe, epoch, window_size, kernel_size, horizon, leverage, weight_decay, l1_lambda FROM intelligence_history ORDER BY level").fetchall()
for r in rows:
    print(r)
