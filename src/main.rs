mod gpu;
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::time::Instant;
use std::collections::HashMap;

const CHUNK_SIZE: usize = 1024 * 1024; // 1 MB
const GOLDEN_RATIO_MAGIC: u64 = 0x9E3779B97F4A7C15;

// ======================= BIT UTILITIES =======================

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

// ======================= CRYPTOGRAPHY & INTEGRITY =======================

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

fn fractal_checksum(data: &[u8]) -> u32 {
    let mut hash: u32 = 0x811C9DC5;
    for &byte in data {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x01000193);
        hash = hash.rotate_left(3);
    }
    hash
}


// ======================= ENTROPY AI ANALYZER =======================

fn calculate_entropy(data: &[u8]) -> f32 {
    if data.is_empty() { return 0.0; }
    let mut counts = [0u32; 256];
    for &byte in data {
        counts[byte as usize] += 1;
    }
    let mut entropy = 0.0f32;
    let len = data.len() as f32;
    for &count in counts.iter() {
        if count > 0 {
            let p = count as f32 / len;
            entropy -= p * p.log2();
        }
    }
    entropy
}

// ======================= NEURORATCHET (SNN CRYPTO) =======================

struct NeuroRatchet {
    potentials: [u32; 4],
    weights: [u32; 4],
    threshold: u32,
}

impl NeuroRatchet {
    fn new(ktime: u64, chunk_idx: u64) -> Self {
        let seed1 = ktime.wrapping_mul(GOLDEN_RATIO_MAGIC).wrapping_add(chunk_idx);
        let seed2 = seed1.rotate_left(17) ^ ktime;
        
        Self {
            potentials: [
                (seed1 & 0xFFFFFFFF) as u32,
                (seed1 >> 32) as u32,
                (seed2 & 0xFFFFFFFF) as u32,
                (seed2 >> 32) as u32,
            ],
            weights: [
                0x9E3779B9, 0x85EBCA6B, 0xC2B2AE35, 0x27D4EB2F
            ],
            threshold: 0xC0000000,
        }
    }

    #[inline(always)]
    fn get_mask(&self) -> u8 {
        ((self.potentials[0] >> 24) ^ (self.potentials[1] >> 16) ^ (self.potentials[2] >> 8) ^ self.potentials[3]) as u8
    }

    #[inline(always)]
    fn pulse(&mut self, spike_val: u8) {
        let impulse = (spike_val as u32).wrapping_mul(0x01000193);
        for i in 0..4 {
            self.potentials[i] = self.potentials[i].wrapping_add(impulse ^ self.weights[i]);
            if self.potentials[i] >= self.threshold {
                let next_i = (i + 1) % 4;
                self.weights[i] = self.weights[i].rotate_left(7).wrapping_add(self.potentials[next_i]);
                self.potentials[i] >>= 1; // Refractory period
            }
        }
    }
}

// ======================= CODEC CORE =======================


struct CodecCore {
    fib_table: Vec<u32>,
    ktime: u64,
    gpu: Option<gpu::GpuContext>,
}

