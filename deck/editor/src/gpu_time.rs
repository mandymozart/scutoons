//! How long the GPU worked on a frame (#456; owner 2026-11-09: "a meter of how much the gpu and cpu are used"): two timestamps
//! around the frame's commands, read back when the GPU is done; a frame is measured only when the last answer has come.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// the features it needs
pub const FEATURES: wgpu::Features = wgpu::Features::TIMESTAMP_QUERY.union(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS);

pub struct GpuTime {
    set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    read: wgpu::Buffer,
    period_ns: f32,
    /// a measurement is on its way
    busy: bool,
    /// written this frame
    armed: bool,
    mapped: Arc<AtomicBool>,
    pub last_ms: f32,
}

impl GpuTime {
    /// None where the adapter cannot time
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<GpuTime> {
        if !device.features().contains(FEATURES) {
            return None;
        }
        let set = device.create_query_set(&wgpu::QuerySetDescriptor { label: Some("gpu time"), ty: wgpu::QueryType::Timestamp, count: 2 });
        let resolve = device.create_buffer(&wgpu::BufferDescriptor { label: Some("gpu time"), size: 16, usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false });
        let read = device.create_buffer(&wgpu::BufferDescriptor { label: Some("gpu time"), size: 16, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        Some(GpuTime { set, resolve, read, period_ns: queue.get_timestamp_period(), busy: false, armed: false, mapped: Arc::new(AtomicBool::new(false)), last_ms: 0.0 })
    }

    /// at the start of the frame's commands
    pub fn begin(&mut self, enc: &mut wgpu::CommandEncoder) {
        self.armed = !self.busy;
        if self.armed {
            enc.write_timestamp(&self.set, 0);
        }
    }

    /// at their end
    pub fn end(&mut self, enc: &mut wgpu::CommandEncoder) {
        if self.armed {
            enc.write_timestamp(&self.set, 1);
            enc.resolve_query_set(&self.set, 0..2, &self.resolve, 0);
            enc.copy_buffer_to_buffer(&self.resolve, 0, &self.read, 0, 16);
        }
    }

    /// after the submit: ask for the answer; and take one that has come
    pub fn after_submit(&mut self, device: &wgpu::Device) {
        if self.armed {
            self.armed = false;
            self.busy = true;
            let flag = self.mapped.clone();
            self.read.slice(..).map_async(wgpu::MapMode::Read, move |r| flag.store(r.is_ok(), Ordering::Release));
        }
        let _ = device.poll(wgpu::PollType::Poll);
        if self.busy && self.mapped.swap(false, Ordering::AcqRel) {
            {
                let data = self.read.slice(..).get_mapped_range();
                let stamps: &[u64] = bytemuck::cast_slice(&data);
                self.last_ms = stamps[1].wrapping_sub(stamps[0]) as f32 * self.period_ns / 1.0e6;
            }
            self.read.unmap();
            self.busy = false;
        }
    }
}
