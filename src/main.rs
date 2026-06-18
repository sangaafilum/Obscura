use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::time::Instant;
use std::collections::HashMap;

const CHUNK_SIZE: usize = 1024 * 1024; // 1 МБ
const GOLDEN_RATIO_MAGIC: u64 = 0x9E3779B97F4A7C15;

// ======================= УТИЛИТЫ ДЛЯ БИТОВ =======================

struct BitWriter {
    buffer: Vec<u8>,
    current_byte: u8,
    bit_count: u8,
}

impl BitWriter {
    fn new(capacity: usize) -> Self {
        Self { buffer: Vec::with_capacity(capacity), current_byte: 0, bit_count: 0 }
    }
    
    #[inline(always)]
    fn write_bit(&mut self, bit: u8) {
        self.current_byte |= bit << (7 - self.bit_count);
        self.bit_count += 1;
        if self.bit_count == 8 {
            self.buffer.push(self.current_byte);
            self.current_byte = 0;
            self.bit_count = 0;
        }
    }
    
    fn finish(mut self) -> (Vec<u8>, u8) {
        let padding = if self.bit_count > 0 {
            self.buffer.push(self.current_byte);
            8 - self.bit_count
        } else {
            0
        };
        (self.buffer, padding)
    }
}

struct BitReader<'a> {
    data: &'a [u8],
    byte_idx: usize,
    bit_idx: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, byte_idx: 0, bit_idx: 0 }
    }
    
    #[inline(always)]
    fn read_bit(&mut self) -> Option<u8> {
        if self.byte_idx >= self.data.len() {
            return None;
        }
        let bit = (self.data[self.byte_idx] >> (7 - self.bit_idx)) & 1;
        self.bit_idx += 1;
        if self.bit_idx == 8 {
            self.byte_idx += 1;
            self.bit_idx = 0;
        }
        Some(bit)
    }
}

// ======================= КРИПТОГРАФИЯ И ЦЕЛОСТНОСТЬ (АВТОРСКАЯ) =======================

fn fractal_mirror_hash(password: &str) -> u64 {
    let mut state: u64 = GOLDEN_RATIO_MAGIC;
    for (i, &byte) in password.as_bytes().iter().enumerate() {
        let mirror_byte = byte.reverse_bits() as u64;
        state ^= mirror_byte.rotate_left((i % 64) as u32);
        state = state.wrapping_mul(GOLDEN_RATIO_MAGIC);
        let shift = byte.count_ones() + 1;
        state = state.rotate_right(shift);
        state ^= !mirror_byte;
    }
    state
}

fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

// ======================= ЯДРО КОДЕКА =======================

struct CodecCore {
    fib_table: Vec<u32>,
    ktime: u64,
}

impl CodecCore {
    fn new(ktime: u64) -> Self {
        let mut fib_table = vec![1, 2];
        loop {
            let next_fib = fib_table[fib_table.len() - 1] + fib_table[fib_table.len() - 2];
            if next_fib > 3000000 { break; }
            fib_table.push(next_fib);
        }
        Self { fib_table, ktime }
    }

    #[inline(always)]
    fn fib_encode_number(&self, mut n: u32, writer: &mut BitWriter) {
        n += 1;
        let mut i = self.fib_table.len() - 1;
        while i > 0 {
            if self.fib_table[i] <= n { break; }
            i -= 1;
        }
        
        let mut bits_mask = 0u32;
        for k in (0..=i).rev() {
            if n >= self.fib_table[k] {
                bits_mask |= 1 << k;
                n -= self.fib_table[k];
            }
        }
        
        for k in 0..=i {
            let bit = ((bits_mask >> k) & 1) as u8;
            writer.write_bit(bit);
        }
        writer.write_bit(1);
    }

    #[inline(always)]
    fn fib_decode_number(&self, reader: &mut BitReader) -> Option<u32> {
        let mut result = 0u32;
        let mut prev_bit = 0;
        let mut fib_idx = 0;
        
        loop {
            let bit = reader.read_bit()?;
            if bit == 1 {
                if prev_bit == 1 {
                    return Some(result - 1);
                }
                result += self.fib_table[fib_idx];
                prev_bit = 1;
            } else {
                prev_bit = 0;
            }
            fib_idx += 1;
        }
    }