impl CodecCore {
    fn new(ktime: u64) -> Self {
        let mut fib_table = vec![1, 2];
        loop {
            let next_fib = fib_table[fib_table.len() - 1] + fib_table[fib_table.len() - 2];
            if next_fib > 3000000 { break; }
            fib_table.push(next_fib);
        }
        
        println!("Инициализация связи с GPU (Apple Metal)...");
        let gpu = pollster::block_on(gpu::GpuContext::new());
        if gpu.is_some() { println!("GPU успешно подключен!"); }
        Self { fib_table, ktime, gpu }

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
    fn crypt_header(&self, data: &mut [u8; 8], chunk_idx: u64) {
        for i in 0..8 {
            let magic = GOLDEN_RATIO_MAGIC
                .wrapping_mul(chunk_idx + 1)
                .wrapping_add((i as u64) * 1337) ^ self.ktime;
            data[i] ^= (magic >> 16) as u8;
        }
    }

    #[inline(always)]
    fn encrypt_chunk(&self, chunk: &[u8], chunk_idx: u64) -> Vec<u8> {
        let mut ratchet = NeuroRatchet::new(self.ktime, chunk_idx);
        let mut out = Vec::with_capacity(chunk.len());
        for &p_byte in chunk {
            let mask = ratchet.get_mask();
            let c_byte = p_byte ^ mask;
            ratchet.pulse(c_byte);
            out.push(c_byte);
        }
        out
    }

    #[inline(always)]
    fn decrypt_chunk(&self, chunk: &[u8], chunk_idx: u64) -> Vec<u8> {
        let mut ratchet = NeuroRatchet::new(self.ktime, chunk_idx);
        let mut out = Vec::with_capacity(chunk.len());
        for &c_byte in chunk {
            let mask = ratchet.get_mask();
            let p_byte = c_byte ^ mask;
            ratchet.pulse(c_byte);
            out.push(p_byte);
        }
        out
    }

    // --- Pulse and Mirror Math (Elias Gamma Exponential Code) ---

    #[inline(always)]
    fn elias_gamma_encode(&self, mut n: u32, writer: &mut BitWriter) {
        n += 1;
        let k = 31 - n.leading_zeros(); // Number of bits
        
        // 1. Mirror (scale): write zeros
        for _ in 0..k {
            writer.write_bit(0);
        }
        // 2. Flash (separator)
        writer.write_bit(1);
        
        // 3. Knowledge (binary remainder)
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

    // --- PACKING (COMPRESSION) ---

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
            let max_lookback = i.min(65535); // Vector B: 64 KB window
            
            if max_len >= 3 {
                let h = hash_func(&chunk[i..]);
                let mut match_pos = head[h];
                let mut chain_length = 50; // Search depth in hash chain
                
                while match_pos > 0 && i - match_pos <= max_lookback && chain_length > 0 {
                    let dist = i - match_pos;
                    let mut current_len = 0;
                    while current_len < max_len && chunk[match_pos + current_len] == chunk[i + current_len] {
                        current_len += 1;
                    }
                    if current_len > best_match_len {
                        best_match_len = current_len;
                        best_match_dist = dist;
                        if best_match_len == max_len { break; } // Perfect match
                    }
                    match_pos = prev[match_pos];
                    chain_length -= 1;
                }
            }
            
            if best_match_len >= 3 {
                writer.write_bit(1);
                self.fib_encode_number(best_match_dist as u32, &mut writer);
                self.fib_encode_number(best_match_len as u32, &mut writer);
                
                // Vector B: Update index for all skipped characters
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
        writer.write_bit(1); writer.write_bit(1); writer.write_bit(0);
        
        let mut pair_counts: HashMap<(u8, u8), u32> = HashMap::new();
        for i in 0..chunk.len().saturating_sub(1) {
            *pair_counts.entry((chunk[i], chunk[i+1])).or_insert(0) += 1;
        }
        
        let mut pairs: Vec<_> = pair_counts.into_iter().collect();
        pairs.sort_by(|a, b| b.1.cmp(&a.1)); 
        
        // Vector B: Adaptive dictionary size (up to 255 pairs, as Exponential Code handles large numbers well)
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
                    // EXPONENTIAL LANGUAGE: Mirror + Knowledge
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


    // ======================= PATH 4: 1D MEMBRANE (GENERATIVE IMPULSE) =======================
    fn compress_path4(&self, chunk: &[u8], is_delta: bool) -> Vec<u8> {
        let mut writer = BitWriter::new(chunk.len());
        writer.write_bit(if is_delta { 1 } else { 0 });
        writer.write_bit(1); writer.write_bit(1); writer.write_bit(1); // ID: 111
        
        let block_size = 16;
        let mut i = 0;
        
        while i < chunk.len() {
            if i + block_size <= chunk.len() {
                let block = &chunk[i..i+block_size];
                let v_start = block[0] as f32;
                let v_end = block[block_size - 1] as f32;
                
                let mut is_smooth = true;
                for j in 0..block_size {
                    let expected = v_start + (v_end - v_start) * (j as f32 / (block_size - 1) as f32);
                    let diff = (block[j] as f32 - expected).abs();
                    // Tolerance for smooth gradient: 2.0
                    if diff > 2.0 {
                        is_smooth = false;
                        break;
                    }
                }
                
                if is_smooth {
                    writer.write_bit(1); // Membrane flag
                    self.fib_encode_number(block[0] as u32, &mut writer);
                    self.fib_encode_number(block[block_size - 1] as u32, &mut writer);
                    i += block_size;
                    continue;
                }
            }
            
            // Fallback
            writer.write_bit(0);
            self.fib_encode_number(chunk[i] as u32, &mut writer);
            i += 1;
        }
        writer.finish().0
    }

    fn decompress_path4(&self, reader: &mut BitReader) -> Vec<u8> {
        let mut out = Vec::new();
        let block_size = 16;
        
        while let Some(flag) = reader.read_bit() {
            if flag == 1 {
                // Generative Membrane
                let v_start = match self.fib_decode_number(reader) {
                    Some(v) => v as f32,
                    None => break,
                };
                let v_end = match self.fib_decode_number(reader) {
                    Some(v) => v as f32,
                    None => break,
                };
                
                for j in 0..block_size {
                    let val = v_start + (v_end - v_start) * (j as f32 / (block_size - 1) as f32);
                    out.push(val.round().clamp(0.0, 255.0) as u8);
                }
            } else {
                // Literal
                if let Some(n) = self.fib_decode_number(reader) {
                    out.push(n as u8);
                } else {
                    break;
                }
            }
        }
        out
    }

    pub fn compress_file(&self, input_path: &str, output_path: &str) -> io::Result<()> {
        use std::io::Seek;
        let mut input_file = File::open(input_path)?;
        let mut output_file = File::create(output_path)?;
        let mut buffer = vec![0u8; CHUNK_SIZE];

        let start_time = Instant::now();
        let mut total_read = 0;
        let mut chunk_idx = 0u64;
        let mut chunk_offsets = Vec::new();

        loop {
            let pos = output_file.stream_position()?;
            let bytes_read = input_file.read(&mut buffer)?;
            if bytes_read == 0 { break; }
            chunk_offsets.push(pos);
            total_read += bytes_read;

            let chunk = &buffer[..bytes_read];
            let checksum = fractal_checksum(chunk);
            
            // Fast Entropy AI Scanner
            if let Some(gpu) = &self.gpu {
                gpu.scan_chunk(chunk);
            }
            let entropy = calculate_entropy(chunk);
            
            let packed_chunk = if entropy > 7.95 {
                // Если энтропия близка к максимуму (8.0), данные уже сжаты (MP4, ZIP) или это шум.
                // Нет смысла тратить ресурсы CPU на поиск паттернов. Моментально пропускаем через Path 0.
                self.compress_path0(chunk, false)
            } else {
                // Если паттерны есть, запускаем тяжелую эвристику: 8 путей
                let delta_chunk = Self::apply_delta(chunk);
                
                let p0 = self.compress_path0(chunk, false);
                let p1 = self.compress_path1(chunk, false);
                let p2 = self.compress_path2(chunk, false);
                let p3 = self.compress_path3(chunk, false);
                let p4 = self.compress_path4(chunk, false); // Membrane
                let d0 = self.compress_path0(&delta_chunk, true);
                let d1 = self.compress_path1(&delta_chunk, true);
                let d2 = self.compress_path2(&delta_chunk, true);
                let d3 = self.compress_path3(&delta_chunk, true);
                let d4 = self.compress_path4(&delta_chunk, true); // Membrane + Delta
                
                let mut candidates = vec![p0, p1, p2, p3, p4, d0, d1, d2, d3, d4];
                candidates.sort_by_key(|c| c.len());
                candidates.remove(0)
            };
            
            let encrypted_chunk = self.encrypt_chunk(&packed_chunk, chunk_idx);
            
            let chunk_len = encrypted_chunk.len() as u32;
            let mut header = [0u8; 8];
            header[0..4].copy_from_slice(&chunk_len.to_le_bytes());
            header[4..8].copy_from_slice(&checksum.to_le_bytes());
            self.crypt_header(&mut header, chunk_idx);
            
            output_file.write_all(&header)?;
            output_file.write_all(&encrypted_chunk)?;
            
            chunk_idx += 1;
        }

        // VFS Footer: write offsets
        let index_start = output_file.stream_position()?;
        for &offset in &chunk_offsets {
            let mut off_buf = [0u8; 8];
            off_buf.copy_from_slice(&offset.to_le_bytes());
            self.crypt_header(&mut off_buf, u64::MAX - 1); // Use special idx for index
            output_file.write_all(&off_buf)?;
        }
        
        // VFS Trailer: write index start
        let mut trailer = [0u8; 8];
        trailer.copy_from_slice(&index_start.to_le_bytes());
        self.crypt_header(&mut trailer, u64::MAX);
        output_file.write_all(&trailer)?;

        let elapsed = start_time.elapsed();
        println!("Compression complete.");
        println!("  Original: {} bytes", total_read);
        println!("  Time:     {:.3}s", elapsed.as_secs_f64());
        Ok(())
    }

    // --- UNPACKING (DECOMPRESSION) ---

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
                // EXPONENTIAL LANGUAGE: Read Mirror + Knowledge
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
        let mut chunk_idx = 0u64;

        let mut len_buf = [0u8; 8];
        loop {
            let bytes_read = input_file.read(&mut len_buf)?;
            if bytes_read == 0 { break; }
            if bytes_read != 8 {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Corrupted file (framing error)"));
            }
            
            self.crypt_header(&mut len_buf, chunk_idx);
            
            let mut len_arr = [0u8; 4];
            len_arr.copy_from_slice(&len_buf[0..4]);
            let chunk_len = u32::from_le_bytes(len_arr) as usize;
            
            let mut chk_arr = [0u8; 4];
            chk_arr.copy_from_slice(&len_buf[4..8]);
            let stored_checksum = u32::from_le_bytes(chk_arr);
            
            let mut enc_chunk = vec![0u8; chunk_len];
            input_file.read_exact(&mut enc_chunk)?;
            
            let packed_chunk = self.decrypt_chunk(&enc_chunk, chunk_idx);
            
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
            } else {
                let bit3 = reader.read_bit().unwrap_or(0);
                if bit3 == 0 {
                    self.decompress_path3(&mut reader)
                } else {
                    self.decompress_path4(&mut reader)
                }
            };
            
            if is_delta {
                decompressed_chunk = Self::remove_delta(&decompressed_chunk);
            }
            
            let calculated_checksum = fractal_checksum(&decompressed_chunk);
            if calculated_checksum != stored_checksum {
                println!("\n! [CRITICAL ERROR] Data block corrupted! Integrity protection triggered.");
                println!("! Expected hash: {}, Calculated hash: {}", stored_checksum, calculated_checksum);
                println!("! The password may be incorrect, or the file is physically damaged.");
            }
            
            output_file.write_all(&decompressed_chunk)?;
            total_written += decompressed_chunk.len();
            chunk_idx += 1;
        }

        let elapsed = start_time.elapsed();
        println!("Decompression complete.");
        println!("  Restored: {} bytes", total_written);
        println!("  Time:     {:.3}s", elapsed.as_secs_f64());
        Ok(())
    }
}

// ======================= ENTRY POINT =======================


// ======================= VFS (VIRTUAL FILE SYSTEM) =======================

use std::io::SeekFrom;
use std::io::Seek;

pub struct ObscuraVFS {
    file: File,
    codec: CodecCore,
    pub chunk_offsets: Vec<u64>,
}

impl ObscuraVFS {
    pub fn open(path: &str, password: &str) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let codec = CodecCore::new(fractal_mirror_hash(password));
        
