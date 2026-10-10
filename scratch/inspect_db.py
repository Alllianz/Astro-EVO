import sqlite3

for db_name in ['models.db', 'candles.db']:
    try:
        conn = sqlite3.connect(db_name)
        tables = conn.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchall()
        print(f"Tables in {db_name}: {tables}")
        for t in tables:
            count = conn.execute(f"SELECT count(*) FROM {t[0]}").fetchone()[0]
            print(f"  Table {t[0]}: {count} rows")
            if 'model' in t[0] or 'record' in t[0]:
                cols = [desc[0] for desc in conn.execute(f"SELECT * FROM {t[0]} LIMIT 1").description]
                print(f"    Columns: {cols}")
                rows = conn.execute(f"SELECT {', '.join(cols[:6])} FROM {t[0]} LIMIT 5").fetchall()
                print(f"    Sample: {rows}")
    except Exception as e:
        print(f"Error {db_name}: {e}")