    #[inline(always)]
    fn encrypt_chunk(&self, chunk: &[u8]) -> Vec<u8> {
        let mut encrypted = Vec::with_capacity(chunk.len());
        for (i, &byte) in chunk.iter().enumerate() {
            let i_u64 = i as u64;
            let magic_val = GOLDEN_RATIO_MAGIC.wrapping_mul(i_u64 + 1) ^ self.ktime;
            let mask = (magic_val & 0xFF) as u8;
            encrypted.push(byte ^ mask);
        }
        encrypted
    }

    // --- Математика Импульсов и Зеркал (Экспоненциальный код Элиаса-Гамма) ---

    #[inline(always)]
    fn elias_gamma_encode(&self, mut n: u32, writer: &mut BitWriter) {
        n += 1;
        let k = 31 - n.leading_zeros(); // Количество бит
        
        // 1. Зеркало (масштаб): пишем нули
        for _ in 0..k {
            writer.write_bit(0);
        }
        // 2. Вспышка (разделитель)
        writer.write_bit(1);
        
        // 3. Знание (бинарный остаток)
        for i in (0..k).rev() {
            let bit = ((n >> i) & 1) as u8;
            writer.write_bit(bit);
        }
    }

    #[inline(always)]
    fn elias_gamma_decode(&self, reader: &mut BitReader) -> Option<u32> {
        let mut k = 0;
        loop {
            match reader.read_bit()? {
                0 => k += 1,
                1 => break,
                _ => return None,
            }
        }
        
        let mut n = 1u32;
        for _ in 0..k {
            let bit = reader.read_bit()? as u32;
            n = (n << 1) | bit;
        }
        Some(n - 1)
    }

    // --- УПАКОВКА (СЖАТИЕ) ---

    fn apply_delta(chunk: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(chunk.len());
        let mut prev = 0u8;
        for &byte in chunk {
            out.push(byte.wrapping_sub(prev));
            prev = byte;
        }
        out
    }

    fn remove_delta(chunk: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(chunk.len());
        let mut prev = 0u8;
        for &byte in chunk {
            let val = byte.wrapping_add(prev);
            out.push(val);
            prev = val;
        }
        out
    }

    fn compress_path0(&self, chunk: &[u8], is_delta: bool) -> Vec<u8> {
        let mut writer = BitWriter::new(chunk.len() + 1);
        writer.write_bit(if is_delta { 1 } else { 0 });
        writer.write_bit(0); writer.write_bit(0);
        for &byte in chunk {
            for i in (0..8).rev() {
                writer.write_bit((byte >> i) & 1);
            }
        }
        writer.finish().0
    }

    fn compress_path1(&self, chunk: &[u8], is_delta: bool) -> Vec<u8> {
        let mut writer = BitWriter::new(chunk.len());
        writer.write_bit(if is_delta { 1 } else { 0 });
        writer.write_bit(0); writer.write_bit(1);
        for &byte in chunk {
            self.fib_encode_number(byte as u32, &mut writer);
        }
        writer.finish().0
    }

