import re

with open("src/main.rs", "r") as f:
    content = f.read()

replacements = {
    '// 1 МБ': '// 1 MB',
    '// ======================= УТИЛИТЫ ДЛЯ БИТОВ =======================': '// ======================= BIT UTILITIES =======================',
    '// ======================= ЯДРО АЛГОРИТМА =======================': '// ======================= CORE ALGORITHM =======================',
    '// Идеальный ИИ-Анализатор (Разведка боем): 6 путей в памяти': '// Perfect AI Analyzer (Trial & Error): 8 paths in RAM',
}

for ru, en in replacements.items():
    content = content.replace(ru, en)

with open("src/main.rs", "w") as f:
    f.write(content)

print("Done")
