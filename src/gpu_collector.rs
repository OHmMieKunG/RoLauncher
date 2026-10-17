use crate::{model::ProcessIdentity, platform};
use std::{
    sync::mpsc::{self, Receiver, SyncSender},
    time::Instant,
};

struct Sample {
    started: chrono::DateTime<chrono::Utc>,
    at: Instant,
    values: Result<platform::GpuCounters, String>,
}

/// One bounded worker; even a stuck provider cannot queue threads or block window control.
pub(crate) struct GpuCollector {
    request: SyncSender<()>,
    response: Receiver<Sample>,
    latest: Option<Sample>,
}
impl GpuCollector {
    pub fn new() -> Self {
        let (request, requests) = mpsc::sync_channel(1);
        let (responses, response) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            while requests.recv().is_ok() {
                let started = chrono::Utc::now();
                let at = Instant::now();
                let values = platform::gpu_engines();
                if responses
                    .send(Sample {
                        started,
                        at,
                        values,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            request,
            response,
            latest: None,
        }
    }
    pub fn request(&mut self) {
        while let Ok(sample) = self.response.try_recv() {
            self.latest = Some(sample);
        }
        let _ = self.request.try_send(());
    }
    pub fn apply(&self, identity: &ProcessIdentity, result: &mut crate::resource::RuntimeControls) {
        let Some(sample) = self
            .latest
            .as_ref()
            .filter(|s| s.at.elapsed().as_secs() < 6 && eligible(s.started, identity))
        else {
            return;
        };
        let Ok(values) = &sample.values else {
            return;
        };
        result.gpu_sampled_at = Some(sample.started);
        if let Some((value, engine)) = values.engines.get(&identity.pid) {
            result.gpu_percent = Some(*value);
            result.gpu_engine = Some(engine.clone());
        }
        if let Some(memory) = values.memory.get(&identity.pid) {
            result.gpu_memory_by_adapter = memory.clone();
            // A single adapter is unambiguous; retain individual counters on multi-adapter systems.
            if memory.len() == 1 {
                result.dedicated_gpu_memory_bytes = Some(memory[0].dedicated_bytes);
            }
        }
    }
}
fn eligible(at: chrono::DateTime<chrono::Utc>, identity: &ProcessIdentity) -> bool {
    identity
        .creation_time
        .parse::<u64>()
        .ok()
        .is_some_and(|ticks| {
            let created_ms = ticks / 10_000;
            let sample_ms = at.timestamp_millis() + 11_644_473_600_000;
            sample_ms >= 0 && created_ms <= sample_ms as u64
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slow_stale_and_unsupported_samples_are_unavailable() {
        let (request, _requests) = mpsc::sync_channel(1);
        let (_responses, response) = mpsc::sync_channel(1);
        let mut collector = GpuCollector {
            request,
            response,
            latest: None,
        };
        let identity = ProcessIdentity {
            pid: 1,
            creation_time: "1".into(),
            tracker: "fixture".into(),
            generation: uuid::Uuid::new_v4(),
        };
        let mut result = crate::resource::RuntimeControls::default();
        collector.request();
        collector.request();
        collector.apply(&identity, &mut result);
        assert!(result.gpu_percent.is_none());
        collector.latest = Some(Sample {
            started: chrono::Utc::now(),
            at: Instant::now() - std::time::Duration::from_secs(7),
            values: Ok(platform::GpuCounters::default()),
        });
        collector.apply(&identity, &mut result);
        assert!(result.gpu_sampled_at.is_none());
        collector.latest.as_mut().unwrap().at = Instant::now();
        collector.apply(&identity, &mut result);
        assert!(result.dedicated_gpu_memory_bytes.is_none());
        assert!(!eligible(
            chrono::Utc::now(),
            &ProcessIdentity {
                creation_time: u64::MAX.to_string(),
                ..identity
            }
        ));
    }
}