    fn compress_path2(&self, chunk: &[u8], is_delta: bool) -> Vec<u8> {
        let mut writer = BitWriter::new(chunk.len());
        writer.write_bit(if is_delta { 1 } else { 0 });
        writer.write_bit(1); writer.write_bit(0);
        
        const HASH_SIZE: usize = 65536;
        let mut head = vec![0usize; HASH_SIZE];
        let mut prev = vec![0usize; chunk.len()];
        
        let hash_func = |data: &[u8]| -> usize {
            if data.len() < 3 { return 0; }
            let h = (data[0] as usize) << 16 | (data[1] as usize) << 8 | (data[2] as usize);
            (h.wrapping_mul(2654435761)) % HASH_SIZE
        };
        
        let mut i = 0;
        while i < chunk.len() {
            let mut best_match_len = 0;
            let mut best_match_dist = 0;
            
            let max_len = (chunk.len() - i).min(255);
            let max_lookback = i.min(65535); // Вектор Б: окно 64 КБ
            
            if max_len >= 3 {
                let h = hash_func(&chunk[i..]);
                let mut match_pos = head[h];
                let mut chain_length = 50; // Глубина поиска в цепочке хэшей
                
                while match_pos > 0 && i - match_pos <= max_lookback && chain_length > 0 {
                    let dist = i - match_pos;
                    let mut current_len = 0;
                    while current_len < max_len && chunk[match_pos + current_len] == chunk[i + current_len] {
                        current_len += 1;
                    }
                    if current_len > best_match_len {
                        best_match_len = current_len;
                        best_match_dist = dist;
                        if best_match_len == max_len { break; } // Идеальное совпадение
                    }
                    match_pos = prev[match_pos];
                    chain_length -= 1;
                }
            }
            
            if best_match_len >= 3 {
                writer.write_bit(1);
                self.fib_encode_number(best_match_dist as u32, &mut writer);
                self.fib_encode_number(best_match_len as u32, &mut writer);
                
                // Вектор Б: Обновляем индекс для всех пропущенных символов
                for k in 0..best_match_len {
                    if i + k + 3 <= chunk.len() {
                        let h = hash_func(&chunk[i+k..]);
                        prev[i+k] = head[h];
                        head[h] = i+k;
                    }
                }
                i += best_match_len;
            } else {
                writer.write_bit(0);
                self.fib_encode_number(chunk[i] as u32, &mut writer);
                
                if i + 3 <= chunk.len() {
                    let h = hash_func(&chunk[i..]);
                    prev[i] = head[h];
                    head[h] = i;
                }
                i += 1;
            }
        }
        writer.finish().0
    }

    fn compress_path3(&self, chunk: &[u8], is_delta: bool) -> Vec<u8> {
        let mut writer = BitWriter::new(chunk.len());
        writer.write_bit(if is_delta { 1 } else { 0 });
        writer.write_bit(1); writer.write_bit(1);
        
        let mut pair_counts: HashMap<(u8, u8), u32> = HashMap::new();
        for i in 0..chunk.len().saturating_sub(1) {
            *pair_counts.entry((chunk[i], chunk[i+1])).or_insert(0) += 1;
        }
        
        let mut pairs: Vec<_> = pair_counts.into_iter().collect();
        pairs.sort_by(|a, b| b.1.cmp(&a.1)); 
        
        // Вектор Б: Адаптивный размер словаря (до 255 пар, так как Экспоненциальный код не боится больших чисел)
        let mut dict_size = 0;
        for &(_, count) in &pairs {
            if count > 5 && dict_size < 255 {
                dict_size += 1;
            } else {
                break;
            }
        }
        
        let mut dict_lookup = HashMap::new();
        self.fib_encode_number(dict_size as u32, &mut writer);
        
        for (idx, &(pair, _)) in pairs.iter().take(dict_size).enumerate() {
            dict_lookup.insert(pair, idx as u32);
            self.fib_encode_number(pair.0 as u32, &mut writer);
            self.fib_encode_number(pair.1 as u32, &mut writer);
        }
        
        let mut i = 0;
        while i < chunk.len() {
            if i < chunk.len() - 1 && dict_size > 0 {
                let pair = (chunk[i], chunk[i+1]);
                if let Some(&idx) = dict_lookup.get(&pair) {
                    writer.write_bit(1);
                    // ЭКСПОНЕНЦИАЛЬНЫЙ ЯЗЫК: Зеркало + Знание
                    self.elias_gamma_encode(idx, &mut writer);
                    i += 2;
                    continue;
                }
            }
            writer.write_bit(0);
            self.fib_encode_number(chunk[i] as u32, &mut writer);
            i += 1;
        }
        writer.finish().0
    }

