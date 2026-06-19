import re
with open("src/main.rs", "r") as f:
    for i, line in enumerate(f):
        if re.search(r'[А-Яа-яЁё]', line):
            print(f"{i+1}: {line.strip()}")
