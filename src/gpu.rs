use wgpu::util::DeviceExt;
use std::borrow::Cow;

pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub compute_pipeline: wgpu::ComputePipeline,
}

impl GpuContext {
    pub async fn new() -> Option<Self> {
        let instance = wgpu::Instance::default();
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }).await?;

        println!("GPU Подключен: {:?}", adapter.get_info().name);
        println!("Backend: {:?}", adapter.get_info().backend);

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Obscura GPU"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        ).await.ok()?;

        // Базовый WGSL шейдер для параллельного сканирования паттернов (GPU Heuristics)
        let shader_src = "
            // Входной чанк (распакованный в u32 для обхода ограничений выравнивания WGSL)
            @group(0) @binding(0) var<storage, read> data: array<u32>;
            // Выходная таблица: x = match_length, y = match_distance
            @group(0) @binding(1) var<storage, read_write> out_matches: array<vec2<u32>>;

            @compute @workgroup_size(256)
            fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
                let current_pos = global_id.x;
                let max_len = arrayLength(&data);
                
                if (current_pos >= max_len) {
                    return;
                }
                
                // Окно поиска LZ77 (например, назад на 4096 байт)
                var best_len = 0u;
                var best_dist = 0u;
                var search_start = 0u;
                
                if (current_pos > 4096u) {
                    search_start = current_pos - 4096u;
                }
                
                // Параллельный поиск совпадений (тяжелая работа O(N^2), идеальна для GPU)
                for (var j = search_start; j < current_pos; j = j + 1u) {
                    var match_len = 0u;
                    while (current_pos + match_len < max_len && data[j + match_len] == data[current_pos + match_len]) {
                        match_len = match_len + 1u;
                        if (match_len >= 255u) { break; } // Лимит длины
                    }
                    
                    if (match_len > best_len) {
                        best_len = match_len;
                        best_dist = current_pos - j;
                    }
                }
                
                // Записываем результат в глобальную память
                out_matches[current_pos] = vec2<u32>(best_len, best_dist);
            }
        ";

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Heuristic Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_src)),
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Heuristic Pipeline"),
            layout: None,
            module: &shader,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        Some(Self { device, queue, compute_pipeline })
    }

    pub fn scan_chunk(&self, chunk: &[u8]) -> u32 {
        // Если блок не кратен 4, пока возвращаем заглушку
        if chunk.len() % 4 != 0 || chunk.is_empty() {
            return 0;
        }
        
        let words: &[u32] = bytemuck::cast_slice(chunk);
        
        let input_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Input Buffer"),
            contents: bytemuck::cast_slice(words),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let out_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: (words.len() * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bind_group_layout = self.compute_pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: out_buf.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (words.len() as u32 + 63) / 64;
            cpass.dispatch_workgroups(workgroups, 1, 1);
        }

        self.queue.submit(Some(encoder.finish()));
        // В реальном коде тут нужно мапить буфер и читать score, но для прототипа достаточно вызова GPU.
        // Эмулируем чтение (синхронизация требует async).
        
        1 // GPU выполнил работу
    }
}
