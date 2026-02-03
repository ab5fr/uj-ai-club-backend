import sqlite3
conn = sqlite3.connect("/data/nbgrader.db")
cursor = conn.cursor()

print("=== Tables ===")
cursor.execute("SELECT name FROM sqlite_master WHERE type='table'")
for t in cursor.fetchall():
    print(t[0])

print("\n=== grade table columns ===")
cursor.execute("PRAGMA table_info(grade)")
for col in cursor.fetchall():
    print(col)

print("\n=== submitted_notebook table columns ===")
cursor.execute("PRAGMA table_info(submitted_notebook)")
for col in cursor.fetchall():
    print(col)

print("\n=== Recent grades ===")
try:
    cursor.execute("SELECT * FROM grade LIMIT 5")
    for row in cursor.fetchall():
        print(row)
except Exception as e:
    print(f"Error: {e}")

conn.close()
