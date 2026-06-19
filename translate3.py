import re
with open("src/main.rs", "r") as f:
    content = f.read()

replacements = {
    '// ======================= КРИПТОГРАФИЯ И ЦЕЛОСТНОСТЬ (АВТОРСКАЯ) =======================': '// ======================= CRYPTOGRAPHY & INTEGRITY =======================',
    '// ======================= ЯДРО КОДЕКА =======================': '// ======================= CODEC CORE =======================',
    '// --- Математика Импульсов и Зеркал (Экспоненциальный код Элиаса-Гамма) ---': '// --- Pulse and Mirror Math (Elias Gamma Exponential Code) ---',
    'let k = 31 - n.leading_zeros(); // Количество бит': 'let k = 31 - n.leading_zeros(); // Number of bits',
    '// 1. Зеркало (масштаб): пишем нули': '// 1. Mirror (scale): write zeros',
    '// 2. Вспышка (разделитель)': '// 2. Flash (separator)',
    '// 3. Знание (бинарный остаток)': '// 3. Knowledge (binary remainder)',
    '// Вектор Б: окно 64 КБ': '// Vector B: 64 KB window',
    '// Глубина поиска в цепочке хэшей': '// Search depth in hash chain',
    '// Идеальное совпадение': '// Perfect match',
    '// Вектор Б: Обновляем индекс для всех пропущенных символов': '// Vector B: Update index for all skipped characters',
    '// Вектор Б: Адаптивный размер словаря (до 255 пар, так как Экспоненциальный код не боится больших чисел)': '// Vector B: Adaptive dictionary size (up to 255 pairs, as Exponential Code handles large numbers well)',
    '// ЭКСПОНЕНЦИАЛЬНЫЙ ЯЗЫК: Зеркало + Знание': '// EXPONENTIAL LANGUAGE: Mirror + Knowledge',
    '// ЭКСПОНЕНЦИАЛЬНЫЙ ЯЗЫК: Читаем Зеркало + Знание': '// EXPONENTIAL LANGUAGE: Read Mirror + Knowledge',
    '"Поврежденный файл (ошибка фрейминга)"': '"Corrupted file (framing error)"',
    '// ======================= ТОЧКА ВХОДА =======================': '// ======================= ENTRY POINT ======================='
}

for ru, en in replacements.items():
    content = content.replace(ru, en)

with open("src/main.rs", "w") as f:
    f.write(content)