    pub fn compress_file(&self, input_path: &str, output_path: &str) -> io::Result<()> {
        let mut input_file = File::open(input_path)?;
        let mut output_file = File::create(output_path)?;
        let mut buffer = vec![0u8; CHUNK_SIZE];

        let start_time = Instant::now();
        let mut total_read = 0;

        loop {
            let bytes_read = input_file.read(&mut buffer)?;
            if bytes_read == 0 { break; }
            total_read += bytes_read;

            let chunk = &buffer[..bytes_read];
            let checksum = adler32(chunk);
            
            // Идеальный ИИ-Анализатор (Разведка боем): 6 путей в памяти
            let delta_chunk = Self::apply_delta(chunk);
            
            let p0 = self.compress_path0(chunk, false);
            let p1 = self.compress_path1(chunk, false);
            let p2 = self.compress_path2(chunk, false);
            let p3 = self.compress_path3(chunk, false);
            let d0 = self.compress_path0(&delta_chunk, true);
            let d1 = self.compress_path1(&delta_chunk, true);
            let d2 = self.compress_path2(&delta_chunk, true);
            let d3 = self.compress_path3(&delta_chunk, true);
            
            let mut candidates = vec![p0, p1, p2, p3, d0, d1, d2, d3];
            candidates.sort_by_key(|c| c.len());
            let packed_chunk = candidates.remove(0);
            
            let encrypted_chunk = self.encrypt_chunk(&packed_chunk);
            
            let chunk_len = encrypted_chunk.len() as u32;
            output_file.write_all(&chunk_len.to_le_bytes())?;
            output_file.write_all(&checksum.to_le_bytes())?;
            output_file.write_all(&encrypted_chunk)?;
        }

        let elapsed = start_time.elapsed();
        println!("Упаковка завершена.");
        println!("  Оригинал: {} байт", total_read);
        println!("  Время:    {:.3}с", elapsed.as_secs_f64());
        Ok(())
    }

    // --- РАСПАКОВКА (ДЕКОМПРЕССИЯ) ---

    fn decompress_path0(&self, reader: &mut BitReader) -> Vec<u8> {
        let mut out = Vec::new();
        while let Some(b7) = reader.read_bit() {
            let mut byte = b7 << 7;
            let mut complete = true;
            for i in (0..7).rev() {
                if let Some(bit) = reader.read_bit() {
                    byte |= bit << i;
                } else {
                    complete = false;
                    break;
                }
            }
            if complete {
                out.push(byte);
            }
        }
        out
    }

    fn decompress_path1(&self, reader: &mut BitReader) -> Vec<u8> {
        let mut out = Vec::new();
        while let Some(n) = self.fib_decode_number(reader) {
            out.push(n as u8);
        }
        out
    }

    fn decompress_path2(&self, reader: &mut BitReader) -> Vec<u8> {
        let mut out = Vec::new();
        while let Some(flag) = reader.read_bit() {
            if flag == 0 {
                if let Some(n) = self.fib_decode_number(reader) {
                    out.push(n as u8);
                } else { break; }
            } else {
                let dist = match self.fib_decode_number(reader) {
                    Some(d) => d as usize,
                    None => break,
                };
                let len = match self.fib_decode_number(reader) {
                    Some(l) => l as usize,
                    None => break,
                };
                for _ in 0..len {
                    let b = out[out.len() - dist];
                    out.push(b);
                }
            }
        }
        out
    }