        // Читаем трейлер (последние 8 байт)
        file.seek(SeekFrom::End(-8))?;
        let mut trailer = [0u8; 8];
        file.read_exact(&mut trailer)?;
        codec.crypt_header(&mut trailer, u64::MAX);
        let index_start = u64::from_le_bytes(trailer);
        
        // Читаем индексы
        let file_len = file.metadata()?.len();
        let index_bytes_len = file_len - 8 - index_start;
        if index_bytes_len % 8 != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "VFS Index corrupted"));
        }
        
        file.seek(SeekFrom::Start(index_start))?;
        let num_chunks = (index_bytes_len / 8) as usize;
        let mut chunk_offsets = Vec::with_capacity(num_chunks);
        
        for _ in 0..num_chunks {
            let mut off_buf = [0u8; 8];
            file.read_exact(&mut off_buf)?;
            codec.crypt_header(&mut off_buf, u64::MAX - 1);
            chunk_offsets.push(u64::from_le_bytes(off_buf));
        }
        
        Ok(Self { file, codec, chunk_offsets })
    }

    /// Мгновенное чтение любого чанка без расшифровки всего файла
    pub fn read_chunk(&mut self, chunk_index: usize) -> io::Result<Vec<u8>> {
        if chunk_index >= self.chunk_offsets.len() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Chunk out of bounds"));
        }
        
        self.file.seek(SeekFrom::Start(self.chunk_offsets[chunk_index]))?;
        
        let mut header = [0u8; 8];
        self.file.read_exact(&mut header)?;
        self.codec.crypt_header(&mut header, chunk_index as u64);
        
        let mut len_arr = [0u8; 4];
        len_arr.copy_from_slice(&header[0..4]);
        let chunk_len = u32::from_le_bytes(len_arr) as usize;
        
        let mut chk_arr = [0u8; 4];
        chk_arr.copy_from_slice(&header[4..8]);
        let stored_checksum = u32::from_le_bytes(chk_arr);
        
        let mut enc_chunk = vec![0u8; chunk_len];
        self.file.read_exact(&mut enc_chunk)?;
        
        let packed_chunk = self.codec.decrypt_chunk(&enc_chunk, chunk_index as u64);
        let mut reader = BitReader::new(&packed_chunk);
        
        let is_delta = reader.read_bit().unwrap_or(0) == 1;
        let bit1 = reader.read_bit().unwrap_or(0);
        let bit2 = reader.read_bit().unwrap_or(0);
        
        let mut decompressed = if bit1 == 0 && bit2 == 0 {
            self.codec.decompress_path0(&mut reader)
        } else if bit1 == 0 && bit2 == 1 {
            self.codec.decompress_path1(&mut reader)
        } else if bit1 == 1 && bit2 == 0 {
            self.codec.decompress_path2(&mut reader)
        } else {
            let bit3 = reader.read_bit().unwrap_or(0);
            if bit3 == 0 {
                self.codec.decompress_path3(&mut reader)
            } else {
                self.codec.decompress_path4(&mut reader)
            }
        };
        
        if is_delta {
            decompressed = CodecCore::remove_delta(&decompressed);
        }
        
        if fractal_checksum(&decompressed) != stored_checksum {
             return Err(io::Error::new(io::ErrorKind::InvalidData, "Chunk integrity error"));
        }
        
        Ok(decompressed)
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        println!("Obscura Stealth Codec - Lossless Cryptographic Container");
        println!("Usage:");
        println!("  {} compress <input> <output> [password]", args[0]);
        println!("  {} decompress <input> <output> [password]", args[0]);
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
            println!("\n[ERROR] Security first: no password provided!");
            
            use std::time::{SystemTime, UNIX_EPOCH};
            let random_val = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
            let mut auto_pass = format!("{:X}", random_val.wrapping_mul(GOLDEN_RATIO_MAGIC));
            auto_pass.truncate(12);
            
            println!("Obscura Codec refuses to encode data without password protection.");
            println!("> Generated strong password: {}", auto_pass);
            println!("  {} compress {} {} {}\n", args[0], input_file, output_file, auto_pass);
            return;
        } else {
            println!("\n[ERROR] Access Denied! Provide a password.");
            return;
        }
    }

    let codec = CodecCore::new(ktime);

    match mode.as_str() {
        "compress" => {
            println!("Compressing file: {}", input_file);
            if let Err(e) = codec.compress_file(input_file, output_file) {
                eprintln!("Compression error: {}", e);
            }
        }
        "decompress" => {
            println!("Decompressing file: {}", input_file);
            let password = if args.len() >= 5 { &args[4] } else { "" };
            match ObscuraVFS::open(input_file, password) {
                Ok(mut vfs) => {
                    use std::io::Write;
                    let mut out = File::create(output_file).unwrap();
                    let mut total = 0;
                    let start = Instant::now();
                    for i in 0..vfs.chunk_offsets.len() {
                        let data = match vfs.read_chunk(i) { Ok(d) => d, Err(e) => { eprintln!("Error reading chunk {}: {}", i, e); return; } };
                        out.write_all(&data).unwrap();
                        total += data.len();
                    }
                    println!("Decompression complete.");
                    println!("  Restored: {} bytes", total);
                    println!("  Time:     {:.3}s", start.elapsed().as_secs_f64());
                }
                Err(e) => eprintln!("Decompression error: {}", e),
            }
        }
        "vfs_test" => {
            println!("Testing Virtual File System (VFS)...");
            let mut vfs = ObscuraVFS::open(input_file, &args[4]).unwrap();
            println!("Found {} chunks via encrypted index.", vfs.chunk_offsets.len());
            // Случайный доступ: читаем средний чанк
            let target_idx = vfs.chunk_offsets.len() / 2;
            let start = Instant::now();
            let data = vfs.read_chunk(target_idx).unwrap();
            let el = start.elapsed();
            println!("Instantly decrypted chunk {} ({} bytes) in {:.3}ms.", target_idx, data.len(), el.as_secs_f64() * 1000.0);
        }
        _ => {
            eprintln!("Unknown mode.");
        }
    }
}
