import sqlite3

conn = sqlite3.connect('models.db')
tables = conn.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchall()
print("Tables in models.db:")
for t in tables:
    name = t[0]
    count = conn.execute(f"SELECT count(*) FROM {name}").fetchone()[0]
    cols = [desc[0] for desc in conn.execute(f"SELECT * FROM {name} LIMIT 1").description]
    print(f"  {name} ({count} rows): {cols}")