    fn decompress_path3(&self, reader: &mut BitReader) -> Vec<u8> {
        let mut out = Vec::new();
        let dict_size = match self.fib_decode_number(reader) {
            Some(s) => s as usize,
            None => return out,
        };
        
        let mut dict = Vec::with_capacity(dict_size);
        for _ in 0..dict_size {
            let b1 = self.fib_decode_number(reader).unwrap() as u8;
            let b2 = self.fib_decode_number(reader).unwrap() as u8;
            dict.push((b1, b2));
        }
        
        while let Some(flag) = reader.read_bit() {
            if flag == 0 {
                if let Some(n) = self.fib_decode_number(reader) {
                    out.push(n as u8);
                } else { break; }
            } else {
                // ЭКСПОНЕНЦИАЛЬНЫЙ ЯЗЫК: Читаем Зеркало + Знание
                if let Some(idx) = self.elias_gamma_decode(reader) {
                    if (idx as usize) < dict_size {
                        let pair = dict[idx as usize];
                        out.push(pair.0);
                        out.push(pair.1);
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        out
    }

    pub fn decompress_file(&self, input_path: &str, output_path: &str) -> io::Result<()> {
        let mut input_file = File::open(input_path)?;
        let mut output_file = File::create(output_path)?;
        
        let start_time = Instant::now();
        let mut total_written = 0;

        let mut len_buf = [0u8; 8];
        loop {
            let bytes_read = input_file.read(&mut len_buf)?;
            if bytes_read == 0 { break; }
            if bytes_read != 8 {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Поврежденный файл (ошибка фрейминга)"));
            }
            
            let mut len_arr = [0u8; 4];
            len_arr.copy_from_slice(&len_buf[0..4]);
            let chunk_len = u32::from_le_bytes(len_arr) as usize;
            
            let mut chk_arr = [0u8; 4];
            chk_arr.copy_from_slice(&len_buf[4..8]);
            let stored_checksum = u32::from_le_bytes(chk_arr);
            
            let mut enc_chunk = vec![0u8; chunk_len];
            input_file.read_exact(&mut enc_chunk)?;
            
            let packed_chunk = self.encrypt_chunk(&enc_chunk);
            
            let mut reader = BitReader::new(&packed_chunk);
            let is_delta = reader.read_bit().unwrap_or(0) == 1;
            let bit1 = reader.read_bit().unwrap_or(0);
            let bit2 = reader.read_bit().unwrap_or(0);
            
            let mut decompressed_chunk = if bit1 == 0 && bit2 == 0 {
                self.decompress_path0(&mut reader)
            } else if bit1 == 0 && bit2 == 1 {
                self.decompress_path1(&mut reader)
            } else if bit1 == 1 && bit2 == 0 {
                self.decompress_path2(&mut reader)
            } else if bit1 == 1 && bit2 == 1 {
                self.decompress_path3(&mut reader)
            } else {
                self.decompress_path0(&mut reader)
            };
            
            if is_delta {
                decompressed_chunk = Self::remove_delta(&decompressed_chunk);
            }
            
            let calculated_checksum = adler32(&decompressed_chunk);
            if calculated_checksum != stored_checksum {
                println!("\n! [КРИТИЧЕСКАЯ ОШИБКА] Блок данных поврежден! Сработала система защиты целостности.");
                println!("! Ожидаемый хэш: {}, Полученный хэш: {}", stored_checksum, calculated_checksum);
                println!("! Возможно, введен неправильный пароль или файл был физически поврежден.");
            }
            
            output_file.write_all(&decompressed_chunk)?;
            total_written += decompressed_chunk.len();
        }

        let elapsed = start_time.elapsed();
        println!("Распаковка завершена.");
        println!("  Восстановлено: {} байт", total_written);
        println!("  Время:         {:.3}с", elapsed.as_secs_f64());
        Ok(())
    }
}

// ======================= ТОЧКА ВХОДА =======================

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        println!("Obscura Stealth Codec - Lossless Cryptographic Container");
        println!("Использование:");
        println!("  {} compress <вход> <выход> [пароль]", args[0]);
        println!("  {} decompress <вход> <выход> [пароль]", args[0]);
        return;
    }

    let mode = &args[1];
    let input_file = &args[2];
    let output_file = &args[3];
    
    let ktime: u64;

    if args.len() >= 5 {
        let password = &args[4];
        ktime = fractal_mirror_hash(password);
    } else {
        if mode == "compress" {
            println!("\n[ОШИБКА] Безопасность превыше всего: пароль не указан!");
            
            use std::time::{SystemTime, UNIX_EPOCH};
            let random_val = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
            let mut auto_pass = format!("{:X}", random_val.wrapping_mul(GOLDEN_RATIO_MAGIC));
            auto_pass.truncate(12);
            
            println!("Алгоритм Obscura запрещает шифровать данные без защиты паролем.");
            println!("> Сгенерирован надежный пароль: {}", auto_pass);
            println!("  {} compress {} {} {}\n", args[0], input_file, output_file, auto_pass);
            return;
        } else {
            println!("\n[ОШИБКА] Отказано в доступе! Укажите пароль.");
            return;
        }
    }

    let codec = CodecCore::new(ktime);

    match mode.as_str() {
        "compress" => {
            println!("Упаковка файла: {}", input_file);
            if let Err(e) = codec.compress_file(input_file, output_file) {
                eprintln!("Ошибка сжатия: {}", e);
            }
        }
        "decompress" => {
            println!("Распаковка файла: {}", input_file);
            if let Err(e) = codec.decompress_file(input_file, output_file) {
                eprintln!("Ошибка декомпрессии: {}", e);
            }
        }
        _ => {
            eprintln!("Неизвестный режим.");
        }
    }
}
